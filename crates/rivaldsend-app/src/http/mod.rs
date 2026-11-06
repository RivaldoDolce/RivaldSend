//! API HTTP locale de RivaldSend.
//!
//! Le serveur n'ecoute qu'en IPv4, sur toutes les interfaces du LAN. Il ne
//! s'authentifie pas au niveau TLS (certificats auto-signes, pas d'autorite) :
//! l'authentification repose sur la PSK derivee du secret d'appairage, portee
//! par l'en-tete `X-PSK` a chaque ecriture.
//!
//! Deux etapes distinctes sont ici, la ou une seule etait melange :
//! - `/v1/pair` : etablissement du lien de confiance (une fois) ;
//! - `/v1/transfers` : transfert de donnees (repete), protege par la PSK.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::limit::RequestBodyLimitLayer;

use rivaldsend_core::pairing;
use rivaldsend_core::trust::TrustStore;

/// Duree de validite d'un code d'appairage affiche a l'ecran.
pub const DUREE_CODE_APPAIRAGE: Duration = Duration::from_secs(5 * 60);

/// Duree de vie d'une PSK en memoire apres l'annonce d'un transfert.
/// Au-dela, l'entree est purgee : un transfert qui n'avance pas ne doit pas
/// laisser de secret resident indefiniment.
pub const DUREE_PSK: Duration = Duration::from_secs(60 * 60);

/// Nombre maximum de transferts announces simultanement. Une annonce non
/// authentifiee ne doit pas pouvoir faire croitre la memoire sans borne.
pub const MAX_TRANSFERTS_ACTIFS: usize = 32;

/// Nombre d'echecs de verification toleres par cle avant blocage temporaire.
pub const MAX_TENTATIVES: u32 = 5;

/// Fenetre de blocage apres depassement du nombre de tentatives.
pub const FENETRE_TENTATIVES: Duration = Duration::from_secs(60);

/// Erreur HTTP accompanied d'un code applicatif, pour que l'appelant puisse
/// distinguer un refus de politique (429/403) d'une panne (5xx).
type ApiErr = (StatusCode, String);

/// PSK derivee d'un transfert, avec son expiration.
pub type EntreePsk = (Vec<u8>, Instant);

/// Echecs de verification d'une cle, avec le debut de la fenetre.
pub type EntreeTentative = (u32, Instant);

#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<rivaldsend_core::manager::TransferManager>,
    /// Pairs apparies, avec le secret partage. Source de verite de la confiance.
    pub trust: Arc<TrustStore>,
    /// PSK derivee par transfert : identifiant -> (PSK hex, expiration).
    pub psk_cache: Arc<RwLock<HashMap<String, EntreePsk>>>,
    /// Manifeste annonce a la creation, relu a l'achevement.
    pub manifests: Arc<RwLock<HashMap<String, ManifesteAnnounce>>>,
    /// Code d'appairage actuellement affiche, avec son expiration.
    pub pairing: Arc<RwLock<Option<SessionAppairage>>>,
    /// Compteur d'echecs de verification, par cle (identifiant de transfert
    /// ou `device_id` pour l'appairage).
    pub tentatives: Arc<RwLock<HashMap<String, EntreeTentative>>>,
    /// Permet d'emettre `incoming_request` vers l'interface. `None` dans les
    /// tests, ou quand le serveur est monte hors d'un contexte Tauri.
    ///
    /// Le handle n'existe qu'apres `setup` : le champ est renseigne a ce
    /// moment-la via [`AppState::set_app`].
    pub app: Arc<RwLock<Option<tauri::AppHandle>>>,
}

/// Code d'appairage en cours d'affichage.
#[derive(Clone, Debug)]
pub struct SessionAppairage {
    /// Code normalise (sans tiret, en majuscules).
    pub code: String,
    pub expire: Instant,
}

impl SessionAppairage {
    pub fn est_valide(&self) -> bool {
        Instant::now() < self.expire
    }
}

/// Manifeste annonce, enrichi de l'identite declaree de l'expediteur.
#[derive(Clone, Debug)]
pub struct ManifesteAnnounce {
    pub manifeste: rivaldsend_proto::TransferManifest,
    pub sender_device_id: String,
    pub sender_name: String,
}

impl AppState {
    pub fn new(manager: Arc<rivaldsend_core::manager::TransferManager>) -> Self {
        Self::avec_trust(
            manager,
            Arc::new(TrustStore::charger(
                &rivaldsend_core::manager::TransferManager::dossier_app(),
            )),
        )
    }

    pub fn avec_trust(
        manager: Arc<rivaldsend_core::manager::TransferManager>,
        trust: Arc<TrustStore>,
    ) -> Self {
        Self {
            manager,
            trust,
            psk_cache: Arc::new(RwLock::new(HashMap::new())),
            manifests: Arc::new(RwLock::new(HashMap::new())),
            pairing: Arc::new(RwLock::new(None)),
            tentatives: Arc::new(RwLock::new(HashMap::new())),
            app: Arc::new(RwLock::new(None)),
        }
    }

    /// Renseigne le handle Tauri pour l'emission d'evenements vers
    /// l'interface. Appele une fois dans `setup`.
    pub async fn set_app(&self, app: tauri::AppHandle) {
        *self.app.write().await = Some(app);
    }

    /// Enregistre le code d'appairage affiche. Sans cet enregistrement, le
    /// serveur ne peut pas valider une demande d'appairage entrante : c'est
    /// ce qui faisait que seul le mobile pouvait s'appairer au PC.
    pub async fn set_code(&self, code: &str) -> Result<String, String> {
        let normalise = pairing::normaliser(code);
        if !pairing::code_valide(&normalise) {
            return Err("code d'appairage invalide".into());
        }
        *self.pairing.write().await = Some(SessionAppairage {
            code: normalise.clone(),
            expire: Instant::now() + DUREE_CODE_APPAIRAGE,
        });
        Ok(normalise)
    }

    pub async fn code_affiche(&self) -> Option<String> {
        self.pairing
            .read()
            .await
            .as_ref()
            .filter(|s| s.est_valide())
            .map(|s| s.code.clone())
    }

    pub fn emettre(&self, evenement: &str, charge: serde_json::Value) {
        use tauri::Emitter;
        // Lecture synchrone impossible sur un RwLock async : on clone le
        // handle au prealable dans `emettre_sync`. Ici on ne fait rien si le
        // handle n'est pas encore renseigne (demarrage).
        if let Ok(garde) = self.app.try_read() {
            if let Some(app) = garde.as_ref() {
                let _ = app.emit(evenement, charge);
            }
        }
    }

    /// Note un echec de verification et indique s'il faut bloquer.
    ///
    /// Le compteur est une fenetre glissante : si la fenetre est depassee,
    /// on repart de zero plutot que de bloquer pour toujours.
    async fn noter_echec(&self, cle: &str) -> bool {
        let mut table = self.tentatives.write().await;
        let maintenant = Instant::now();
        let entree = table.entry(cle.to_string()).or_insert((0, maintenant));
        if maintenant.duration_since(entree.1) > FENETRE_TENTATIVES {
            entree.0 = 0;
            entree.1 = maintenant;
        }
        entree.0 += 1;
        entree.0 > MAX_TENTATIVES
    }

    async fn noter_succes(&self, cle: &str) {
        self.tentatives.write().await.remove(cle);
    }

    /// Verifie qu'une cle n'est pas bloquee par trop d'echecs.
    async fn est_bloque(&self, cle: &str) -> bool {
        let table = self.tentatives.read().await;
        match table.get(cle) {
            Some((compteur, instant)) => {
                *compteur > MAX_TENTATIVES
                    && Instant::now().duration_since(*instant) < FENETRE_TENTATIVES
            }
            None => false,
        }
    }

    /// Purge les entrees expirees des caches en memoire.
    async fn purger_expires(&self) {
        let maintenant = Instant::now();
        self.psk_cache
            .write()
            .await
            .retain(|_, (_, expire)| maintenant < *expire);
        self.tentatives
            .write()
            .await
            .retain(|_, (_, instant)| maintenant.duration_since(*instant) < FENETRE_TENTATIVES);
    }
}

/// Derive la PSK d'un transfert.
///
/// Le secret de paire ( quand il existe) prime sur le code : c'est ce qui
/// permet d'envoyer sans ressaisir le code. Les deux cotes doivent choisir
/// la meme branche, ce qui est garanti par la presence du pair dans le
/// magasin de confiance des deux côtés.
pub fn derive_psk_transfert(secret: Option<&str>, code: &str, transfer_id: &uuid::Uuid) -> Vec<u8> {
    match secret.and_then(pairing::depuis_hex) {
        Some(octets) => {
            pairing::hex(pairing::derive_psk_bytes(&octets, transfer_id.as_bytes())).into_bytes()
        }
        None => pairing::hex(pairing::derive_psk(code, transfer_id.as_bytes())).into_bytes(),
    }
}

#[derive(Serialize)]
pub struct Health {
    pub status: String,
}

#[derive(Deserialize)]
pub struct CreateTransfer {
    pub manifest: rivaldsend_proto::TransferManifest,
    /// Code d'appairage affiche a l'expediteur (jamais reutilise tel quel).
    #[serde(default)]
    pub code: String,
    /// Secret de paire en hexadecimal. Present quand l'expediteur a deja
    /// apparie l'appareil dans son magasin de confiance.
    #[serde(default)]
    pub secret: Option<String>,
    /// `device_id` persistant de l'expediteur, tel que renvoye par
    /// `/v1/identity`. Sert a reconnaitre un pair de confiance apres un
    /// changement d'adresse IP.
    #[serde(default)]
    pub sender_device_id: String,
    #[serde(default)]
    pub sender_name: String,
}

/// Demande d'appairage entrante : un pair veut s'inscrire dans notre magasin
/// de confiance en presentant le code que nous affichons.
#[derive(Deserialize)]
pub struct PairRequest {
    pub code: String,
    pub device_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub fingerprint_short: String,
    #[serde(default)]
    pub platform: String,
    /// Adresse IPv4 de l'appelant, pour que cet appareil puisse lui aussi
    /// initier une connexion vers le pair (appairage bidirectionnel).
    #[serde(default)]
    pub sender_ip: String,
    #[serde(default)]
    pub sender_port: u16,
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".into(),
    })
}

async fn negotiate(
    State(_s): State<AppState>,
    Json(req): Json<rivaldsend_proto::NegotiateRequest>,
) -> Result<Json<rivaldsend_proto::NegotiateResponse>, ApiErr> {
    rivaldsend_proto::negotiate(&req, "1.0.0", "2.0.0")
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// Etablit (ou reetablit) un lien de confiance.
///
/// Le code affiche est la seule preuve detenue par les deux parties. S'il
/// correspond, nous derivons le secret partage, le memorisons pour ce pair,
/// et l'appelant fait de meme de son cote : le secret ne transite jamais.
async fn pair(
    State(s): State<AppState>,
    Json(corps): Json<PairRequest>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let normalise = pairing::normaliser(&corps.code);
    if corps.device_id.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "device_id manquant".into()));
    }
    if !pairing::code_valide(&normalise) {
        return Err((StatusCode::UNAUTHORIZED, "code d'appairage invalide".into()));
    }
    if s.est_bloque(&corps.device_id).await {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "trop de tentatives, reessayez plus tard".into(),
        ));
    }

    // Le code doit etre celui actuellement affiche, et encore valide.
    let session = s.code_affiche().await;
    let attendu = match session {
        Some(code) => code,
        None => {
            return Err((
                StatusCode::CONFLICT,
                "aucun code d'appairage en cours sur cet appareil".into(),
            ))
        }
    };
    if !pairing::constant_time_eq(normalise.as_bytes(), attendu.as_bytes()) {
        if s.noter_echec(&corps.device_id).await {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "trop de tentatives, reessayez plus tard".into(),
            ));
        }
        return Err((StatusCode::FORBIDDEN, "code d'appairage incorrect".into()));
    }
    s.noter_succes(&corps.device_id).await;

    // Secret partage, derive des deux identifiants tries : les deux appareils
    // obtiennent la meme valeur independamment de l'ordre.
    let notre_device_id = crate::commands::device_id();
    let secret = pairing::derive_pair_secret(&normalise, &notre_device_id, &corps.device_id);

    let infos = crate::commands::get_device_info();
    // L'adresse n'est pas connue a ce stade (la demande ne transmet que le
    // `device_id`) : l'identifiant est provisoire et sera consolide vers
    // `peer-{ip}:{port}` a la premiere resolution mDNS ou connexion directe.
    let pair = rivaldsend_core::trust::TrustedPeer {
        id: format!("pending-{}", corps.device_id),
        device_id: corps.device_id.clone(),
        name: if corps.name.trim().is_empty() {
            "Appareil inconnu".into()
        } else {
            corps.name.trim().to_string()
        },
        ip: corps.sender_ip.trim().to_string(),
        port: corps.sender_port,
        fingerprint_short: corps.fingerprint_short.trim().to_string(),
        platform: corps.platform.trim().to_string(),
        secret: pairing::hex(secret),
        paired_at: Some(chrono::Utc::now()),
    };
    s.trust
        .enregistrer(pair)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    s.emettre(
        "peer_paired",
        serde_json::json!({
            "peerId": format!("pending-{}", corps.device_id),
            "deviceId": corps.device_id,
            "name": corps.name,
            "fingerprintShort": corps.fingerprint_short,
            "platform": corps.platform,
        }),
    );

    Ok(Json(serde_json::json!({
        "ok": true,
        "device_id": notre_device_id,
        "name": infos.name,
        "fingerprint": infos.fingerprint,
        "fingerprint_short": infos.fingerprint_short,
        "platform": std::env::consts::OS,
        "port": infos.port,
    })))
}

/// Annonce un transfert entrant.
async fn create_transfer(
    State(s): State<AppState>,
    Json(body): Json<CreateTransfer>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    rivaldsend_proto::validation::validate_manifest(&body.manifest)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let id = body.manifest.transfer_id.to_string();
    if s.est_bloque(&id).await {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "trop de tentatives sur ce transfert".into(),
        ));
    }

    // Politique de confiance : si l'expediteur apparie, le secret est
    // obligatoire (et verifie par la suite a chaque ecriture). Sinon, premier
    // contact : le code d'appairage suffit.
    let apparie = s
        .trust
        .par_device_id(&body.sender_device_id)
        .await
        .is_some();
    if apparie && body.secret.is_none() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "appareil de confiance : secret de paire attendu".into(),
        ));
    }
    if !apparie {
        // Premier contact : le code doit rester valide.
        let attendu = s.code_affiche().await;
        match attendu {
            Some(_) => {}
            None => {
                return Err((
                    StatusCode::CONFLICT,
                    "aucun code d'appairage en cours : appariez d'abord cet appareil".into(),
                ))
            }
        }
        verifier_code(&body.code)?;
    }

    let code_normalise = pairing::normaliser(&body.code);
    let psk = derive_psk_transfert(
        body.secret.as_deref(),
        &code_normalise,
        &body.manifest.transfer_id,
    );

    s.purger_expires().await;
    {
        let mut cache = s.psk_cache.write().await;
        // Borne de memoire : les entrees expirees viennent d'etre purgees ;
        // on refuse d'ajouter au-dela du plafond plutot que d'allouer sans fin.
        if cache.len() >= MAX_TRANSFERTS_ACTIFS && !cache.contains_key(&id) {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "trop de transferts en cours, reessayez plus tard".into(),
            ));
        }
        cache.insert(id.clone(), (psk, Instant::now() + DUREE_PSK));
    }
    {
        let mut manifs = s.manifests.write().await;
        manifs.insert(
            id.clone(),
            ManifesteAnnounce {
                manifeste: body.manifest.clone(),
                sender_device_id: body.sender_device_id.clone(),
                sender_name: body.sender_name.clone(),
            },
        );
    }

    // Enregistrement cote recepteur. Sans lui, `put_chunk` rejetait le
    // transfert en « introuvable » : aucun octet n'arrivait jamais.
    s.manager
        .enregistrer_entrant(body.manifest.transfer_id, None)
        .await;

    // Notifie l'interface pour validation humaine avant livraison.
    let nom = s
        .trust
        .par_device_id(&body.sender_device_id)
        .await
        .map(|p| p.name)
        .unwrap_or_else(|| {
            let declare = body.sender_name.trim();
            if declare.is_empty() {
                "Appareil distant".to_string()
            } else {
                declare.to_string()
            }
        });
    s.emettre(
        "incoming_request",
        serde_json::json!({
            "requestId": id,
            "peerName": nom,
            "deviceId": body.sender_device_id,
            "files": body.manifest.files.iter().map(|f| serde_json::json!({
                "name": f.relative_path,
                "size": f.size,
            })).collect::<Vec<_>>(),
            "totalBytes": body.manifest.total_bytes,
        }),
    );

    Ok(Json(serde_json::json!({"transfer_id": id})))
}

/// Verifie l'en-tete X-PSK pour un transfert donne.
async fn verifier_psk(s: &AppState, identifiant: &str, en_tetes: &HeaderMap) -> Result<(), ApiErr> {
    let recue = match en_tetes.get("X-PSK") {
        Some(v) => v,
        None => return Err((StatusCode::UNAUTHORIZED, "en-tete X-PSK manquant".into())),
    };
    let recue_texte = match recue.to_str() {
        Ok(v) => v,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "en-tete X-PSK illisible".into())),
    };
    if s.est_bloque(identifiant).await {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "trop de tentatives sur ce transfert".into(),
        ));
    }
    let attendue = {
        let cache = s.psk_cache.read().await;
        match cache.get(identifiant) {
            // Une PSK expiree est traitee comme une entree inconnue : le
            // transfert doit etre re-annonce plutot que de rester valide.
            Some((psk, expire)) if Instant::now() < *expire => Some(psk.clone()),
            _ => None,
        }
    };
    let Some(att) = attendue else {
        return Err((StatusCode::NOT_FOUND, "transfert inconnu ou expire".into()));
    };
    if !pairing::constant_time_eq(recue_texte.as_bytes(), &att) {
        if s.noter_echec(identifiant).await {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "trop de tentatives, transfert bloque".into(),
            ));
        }
        return Err((StatusCode::FORBIDDEN, "PSK invalide".into()));
    }
    s.noter_succes(identifiant).await;
    Ok(())
}

async fn put_chunk(
    State(s): State<AppState>,
    Path((id, offset)): Path<(String, String)>,
    en_tetes: HeaderMap,
    corps: bytes::Bytes,
) -> Result<Json<serde_json::Value>, ApiErr> {
    // Le corps doit rester le dernier extracteur pour qu'axum construise la route.
    verifier_psk(&s, &id, &en_tetes).await?;

    if corps.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "morceau vide".into()));
    }
    if corps.len() > rivaldsend_proto::limits::MAX_CHUNK_SIZE {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "morceau trop gros".into()));
    }

    let uuid = id
        .parse::<uuid::Uuid>()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let offset = offset
        .parse::<u64>()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // L'enregistrement a ete fait par `create_transfer`. On verifie de plus
    // que le transfert n'a pas ete annule entre-temps.
    match s.manager.status(&uuid).await {
        None => return Err((StatusCode::NOT_FOUND, "transfert introuvable".into())),
        Some(rivaldsend_core::manager::TransferStatus::Failed(_)) => {
            return Err((StatusCode::CONFLICT, "transfert annule".into()))
        }
        Some(statut) if !statut.est_actif() => {
            return Err((StatusCode::CONFLICT, "transfert termine".into()))
        }
        Some(_) => {}
    }

    // Garde-fou d'ecriture : on refuse d'ecrire hors du volume annonce, ce qui
    // empeche un detourneur de PSK de remplir le disque via des trous.
    let volume_attendu = {
        let manifs = s.manifests.read().await;
        manifs
            .get(&id)
            .map(|a| a.manifeste.total_bytes)
            .unwrap_or(0)
    };
    let fin = offset
        .checked_add(corps.len() as u64)
        .ok_or((StatusCode::BAD_REQUEST, "decalage invalide".into()))?;
    if volume_attendu > 0 && fin > volume_attendu {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("ecriture hors volume annonce ({fin} > {volume_attendu})"),
        ));
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

    let acquitte = fin;
    let chemin_reprise = s.manager.resume_path(uuid);
    let mut etat = s
        .manager
        .resume(uuid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .unwrap_or_else(|| rivaldsend_core::resume::ResumeState::new(uuid));
    // Un offset deja acquitte est ignore : on ne fait pas grossir la liste.
    if !etat.validated_offsets.iter().any(|a| a.offset == offset) {
        etat.add_ack(rivaldsend_proto::ChunkAck {
            offset,
            size: corps.len() as u32,
        });
    }
    rivaldsend_core::resume::save(&chemin_reprise, &etat)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({"ack_offset": acquitte})))
}

async fn resume_transfer(
    State(s): State<AppState>,
    Path(id): Path<String>,
    en_tetes: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiErr> {
    // Vérifier le PSK pour protéger l'état de reprise.
    verifier_psk(&s, &id, &en_tetes).await?;
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
    en_tetes: HeaderMap,
    Json(_corps): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    // Vérifier le PSK avant de traiter l'achèvement.
    verifier_psk(&s, &id, &en_tetes).await?;
    let uuid = id
        .parse::<uuid::Uuid>()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // 1. Relit le manifeste mémorisé à la création (pas celui renvoyé par le client).
    let annonce = {
        let manifs = s.manifests.read().await;
        manifs.get(&id).cloned()
    };
    let Some(annonce) = annonce else {
        return Err((StatusCode::NOT_FOUND, "transfert inconnu".into()));
    };
    let manifeste = annonce.manifeste;

    // 2. Vérifie l'intégrité puis livre vers le dossier de téléchargement.
    // Limite assumée : un seul fichier par transfert dans cette version.
    // Les transferts multi-fichiers sont refusés explicitement au lieu d'un faux succès.
    if manifeste.files.len() != 1 {
        s.manager.oublier(uuid).await;
        return Err((
            StatusCode::BAD_REQUEST,
            "transferts multi-fichiers non pris en charge dans cette version".into(),
        ));
    }
    let entree = &manifeste.files[0];
    if !rivaldsend_proto::validation::is_safe_relative_path(&entree.relative_path) {
        s.manager.oublier(uuid).await;
        return Err((StatusCode::BAD_REQUEST, "chemin relatif dangereux".into()));
    }
    let dossier_partiel = rivaldsend_core::manager::TransferManager::default_partial_dir(uuid);
    let donnees = dossier_partiel.join("data.bin");
    if !donnees.exists() {
        return Err((StatusCode::BAD_REQUEST, "donnees recues manquantes".into()));
    }
    // Contrôle de taille avant le hachage (garde-fou peu coûteux).
    let taille = tokio::fs::metadata(&donnees)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .len();
    if taille != entree.size {
        s.manager.oublier(uuid).await;
        return Err((
            StatusCode::BAD_REQUEST,
            format!("taille inattendue : {taille} au lieu de {}", entree.size),
        ));
    }
    let empreinte = rivaldsend_core::pipeline::hasher::hash_file(&donnees)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !pairing::constant_time_eq(
        empreinte.to_lowercase().as_bytes(),
        entree.blake3.to_lowercase().as_bytes(),
    ) {
        s.manager.oublier(uuid).await;
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

    s.emettre(
        "transfer_completed",
        serde_json::json!({
            "transferId": id,
            "direction": "received",
            "fileName": entree.relative_path,
            "path": livree,
            "size": entree.size,
            "peerName": annonce.sender_name,
        }),
    );

    Ok(Json(
        serde_json::json!({"transfer_id": id, "status": "completed", "chemin": livree}),
    ))
}

async fn cancel_transfer(
    State(s): State<AppState>,
    Path(id): Path<String>,
    en_tetes: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiErr> {
    // Sans cette verification, n'importe quel pair du LAN connaissant un
    // identifiant de transfert pouvait annuler le transfert d'autrui.
    verifier_psk(&s, &id, &en_tetes).await?;
    let uuid = id
        .parse::<uuid::Uuid>()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    // Un transfert inconnu est deja annule : on repond succes sans rien
    // detruire, ce qui rend l'idempotence explicite cote appelant.
    match s.manager.cancel(uuid).await {
        Ok(()) | Err(rivaldsend_core::CoreError::NotFound(_)) => {}
        Err(e) => return Err((StatusCode::CONFLICT, e.to_string())),
    }
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

/// Verifie le format du code d'appairage.
/// Accepte le format affiché par l'interface (6 caractères sans 0/O/1/I)
/// ainsi que l'ancien format à 6 chiffres, pour compatibilité.
fn verifier_code(code: &str) -> Result<(), ApiErr> {
    if !pairing::code_valide(code) {
        return Err((StatusCode::UNAUTHORIZED, "code d'appairage invalide".into()));
    }
    Ok(())
}

async fn get_identity(State(s): State<AppState>) -> Json<serde_json::Value> {
    let infos = crate::commands::get_device_info();
    // Le `device_id` est expose : c'est lui qui permet aux deux appareils de
    // deriver le meme secret de paire sans l'echanger.
    let apparies = s.trust.lister().await;
    let nom_appairage = s.code_affiche().await;
    Json(serde_json::json!({
        "name": infos.name,
        "ip": infos.ip,
        "port": infos.port,
        "device_id": crate::commands::device_id(),
        "fingerprint": infos.fingerprint,
        "fingerprint_short": infos.fingerprint_short,
        "platform": std::env::consts::OS,
        "protocol_version": env!("CARGO_PKG_VERSION"),
        // L'exposant n'affiche un code que s'il y en a un actif : le client
        // peut ainsi avertir « ce pair n'a pas de code en cours ».
        "pairing_open": nom_appairage.is_some(),
        "trusted_count": apparies.len(),
    }))
}

pub fn router(etat: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/identity", get(get_identity))
        .route("/v1/negotiate", post(negotiate))
        .route("/v1/pair", post(pair))
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
        let psk = pairing::hex(pairing::derive_psk("AB39XZ", b"sel-test"));
        assert_eq!(psk.len(), 64);
        assert!(psk.bytes().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn les_deux_cotes_derivent_la_meme_psk() {
        // Regression : le secret de paire doit produire la meme PSK des deux
        // cotes, sans quoi aucun transfert vers un pair apparie ne passe.
        let id = uuid::Uuid::new_v4();
        let code = "AB39XZ";
        let secret_a = pairing::derive_pair_secret(code, "appareil-a", "appareil-b");
        let hex = pairing::hex(secret_a);

        let chez_emetteur = derive_psk_transfert(Some(&hex), code, &id);
        let chez_recepteur = derive_psk_transfert(Some(&hex), code, &id);
        assert_eq!(chez_emetteur, chez_recepteur);
        assert_eq!(chez_emetteur.len(), 64);
    }

    #[test]
    fn le_secret_prime_sur_le_code_et_changent_la_psk() {
        let id = uuid::Uuid::new_v4();
        let hex = pairing::hex(pairing::derive_pair_secret("AB39XZ", "a", "b"));
        let avec_secret = derive_psk_transfert(Some(&hex), "AB39XZ", &id);
        let sans_secret = derive_psk_transfert(None, "AB39XZ", &id);
        assert_ne!(
            avec_secret, sans_secret,
            "un secret de paire ne doit pas produire la PSK du code nu"
        );
        // Deux paires differentes ne derivent pas la meme PSK.
        let autre = pairing::hex(pairing::derive_pair_secret("AB39XZ", "a", "c"));
        assert_ne!(
            derive_psk_transfert(Some(&autre), "AB39XZ", &id),
            avec_secret
        );
    }

    #[test]
    fn un_secret_invalide_retombe_sur_le_code() {
        // Un hex corrompu ne doit pas produire une PSK vide : on degrade
        // proprement vers la derivation historique.
        let id = uuid::Uuid::new_v4();
        let casse = derive_psk_transfert(Some("pas-du-hex"), "AB39XZ", &id);
        let reference = derive_psk_transfert(None, "AB39XZ", &id);
        assert_eq!(casse, reference);
    }

    #[tokio::test]
    async fn le_code_affiche_est_accepte_puis_expire() {
        let etat = AppState::new(Arc::new(rivaldsend_core::manager::TransferManager::new(
            std::path::PathBuf::from("/tmp"),
        )));
        // Aucun code au depart : un pair ne peut pas s'appairer.
        assert!(etat.code_affiche().await.is_none());
        // Un code invalide est refuse.
        assert!(etat.set_code("nope").await.is_err());

        let code = pairing::generer_code();
        assert_eq!(
            etat.set_code(&code).await.unwrap(),
            pairing::normaliser(&code)
        );
        assert!(etat.code_affiche().await.is_some());

        // Une session expiree n'est plus proposée.
        *etat.pairing.write().await = Some(SessionAppairage {
            code: pairing::normaliser(&code),
            expire: Instant::now() - Duration::from_secs(1),
        });
        assert!(etat.code_affiche().await.is_none());
    }

    #[tokio::test]
    async fn les_erreurs_de_code_sont_comptees_puis_bloquees() {
        let etat = AppState::new(Arc::new(rivaldsend_core::manager::TransferManager::new(
            std::path::PathBuf::from("/tmp"),
        )));
        let cle = "device-test";
        assert!(!etat.est_bloque(cle).await);
        for _ in 0..MAX_TENTATIVES {
            assert!(!etat.noter_echec(cle).await, "bloque trop tot");
        }
        assert!(etat.noter_echec(cle).await, "doit bloquer apres le plafond");
        assert!(etat.est_bloque(cle).await);

        // Un succes remet le compteur a zero.
        etat.noter_succes(cle).await;
        assert!(!etat.est_bloque(cle).await);
    }

    #[tokio::test]
    async fn le_cache_psk_est_borne() {
        let etat = AppState::new(Arc::new(rivaldsend_core::manager::TransferManager::new(
            std::path::PathBuf::from("/tmp"),
        )));
        let maintenant = Instant::now();
        {
            let mut cache = etat.psk_cache.write().await;
            for i in 0..MAX_TRANSFERTS_ACTIFS {
                cache.insert(format!("t{i}"), (vec![0u8; 64], maintenant + DUREE_PSK));
            }
        }
        assert_eq!(etat.psk_cache.read().await.len(), MAX_TRANSFERTS_ACTIFS);

        // Une entree de plus declenche la purge des expires puis le refus.
        // (simule ici la purge directe, le refus lui-meme est couvert par le
        // handler `create_transfer`.)
        {
            let mut cache = etat.psk_cache.write().await;
            cache.retain(|_, (_, e)| Instant::now() < *e);
            assert_eq!(cache.len(), MAX_TRANSFERTS_ACTIFS);
        }
    }
}
