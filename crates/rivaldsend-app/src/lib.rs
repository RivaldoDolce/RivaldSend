pub mod commands;
pub mod events;
pub mod http;
use std::sync::Arc;
pub fn build_router() -> axum::Router {
    let manager = Arc::new(rivaldsend_core::manager::TransferManager::new(std::path::PathBuf::from("/tmp/rivaldsend-resume")));
    http::router(http::AppState { manager })
}
pub fn run_tauri() {
    if let Ok(profile) = rivaldsend_core::firewall::detect_windows_firewall() {
        if rivaldsend_core::firewall::should_block_server(&profile) {
            eprintln!("Réseau public détecté — serveur non démarré");
        }
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let router = build_router();
                let listener = tokio::net::TcpListener::bind("0.0.0.0:53317").await;
                if let Ok(l) = listener {
                    let _ = axum::serve(l, router).await;
                } else {
                    eprintln!("failed to bind http server");
                }
                let _ = handle;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_device_info,
            commands::open_file_dialog,
            commands::start_transfer,
            commands::cancel_transfer,
            commands::pause_transfer,
            commands::resume_transfer,
            commands::accept_incoming,
            commands::reject_incoming,
            commands::list_history,
            commands::generate_pairing_qr,
            commands::check_firewall,
            commands::list_network_interfaces,
            commands::ping_peer,
            commands::connect_by_ip,
            commands::rescan_peers,
            commands::approve_peer
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| { eprintln!("tauri error: {e}"); });
}
