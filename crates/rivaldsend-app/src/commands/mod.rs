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
