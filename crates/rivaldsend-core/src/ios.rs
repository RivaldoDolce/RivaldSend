pub fn handle_background_suspension() -> &'static str {
    "Garder l'app ouverte pendant le transfert — iOS coupe les sockets en arrière-plan"
}
pub fn local_network_troubleshooting() -> Vec<&'static str> {
    vec![
        "Réglages > Confidentialité > Réseau local > Activer RivaldSend",
        "Si mDNS échoue, utilisez le QR ou l'IP manuelle",
        "Gardez l'app au premier plan pendant le transfert",
    ]
}
