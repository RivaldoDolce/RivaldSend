pub mod commands;
pub mod events;
pub mod http;
pub mod sender;
pub mod tls;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;

/// Daemon mDNS partagé via l'état Tauri
pub struct MdnsState {
    pub daemon: Arc<ServiceDaemon>,
}

/// Cache des pairs découverts par le listener mDNS persistant.
/// Chaque entrée est horodatée pour permettre l'expiration (anti-fuite).
#[derive(Clone, Default)]
pub struct PeerCacheState(
    pub Arc<tokio::sync::Mutex<HashMap<String, (std::time::Instant, events::PeerDiscoveredEvent)>>>,
);

/// Durée de vie max d'une entrée du cache sans re-résolution
const PEER_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(15 * 60);
/// Borne de sécurité sur le nombre de pairs en cache
const PEER_CACHE_MAX: usize = 256;

/// Décision prise sur une demande entrante (acceptée → dossier cible, refusée → None).
/// Conservée pour que le pipeline de réception sache où écrire (ou refuser) les chunks.
#[derive(Clone, Debug)]
pub struct IncomingDecision {
    pub target_dir: Option<std::path::PathBuf>,
    pub decided_at: std::time::SystemTime,
}

/// Demandes entrantes traitées, indexées par request_id
#[derive(Clone, Default, Debug)]
pub struct IncomingState(pub Arc<tokio::sync::Mutex<HashMap<String, IncomingDecision>>>);

pub fn build_router() -> axum::Router {
    let dir = rivaldsend_core::manager::TransferManager::default_resume_dir();
    let manager = Arc::new(rivaldsend_core::manager::TransferManager::new(dir));
    http::router(http::AppState::new(manager))
}

/// Point d'entrée mobile (génère le wrapper JNI sur Android).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_tauri() {
    // rustls refuse de choisir seul entre les providers `ring` et `aws-lc-rs`
    // (tous deux activés via reqwest) et panique sans défaut explicite.
    // On fige `ring` ici, avant tout usage TLS (serveur comme client).
    let _ = rustls::crypto::ring::default_provider().install_default();

    let should_block = rivaldsend_core::firewall::detect_network_profile()
        .map(|p| rivaldsend_core::firewall::should_block_server(&p))
        .unwrap_or(false);
    if should_block {
        eprintln!("Réseau public détecté — serveur non démarré");
        std::process::exit(1);
    }

    // 1. UN SEUL daemon mDNS pour toute la vie de l'app
    let daemon = Arc::new(ServiceDaemon::new().expect("Failed to create mDNS daemon"));

    // 2. Infos appareil + identifiant stable pour filtrer l'auto-découverte.
    let device_info = commands::get_device_info();
    let device_id = commands::initialiser_device_id();
    let dossier_app = rivaldsend_core::manager::TransferManager::dossier_app();

    // Magasin de confiance : charge une fois, partage entre les commandes
    // (appairage, envoi) et le serveur HTTP (verification des transferts).
    let trust = Arc::new(rivaldsend_core::trust::TrustStore::charger(&dossier_app));

    // 3. Publication du service local.
    //
    // Le nom d'instance et l'hote sont assainis en libelles DNS : un nom
    // d'hote systeme avec underscore faisait echouer `ServiceInfo::new` et
    // toute l'application tombait au demarrage.
    let instance = format!(
        "{}{}",
        rivaldsend_core::discovery::SERVICE_NAME_PREFIX,
        rivaldsend_core::discovery::sanitize_label(&device_info.name)
    );
    let txt_properties: HashMap<String, String> = HashMap::from([
        ("device_name".to_string(), device_info.name.clone()),
        ("platform".to_string(), std::env::consts::OS.to_string()),
        (
            "fingerprint_short".to_string(),
            device_info.fingerprint_short.clone(),
        ),
        ("device_id".to_string(), device_id.clone()),
        (
            "protocol_version".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        ),
    ]);

    let service_info = ServiceInfo::new(
        rivaldsend_core::discovery::SERVICE_TYPE,
        &instance,
        &format!("{instance}.local."),
        &device_info.ip,
        device_info.port,
        txt_properties,
    )
    .expect("Invalid service info");

    if let Err(e) = daemon.register(service_info) {
        tracing::error!("Failed to register mDNS service: {e}");
    } else {
        tracing::info!(
            "mDNS service registered: {} at {}:{}",
            device_info.name,
            device_info.ip,
            device_info.port
        );
    }

    // 4. Manager HTTP
    let manager = Arc::new(rivaldsend_core::manager::TransferManager::new(
        rivaldsend_core::manager::TransferManager::default_resume_dir(),
    ));

    // Etat HTTP unique, partage entre le serveur et les commandes : le serveur
    // voit le code affiche (`set_pairing_code`) et le magasin de confiance,
    // sans quoi l'appairage et la reception refusaient tout.
    let http_state = http::AppState::avec_trust(manager.clone(), trust.clone());
    let http_state_server = http_state.clone();
    // `http_state.app` est renseigne dans `setup`, quand le handle existe.

    // Le scan QR n'existe que sur mobile (la crate est vide sur desktop).
    #[allow(unused_mut)]
    let mut constructeur = tauri::Builder::default();
    #[cfg(mobile)]
    {
        constructeur = constructeur.plugin(tauri_plugin_barcode_scanner::init());
    }
    constructeur
        .manage(manager)
        .manage(MdnsState { daemon })
        .manage(PeerCacheState::default())
        .manage(IncomingState::default())
        .manage(trust)
        .manage(http_state)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_os::init())
        .setup(move |app| {
            let http_handle = app.handle().clone();
            let ev_handle = app.handle().clone();

            // Le serveur HTTP peut desormais emettre vers l'interface
            // (`incoming_request`, `transfer_completed`, `peer_paired`).
            let etat_http = app.handle().state::<http::AppState>().inner().clone();
            let http_handle_emission = http_handle.clone();
            let etat_pour_setup = etat_http.clone();
            tauri::async_runtime::spawn(async move {
                etat_pour_setup.set_app(http_handle_emission).await;
            });

            // Hydratation : les pairs apparies sont visibles des le demarrage,
            // meme sans annonce mDNS fraiche. Sans cela, un pair connu mais
            // silencieux etait injoignable jusqu'a sa prochaine annonce.
            let cache_hydratation = app.handle().state::<PeerCacheState>().inner().0.clone();
            let trust_hydratation = app
                .handle()
                .state::<Arc<rivaldsend_core::trust::TrustStore>>()
                .inner()
                .clone();
            let ev_hydratation = ev_handle.clone();
            tauri::async_runtime::spawn(async move {
                for pair in trust_hydratation.lister().await {
                    if pair.ip.is_empty() || pair.port == 0 {
                        continue;
                    }
                    let ev = events::PeerDiscoveredEvent {
                        id: pair.id.clone(),
                        name: pair.name.clone(),
                        ip: pair.ip.clone(),
                        port: pair.port,
                        fingerprint_short: pair.fingerprint_short.clone(),
                        trusted: true,
                        platform: pair.platform.clone(),
                    };
                    cache_hydratation
                        .lock()
                        .await
                        .insert(pair.id.clone(), (std::time::Instant::now(), ev.clone()));
                    let _ = ev_hydratation.emit("peer_discovered", ev);
                }
            });

            // --- Serveur HTTPS (certificat auto-signé, PSK exigée sur les transferts) ---
            tauri::async_runtime::spawn(async move {
                let routeur = http::router(http_state_server);
                let (config_tls, empreinte) = match crate::tls::assurer_config_tls() {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("TLS indisponible : {e}");
                        let _ = http_handle.emit(
                            "server_ready",
                            serde_json::json!({"port":53317, "tls":false, "erreur":e}),
                        );
                        return;
                    }
                };
                tracing::info!("empreinte TLS locale : {empreinte}");
                let accepteur = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(config_tls));
                let ecouteur = match tokio::net::TcpListener::bind("0.0.0.0:53317").await {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::error!("échec du bind HTTPS : {e}");
                        return;
                    }
                };
                tracing::info!("serveur HTTPS en écoute sur 0.0.0.0:53317");
                let _ = http_handle.emit(
                    "server_ready",
                    serde_json::json!({"port":53317, "tls":true, "empreinte":empreinte}),
                );
                use hyper_util::rt::{TokioExecutor, TokioIo};
                loop {
                    let (flux_tcp, adresse) = match ecouteur.accept().await {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::error!("acceptation refusée : {e}");
                            continue;
                        }
                    };
                    let accepteur = accepteur.clone();
                    let routes = routeur.clone();
                    tauri::async_runtime::spawn(async move {
                        let flux_tls = match accepteur.accept(flux_tcp).await {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::warn!(
                                    "poignée de main TLS refusée depuis {adresse} : {e}"
                                );
                                return;
                            }
                        };
                        let io = TokioIo::new(flux_tls);
                        let service = hyper_util::service::TowerToHyperService::new(routes);
                        let _ = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                            .serve_connection(io, service)
                            .await;
                    });
                }
            });

            // --- Listener mDNS PERSISTANT : créé une fois, jamais droppé ---
            let mdns = app.handle().state::<MdnsState>().inner().daemon.clone();
            let cache = app.handle().state::<PeerCacheState>().inner().0.clone();
            let trust_listener = app
                .handle()
                .state::<Arc<rivaldsend_core::trust::TrustStore>>()
                .inner()
                .clone();

            // Filtrage de l'auto-découverte : on ignore notre propre annonce
            // via l'identifiant stable du TXT (pas via l'IP, qui peut changer).
            let device_id_local = device_id.clone();

            // UN SEUL browse() pour toute la vie de l'app
            let receiver = mdns
                .browse(rivaldsend_core::discovery::SERVICE_TYPE)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            tauri::async_runtime::spawn(async move {
                use mdns_sd::ServiceEvent;
                loop {
                    match receiver.recv_async().await {
                        Ok(ServiceEvent::ServiceResolved(info)) => {
                            let annonce_id = info.get_property_val_str("device_id").unwrap_or("");
                            if annonce_id == device_id_local {
                                continue; // C'est nous (même appareil).
                            }
                            // Le serveur n'ecoute qu'en IPv4 : une annonce qui
                            // ne propose que de l'IPv6 est injoignable. On
                            // prefere l'IPv4 du LAN, jamais le loopback.
                            let adresses: Vec<std::net::IpAddr> =
                                info.get_addresses().iter().cloned().collect();
                            let Some(ip) = rivaldsend_core::discovery::choisir_adresse(&adresses)
                            else {
                                continue;
                            };
                            let port = info.get_port();
                            let fullname = info.get_fullname().to_string();
                            let apparie = !annonce_id.is_empty()
                                && trust_listener.par_device_id(annonce_id).await.is_some();
                            let fingerprint_court = info
                                .get_property_val_str("fingerprint_short")
                                .unwrap_or("0000")
                                .to_string();
                            let plateforme = info
                                .get_property_val_str("platform")
                                .unwrap_or("unknown")
                                .to_string();
                            let nom = info
                                .get_property_val_str("device_name")
                                .unwrap_or(info.get_hostname())
                                .to_string();
                            // Un pair apparie dont l'IP a change est mis a jour
                            // dans le magasin : les prochains envois suivent.
                            if apparie {
                                let mut pairs_connus = trust_listener.lister().await;
                                if let Some(pair) =
                                    pairs_connus.iter_mut().find(|p| p.device_id == annonce_id)
                                {
                                    if pair.ip != ip || pair.port != port {
                                        let mut maj = pair.clone();
                                        maj.ip = ip.clone();
                                        maj.port = port;
                                        maj.id = rivaldsend_core::pairing::id_de_pair(&ip, port);
                                        if maj.name.trim().is_empty()
                                            || maj.name.starts_with("Appareil ")
                                        {
                                            maj.name = nom.clone();
                                        }
                                        let _ = trust_listener.enregistrer(maj).await;
                                    }
                                }
                            }
                            let ev = events::PeerDiscoveredEvent {
                                id: rivaldsend_core::pairing::id_de_pair(&ip, port),
                                name: nom,
                                ip,
                                port,
                                fingerprint_short: fingerprint_court,
                                trusted: apparie,
                                platform: plateforme,
                            };
                            {
                                let mut guard = cache.lock().await;
                                // Anti-fuite : expire les entrées non re-résolues…
                                let now = std::time::Instant::now();
                                guard.retain(|_, (seen, _)| {
                                    now.duration_since(*seen) < PEER_CACHE_TTL
                                });
                                // …et borne la taille en évinçant la plus ancienne.
                                if guard.len() >= PEER_CACHE_MAX {
                                    if let Some(oldest) = guard
                                        .iter()
                                        .min_by_key(|(_, (seen, _))| *seen)
                                        .map(|(k, _)| k.clone())
                                    {
                                        guard.remove(&oldest);
                                    }
                                }
                                guard.insert(fullname, (now, ev.clone()));
                            }
                            let _ = ev_handle.emit("peer_discovered", ev);
                        }
                        Ok(ServiceEvent::ServiceRemoved(_ty, fullname)) => {
                            if let Some((_, ev)) = cache.lock().await.remove(&fullname) {
                                let _ =
                                    ev_handle.emit("peer_lost", serde_json::json!({ "id": ev.id }));
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::warn!("mDNS listener stopped: {e}");
                            break;
                        }
                    }
                }
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
            commands::approve_peer,
            commands::generer_code_appairage,
            commands::set_pairing_code,
            commands::pair_with_peer,
            commands::list_trusted_peers,
            commands::forget_peer,
            commands::forget_all_peers,
            commands::set_download_dir
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("tauri error: {e}");
        });
}
