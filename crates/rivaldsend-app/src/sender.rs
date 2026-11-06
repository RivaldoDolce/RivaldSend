//! Travailleur d'envoi : pousse un transfert vers le pair en HTTPS.
//!
//! Protocole (un seul fichier par transfert dans cette version) :
//! 1. POST `/v1/transfers` avec `{ manifest, secret?, code?, sender_device_id,
//!    sender_name }` (le receveur en derive la PSK) ;
//! 2. GET `/v1/transfers/:id/resume` avec `X-PSK` : intervalles deja recus,
//!    pour reprendre apres une coupure sans tout renvoyer ;
//! 3. PUT `/v1/transfers/:id/chunks/:offset` avec `X-PSK`, par morceaux de
//!    4 Mio ;
//! 4. POST `/v1/transfers/:id/complete` avec le manifeste pour verification
//!    BLAKE3.
//!
//! Le certificat du receveur est auto-signé : comme pour `connect_by_ip`,
//! l'authentification passe par la PSK et l'empreinte d'appareil (QR),
//! le TLS ne servant qu'à chiffrer le transport (approche TOFU).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use rivaldsend_core::pairing;

/// Taille d'un morceau envoyé (bien sous la limite serveur de 16 Mio).
const TAILLE_MORCEAU: usize = 4 * 1024 * 1024;

/// Delai maximal d'une requete individuelle. Assez large pour un morceau de
/// 4 Mio sur un Wi-Fi lent, assez court pour ne pas rester bloque.
const DELAI_REQUETE: Duration = Duration::from_secs(60);

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

/// Parametres d'un envoi, regoupes pour rester lisibles.
pub struct ParametresEnvoi {
    pub app: AppHandle,
    pub gestionnaire: Arc<rivaldsend_core::manager::TransferManager>,
    pub identifiant: uuid::Uuid,
    pub chemin: PathBuf,
    pub ip: String,
    pub port: u16,
    pub code: String,
    pub secret: Option<String>,
}

/// Lance l'envoi en tâche de fond. Les erreurs sont rapportées via les
/// événements `transfer_progress` (statut `failed`) de l'interface.
pub fn spawn_envoi(params: ParametresEnvoi) {
    tauri::async_runtime::spawn(async move {
        match executer_envoi(&params).await {
            Ok(()) => {
                let _ = params.app.emit(
                    "transfer_completed",
                    serde_json::json!({ "transferId": params.identifiant, "direction": "sent" }),
                );
            }
            Err(e) => {
                emettre_progression(
                    &params.app,
                    &params.identifiant,
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

async fn executer_envoi(params: &ParametresEnvoi) -> Result<(), String> {
    use tokio::io::AsyncReadExt;

    let app = &params.app;
    let gestionnaire = &params.gestionnaire;
    let identifiant = &params.identifiant;
    let chemin = params.chemin.as_path();
    let ip = params.ip.as_str();
    let port = params.port;
    let code = params.code.as_str();
    let secret = params.secret.as_deref();

    let debut = Instant::now();
    let manifeste = construire_manifeste(*identifiant, chemin).await?;
    let total = manifeste.total_bytes;

    // Meme derivation que le receveur : secret de paire s'il existe, sinon
    // le code d'appairage. `derive_psk_transfert` choisit la meme branche
    // des deux cotes.
    let psk = pairing::hex(match secret.and_then(pairing::depuis_hex) {
        Some(octets) => pairing::derive_psk_bytes(&octets, identifiant.as_bytes()),
        None => pairing::derive_psk(code, identifiant.as_bytes()),
    });

    let client = reqwest::Client::builder()
        .timeout(DELAI_REQUETE)
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("client HTTP : {e}"))?;
    let base = format!("https://{ip}:{port}");

    // 1. Annonce du transfert, avec notre identite pour la reconciliation.
    let mut annonce = serde_json::json!({
        "manifest": manifeste,
        "sender_device_id": crate::commands::device_id(),
        "sender_name": crate::commands::get_device_info().name,
    });
    if let Some(s) = secret {
        annonce["secret"] = serde_json::json!(s);
    } else {
        annonce["code"] = serde_json::json!(pairing::normaliser(code));
    }
    let reponse = client
        .post(format!("{base}/v1/transfers"))
        .json(&annonce)
        .send()
        .await
        .map_err(|e| format!("annonce impossible : {e}"))?;
    if !reponse.status().is_success() {
        let statut = reponse.status();
        let corps = reponse.text().await.unwrap_or_default();
        return Err(format!("annonce refusée : HTTP {statut} {corps}"));
    }

    // 2. Intervalles deja recus cote receveur (reprise apres coupure).
    //
    // Avant, on ne lisait que le fichier local, qui n'existe que sur le
    // receveur : la reprise ne se declenchait donc jamais. On interroge
    // desormais le receveur, qui est la seule source fiable.
    let mut acquittes: HashSet<(u64, u32)> = HashSet::new();
    match client
        .post(format!("{base}/v1/transfers/{identifiant}/resume"))
        .header("X-PSK", psk.clone())
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {
            if let Ok(etat) = r.json::<serde_json::Value>().await {
                if let Some(tableau) = etat.get("validated_offsets").and_then(|v| v.as_array()) {
                    for ack in tableau {
                        if let (Some(offset), Some(taille)) = (
                            ack.get("offset").and_then(|v| v.as_u64()),
                            ack.get("size").and_then(|v| v.as_u64()),
                        ) {
                            if let Ok(taille) = u32::try_from(taille) {
                                acquittes.insert((offset, taille));
                            }
                        }
                    }
                }
            }
        }
        Ok(r) => {
            // Pas d'etat de reprise (transfert frais) : on part de zero.
            tracing::debug!("pas d'etat de reprise : HTTP {}", r.status());
        }
        Err(e) => {
            tracing::debug!("reprise indisponible ({e}), envoi complet");
        }
    }
    // Deja tout recu ? On passe directement a l'achevement.
    let deja_recus: u64 = acquittes.iter().map(|(_, t)| u64::from(*t)).sum();
    if total > 0 && deja_recus >= total {
        tracing::info!("transfert {identifiant} deja complet cote receveur, achevement direct");
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
            Some(rivaldsend_core::manager::TransferStatus::Failed(_)) | None => {
                return Err("transfert annulé".into())
            }
            Some(rivaldsend_core::manager::TransferStatus::Paused) => {
                tokio::time::sleep(Duration::from_millis(500)).await;
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
            emettre_progression(app, identifiant, decalage, total, debut, "running", None);
            continue;
        }
        let reponse = client
            .put(format!(
                "{base}/v1/transfers/{identifiant}/chunks/{decalage}"
            ))
            .header("X-PSK", psk.clone())
            .body(morceau.to_vec())
            .send()
            .await
            .map_err(|e| format!("envoi du morceau {decalage} impossible : {e}"))?;
        if !reponse.status().is_success() {
            let statut = reponse.status();
            let corps = reponse.text().await.unwrap_or_default();
            return Err(format!("morceau {decalage} refusé : HTTP {statut} {corps}"));
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
        let statut = reponse.status();
        let corps = reponse.text().await.unwrap_or_default();
        return Err(format!("achèvement refusé : HTTP {statut} {corps}"));
    }

    gestionnaire.marquer_termine(*identifiant).await;
    emettre_progression(app, identifiant, total, total, debut, "completed", None);
    Ok(())
}
