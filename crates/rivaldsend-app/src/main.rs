fn main() {
    // Logs lisibles : le daemon mDNS inonde la sortie avec les interfaces
    // virtuelles (Docker, veth) qui ne servent pas à la découverte.
    let filtre = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,mdns_sd=warn"));
    tracing_subscriber::fmt().with_env_filter(filtre).init();
    rivaldsend_app::run_tauri();
}
