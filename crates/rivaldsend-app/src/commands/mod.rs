use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;
use std::sync::Arc;
use std::sync::OnceLock;

// Empreinte stable persistée
static DEVICE_FINGERPRINT: OnceLock<String> = OnceLock::new();

fn get_or_create_fingerprint() -> String {
    DEVICE_FINGERPRINT.get_or_init(|| {
        let path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("rivaldsend")
            .join("fingerprint.txt");

        if let Ok(content) = std::fs::read_to_string(&path) {
            let trimmed = content.trim().to_string();
            if !trimmed.is_empty() { return trimmed; }
        }

        // Générer une nouvelle empreinte (16 octets = 32 caractères hex)
        let fingerprint = format!("{:x}", Uuid::new_v4().as_u128());
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, &fingerprint);
        fingerprint
    }).clone()
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
        port: 53317
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
fn client_tls(duree_s: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(duree_s))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())
}

/// Vérifie le format d'un code d'appairage (6 caractères affichés, tiret toléré).
fn code_appairage_valide(code: &str) -> bool {
    let normalise = code.trim().replace('-', "").to_ascii_uppercase();
    normalise.len() == 6
        && (normalise.bytes().all(|c| c.is_ascii_digit())
            || normalise
                .bytes()
                .all(|c| matches!(c, b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z' | b'2'..=b'9')))
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn start_transfer(
    app: AppHandle,
    manager: State<'_, Arc<rivaldsend_core::manager::TransferManager>>,
    cache: State<'_, crate::PeerCacheState>,
    peerId: String,
    filePaths: Vec<String>,
    code: Option<String>,
) -> Result<StartTransferResponse, String> {
    // Limite assumée : un seul fichier par transfert (le receveur refuse le reste).
    if filePaths.len() != 1 {
        return Err("un seul fichier par transfert dans cette version".into());
    }
    let chemin = std::path::PathBuf::from(&filePaths[0]);
    if !chemin.exists() {
        return Err(format!("fichier introuvable: {}", filePaths[0]));
    }

    // Le code d'appairage est obligatoire : le receveur en dérive la PSK.
    let Some(c) = code else {
        return Err("code d'appairage requis".into());
    };
    if !code_appairage_valide(&c) {
        return Err("code d'appairage invalide".into());
    }

    // Retrouver l'adresse du pair dans le cache de découverte.
    let (ip, port) = {
        let garde = cache.0.lock().await;
        garde
            .values()
            .map(|(_, ev)| ev)
            .find(|ev| ev.id == peerId)
            .map(|ev| (ev.ip.clone(), ev.port))
    }
    .ok_or_else(|| "pair introuvable, reconnectez-vous".to_string())?;

    // Calculer la taille totale réelle
    let total_bytes: u64 = std::fs::metadata(&chemin).map(|m| m.len()).unwrap_or(0);

    let transfer_id = Uuid::new_v4();

    // Mémoriser le code et la cible pour le travailleur d'envoi HTTPS.
    manager.set_pairing_code(transfer_id, c.clone()).await;

    // Enfiler le fichier pour suivi local
    let _ = manager.enqueue(chemin.clone()).await;

    // Associer le peer cible
    manager.set_target_peer(transfer_id, peerId).await;

    // Démarrer le transfert
    manager.start_transfer(transfer_id).await.map_err(|e| e.to_string())?;

    // Lancer le travailleur d'envoi HTTPS (annonce + morceaux + achèvement).
    crate::sender::spawn_envoi(
        app.clone(),
        manager.inner().clone(),
        transfer_id,
        chemin,
        ip,
        port,
        c,
    );

    // Émettre avec total_bytes réel
    let _ = app.emit("transfer_progress", crate::events::ProgressEvent {
        transfer_id: transfer_id.to_string(),
        bytes_done: 0,
        total_bytes,
        speed_bps: 0,
        eta_secs: 0,
        status: "running".into(),
        error: None,
    });
    Ok(StartTransferResponse { transfer_id: transfer_id.to_string() })
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
    let _ = app.emit("transfer_progress", crate::events::ProgressEvent {
        transfer_id: transferId,
        bytes_done: 0,
        total_bytes: 0,
        speed_bps: 0,
        eta_secs: 0,
        status: "cancelled".into(),
        error: None,
    });
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
    manager.pause_transfer(id).await.map_err(|e| e.to_string())?;
    let _ = app.emit("transfer_progress", crate::events::ProgressEvent {
        transfer_id: transferId,
        bytes_done: 0,
        total_bytes: 0,
        speed_bps: 0,
        eta_secs: 0,
        status: "paused".into(),
        error: None,
    });
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
    manager.resume_transfer(id).await.map_err(|e| e.to_string())?;
    let _ = app.emit("transfer_progress", crate::events::ProgressEvent {
        transfer_id: transferId,
        bytes_done: 0,
        total_bytes: 0,
        speed_bps: 0,
        eta_secs: 0,
        status: "running".into(),
        error: None,
    });
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
        let home = dirs::download_dir().or_else(dirs::home_dir).unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        if targetDir == "~" { home } else { home.join(targetDir.trim_start_matches("~/")) }
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
    incoming.0.lock().await.insert(requestId.clone(), crate::IncomingDecision {
        target_dir: Some(dir.clone()),
        decided_at: std::time::SystemTime::now(),
    });
    let _ = app.emit("incoming_accepted", serde_json::json!({"requestId": requestId, "targetDir": dir, "stagingDir": staging}));
    Ok(())
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn reject_incoming(app: AppHandle, incoming: State<'_, crate::IncomingState>, requestId: String) -> Result<(), String> {
    // Mémoriser le refus : les chunks tardifs de cette demande seront rejetés
    incoming.0.lock().await.insert(requestId.clone(), crate::IncomingDecision {
        target_dir: None,
        decided_at: std::time::SystemTime::now(),
    });
    let _ = app.emit("incoming_rejected", serde_json::json!({"requestId": requestId}));
    Ok(())
}
#[tauri::command]
pub async fn open_file_dialog(app: AppHandle) -> Result<Option<Vec<String>>, String> {
    use tauri_plugin_dialog::DialogExt;
    // Le dialogue natif est bloquant : l'exécuter hors du runtime async.
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog().file().blocking_pick_files()
    })
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
pub async fn list_history(history_path: Option<String>) -> Result<Vec<rivaldsend_core::history::HistoryEntry>, String> {
    let p = if let Some(path) = history_path {
        if path.starts_with("~/") {
            let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
            home.join(path.trim_start_matches("~/")).to_string_lossy().to_string()
        } else { path }
    } else {
        dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp")).join("rivaldsend/history.jsonl").to_string_lossy().to_string()
    };
    rivaldsend_core::history::load_all(std::path::Path::new(&p)).await.map_err(|e| e.to_string())
}

#[allow(non_snake_case)]
#[tauri::command]
pub fn generate_pairing_qr(ip: String, port: u16, code: String, fingerprintShort: String) -> Result<String, String> {
    use qrcode::QrCode;
    let payload = format!("rivaldsend://{}:{}?code={}&fp={}", ip, port, code, fingerprintShort);
    let qr = QrCode::new(payload.as_bytes()).map_err(|e| e.to_string())?;
    let svg = qr.render::<qrcode::render::svg::Color<'_>>().min_dimensions(200, 200).build();
    Ok(svg)
}

#[tauri::command]
pub fn check_firewall() -> Result<String, String> {
    let profile = rivaldsend_core::firewall::detect_windows_firewall().map_err(|e| e.to_string())?;
    if rivaldsend_core::firewall::should_block_server(&profile) {
        Err("Réseau public détecté — serveur bloqué. Passez en réseau privé.".into())
    } else {
        Ok(format!("{:?}", profile))
    }
}

#[tauri::command]
pub fn list_network_interfaces() -> Vec<(String, String)> {
    rivaldsend_core::discovery::list_interfaces().into_iter().map(|(n, ip)| (n, ip.to_string())).collect()
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn ping_peer(ip: String, port: u16) -> Result<u32, String> {
    let start = std::time::Instant::now();
    let addr = format!("{ip}:{port}");
    tokio::time::timeout(std::time::Duration::from_millis(800), tokio::net::TcpStream::connect(addr))
        .await
        .map_err(|_| "timeout".to_string())?
        .map_err(|e| e.to_string())?;
    Ok(start.elapsed().as_millis() as u32)
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn connect_by_ip(app: AppHandle, ip: String, port: u16) -> Result<crate::events::PeerDiscoveredEvent, String> {
    // 1. Vérifier la connectivité
    ping_peer(ip.clone(), port).await?;

    // 2. Récupérer l'identité du pair en HTTPS d'abord, HTTP en repli
    // (compatibilité avec les pairs d'ancienne version encore en clair).
    let client = client_tls(3)?;

    let mut reponse = None;
    let mut derniere_erreur = String::new();
    for schema in ["https", "http"] {
        let url = format!("{schema}://{ip}:{port}/v1/identity");
        match client.get(&url).send().await {
            Ok(r) if r.status().is_success() => {
                reponse = Some(r);
                break;
            }
            Ok(r) => {
                derniere_erreur = format!("HTTP {}", r.status());
            }
            Err(e) => {
                derniere_erreur = format!("{schema} : {e}");
            }
        }
    }
    let reponse = match reponse {
        Some(r) => r,
        None => return Err(format!("pair injoignable ({derniere_erreur})")),
    };

    let identity: serde_json::Value = reponse.json().await
        .map_err(|e| e.to_string())?;

    let name = identity["name"].as_str().unwrap_or(&format!("Appareil {ip}")).to_string();
    let fingerprint_short = identity["fingerprint_short"].as_str().unwrap_or("0000").to_string();
    let platform = identity["platform"].as_str().unwrap_or("unknown").to_string();

    let ev = crate::events::PeerDiscoveredEvent {
        id: format!("peer-{ip}:{port}"),
        name,
        ip: ip.clone(),
        port,
        fingerprint_short,
        trusted: false,
        platform,
    };
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
    let peers: Vec<crate::events::PeerDiscoveredEvent> =
        cache.0.lock().await.values().map(|(_, ev)| ev.clone()).collect();
    for peer in peers {
        let _ = app.emit("peer_discovered", peer);
    }
    Ok(())
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn approve_peer(app: AppHandle, peerId: String, code: Option<String>) -> Result<(), String> {
    if let Some(c) = code.as_deref() {
        if !code_appairage_valide(c) {
            return Err("code d'appairage invalide".into());
        }
    }
    let _ = app.emit("peer_approved", serde_json::json!({"peerId": peerId, "code": code}));
    Ok(())
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

    manager.set_default_download_dir(dossier).await
        .map_err(|e| e.to_string())
}
