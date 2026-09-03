use serde::Serialize;
use uuid::Uuid;
#[derive(Debug, Serialize)]
pub struct DeviceInfoResponse {
    pub name: String,
    pub fingerprint: String,
}
#[tauri::command]
pub fn get_device_info() -> DeviceInfoResponse {
    DeviceInfoResponse { name: "RivaldSend".into(), fingerprint: String::new() }
}
#[tauri::command]
pub fn open_file_dialog() -> Option<Vec<String>> {
    None
}
#[tauri::command]
pub async fn start_transfer(path: String) -> Result<String, String> {
    let _ = path;
    Ok(Uuid::new_v4().to_string())
}
#[tauri::command]
pub async fn list_history(history_path: Option<String>) -> Result<Vec<rivaldsend_core::history::HistoryEntry>, String> {
    let p = history_path.unwrap_or_else(|| "history.jsonl".to_string());
    rivaldsend_core::history::load_all(std::path::Path::new(&p)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn generate_pairing_qr(ip: String, port: u16, code: String, fingerprint_short: String) -> Result<String, String> {
    use qrcode::QrCode;
    let payload = format!("rivaldsend://{}:{}?code={}&fp={}", ip, port, code, fingerprint_short);
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
