use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::limit::RequestBodyLimitLayer;

#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<rivaldsend_core::manager::TransferManager>,
    // Mémoire des PSK par transfert : identifiant -> PSK encodée en hexadécimal.
    pub psk_cache: Arc<tokio::sync::RwLock<HashMap<String, Vec<u8>>>>,
    // Mémoire des manifestes annoncés à la création, pour vérification à la fin.
    pub manifests: Arc<tokio::sync::RwLock<HashMap<String, rivaldsend_proto::TransferManifest>>>,
}

impl AppState {
    pub fn new(manager: Arc<rivaldsend_core::manager::TransferManager>) -> Self {
        Self {
            manager,
            psk_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            manifests: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}

/// Convertit une PSK brute en hexadécimal transportable dans un en-tête HTTP.
fn psk_en_hex(psk: [u8; 32]) -> Vec<u8> {
    let mut sortie = String::with_capacity(64);
    for octet in psk {
        sortie.push_str(&format!("{octet:02x}"));
    }
    sortie.into_bytes()
}

#[derive(Serialize)]
pub struct Health {
    pub status: String,
}

#[derive(Deserialize)]
pub struct CreateTransfer {
    pub manifest: rivaldsend_proto::TransferManifest,
    /// Code d'appairage affiché à l'expéditeur (jamais réutilisé tel quel).
    #[serde(default)]
    pub code: String,
}

/// Normalise un code saisi ou scanné : retire le tiret, passe en majuscules.
fn normaliser_code(code: &str) -> String {
    code.trim().replace('-', "").to_ascii_uppercase()
}

/// Vérifie le format du code d'appairage.
/// Accepte le format affiché par l'interface (6 caractères sans 0/O/1/I)
/// ainsi que l'ancien format à 6 chiffres, pour compatibilité.
fn verifier_code(code: &str) -> Result<(), (StatusCode, String)> {
    let normalise = normaliser_code(code);
    let valide = normalise.len() == 6
        && (normalise.bytes().all(|c| c.is_ascii_digit())
            || normalise
                .bytes()
                .all(|c| matches!(c, b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z' | b'2'..=b'9')));
    if !valide {
        return Err((
            StatusCode::UNAUTHORIZED,
            "code d'appairage invalide".into(),
        ));
    }
    Ok(())
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok".into() })
}

async fn negotiate(
    State(_s): State<AppState>,
    Json(req): Json<rivaldsend_proto::NegotiateRequest>,
) -> Result<Json<rivaldsend_proto::NegotiateResponse>, (StatusCode, String)> {
    rivaldsend_proto::negotiate(&req, "1.0.0", "2.0.0")
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn create_transfer(
    State(s): State<AppState>,
    Json(body): Json<CreateTransfer>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    rivaldsend_proto::validation::validate_manifest(&body.manifest)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Mémorise le manifeste et dérive la PSK du code d'appairage réel.
    // Le sel reprend l'identifiant du transfert, connu des deux pairs, de sorte
    // que l'expéditeur retrouve la même PSK sans qu'elle transite en clair.
    verifier_code(&body.code)?;
    let code_normalise = normaliser_code(&body.code);
    let identifiant = body.manifest.transfer_id.to_string();
    let sel = body.manifest.transfer_id.as_bytes().to_vec();
    let psk = psk_en_hex(rivaldsend_core::pairing::derive_psk(&code_normalise, &sel));

    {
        let mut cache = s.psk_cache.write().await;
        cache.insert(identifiant.clone(), psk);
    }
    {
        let mut manifs = s.manifests.write().await;
        manifs.insert(identifiant, body.manifest.clone());
    }

    Ok(Json(
        serde_json::json!({"transfer_id": body.manifest.transfer_id}),
    ))
}

/// Vérifie l'en-tête X-PSK pour un transfert donné.
async fn verifier_psk(
    s: &AppState,
    identifiant: &str,
    en_tetes: &HeaderMap,
) -> Result<(), (StatusCode, String)> {
    let recue = match en_tetes.get("X-PSK") {
        Some(v) => v,
        None => return Err((StatusCode::UNAUTHORIZED, "en-tête X-PSK manquant".into())),
    };
    let recue_texte = match recue.to_str() {
        Ok(v) => v,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "en-tête X-PSK illisible".into())),
    };
    let attendue = {
        let cache = s.psk_cache.read().await;
        cache.get(identifiant).cloned()
    };
    let Some(att) = attendue else {
        return Err((StatusCode::NOT_FOUND, "transfert inconnu".into()));
    };
    if !rivaldsend_core::pairing::constant_time_eq(recue_texte.as_bytes(), &att) {
        return Err((StatusCode::FORBIDDEN, "PSK invalide".into()));
    }
    Ok(())
}

async fn put_chunk(
    State(s): State<AppState>,
    Path((id, offset)): Path<(String, String)>,
    en_tetes: HeaderMap,
    corps: bytes::Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Le corps doit rester le dernier extracteur pour qu'axum construise la route.
    verifier_psk(&s, &id, &en_tetes).await?;

    if corps.len() > rivaldsend_proto::limits::MAX_CHUNK_SIZE {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "morceau trop gros".into()));
    }

    let uuid = id
        .parse::<uuid::Uuid>()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let offset = offset
        .parse::<u64>()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    if s.manager.status(&uuid).await.is_none() {
        return Err((StatusCode::NOT_FOUND, "transfert introuvable".into()));
    }

    let dossier = rivaldsend_core::manager::TransferManager::default_partial_dir(uuid);
    tokio::fs::create_dir_all(&dossier)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Écriture clairsemée au vrai décalage (reprise possible après coupure).
    let chemin_donnees = dossier.join("data.bin");
    rivaldsend_core::pipeline::writer::write_chunk(&chemin_donnees, offset, &corps)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let acquitte = offset + corps.len() as u64;
    let chemin_reprise = s.manager.resume_path(uuid);
    let mut etat = s
        .manager
        .resume(uuid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .unwrap_or_else(|| rivaldsend_core::resume::ResumeState::new(uuid));
    etat.add_ack(rivaldsend_proto::ChunkAck {
        offset,
        size: corps.len() as u32,
    });
    let _ = rivaldsend_core::resume::save(&chemin_reprise, &etat).await;

    Ok(Json(serde_json::json!({"ack_offset": acquitte})))
}

async fn resume_transfer(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let uuid = id
        .parse::<uuid::Uuid>()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let etat = s
        .manager
        .resume(uuid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(
        serde_json::to_value(etat).unwrap_or(serde_json::Value::Null),
    ))
}

async fn complete_transfer(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(_corps): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let uuid = id
        .parse::<uuid::Uuid>()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // 1. Relit le manifeste mémorisé à la création (pas celui renvoyé par le client).
    let manifeste = {
        let manifs = s.manifests.read().await;
        manifs.get(&id).cloned()
    };
    let Some(manifeste) = manifeste else {
        return Err((StatusCode::NOT_FOUND, "transfert inconnu".into()));
    };

    // 2. Vérifie l'intégrité puis livre vers le dossier de téléchargement.
    // Limite assumée : un seul fichier par transfert dans cette version.
    // Les transferts multi-fichiers sont refusés explicitement au lieu d'un faux succès.
    if manifeste.files.len() != 1 {
        return Err((
            StatusCode::BAD_REQUEST,
            "transferts multi-fichiers non pris en charge dans cette version".into(),
        ));
    }
    let entree = &manifeste.files[0];
    if !rivaldsend_proto::validation::is_safe_relative_path(&entree.relative_path) {
        return Err((StatusCode::BAD_REQUEST, "chemin relatif dangereux".into()));
    }
    let dossier_partiel = rivaldsend_core::manager::TransferManager::default_partial_dir(uuid);
    let donnees = dossier_partiel.join("data.bin");
    if !donnees.exists() {
        return Err((StatusCode::BAD_REQUEST, "données reçues manquantes".into()));
    }
    // Contrôle de taille avant le hachage (garde-fou peu coûteux).
    let taille = tokio::fs::metadata(&donnees)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .len();
    if taille != entree.size {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("taille inattendue : {taille} au lieu de {}", entree.size),
        ));
    }
    let empreinte = rivaldsend_core::pipeline::hasher::hash_file(&donnees)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if empreinte.to_lowercase() != entree.blake3.to_lowercase() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("empreinte invalide pour {}", entree.relative_path),
        ));
    }

    // 3. Livre le fichier et nettoie les mémoires.
    let dossier_base = s.manager.get_download_dir().await;
    let destination = dossier_base.join(&entree.relative_path);
    let livree = s
        .manager
        .finalize_transfer(uuid, destination)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    {
        let mut cache = s.psk_cache.write().await;
        cache.remove(&id);
    }
    {
        let mut manifs = s.manifests.write().await;
        manifs.remove(&id);
    }

    Ok(Json(
        serde_json::json!({"transfer_id": id, "status": "completed", "chemin": livree}),
    ))
}

async fn cancel_transfer(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let uuid = id
        .parse::<uuid::Uuid>()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    s.manager
        .cancel(uuid)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    {
        let mut cache = s.psk_cache.write().await;
        cache.remove(&id);
    }
    {
        let mut manifs = s.manifests.write().await;
        manifs.remove(&id);
    }
    Ok(Json(
        serde_json::json!({"transfer_id": uuid, "status":"cancelled"}),
    ))
}

async fn get_identity() -> Json<serde_json::Value> {
    let infos = crate::commands::get_device_info();
    Json(serde_json::json!({
        "name": infos.name,
        "ip": infos.ip,
        "port": infos.port,
        "fingerprint": infos.fingerprint,
        "fingerprint_short": infos.fingerprint_short,
        "platform": std::env::consts::OS,
    }))
}

pub fn router(etat: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/identity", get(get_identity))
        .route("/v1/negotiate", post(negotiate))
        .route("/v1/transfers", post(create_transfer))
        .route("/v1/transfers/:id/chunks/:offset", put(put_chunk))
        .route("/v1/transfers/:id/resume", post(resume_transfer))
        .route("/v1/transfers/:id/complete", post(complete_transfer))
        .route("/v1/transfers/:id", delete(cancel_transfer))
        .layer(
            ServiceBuilder::new()
                .layer(RequestBodyLimitLayer::new(32 * 1024 * 1024))
                .layer(tower::limit::ConcurrencyLimitLayer::new(50))
                .layer(tower_http::trace::TraceLayer::new_for_http()),
        )
        .with_state(etat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_code_pin_affiche_est_accepte() {
        // Format réel de PairingView : 6 caractères, tiret d'affichage toléré.
        assert!(verifier_code("AB3-9XZ").is_ok());
        assert!(verifier_code("ab39xz").is_ok());
        // Ancien format numérique conservé pour compatibilité.
        assert!(verifier_code("123456").is_ok());
        // Formats rejetés.
        assert!(verifier_code("").is_err());
        assert!(verifier_code("ABC").is_err());
        assert!(verifier_code("AB39XZO").is_err());
        assert!(verifier_code("AB39X0").is_err());
        assert!(verifier_code("AB39X1").is_err());
    }

    #[test]
    fn la_psk_hex_fait_64_caracteres() {
        let psk = psk_en_hex(rivaldsend_core::pairing::derive_psk("AB39XZ", b"sel-test"));
        assert_eq!(psk.len(), 64);
        assert!(psk.iter().all(|c| c.is_ascii_hexdigit()));
    }
}
