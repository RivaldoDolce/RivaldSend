pub mod commands;
pub mod events;
pub mod http;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use mdns_sd::{ServiceDaemon, ServiceInfo};

/// Wrapper pour partager le daemon mDNS via Tauri state
pub struct MdnsState {
    pub daemon: Arc<ServiceDaemon>,
}

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

    // 1. Créer UN SEUL daemon mDNS au démarrage
    let daemon = Arc::new(
        ServiceDaemon::new().expect("Failed to create mDNS daemon")
    );

    // 2. Récupérer les infos de l'appareil
    let device_info = commands::get_device_info();

    // 3. Publier le service local
    let txt_properties: HashMap<String, String> = HashMap::from([
        ("device_name".to_string(), "RivaldSend".to_string()),
        ("platform".to_string(), std::env::consts::OS.to_string()),
        ("fingerprint_short".to_string(), device_info.fingerprint_short.clone()),
    ]);

    let service_info = ServiceInfo::new(
        rivaldsend_core::discovery::SERVICE_TYPE,
        &device_info.name,
        &format!("{}.local.", device_info.name.to_lowercase().replace(' ', "-")),
        &device_info.ip,
        device_info.port,
        txt_properties,
    )
    .expect("Invalid service info");

    if let Err(e) = daemon.register(service_info) {
        tracing::error!("Failed to register mDNS service: {e}");
    } else {
        tracing::info!("mDNS service registered: {} at {}:{}", device_info.name, device_info.ip, device_info.port);
    }

    // 4. Manager HTTP
    let manager = Arc::new(rivaldsend_core::manager::TransferManager::new(
        rivaldsend_core::manager::TransferManager::default_resume_dir(),
    ));
    let http_manager = manager.clone();

    tauri::Builder::default()
        .manage(manager)
        .manage(MdnsState { daemon }) // ← daemon partagé
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
                match tokio::net::TcpListener::bind("0.0.0.0:53317").await {
                    Ok(l) => {
                        tracing::info!("HTTP server listening on 0.0.0.0:53317");
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
