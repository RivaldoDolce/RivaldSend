pub mod commands;
pub mod events;
pub mod http;
use std::sync::Arc;
pub fn build_router() -> axum::Router {
    let manager = Arc::new(rivaldsend_core::manager::TransferManager::new(std::path::PathBuf::from("/tmp/rivaldsend-resume")));
    http::router(http::AppState { manager })
}
#[cfg(feature = "tauri-app")]
pub fn run_tauri() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::get_device_info, commands::open_file_dialog, commands::start_transfer, commands::list_history])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| { eprintln!("tauri error: {e}"); });
}
