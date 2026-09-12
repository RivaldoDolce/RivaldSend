//! Travailleur d'envoi : pousse un transfert vers le pair en HTTPS.
//!
//! Protocole (un seul fichier par transfert dans cette version) :
//! 1. POST `/v1/transfers` avec `{ manifest, code }` (le receveur en dérive la PSK) ;
//! 2. PUT `/v1/transfers/:id/chunks/:offset` avec l'en-tête `X-PSK`, par morceaux
//!    de 4 Mio, en sautant les intervalles déjà acquittés (reprise) ;
//! 3. POST `/v1/transfers/:id/complete` avec le manifeste pour vérification BLAKE3.
//!
//! Le certificat du receveur est auto-signé : comme pour `connect_by_ip`,
//! l'authentification passe par la PSK et l'empreinte d'appareil (QR),
//! le TLS ne servant qu'à chiffrer le transport (approche TOFU).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

/// Taille d'un morceau envoyé (bien sous la limite serveur de 16 Mio).
const TAILLE_MORCEAU: usize = 4 * 1024 * 1024;

/// PSK brute -> hexadécimal transportable dans l'en-tête `X-PSK`.
fn psk_en_hex(psk: [u8; 32]) -> String {
    let mut sortie = String::with_capacity(64);
    for octet in psk {
        sortie.push_str(&format!("{octet:02x}"));
    }
    sortie
}

/// Construit le manifeste d'un fichier local.
pub async fn construire_manifeste(
    id: uuid::Uuid,
    chemin: &Path,
) -> Result<rivaldsend_proto::TransferManifest, String> {
    let nom = chemin
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "nom de fichier illisible".to_string())?
        .to_string();
    if !rivaldsend_proto::validation::is_safe_relative_path(&nom) {
        return Err(format!("nom de fichier refusé : {nom}"));
    }
    let metailles = tokio::fs::metadata(chemin)
        .await
        .map_err(|e| format!("fichier illisible : {e}"))?;
    if !metailles.is_file() {
        return Err("seuls les fichiers simples sont pris en charge".to_string());
    }
    let empreinte = rivaldsend_core::pipeline::hasher::hash_file(chemin)
        .await
        .map_err(|e| format!("hachage impossible : {e}"))?;
    Ok(rivaldsend_proto::TransferManifest {
        transfer_id: id,
        protocol_version: rivaldsend_proto::messages::SemVer {
            major: 2,
            minor: 0,
            patch: 0,
        },
        files: vec![rivaldsend_proto::manifest::ManifestEntry {
            relative_path: nom,
            size: metailles.len(),
            blake3: empreinte,
            mode: rivaldsend_proto::manifest::FileMode::File,
        }],
        total_bytes: metailles.len(),
        created_at: chrono::Utc::now(),
    })
}

fn emettre_progression(
    app: &AppHandle,
    id: &uuid::Uuid,
    faits: u64,
    total: u64,
    debut: Instant,
    statut: &str,
    erreur: Option<String>,
) {
    let secondes = debut.elapsed().as_secs().max(1);
    let vitesse = faits / secondes;
    let restant = if vitesse > 0 && total > faits {
        (total - faits) / vitesse
    } else {
        0
    };
    let _ = app.emit(
        "transfer_progress",
        crate::events::ProgressEvent {
            transfer_id: id.to_string(),
            bytes_done: faits,
            total_bytes: total,
            speed_bps: vitesse,
            eta_secs: restant,
            status: statut.into(),
            error: erreur,
        },
    );
}

/// Lance l'envoi en tâche de fond. Les erreurs sont rapportées via les
/// événements `transfer_progress` (statut `failed`) de l'interface.
pub fn spawn_envoi(
    app: AppHandle,
    gestionnaire: Arc<rivaldsend_core::manager::TransferManager>,
    identifiant: uuid::Uuid,
    chemin: PathBuf,
    ip: String,
    port: u16,
    code: String,
) {
    tauri::async_runtime::spawn(async move {
        match executer_envoi(&app, &gestionnaire, &identifiant, &chemin, &ip, port, &code).await
        {
            Ok(()) => {
                let _ = app.emit(
                    "transfer_completed",
                    serde_json::json!({ "transferId": identifiant, "direction": "sent" }),
                );
            }
            Err(e) => {
                emettre_progression(
                    &app,
                    &identifiant,
                    0,
                    0,
                    Instant::now(),
                    "failed",
                    Some(e),
                );
            }
        }
    });
}

async fn executer_envoi(
    app: &AppHandle,
    gestionnaire: &Arc<rivaldsend_core::manager::TransferManager>,
    identifiant: &uuid::Uuid,
    chemin: &Path,
    ip: &str,
    port: u16,
    code: &str,
) -> Result<(), String> {
    use tokio::io::AsyncReadExt;

    let debut = Instant::now();
    let manifeste = construire_manifeste(*identifiant, chemin).await?;
    let total = manifeste.total_bytes;

    // Même dérivation que le receveur : code normalisé + transfer_id en sel.
    let normalise = code.trim().replace('-', "").to_ascii_uppercase();
    let psk = psk_en_hex(rivaldsend_core::pairing::derive_psk(
        &normalise,
        identifiant.as_bytes(),
    ));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("client HTTP : {e}"))?;
    let base = format!("https://{ip}:{port}");

    // 1. Annonce du transfert.
    let reponse = client
        .post(format!("{base}/v1/transfers"))
        .json(&serde_json::json!({ "manifest": manifeste, "code": normalise }))
        .send()
        .await
        .map_err(|e| format!("annonce impossible : {e}"))?;
    if !reponse.status().is_success() {
        return Err(format!("annonce refusée : HTTP {}", reponse.status()));
    }

    // 2. Intervalles déjà acquittés (reprise après coupure).
    let mut acquittes: HashSet<(u64, u32)> = HashSet::new();
    if let Ok(peut_etre) = gestionnaire.resume(*identifiant).await {
        if let Some(etat) = peut_etre {
            acquittes.extend(etat.validated_offsets.iter().map(|a| (a.offset, a.size)));
        }
    }

    // 3. Envoi des morceaux.
    let mut fichier = tokio::fs::File::open(chemin)
        .await
        .map_err(|e| format!("ouverture impossible : {e}"))?;
    let mut decalage: u64 = 0;
    let mut tampon = vec![0u8; TAILLE_MORCEAU];
    loop {
        // Annulation / pause demandée depuis l'interface.
        match gestionnaire.status(identifiant).await {
            Some(rivaldsend_core::manager::TransferStatus::Failed(_))
            | None => return Err("transfert annulé".into()),
            Some(rivaldsend_core::manager::TransferStatus::Paused) => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }
            _ => {}
        }
        let lus = fichier
            .read(&mut tampon)
            .await
            .map_err(|e| format!("lecture impossible : {e}"))?;
        if lus == 0 {
            break;
        }
        let morceau = &tampon[..lus];
        if acquittes.contains(&(decalage, lus as u32)) {
            decalage += lus as u64;
            continue;
        }
        let reponse = client
            .put(format!("{base}/v1/transfers/{identifiant}/chunks/{decalage}"))
            .header("X-PSK", psk.clone())
            .body(morceau.to_vec())
            .send()
            .await
            .map_err(|e| format!("envoi du morceau {decalage} impossible : {e}"))?;
        if !reponse.status().is_success() {
            return Err(format!(
                "morceau {decalage} refusé : HTTP {}",
                reponse.status()
            ));
        }
        decalage += lus as u64;
        emettre_progression(app, identifiant, decalage, total, debut, "running", None);
    }

    // 4. Demande de vérification et de livraison côté receveur.
    let reponse = client
        .post(format!("{base}/v1/transfers/{identifiant}/complete"))
        .header("X-PSK", psk)
        .json(&serde_json::json!({ "manifest": manifeste }))
        .send()
        .await
        .map_err(|e| format!("achèvement impossible : {e}"))?;
    if !reponse.status().is_success() {
        return Err(format!("achèvement refusé : HTTP {}", reponse.status()));
    }

    gestionnaire.marquer_termine(*identifiant).await;
    emettre_progression(app, identifiant, total, total, debut, "completed", None);
    Ok(())
}
