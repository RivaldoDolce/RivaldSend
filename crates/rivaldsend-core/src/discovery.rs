use crate::error::CoreError;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;

pub const SERVICE_TYPE: &str = "_rivaldsend._tcp.local.";
pub const SERVICE_NAME_PREFIX: &str = "rivaldsend-";

#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub instance_name: String,
    pub hostname: String,
    pub port: u16,
    pub addresses: Vec<std::net::IpAddr>,
    pub txt: HashMap<String, String>,
}

pub struct Discovery {
    daemon: ServiceDaemon,
}

impl Discovery {
    pub fn new() -> Result<Self, CoreError> {
        let daemon = ServiceDaemon::new().map_err(|e| CoreError::IoString(e.to_string()))?;
        Ok(Self { daemon })
    }

    pub fn register(
        &self,
        device_name: &str,
        port: u16,
        fingerprint_short: &str,
        protocol_version: &str,
    ) -> Result<ServiceInfo, CoreError> {
        let instance = format!("{}{}", SERVICE_NAME_PREFIX, sanitize_label(device_name));
        let host = format!("{instance}.local.");
        let props: HashMap<String, String> = HashMap::from([
            ("protocol_version".into(), protocol_version.into()),
            ("device_name".into(), device_name.into()),
            ("fingerprint_short".into(), fingerprint_short.into()),
            ("platform".into(), std::env::consts::OS.into()),
        ]);
        let info = ServiceInfo::new(SERVICE_TYPE, &instance, &host, "", port, props)
            .map_err(|e| CoreError::IoString(e.to_string()))?;
        self.daemon
            .register(info.clone())
            .map_err(|e| CoreError::IoString(e.to_string()))?;
        Ok(info)
    }

    pub fn browse(&self) -> Result<mdns_sd::ServiceDaemon, CoreError> {
        Ok(self.daemon.clone())
    }

    pub fn shutdown(self) -> Result<(), CoreError> {
        self.daemon
            .shutdown()
            .map_err(|e| CoreError::IoString(e.to_string()))?;
        Ok(())
    }
}

/// Rend un nom d'appareil utilisable comme libelle DNS mDNS.
///
/// Un nom d'hote de systeme n'est pas forcement un libelle DNS valide : les
/// separateurs underscore sont courants sous Linux, et Android peut renvoyer
/// un nom aleatoire. `ServiceInfo::new` echoue sur ces caracteres, ce qui
/// faisait tomber toute l'application au demarrage. On remplace donc tout ce
/// qui n'est pas alphanumerique ou tiret, on borne a 63 octets (limite d'un
/// libelle DNS) et on garantit un resultat non vide.
pub fn sanitize_label(nom: &str) -> String {
    let mut sortie = String::with_capacity(nom.len());
    for caractere in nom.chars() {
        if sortie.len() >= 60 {
            break;
        }
        if caractere.is_ascii_alphanumeric() || caractere == '-' {
            sortie.push(caractere);
        } else {
            sortie.push('-');
        }
    }
    let sortie = sortie.trim_matches('-').to_ascii_lowercase();
    if sortie.is_empty() {
        "rivaldsend".to_string()
    } else {
        sortie
    }
}

/// Choisit l'adresse IP a utiliser pour joindre un pair annonce.
///
/// Le serveur n'ecoute qu'en IPv4 (`0.0.0.0`), donc une adresse IPv6 decouverte
/// ne mènerait nulle part : on ne retient que les IPv4 routables. Sans ce tri,
/// la premiere adresse trouvée pouvait être une IPv6 (liée au locale ou globale)
/// et le pair restait injoignable alors qu'il etait correctement annonce.
pub fn choisir_adresse(les_adresses: &[std::net::IpAddr]) -> Option<String> {
    let mut ipv4 = None;
    for adresse in les_adresses {
        match adresse {
            std::net::IpAddr::V4(v4) => {
                if v4.is_loopback() || v4.is_unspecified() {
                    continue;
                }
                // On prefere une adresse de reseau local prive.
                let octets = v4.octets();
                let prive = octets[0] == 10
                    || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                    || (octets[0] == 192 && octets[1] == 168);
                if prive {
                    return Some(v4.to_string());
                }
                if ipv4.is_none() {
                    ipv4 = Some(v4.to_string());
                }
            }
            std::net::IpAddr::V6(_) => {
                // Le serveur n'ecoute pas en IPv6 : inutile de la proposer.
                continue;
            }
        }
    }
    ipv4
}

pub fn list_interfaces() -> Vec<(String, std::net::IpAddr)> {
    let mut out = Vec::new();
    if let Ok(ifaces) = if_addrs::get_if_addrs() {
        for iface in ifaces {
            let ip = iface.ip();
            if ip.is_loopback() {
                continue;
            }
            if iface.name.starts_with("docker")
                || iface.name.starts_with("br-")
                || iface.name.contains("veth")
            {
                continue;
            }
            if let std::net::IpAddr::V6(v6) = ip {
                if v6.is_unicast_link_local() {
                    continue;
                }
            }
            out.push((iface.name, ip));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_label_accepte_les_noms_hotes_reels() {
        // Noms qui cassaient `ServiceInfo::new` avant.
        assert_eq!(sanitize_label("poste_de_bureau"), "poste-de-bureau");
        assert_eq!(sanitize_label("Mon PC"), "mon-pc");
        assert_eq!(sanitize_label("riva_01.local"), "riva-01-local");
        assert_eq!(sanitize_label("android-1234"), "android-1234");
        // Cas degrades : jamais vide, jamais de tiret en bord.
        assert_eq!(sanitize_label("___"), "rivaldsend");
        assert_eq!(sanitize_label(""), "rivaldsend");
        assert_eq!(sanitize_label("--x--"), "x");
    }

    #[test]
    fn sanitize_label_borne_la_longueur_dns() {
        let long = "a".repeat(200);
        let sortie = sanitize_label(&long);
        assert!(sortie.len() <= 63, "longueur {} > 63", sortie.len());
    }

    #[test]
    fn choisir_adresse_privilege_l_ipv4_du_lan() {
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
        let adresses = vec![
            IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42)),
        ];
        assert_eq!(
            choisir_adresse(&adresses),
            Some("192.168.1.42".to_string()),
            "l'IPv4 du LAN doit primer sur toute IPv6"
        );
    }

    #[test]
    fn choisir_adresse_ecarte_loopback_et_non_specifie() {
        use std::net::{IpAddr, Ipv4Addr};
        let adresses = vec![
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        ];
        assert_eq!(choisir_adresse(&adresses), None);

        let publique = vec![IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7))];
        assert_eq!(choisir_adresse(&publique), Some("203.0.113.7".into()));
    }

    #[test]
    fn choisir_adresse_ne_propose_jamais_d_ipv6() {
        use std::net::{IpAddr, Ipv6Addr};
        let adresses = vec![IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1))];
        assert_eq!(
            choisir_adresse(&adresses),
            None,
            "le serveur n'ecoute qu'en IPv4"
        );
    }
}
