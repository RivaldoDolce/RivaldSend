use crate::error::CoreError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkProfile { Private, Public, Domain, Unknown }

#[cfg(windows)]
pub fn detect_windows_firewall() -> Result<NetworkProfile, CoreError> {
    let sortie = std::process::Command::new("powershell")
        .args(["-Command", "Get-NetConnectionProfile | Select-Object -ExpandProperty NetworkCategory"])
        .output()
        .map_err(|e| CoreError::IoString(e.to_string()))?;
    let texte = String::from_utf8_lossy(&sortie.stdout).to_lowercase();
    if texte.contains("public") { Ok(NetworkProfile::Public) }
    else if texte.contains("private") { Ok(NetworkProfile::Private) }
    else if texte.contains("domain") { Ok(NetworkProfile::Domain) }
    else { Ok(NetworkProfile::Unknown) }
}

#[cfg(not(windows))]
pub fn detect_windows_firewall() -> Result<NetworkProfile, CoreError> {
    Ok(NetworkProfile::Private)
}

/// Détection du profil réseau sur macOS via scutil.
/// Heuristique fragile : à durcir avec le vrai état du réseau.
#[cfg(target_os = "macos")]
pub fn detect_network_profile() -> Result<NetworkProfile, CoreError> {
    let sortie = std::process::Command::new("scutil")
        .args(["--nwi"])
        .output()
        .map_err(|e| CoreError::IoString(e.to_string()))?;
    let texte = String::from_utf8_lossy(&sortie.stdout).to_lowercase();
    if texte.contains("public") { Ok(NetworkProfile::Public) }
    else if texte.contains("private") { Ok(NetworkProfile::Private) }
    else if texte.contains("domain") { Ok(NetworkProfile::Domain) }
    else { Ok(NetworkProfile::Unknown) }
}

/// Détection du profil réseau sur Linux via NetworkManager.
/// Secours : présence d'une interface filaire ou wifi active, considérée comme privée.
#[cfg(target_os = "linux")]
pub fn detect_network_profile() -> Result<NetworkProfile, CoreError> {
    // Essaie d'abord NetworkManager.
    let sortie = std::process::Command::new("nmcli")
        .args(["-t", "-f", "TYPE,DEVICE,STATE", "device"])
        .output()
        .map_err(|e| CoreError::IoString(e.to_string()))?;
    let texte = String::from_utf8_lossy(&sortie.stdout);
    // Recherche un réseau marqué public ou non fiable.
    if texte.contains("public") || texte.contains("untrusted") {
        return Ok(NetworkProfile::Public);
    }
    // Secours : vérifie les interfaces dans /sys/class/net.
    if let Ok(peripheriques) = std::fs::read_dir("/sys/class/net/") {
        for entree in peripheriques.flatten() {
            let nom = entree.file_name();
            let nom_texte = nom.to_string_lossy();
            if nom_texte.starts_with("eth") || nom_texte.starts_with("wlan") || nom_texte.starts_with("wlp") || nom_texte.starts_with("enp") {
                if let Ok(etat) = std::fs::read_to_string(
                    format!("/sys/class/net/{nom_texte}/operstate")
                ) {
                    if etat.trim() == "up" {
                        return Ok(NetworkProfile::Private);
                    }
                }
            }
        }
    }
    Ok(NetworkProfile::Unknown)
}

/// Détection du profil réseau sur Windows (délègue au pare-feu Windows).
#[cfg(target_os = "windows")]
pub fn detect_network_profile() -> Result<NetworkProfile, CoreError> {
    detect_windows_firewall()
}

/// Détection sur Android : pas de notion public/privé accessible simplement.
/// On ne bloque jamais le serveur sur mobile, le partage y est explicite.
#[cfg(target_os = "android")]
pub fn detect_network_profile() -> Result<NetworkProfile, CoreError> {
    Ok(NetworkProfile::Private)
}

/// Détection générique : appelle la méthode adaptée à la plateforme.
pub fn detect_network_profile_generic() -> Result<NetworkProfile, CoreError> {
    #[cfg(target_os = "macos")]
    {
        detect_network_profile()
    }
    #[cfg(target_os = "linux")]
    {
        detect_network_profile()
    }
    #[cfg(target_os = "windows")]
    {
        detect_windows_firewall()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        // Secours pour les autres plateformes.
        Ok(NetworkProfile::Private)
    }
}

pub fn should_block_server(profil: &NetworkProfile) -> bool {
    matches!(profil, NetworkProfile::Public)
}
