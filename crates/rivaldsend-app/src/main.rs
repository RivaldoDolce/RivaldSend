fn main() {
    // Évite l'écran noir sur les pilotes qui échouent en EGL/DRI2 (vu sur Pop!_OS).
    // Doit être posé avant toute initialisation WebKit : le faire ici plutôt que
    // dans un script garantit que ça marche aussi depuis le dock, où les patchs
    // .desktop ne sont pas toujours pris en compte. Un choix explicite est respecté.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    // Logs lisibles : le daemon mDNS inonde la sortie d'erreurs sur les
    // interfaces virtuelles (Docker, veth) qui ne servent pas à la découverte.
    // Ces erreurs sont du bruit connu côté mdns_sd : on les coupe (réactiver
    // avec RUST_LOG=debug en cas de vrai problème de découverte).
    let filtre = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,mdns_sd=off"));
    tracing_subscriber::fmt().with_env_filter(filtre).init();
    rivaldsend_app::run_tauri();
}
