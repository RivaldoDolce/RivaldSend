use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;
use std::sync::Arc;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfoResponse {
    pub name: String,
    pub ip: String,
    pub fingerprint: String,
    pub fingerprint_short: String,
    pub port: u16,
}
#[tauri::command]
pub fn get_device_info() -> DeviceInfoResponse {
    let ip = rivaldsend_core::discovery::list_interfaces()
        .into_iter()
        .find(|(_, ip)| ip.is_ipv4())
        .map(|(_, ip)| ip.to_string())
        .unwrap_or_else(|| "127.0.0.1".into());
    DeviceInfoResponse { name: "RivaldSend".into(), ip, fingerprint: String::new(), fingerprint_short: String::new(), port: 53317 }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTransferResponse {
    pub transfer_id: String,
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn start_transfer(
    app: AppHandle,
    manager: State<'_, Arc<rivaldsend_core::manager::TransferManager>>,
    peerId: String,
    filePaths: Vec<String>,
) -> Result<StartTransferResponse, String> {
    if filePaths.is_empty() {
        return Err("aucun fichier".into());
    }
    for p in &filePaths {
        let path = std::path::Path::new(p);
        if !path.exists() {
            return Err(format!("fichier introuvable: {p}"));
        }
    }
    let transfer_id = Uuid::new_v4();
    let first_path = std::path::PathBuf::from(&filePaths[0]);
    let _ = manager.enqueue(first_path).await;
    manager.start_transfer(transfer_id).await.map_err(|e| e.to_string())?;
    let _ = app.emit("transfer_progress", crate::events::ProgressEvent {
        transfer_id: transfer_id.to_string(),
        bytes_done: 0,
        total_bytes: 0,
        speed_bps: 0,
        eta_secs: 0,
        status: "running".into(),
        error: None,
    });
    let _ = peerId;
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
pub async fn pause_transfer(transferId: String) -> Result<(), String> {
    let _ = transferId.parse::<Uuid>().map_err(|e| e.to_string())?;
    Ok(())
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn resume_transfer(transferId: String) -> Result<(), String> {
    let _ = transferId.parse::<Uuid>().map_err(|e| e.to_string())?;
    Ok(())
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn accept_incoming(app: AppHandle, requestId: String, targetDir: String) -> Result<(), String> {
    let dir = if targetDir == "~" || targetDir.starts_with("~/") {
        let home = dirs::download_dir().or_else(dirs::home_dir).unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        if targetDir == "~" { home } else { home.join(targetDir.trim_start_matches("~/")) }
    } else {
        std::path::PathBuf::from(&targetDir)
    };
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let _ = app.emit("incoming_accepted", serde_json::json!({"requestId": requestId, "targetDir": dir}));
    Ok(())
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn reject_incoming(app: AppHandle, requestId: String) -> Result<(), String> {
    let _ = app.emit("incoming_rejected", serde_json::json!({"requestId": requestId}));
    Ok(())
}
#[tauri::command]
pub fn open_file_dialog() -> Option<Vec<String>> {
    None
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
    let _ = ping_peer(ip.clone(), port).await;
    let ev = crate::events::PeerDiscoveredEvent {
        id: format!("peer-{ip}:{port}"),
        name: format!("Appareil {ip}"),
        ip: ip.clone(),
        port,
        fingerprint_short: "0000".into(),
        trusted: false,
        platform: std::env::consts::OS.into(),
    };
    let _ = app.emit("peer_discovered", ev.clone());
    Ok(ev)
}

#[tauri::command]
pub async fn rescan_peers(app: AppHandle) -> Result<(), String> {
    let ifaces = rivaldsend_core::discovery::list_interfaces();
    let mut seen = std::collections::HashSet::new();
    for (name, ip) in ifaces {
        let ip_str = ip.to_string();
        let key = format!("{ip_str}:53317");
        if !seen.insert(key.clone()) {
            continue;
        }
        let display_name = if name.starts_with("eth") || name.starts_with("wlan") || name.starts_with("en") {
            format!("Appareil {ip_str}")
        } else {
            name.clone()
        };
        let ev = crate::events::PeerDiscoveredEvent {
            id: format!("peer-{key}"),
            name: display_name,
            ip: ip_str,
            port: 53317,
            fingerprint_short: "0000".into(),
            trusted: false,
            platform: std::env::consts::OS.into(),
        };
        let _ = app.emit("peer_discovered", ev);
    }
    Ok(())
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn approve_peer(app: AppHandle, peerId: String) -> Result<(), String> {
    let _ = app.emit("peer_approved", serde_json::json!({"peerId": peerId}));
    Ok(())
}
