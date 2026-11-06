use serde::Serialize;
use std::sync::Arc;
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

// Empreinte stable persistée
static DEVICE_FINGERPRINT: OnceLock<String> = OnceLock::new();
// Identifiant stable de l'appareil, charge une fois au demarrage.
static APPAREIL_ID: OnceLock<String> = OnceLock::new();

/// Initialise l'identifiant d'appareil au demarrage. Doit etre appele avant
/// tout usage de [`device_id`], typiquement dans `run_tauri`.
pub fn initialiser_device_id() -> String {
    APPAREIL_ID
        .get_or_init(|| {
            rivaldsend_core::device_id::DeviceId::charger_ou_creer(
                &rivaldsend_core::manager::TransferManager::dossier_app(),
            )
            .as_str()
            .to_string()
        })
        .clone()
}

/// Identifiant stable de cet appareil, echange via `/v1/identity` pour
/// deriver le secret de paire. Charge au demarrage, ne change jamais.
pub fn device_id() -> String {
    APPAREIL_ID
        .get()
        .cloned()
        .unwrap_or_else(initialiser_device_id)
}

fn get_or_create_fingerprint() -> String {
    DEVICE_FINGERPRINT
        .get_or_init(|| {
            let path = dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                .join("rivaldsend")
                .join("fingerprint.txt");

            if let Ok(content) = std::fs::read_to_string(&path) {
                let trimmed = content.trim().to_string();
                if !trimmed.is_empty() {
                    return trimmed;
                }
            }

            // Générer une nouvelle empreinte (16 octets = 32 caractères hex)
            let fingerprint = format!("{:x}", Uuid::new_v4().as_u128());
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&path, &fingerprint);
            fingerprint
        })
        .clone()
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfoResponse {
    pub name: String,
    pub ip: String,
    pub fingerprint: String,
    pub fingerprint_short: String,
    pub port: u16,
}
/// Choisit l'IP à annoncer : de préférence une IPv4 de réseau local privé
/// (celle que le téléphone du même Wi-Fi pourra joindre), sinon la première
/// IPv4 disponible, sinon le loopback.
fn choisir_ip_locale(interfaces: &[(String, std::net::IpAddr)]) -> String {
    use std::net::IpAddr;
    let mut premiere_ipv4: Option<String> = None;
    for (_, ip) in interfaces {
        if let IpAddr::V4(v4) = ip {
            if v4.is_loopback() || v4.is_link_local() {
                continue;
            }
            let texte = v4.to_string();
            if premiere_ipv4.is_none() {
                premiere_ipv4 = Some(texte.clone());
            }
            let o = v4.octets();
            let lan_prive = o[0] == 10
                || (o[0] == 172 && (16..=31).contains(&o[1]))
                || (o[0] == 192 && o[1] == 168);
            if lan_prive {
                return texte;
            }
        }
    }
    premiere_ipv4.unwrap_or_else(|| "127.0.0.1".into())
}

#[tauri::command]
pub fn get_device_info() -> DeviceInfoResponse {
    let interfaces = rivaldsend_core::discovery::list_interfaces();
    let ip = choisir_ip_locale(&interfaces);

    let fingerprint = get_or_create_fingerprint();
    let fingerprint_short = fingerprint.chars().take(8).collect();

    let name = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "RivaldSend".into());

    DeviceInfoResponse {
        name,
        ip,
        fingerprint: fingerprint.clone(),
        fingerprint_short,
        port: 53317,
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTransferResponse {
    pub transfer_id: String,
}
/// Construit le client HTTPS tolérant aux certificats auto-signés.
/// Contexte : réseau local sans autorité de certification. Le chiffrement TLS
/// bloque l'écoute passive ; l'authentification passe par la PSK d'appairage
/// et l'empreinte d'appareil échangée hors bande (QR), pas par le certificat.
pub(crate) fn client_tls(duree_s: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(duree_s))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())
}

/// Vérifie le format d'un code d'appairage (6 caractères affichés, tiret toléré).
///
/// Delegue a [`rivaldsend_core::pairing`], point d'entree unique, pour que le
/// format accepte ici soit exactement celui que le serveur valide.
fn code_appairage_valide(code: &str) -> bool {
    rivaldsend_core::pairing::code_valide(code)
}

/// Adresse de connexion d'un pair : le cache de decouverte d'abord, puis le
/// magasin de confiance si l'adresse a change depuis.
///
/// C'est ce qui manquait cote PC : un pair connu uniquement via le QR n'etait
/// dans aucun cache, et `start_transfer` echouait sur « pair introuvable ».
async fn adresse_du_pair(
    cache: &State<'_, crate::PeerCacheState>,
    trust: &rivaldsend_core::trust::TrustStore,
    peer_id: &str,
) -> Option<(String, u16)> {
    // 1. Cache de decouverte en direct.
    {
        let garde = cache.0.lock().await;
        if let Some((_, ev)) = garde.values().find(|(_, ev)| ev.id == peer_id) {
            return Some((ev.ip.clone(), ev.port));
        }
    }
    // 2. Magasin de confiance : la cle peut etre l'identifiant `peer-ip:port`
    // ou un identifiant provisoire `pending-{device_id}`.
    if let Some(pair) = trust.par_id(peer_id).await {
        if !pair.ip.is_empty() && pair.port != 0 {
            return Some((pair.ip, pair.port));
        }
    }
    if let Some(suffixe) = peer_id.strip_prefix("pending-") {
        if let Some(pair) = trust.par_device_id(suffixe).await {
            if !pair.ip.is_empty() && pair.port != 0 {
                return Some((pair.ip, pair.port));
            }
        }
    }
    None
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn start_transfer(
    app: AppHandle,
    manager: State<'_, Arc<rivaldsend_core::manager::TransferManager>>,
    cache: State<'_, crate::PeerCacheState>,
    trust: State<'_, Arc<rivaldsend_core::trust::TrustStore>>,
    peerId: String,
    filePaths: Vec<String>,
    code: Option<String>,
) -> Result<StartTransferResponse, String> {
    // Vérifier que tous les chemins existent.
    let chemins: Vec<std::path::PathBuf> = filePaths.iter().map(std::path::PathBuf::from).collect();
    for c in &chemins {
        if !c.exists() {
            return Err(format!("fichier introuvable: {}", c.display()));
        }
    }

    // Secret de paire si le pair est apparie : aucun code a ressaisir.
    // Sinon le code reste obligatoire, le receveur en derive la PSK.
    let mut secret: Option<String> = trust.par_id(&peerId).await.map(|p| p.secret.clone());
    if secret.is_none() {
        if let Some(suffixe) = peerId.strip_prefix("pending-") {
            secret = trust.par_device_id(suffixe).await.map(|p| p.secret.clone());
        }
    }
    if secret.is_none() {
        let valide = code.as_deref().is_some_and(code_appairage_valide);
        if !valide {
            return Err("code d'appairage requis (ou appariez d'abord l'appareil)".into());
        }
    }
    let code = code.unwrap_or_default();

    // Retrouver l'adresse du pair : cache de decouverte, puis magasin.
    let (ip, port) = adresse_du_pair(&cache, &trust, &peerId)
        .await
        .ok_or_else(|| "pair introuvable, reconnectez-vous".to_string())?;

    // Calculer la taille totale réelle (somme de tous les fichiers).
    let _total_bytes: u64 = chemins
        .iter()
        .map(|c| std::fs::metadata(c).map(|m| m.len()).unwrap_or(0))
        .sum();

    let mut premier_id: Option<Uuid> = None;

    // Lancer un transfert par fichier (chaque fichier a son propre transfer_id
    // et son propre travailleur d'envoi HTTPS). Les dossiers sont recus tels
    // quels : le sender construira un manifeste multi-fichiers si le chemin
    // est un repertoire.
    for chemin in &chemins {
        let transfer_id = Uuid::new_v4();
        if premier_id.is_none() {
            premier_id = Some(transfer_id);
        }

        // Memoriser le code et la cible pour le travailleur d'envoi HTTPS.
        manager.set_pairing_code(transfer_id, code.clone()).await;

        // Enfiler le fichier pour suivi local.
        let _ = manager.enqueue(chemin.clone()).await;

        // Associer le peer cible.
        manager.set_target_peer(transfer_id, peerId.clone()).await;

        // Demarrer le transfert (acquiert un slot de concurrence).
        manager
            .start_transfer(transfer_id)
            .await
            .map_err(|e| e.to_string())?;

        // Lancer le travailleur d'envoi HTTPS (annonce + morceaux + achèvement).
        crate::sender::spawn_envoi(crate::sender::ParametresEnvoi {
            app: app.clone(),
            gestionnaire: manager.inner().clone(),
            identifiant: transfer_id,
            chemin: chemin.clone(),
            ip: ip.clone(),
            port,
            code: code.clone(),
            secret: secret.clone(),
        });

        // Emettre la progression pour ce transfert.
        let taille = std::fs::metadata(chemin).map(|m| m.len()).unwrap_or(0);
        let _ = app.emit(
            "transfer_progress",
            crate::events::ProgressEvent {
                transfer_id: transfer_id.to_string(),
                bytes_done: 0,
                total_bytes: taille,
                speed_bps: 0,
                eta_secs: 0,
                status: "running".into(),
                error: None,
            },
        );
    }

    let transfer_id = premier_id.expect("au moins un fichier");
    Ok(StartTransferResponse {
        transfer_id: transfer_id.to_string(),
    })
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn cancel_transfer(
    app: AppHandle,
    manager: State<'_, Arc<rivaldsend_core::manager::TransferManager>>,
    transferId: String,
) -> Result<(), String> {
    let id = transferId.parse::<Uuid>().map_err(|e| e.to_string())?;
    manager.cancel(id).await.map_err(|e| e.to_string())?;
    let _ = app.emit(
        "transfer_progress",
        crate::events::ProgressEvent {
            transfer_id: transferId,
            bytes_done: 0,
            total_bytes: 0,
            speed_bps: 0,
            eta_secs: 0,
            status: "cancelled".into(),
            error: None,
        },
    );
    Ok(())
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn pause_transfer(
    manager: State<'_, Arc<rivaldsend_core::manager::TransferManager>>,
    app: AppHandle,
    transferId: String,
) -> Result<(), String> {
    let id = transferId.parse::<Uuid>().map_err(|e| e.to_string())?;
    manager
        .pause_transfer(id)
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit(
        "transfer_progress",
        crate::events::ProgressEvent {
            transfer_id: transferId,
            bytes_done: 0,
            total_bytes: 0,
            speed_bps: 0,
            eta_secs: 0,
            status: "paused".into(),
            error: None,
        },
    );
    Ok(())
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn resume_transfer(
    manager: State<'_, Arc<rivaldsend_core::manager::TransferManager>>,
    app: AppHandle,
    transferId: String,
) -> Result<(), String> {
    let id = transferId.parse::<Uuid>().map_err(|e| e.to_string())?;
    manager
        .resume_transfer(id)
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit(
        "transfer_progress",
        crate::events::ProgressEvent {
            transfer_id: transferId,
            bytes_done: 0,
            total_bytes: 0,
            speed_bps: 0,
            eta_secs: 0,
            status: "running".into(),
            error: None,
        },
    );
    Ok(())
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn accept_incoming(
    app: AppHandle,
    incoming: State<'_, crate::IncomingState>,
    requestId: String,
    targetDir: String,
) -> Result<(), String> {
    let dir = if targetDir == "~" || targetDir.starts_with("~/") {
        let home = dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        if targetDir == "~" {
            home
        } else {
            home.join(targetDir.trim_start_matches("~/"))
        }
    } else {
        std::path::PathBuf::from(&targetDir)
    };
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    // Dossier de staging pour les chunks de cette demande
    let staging = dir.join(".rivaldsend-incoming").join(&requestId);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    // Mémoriser la décision pour le pipeline de réception
    incoming.0.lock().await.insert(
        requestId.clone(),
        crate::IncomingDecision {
            target_dir: Some(dir.clone()),
            decided_at: std::time::SystemTime::now(),
        },
    );
    let _ = app.emit(
        "incoming_accepted",
        serde_json::json!({"requestId": requestId, "targetDir": dir, "stagingDir": staging}),
    );
    Ok(())
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn reject_incoming(
    app: AppHandle,
    incoming: State<'_, crate::IncomingState>,
    requestId: String,
) -> Result<(), String> {
    // Mémoriser le refus : les chunks tardifs de cette demande seront rejetés
    incoming.0.lock().await.insert(
        requestId.clone(),
        crate::IncomingDecision {
            target_dir: None,
            decided_at: std::time::SystemTime::now(),
        },
    );
    let _ = app.emit(
        "incoming_rejected",
        serde_json::json!({"requestId": requestId}),
    );
    Ok(())
}
#[tauri::command]
pub async fn open_file_dialog(app: AppHandle) -> Result<Option<Vec<String>>, String> {
    use tauri_plugin_dialog::DialogExt;
    // Le dialogue natif est bloquant : l'exécuter hors du runtime async.
    let picked =
        tauri::async_runtime::spawn_blocking(move || app.dialog().file().blocking_pick_files())
            .await
            .map_err(|e| e.to_string())?;
    Ok(picked.map(|paths| {
        paths
            .into_iter()
            .filter_map(|p| p.into_path().ok())
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }))
}
#[tauri::command]
pub async fn list_history(
    history_path: Option<String>,
) -> Result<Vec<rivaldsend_core::history::HistoryEntry>, String> {
    let p = if let Some(path) = history_path {
        if path.starts_with("~/") {
            let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
            home.join(path.trim_start_matches("~/"))
                .to_string_lossy()
                .to_string()
        } else {
            path
        }
    } else {
        dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("rivaldsend/history.jsonl")
            .to_string_lossy()
            .to_string()
    };
    rivaldsend_core::history::load_all(std::path::Path::new(&p))
        .await
        .map_err(|e| e.to_string())
}

#[allow(non_snake_case)]
#[tauri::command]
pub fn generate_pairing_qr(
    ip: String,
    port: u16,
    code: String,
    fingerprintShort: String,
) -> Result<String, String> {
    use qrcode::QrCode;
    let payload = format!(
        "rivaldsend://{}:{}?code={}&fp={}",
        ip, port, code, fingerprintShort
    );
    let qr = QrCode::new(payload.as_bytes()).map_err(|e| e.to_string())?;
    let svg = qr
        .render::<qrcode::render::svg::Color<'_>>()
        .min_dimensions(200, 200)
        .build();
    Ok(svg)
}

#[tauri::command]
pub fn check_firewall() -> Result<String, String> {
    let profile =
        rivaldsend_core::firewall::detect_windows_firewall().map_err(|e| e.to_string())?;
    if rivaldsend_core::firewall::should_block_server(&profile) {
        Err("Réseau public détecté — serveur bloqué. Passez en réseau privé.".into())
    } else {
        Ok(format!("{:?}", profile))
    }
}

#[tauri::command]
pub fn list_network_interfaces() -> Vec<(String, String)> {
    rivaldsend_core::discovery::list_interfaces()
        .into_iter()
        .map(|(n, ip)| (n, ip.to_string()))
        .collect()
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn ping_peer(ip: String, port: u16) -> Result<u32, String> {
    let start = std::time::Instant::now();
    let addr = format!("{ip}:{port}");
    tokio::time::timeout(
        std::time::Duration::from_millis(800),
        tokio::net::TcpStream::connect(addr),
    )
    .await
    .map_err(|_| "timeout".to_string())?
    .map_err(|e| e.to_string())?;
    Ok(start.elapsed().as_millis() as u32)
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn connect_by_ip(
    app: AppHandle,
    cache: State<'_, crate::PeerCacheState>,
    trust: State<'_, Arc<rivaldsend_core::trust::TrustStore>>,
    ip: String,
    port: u16,
) -> Result<crate::events::PeerDiscoveredEvent, String> {
    let ip = ip.trim().to_string();
    if ip.is_empty() {
        return Err("adresse IP vide".into());
    }
    if port == 0 {
        return Err("port invalide".into());
    }
    // N'accepte qu'une adresse IP litterale (v4 ou v6), jamais un nom d'hote :
    // cela empeche de transformer l'application en relais vers un tiers.
    if ip.parse::<std::net::IpAddr>().is_err() {
        return Err("adresse IP invalide".into());
    }

    // 1. Vérifier la connectivité
    ping_peer(ip.clone(), port).await?;

    // 2. Récupérer l'identité du pair en HTTPS uniquement.
    //
    // Le repli HTTP en clair a ete supprime : un attaquant local pouvait y
    // repondre avec une fausse identite, qui etait ensuite stockee dans le
    // cache de pairs et affichee comme empreinte « de confiance ».
    let client = client_tls(3)?;
    let url = format!("https://{ip}:{port}/v1/identity");
    let reponse = client.get(&url).send().await.map_err(|e| {
        format!("pair injoignable en HTTPS ({e}) : verifiez qu'il s'agit bien de RivaldSend")
    })?;
    if !reponse.status().is_success() {
        return Err(format!("pair injoignable (HTTP {})", reponse.status()));
    }

    let identity: serde_json::Value = reponse.json().await.map_err(|e| e.to_string())?;

    let name = identity["name"]
        .as_str()
        .unwrap_or(&format!("Appareil {ip}"))
        .to_string();
    let fingerprint_short = identity["fingerprint_short"]
        .as_str()
        .unwrap_or("0000")
        .to_string();
    let platform = identity["platform"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let device_id_distant = identity["device_id"].as_str().unwrap_or("").to_string();

    // Reconciliation avec le magasin : un pair apparie dont l'IP a change
    // (DHCP) est reconnu par son `device_id`, pas rejete comme inconnu.
    // Son adresse memorisee est mise a jour pour les prochains envois.
    let mut apparie = false;
    if !device_id_distant.is_empty() {
        if let Some(mut pair) = trust.par_device_id(&device_id_distant).await {
            apparie = true;
            if pair.ip != ip || pair.port != port {
                pair.ip = ip.clone();
                pair.port = port;
                pair.id = rivaldsend_core::pairing::id_de_pair(&ip, port);
                if let Err(e) = trust.enregistrer(pair).await {
                    tracing::warn!("mise a jour de l'adresse du pair impossible : {e}");
                }
            }
        }
    }

    let ev = crate::events::PeerDiscoveredEvent {
        id: rivaldsend_core::pairing::id_de_pair(&ip, port),
        name,
        ip: ip.clone(),
        port,
        fingerprint_short,
        trusted: apparie,
        platform,
    };

    // Insérer dans le cache Rust pour que start_transfer puisse retrouver le peer.
    {
        let mut garde = cache.0.lock().await;
        garde.insert(ev.id.clone(), (std::time::Instant::now(), ev.clone()));
    }

    let _ = app.emit("peer_discovered", ev.clone());
    Ok(ev)
}

#[tauri::command]
pub async fn rescan_peers(
    app: AppHandle,
    cache: State<'_, crate::PeerCacheState>,
) -> Result<(), String> {
    // Le browse persistant (lib.rs) maintient le cache à jour en temps réel.
    // « Rescanner » = ré-émettre instantanément les pairs connus vers l'UI.
    let peers: Vec<crate::events::PeerDiscoveredEvent> = cache
        .0
        .lock()
        .await
        .values()
        .map(|(_, ev)| ev.clone())
        .collect();
    for peer in peers {
        let _ = app.emit("peer_discovered", peer);
    }
    Ok(())
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn approve_peer(
    app: AppHandle,
    peerId: String,
    code: Option<String>,
) -> Result<(), String> {
    if let Some(c) = code.as_deref() {
        if !code_appairage_valide(c) {
            return Err("code d'appairage invalide".into());
        }
    }
    let _ = app.emit(
        "peer_approved",
        serde_json::json!({"peerId": peerId, "code": code}),
    );
    Ok(())
}

/// Genere un code d'appairage depuis une source cryptographique.
///
/// Remplace la generation cote interface (`Math.random`), qui n'est pas
/// concue pour des secrets. L'appelant l'affiche, puis l'enregistre via
/// `set_pairing_code` pour que le serveur puisse valider les demandes.
#[tauri::command]
pub fn generer_code_appairage() -> String {
    rivaldsend_core::pairing::generer_code()
}

/// Enregistre le code actuellement affiche a l'ecran, avec sa validite.
///
/// Tant que cette commande n'est pas appelee, `/v1/pair` et `/v1/transfers`
/// refusent tout (409) : un pair ne peut pas s'appairer a un appareil qui
/// n'affiche aucun code. C'etait le chaînon manquant cote PC : le QR etait
/// affiche mais le serveur n'en savait rien.
#[tauri::command]
pub async fn set_pairing_code(
    etat: State<'_, crate::http::AppState>,
    code: String,
) -> Result<String, String> {
    etat.set_code(&code).await
}

/// Appaire cet appareil a un pair distant, dans les deux sens.
///
/// 1. On demande l'identite du pair (`/v1/identity`) : s'il n'affiche aucun
///    code (`pairing_open: false`), on refuse tout de suite avec un message
///    clair au lieu d'echouer plus loin.
/// 2. On derive le secret partage et on le memorise.
/// 3. On appelle `/v1/pair` sur le pair pour qu'il nous memorise aussi.
///    Le pair nous repond avec sa propre identite, que l'on reverifie.
///
/// A la fin, les deux appareils se connaissent : chacun peut envoyer sans
/// ressaisir de code.
#[allow(non_snake_case)]
#[tauri::command]
pub async fn pair_with_peer(
    app: AppHandle,
    trust: State<'_, Arc<rivaldsend_core::trust::TrustStore>>,
    ip: String,
    port: u16,
    code: String,
) -> Result<serde_json::Value, String> {
    let normalise = rivaldsend_core::pairing::normaliser(&code);
    if !code_appairage_valide(&normalise) {
        return Err("code d'appairage invalide".into());
    }
    let ip = ip.trim().to_string();
    if ip.parse::<std::net::IpAddr>().is_err() {
        return Err("adresse IP invalide".into());
    }

    ping_peer(ip.clone(), port).await?;
    let client = client_tls(10)?;

    // 1. Identite du pair, et verification qu'il affiche un code.
    let identite: serde_json::Value = client
        .get(format!("https://{ip}:{port}/v1/identity"))
        .send()
        .await
        .map_err(|e| format!("pair injoignable : {e}"))?
        .error_for_status()
        .map_err(|e| format!("identite illisible : {e}"))?
        .json()
        .await
        .map_err(|e| format!("identite illisible : {e}"))?;
    if identite["pairing_open"].as_bool() == Some(false) {
        return Err(
            "cet appareil n'affiche aucun code : ouvrez son ecran d'appairage d'abord".into(),
        );
    }
    let device_distant = identite["device_id"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    if device_distant.is_empty() {
        return Err("le pair ne s'identifie pas (ancienne version ?)".into());
    }
    let nom_distant = identite["name"]
        .as_str()
        .unwrap_or("Appareil distant")
        .to_string();

    // 2. Secret partage, memorise localement des maintenant.
    let secret =
        rivaldsend_core::pairing::derive_pair_secret(&normalise, &device_id(), &device_distant);
    let pair = rivaldsend_core::trust::TrustedPeer {
        id: rivaldsend_core::pairing::id_de_pair(&ip, port),
        device_id: device_distant.clone(),
        name: nom_distant.clone(),
        ip: ip.clone(),
        port,
        fingerprint_short: identite["fingerprint_short"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        platform: identite["platform"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        secret: rivaldsend_core::pairing::hex(secret),
        paired_at: Some(chrono::Utc::now()),
    };
    trust
        .enregistrer(pair.clone())
        .await
        .map_err(|e| format!("memorisation impossible : {e}"))?;

    // 3. Demande reciproque : le pair nous memorise a son tour.
    let infos = get_device_info();
    let reponse: serde_json::Value = client
        .post(format!("https://{ip}:{port}/v1/pair"))
        .json(&serde_json::json!({
            "code": normalise,
            "device_id": device_id(),
            "name": infos.name,
            "fingerprint_short": infos.fingerprint_short,
            "platform": std::env::consts::OS,
            "sender_ip": infos.ip,
            "sender_port": infos.port,
        }))
        .send()
        .await
        .map_err(|e| format!("appairage refuse par le pair : {e}"))?
        .error_for_status()
        .map_err(|e| format!("appairage refuse par le pair : {e}"))?
        .json()
        .await
        .map_err(|e| format!("reponse d'appairage illisible : {e}"))?;

    // 4. Coherence : le pair qui repond doit etre celui qu'on a interroge.
    if let Some(repondu) = reponse["device_id"].as_str() {
        if !repondu.is_empty() && repondu != device_distant {
            trust.oublier(&pair.id).await.map_err(|e| e.to_string())?;
            return Err("le pair a change d'identite pendant l'appairage : abandon".into());
        }
    }

    let evenement = serde_json::json!({
        "peerId": pair.id,
        "deviceId": device_distant,
        "name": nom_distant,
        "ip": ip,
        "port": port,
        "fingerprintShort": pair.fingerprint_short,
        "platform": pair.platform,
    });
    let _ = app.emit("peer_paired", evenement.clone());
    let _ = app.emit(
        "peer_discovered",
        crate::events::PeerDiscoveredEvent {
            id: pair.id.clone(),
            name: nom_distant,
            ip,
            port,
            fingerprint_short: pair.fingerprint_short.clone(),
            trusted: true,
            platform: pair.platform.clone(),
        },
    );
    Ok(evenement)
}

/// Liste les pairs apparies (magasin de confiance persiste).
#[tauri::command]
pub async fn list_trusted_peers(
    trust: State<'_, Arc<rivaldsend_core::trust::TrustStore>>,
) -> Result<Vec<rivaldsend_core::trust::TrustedPeer>, String> {
    Ok(trust.lister().await)
}

/// Oublie un pair apparie : retire le secret partage et la confiance.
///
/// Apres l'oubli, tout envoi vers ce pair exigera un nouvel appairage, et le
/// pair est retire de la liste affichee. Le pair distant n'est pas prevenu
/// (il n'existe pas de canal pour cela) : il devra nous oublier a son tour
/// ou son prochain envoi echouera proprement.
#[allow(non_snake_case)]
#[tauri::command]
pub async fn forget_peer(
    app: AppHandle,
    cache: State<'_, crate::PeerCacheState>,
    trust: State<'_, Arc<rivaldsend_core::trust::TrustStore>>,
    peerId: String,
) -> Result<bool, String> {
    let mut oublie = trust.oublier(&peerId).await.map_err(|e| e.to_string())?;
    if !oublie {
        if let Some(suffixe) = peerId.strip_prefix("pending-") {
            if let Some(pair) = trust.par_device_id(suffixe).await {
                oublie = trust.oublier(&pair.id).await.map_err(|e| e.to_string())?;
            }
        }
    }
    // Retire aussi l'entree du cache de decouverte en direct.
    {
        let mut garde = cache.0.lock().await;
        garde.retain(|_, (_, ev)| ev.id != peerId);
    }
    if oublie {
        let _ = app.emit("peer_forgotten", serde_json::json!({"peerId": peerId}));
    }
    Ok(oublie)
}

/// Oublie tous les pairs apparies. Renvoie le nombre de pairs oublies.
#[tauri::command]
pub async fn forget_all_peers(
    app: AppHandle,
    cache: State<'_, crate::PeerCacheState>,
    trust: State<'_, Arc<rivaldsend_core::trust::TrustStore>>,
) -> Result<usize, String> {
    let n = trust.oublier_tout().await.map_err(|e| e.to_string())?;
    cache.0.lock().await.clear();
    let _ = app.emit("peers_forgotten", serde_json::json!({"count": n}));
    Ok(n)
}

/// Mémorise le dossier de téléchargement choisi dans les réglages.
/// Gère le préfixe ~/ puis délègue au gestionnaire de transferts.
#[allow(non_snake_case)]
#[tauri::command]
pub async fn set_download_dir(
    manager: State<'_, Arc<rivaldsend_core::manager::TransferManager>>,
    targetDir: String,
) -> Result<(), String> {
    let dossier = if let Some(reste) = targetDir.strip_prefix("~/") {
        dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join(reste)
    } else {
        std::path::PathBuf::from(&targetDir)
    };

    manager
        .set_default_download_dir(dossier)
        .await
        .map_err(|e| e.to_string())
}
