pub mod commands;
pub mod events;
pub mod http;
use std::sync::Arc;
use tauri::Emitter;
pub fn build_router() -> axum::Router {
    let dir = rivaldsend_core::manager::TransferManager::default_resume_dir();
    let manager = Arc::new(rivaldsend_core::manager::TransferManager::new(dir));
    http::router(http::AppState { manager })
}
pub fn run_tauri() {
    let should_block = rivaldsend_core::firewall::detect_windows_firewall()
        .map(|p| rivaldsend_core::firewall::should_block_server(&p))
        .unwrap_or(false);
    if should_block {
        eprintln!("Réseau public détecté — serveur non démarré");
        std::process::exit(1);
    }
    let manager = Arc::new(rivaldsend_core::manager::TransferManager::new(
        rivaldsend_core::manager::TransferManager::default_resume_dir(),
    ));
    let http_manager = manager.clone();
    tauri::Builder::default()
        .manage(manager)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_os::init())
        .setup(move |app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let router = http::router(http::AppState { manager: http_manager });
                match tokio::net::TcpListener::bind("127.0.0.1:53317").await {
                    Ok(l) => {
                        tracing::info!("HTTP server listening on 127.0.0.1:53317");
                        if let Err(e) = axum::serve(l, router).await {
                            tracing::error!("http server error: {e}");
                        }
                    }
                    Err(e) => tracing::error!("failed to bind http server: {e}"),
                }
                let _ = handle.emit("server_ready", serde_json::json!({"port":53317}));
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
