//! Certificat auto-signé et configuration TLS du serveur HTTPS.
//!
//! Contexte : outil pair-à-pair sur réseau local, sans autorité de certification.
//! Chaque appareil génère son propre certificat auto-signé, conservé dans le
//! dossier de données applicatives. Le TLS chiffre le transport contre l'écoute
//! passive ; l'authentification réelle reste assurée par la PSK d'appairage
//! (en-tête X-PSK) et l'empreinte d'appareil échangée par QR (approche TOFU).

use std::path::PathBuf;

/// Noms présentés dans le certificat auto-signé.
fn noms_alternatifs() -> Vec<String> {
    let mut noms = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];
    for (_, ip) in rivaldsend_core::discovery::list_interfaces() {
        let texte = ip.to_string();
        if !noms.contains(&texte) {
            noms.push(texte);
        }
    }
    noms
}

fn dossier_tls() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("rivaldsend")
        .join("tls")
}

fn empreinte_sha256(der: &[u8]) -> String {
    // Empreinte affichable du certificat, pour une vérification hors bande.
    let resume = blake3::hash(der);
    resume.to_hex().to_string()
}

/// Charge le certificat existant ou en génère un nouveau, puis construit la
/// configuration serveur rustls. Retourne aussi l'empreinte BLAKE3 du DER.
pub fn assurer_config_tls() -> Result<(rustls::ServerConfig, String), String> {
    assurer_config_tls_dans(&dossier_tls())
}

/// Même chose dans un dossier injecté (tests, sans toucher aux vraies données).
pub fn assurer_config_tls_dans(
    dossier: &std::path::Path,
) -> Result<(rustls::ServerConfig, String), String> {
    let chemin_cert = dossier.join("cert.der");
    let chemin_cle = dossier.join("cle.der");

    if let (Ok(cert_der), Ok(cle_der)) = (
        std::fs::read(&chemin_cert),
        std::fs::read(&chemin_cle),
    ) {
        if let Ok(config) = construire_config(&cert_der, &cle_der) {
            let empreinte = empreinte_sha256(&cert_der);
            return Ok((config, empreinte));
        }
        // Certificat illisible : on en génère un nouveau plus bas.
        tracing::warn!("certificat TLS existant illisible, régénération");
    }

    let cles = rcgen::generate_simple_self_signed(noms_alternatifs())
        .map_err(|e| format!("génération du certificat : {e}"))?;
    let cert_der = cles.cert.der().to_vec();
    let cle_der = cles.key_pair.serialize_der();

    if let Some(parent) = chemin_cert.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Écriture best-effort : un échec ici n'empêche pas de servir avec le
    // certificat en mémoire pour cette session.
    let _ = std::fs::write(&chemin_cert, &cert_der);
    let _ = std::fs::write(&chemin_cle, &cle_der);

    let config = construire_config(&cert_der, &cle_der)?;
    let empreinte = empreinte_sha256(&cert_der);
    Ok((config, empreinte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_config_tls_se_construit_et_persiste() {
        // Le provider doit exister avant tout usage rustls (comme dans run_tauri).
        let _ = rustls::crypto::ring::default_provider().install_default();
        let dossier = tempfile::tempdir().unwrap();
        let (config, empreinte) = assurer_config_tls_dans(dossier.path()).unwrap();
        // Empreinte BLAKE3 hexadécimale de 64 caractères.
        assert_eq!(empreinte.len(), 64);
        assert!(empreinte.bytes().all(|c| c.is_ascii_hexdigit()));
        // Le certificat doit être réutilisé au second appel (même empreinte).
        let (_, empreinte2) = assurer_config_tls_dans(dossier.path()).unwrap();
        assert_eq!(empreinte, empreinte2);
        // Les fichiers persistent sur disque pour les prochains démarrages.
        assert!(dossier.path().join("cert.der").exists());
        assert!(dossier.path().join("cle.der").exists());
        // La config refuse les certificats vides (garde-fou du constructeur).
        assert!(construire_config(&[], &[]).is_err());
        let _ = config;
    }
}

fn construire_config(
    cert_der: &[u8],
    cle_der: &[u8],
) -> Result<rustls::ServerConfig, String> {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

    let chaine = vec![CertificateDer::from(cert_der.to_vec())];
    let cle = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cle_der.to_vec()));
    rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(chaine, cle)
        .map_err(|e| format!("configuration TLS invalide : {e}"))
}
