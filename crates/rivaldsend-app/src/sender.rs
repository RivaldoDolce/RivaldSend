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
use tokio::io::AsyncReadExt;

use crate::commands::client_tls;
use crate::http::derive_psk_transfert;
use rivaldsend_core::folders;
use rivaldsend_core::pairing;

/// Taille d'un morceau envoyé (bien sous la limite serveur de 16 Mio).
const TAILLE_MORCEAU: usize = 4 * 1024 * 1024;

/// Delai maximal d'une requete individuelle. Assez large pour un morceau de
/// 4 Mio sur un Wi-Fi lent, assez court pour ne pas rester bloque.
const DELAI_REQUETE: Duration = Duration::from_secs(60);

/// Construit le manifeste d'un fichier ou d'un dossier local.
/// Si `chemin` est un dossier, toutes les entrées sont collectées récursivement
/// et le manifeste contient une entrée par fichier.
pub async fn construire_manifeste(
    id: uuid::Uuid,
    chemin: &Path,
) -> Result<rivaldsend_proto::TransferManifest, String> {
    let metailles = tokio::fs::metadata(chemin)
        .await
        .map_err(|e| format!("fichier illisible : {e}"))?;

    let files = if metailles.is_dir() {
        folders::collect_manifest_entries(chemin, false)
            .map_err(|e| format!("collecte du dossier impossible : {e}"))?
    } else if metailles.is_file() {
        let nom = chemin
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "nom de fichier illisible".to_string())?
            .to_string();
        if !rivaldsend_proto::validation::is_safe_relative_path(&nom) {
            return Err(format!("nom de fichier refusé : {nom}"));
        }
        let empreinte = rivaldsend_core::pipeline::hasher::hash_file(chemin)
            .await
            .map_err(|e| format!("hachage impossible : {e}"))?;
        vec![rivaldsend_proto::manifest::ManifestEntry {
            relative_path: nom,
            size: metailles.len(),
            blake3: empreinte,
            mode: rivaldsend_proto::manifest::FileMode::File,
        }]
    } else {
        return Err("type de fichier non pris en charge".to_string());
    };

    let total_bytes: u64 = files.iter().map(|f| f.size).sum();

    Ok(rivaldsend_proto::TransferManifest {
        transfer_id: id,
        protocol_version: rivaldsend_proto::messages::SemVer {
            major: 2,
            minor: 0,
            patch: 0,
        },
        files,
        total_bytes,
        created_at: chrono::Utc::now(),
    })
}

/// Collecte récursivement tous les fichiers sous un dossier.
fn collecter_fichiers_dossier(racine: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let _ = collecter_recursif(racine, &mut out);
    out
}

fn collecter_recursif(current: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if current.is_dir() {
        for entree in std::fs::read_dir(current)? {
            let entree = entree?;
            let path = entree.path();
            if path.is_dir() {
                let _ = collecter_recursif(&path, out);
            } else if path.is_file() {
                out.push(path);
            }
        }
    }
    Ok(())
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

/// Envoie un seul fichier vers le receveur (annonce, morceaux, achèvement).
/// Cette fonction est appelée soit pour un fichier isolé, soit pour chaque
/// fichier d'un dossier.
#[allow(clippy::too_many_arguments)]
async fn envoyer_fichier_unique(
    app: &AppHandle,
    gestionnaire: &Arc<rivaldsend_core::manager::TransferManager>,
    identifiant: uuid::Uuid,
    chemin: &Path,
    ip: &str,
    port: u16,
    code: &str,
    secret: Option<&str>,
) -> Result<(), String> {
    let base = format!("https://{ip}:{port}");
    let client = client_tls(DELAI_REQUETE.as_secs())?;

    // 1. Construire le manifeste de ce fichier.
    let manifeste = construire_manifeste(identifiant, chemin).await?;
    let total: u64 = manifeste.total_bytes;

    // 2. Annonce du transfert vers le receveur (POST /v1/transfers).
    let psk = derive_psk_transfert(secret, code, &identifiant);
    let manche = serde_json::json!({
        "manifest": manifeste,
        "code": code,
        "secret": secret.and_then(pairing::depuis_hex),
        "sender_device_id": crate::commands::device_id(),
        "sender_name": crate::commands::get_device_info().name,
    });
    let reponse = client
        .post(format!("{base}/v1/transfers"))
        .json(&manche)
        .send()
        .await
        .map_err(|e| format!("annonce du transfert impossible : {e}"))?;
    if !reponse.status().is_success() {
        let statut = reponse.status();
        let corps = reponse.text().await.unwrap_or_default();
        return Err(format!("annonce refusée : HTTP {statut} {corps}"));
    }
    let corps: serde_json::Value = reponse.json().await.map_err(|e| e.to_string())?;
    let identifiant_serveur = corps["transfer_id"].as_str().unwrap_or("");
    if !identifiant_serveur.is_empty() && identifiant_serveur != identifiant.to_string() {
        return Err("l'identifiant de transfert retourné ne correspond pas".into());
    }

    // 3. Reprise : demander au receveur quels morceaux il a déjà reçus.
    let mut acquittes: HashSet<(u64, u32)> = HashSet::new();
    {
        let r = client
            .post(format!("{base}/v1/transfers/{identifiant}/resume"))
            .header("X-PSK", psk.clone())
            .send()
            .await;
        match r {
            Ok(r) if r.status().is_success() => {
                let etat: Option<rivaldsend_core::resume::ResumeState> =
                    r.json().await.ok().flatten();
                if let Some(etat) = etat {
                    for ack in etat.validated_offsets {
                        acquittes.insert((ack.offset, ack.size));
                    }
                }
            }
            Ok(r) => {
                tracing::debug!("pas d'etat de reprise : HTTP {}", r.status());
            }
            Err(e) => {
                tracing::debug!("reprise indisponible ({e}), envoi complet");
            }
        }
    }
    let debut = Instant::now();
    let deja_recus: u64 = acquittes.iter().map(|(_, t)| u64::from(*t)).sum();
    if total > 0 && deja_recus >= total {
        tracing::info!("transfert {identifiant} deja complet cote receveur, achevement direct");
    }

    // 4. Envoi des morceaux.
    let mut fichier = tokio::fs::File::open(chemin)
        .await
        .map_err(|e| format!("ouverture impossible : {e}"))?;
    let mut decalage: u64 = 0;
    let mut tampon = vec![0u8; TAILLE_MORCEAU];
    loop {
        match gestionnaire.status(&identifiant).await {
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
            emettre_progression(app, &identifiant, decalage, total, debut, "running", None);
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
        emettre_progression(app, &identifiant, decalage, total, debut, "running", None);
    }

    // 5. Demande de vérification et de livraison côté receveur.
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

    gestionnaire.marquer_termine(identifiant).await;
    emettre_progression(app, &identifiant, total, total, debut, "completed", None);
    Ok(())
}

async fn executer_envoi(params: &ParametresEnvoi) -> Result<(), String> {
    let metailles = tokio::fs::metadata(&params.chemin)
        .await
        .map_err(|e| format!("metadata impossible : {e}"))?;

    if metailles.is_dir() {
        // Dossier : envoyer chaque fichier comme un transfert séparé.
        let fichiers = collecter_fichiers_dossier(&params.chemin);
        if fichiers.is_empty() {
            return Err("dossier vide".into());
        }
        for fichier in fichiers {
            let nouvel_id = uuid::Uuid::new_v4();
            params
                .gestionnaire
                .set_pairing_code(nouvel_id, params.code.clone())
                .await;
            params.gestionnaire.enqueue(fichier.clone()).await;
            params
                .gestionnaire
                .start_transfer(nouvel_id)
                .await
                .map_err(|e| e.to_string())?;
            if let Err(e) = envoyer_fichier_unique(
                &params.app,
                &params.gestionnaire,
                nouvel_id,
                &fichier,
                &params.ip,
                params.port,
                &params.code,
                params.secret.as_deref(),
            )
            .await
            {
                tracing::warn!("échec envoi {:?}: {}", fichier.display(), e);
            }
        }
        return Ok(());
    }

    // Fichier unique : déléguer à envoyer_fichier_unique.
    envoyer_fichier_unique(
        &params.app,
        &params.gestionnaire,
        params.identifiant,
        &params.chemin,
        &params.ip,
        params.port,
        &params.code,
        params.secret.as_deref(),
    )
    .await
}
