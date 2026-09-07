use serde::Serialize;
use uuid::Uuid;
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
pub async fn start_transfer(peerId: String, filePaths: Vec<String>) -> Result<StartTransferResponse, String> {
    let _ = (peerId, filePaths);
    Ok(StartTransferResponse { transfer_id: Uuid::new_v4().to_string() })
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn cancel_transfer(transferId: String) -> Result<(), String> {
    let _ = transferId;
    Ok(())
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn pause_transfer(transferId: String) -> Result<(), String> {
    let _ = transferId;
    Ok(())
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn resume_transfer(transferId: String) -> Result<(), String> {
    let _ = transferId;
    Ok(())
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn accept_incoming(requestId: String, targetDir: String) -> Result<(), String> {
    let _ = (requestId, targetDir);
    Ok(())
}
#[allow(non_snake_case)]
#[tauri::command]
pub async fn reject_incoming(requestId: String) -> Result<(), String> {
    let _ = requestId;
    Ok(())
}
#[tauri::command]
pub fn open_file_dialog() -> Option<Vec<String>> {
    None
}
#[tauri::command]
pub async fn list_history(history_path: Option<String>) -> Result<Vec<rivaldsend_core::history::HistoryEntry>, String> {
    let p = history_path.unwrap_or_else(|| "history.jsonl".to_string());
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
pub async fn connect_by_ip(ip: String, port: u16) -> Result<crate::events::PeerDiscoveredEvent, String> {
    let _ = ping_peer(ip.clone(), port).await;
    Ok(crate::events::PeerDiscoveredEvent {
        id: format!("manual-{ip}:{port}"),
        name: format!("Appareil {ip}"),
        ip,
        port,
        fingerprint_short: "0000".into(),
        trusted: false,
        platform: std::env::consts::OS.into(),
    })
}

#[tauri::command]
pub async fn rescan_peers() -> Result<(), String> {
    Ok(())
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn approve_peer(peerId: String) -> Result<(), String> {
    let _ = peerId;
    Ok(())
}
