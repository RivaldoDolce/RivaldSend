//! Magasin de confiance : les pairs apparies, avec le secret partage derive
//! du code d'appairage.
//!
//! C'est ce qui manquait pour que l'appairage tienne dans la duree. Le code
//! affiche est ephemere et le magasin de pairs cote interface n'etait pas
//! persiste ; apres un redemarrage, plus aucun transfert n'etait possible
//! sans ressaisir le code. Ici, l'appairage ecrit un secret long-terme derive
//! du code, ce qui permet de deriver la PSK de chaque transfert sans
//! reressaisir quoi que ce soit, et de pouvoir oublier un appareil.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

use crate::error::CoreError;
use crate::pipeline::writer::atomic_write;

/// Nombre maximum de pairs memorises. Au-dela, l'ajout le plus ancien est
/// evince : un magasin de confiance ne doit pas grossir sans fin.
pub const MAX_PAIRS: usize = 64;

/// Un pair avec lequel nous avons partage un secret d'appairage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedPeer {
    /// Identifiant stable derive de l'adresse servie (voir `pairing::id_de_pair`).
    pub id: String,
    /// `device_id` persistant du pair, echange via `/v1/identity`.
    /// C'est ce qui permet de le reconnaitre apres un changement d'adresse IP.
    pub device_id: String,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub fingerprint_short: String,
    pub platform: String,
    /// Secret partage, en hexadecimal (64 caracteres). Derivé du code
    /// d'appairage, jamais transmis sur le reseau.
    pub secret: String,
    #[serde(default)]
    pub paired_at: Option<DateTime<Utc>>,
}

impl TrustedPeer {
    /// Identifiant de cache derivable de l'adresse, compare a `pairing::id_de_pair`.
    pub fn id_pour(ip: &str, port: u16) -> String {
        crate::pairing::id_de_pair(ip, port)
    }
}

/// Magasin de confiance persiste sur disque (JSON).
pub struct TrustStore {
    path: PathBuf,
    pairs: RwLock<Vec<TrustedPeer>>,
}

impl TrustStore {
    /// Charge le magasin depuis `dossier/trusted-peers.json`.
    ///
    /// Ne Echoue jamais : un fichier absent, illisible ou corrompu donne un
    /// magasin vide et un avertissement. Perdre la liste des pairs apparies
    /// est penible mais ne doit pas empecher l'application de demarrer.
    pub fn charger(dossier: &Path) -> Self {
        let path = dossier.join("trusted-peers.json");
        let pairs = match std::fs::read(&path) {
            Ok(octets) => match serde_json::from_slice::<Vec<TrustedPeer>>(&octets) {
                Ok(pairs) => {
                    tracing::info!("{} pairs de confiance charges", pairs.len());
                    pairs
                }
                Err(e) => {
                    tracing::warn!("magasin de confiance illisible ({e}), demarrage a vide");
                    Vec::new()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => {
                tracing::warn!("magasin de confiance inaccessible ({e}), demarrage a vide");
                Vec::new()
            }
        };
        Self {
            path,
            pairs: RwLock::new(pairs),
        }
    }

    pub fn chemin(&self) -> &Path {
        &self.path
    }

    pub async fn lister(&self) -> Vec<TrustedPeer> {
        self.pairs.read().await.clone()
    }

    pub async fn par_id(&self, id: &str) -> Option<TrustedPeer> {
        self.pairs.read().await.iter().find(|p| p.id == id).cloned()
    }

    /// Retrouve un pair par `device_id`. C'est la recherche la plus utile en
    /// pratique : l'adresse IP change au DHCP, l'identifiant non.
    pub async fn par_device_id(&self, device_id: &str) -> Option<TrustedPeer> {
        if device_id.is_empty() {
            return None;
        }
        self.pairs
            .read()
            .await
            .iter()
            .find(|p| p.device_id == device_id)
            .cloned()
    }

    pub async fn par_adresse(&self, ip: &str, port: u16) -> Option<TrustedPeer> {
        self.pairs
            .read()
            .await
            .iter()
            .find(|p| p.ip == ip && p.port == port)
            .cloned()
    }

    /// Ajoute ou met a jour un pair, puis persiste.
    ///
    /// L'upsert se fait par `device_id` : re-appairer le meme appareil apres
    /// un changement d'IP met a jour l'existant plutot que d'en creer un
    /// doublon. Le `id` est ensuite recalcule depuis l'adresse courante.
    pub async fn enregistrer(&self, pair: TrustedPeer) -> Result<(), CoreError> {
        let mut pairs = self.pairs.write().await;
        if let Some(existant) = pairs
            .iter_mut()
            .find(|p| !pair.device_id.is_empty() && p.device_id == pair.device_id)
        {
            *existant = pair;
        } else if pairs.len() >= MAX_PAIRS {
            // Evince le plus ancien appairage, pas le moins fiable : tous les
            // pairs memorises ont ete apparies explicitement.
            if let Some(idx) = pairs
                .iter()
                .enumerate()
                .min_by_key(|(_, p)| p.paired_at)
                .map(|(i, _)| i)
            {
                pairs.remove(idx);
            }
            pairs.push(pair);
        } else {
            pairs.push(pair);
        }
        self.persister(&pairs).await
    }

    /// Oublie un pair. `true` si le pair etait bien memorise.
    ///
    /// Appele depuis l'interface (« oublier cet appareil ») : le pair est
    /// retire du magasin, donc ses transferts exigeront un nouvel appairage.
    pub async fn oublier(&self, id: &str) -> Result<bool, CoreError> {
        let mut pairs = self.pairs.write().await;
        let avant = pairs.len();
        pairs.retain(|p| p.id != id);
        let retire = pairs.len() != avant;
        if retire {
            self.persister(&pairs).await?;
        }
        Ok(retire)
    }

    /// Vide tout le magasin. Renvoie le nombre de pairs oublies.
    pub async fn oublier_tout(&self) -> Result<usize, CoreError> {
        let mut pairs = self.pairs.write().await;
        let n = pairs.len();
        pairs.clear();
        self.persister(&pairs).await?;
        Ok(n)
    }

    async fn persister(&self, pairs: &[TrustedPeer]) -> Result<(), CoreError> {
        let donnees = serde_json::to_vec_pretty(pairs)?;
        atomic_write(&self.path, &donnees).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(id: &str, device: &str, ip: &str) -> TrustedPeer {
        TrustedPeer {
            id: id.to_string(),
            device_id: device.to_string(),
            name: "Poste".into(),
            ip: ip.into(),
            port: 53317,
            fingerprint_short: "ABCD1234".into(),
            platform: "linux".into(),
            secret: "ab".repeat(32),
            paired_at: Some(Utc::now()),
        }
    }

    #[tokio::test]
    async fn un_pair_ajoute_est_retrouve_puis_persiste() {
        let dossier = tempfile::tempdir().unwrap();
        let magasin = TrustStore::charger(dossier.path());
        magasin
            .enregistrer(pair("peer-192.168.1.5:53317", "device-1", "192.168.1.5"))
            .await
            .unwrap();

        assert_eq!(magasin.lister().await.len(), 1);
        assert!(magasin.par_adresse("192.168.1.5", 53317).await.is_some());
        assert!(magasin.par_device_id("device-1").await.is_some());

        // Rechargement depuis le disque : la confiance survit au redemarrage.
        let reloaded = TrustStore::charger(dossier.path());
        assert_eq!(reloaded.lister().await.len(), 1);
        assert_eq!(
            reloaded.par_device_id("device-1").await.unwrap().secret,
            "ab".repeat(32)
        );
    }

    #[tokio::test]
    async fn reappairer_une_autre_ip_met_a_jour_sans_doublon() {
        let dossier = tempfile::tempdir().unwrap();
        let magasin = TrustStore::charger(dossier.path());
        magasin
            .enregistrer(pair("peer-192.168.1.5:53317", "device-1", "192.168.1.5"))
            .await
            .unwrap();
        // Changement d'adresse DHCP pour le meme appareil.
        magasin
            .enregistrer(pair("peer-192.168.1.77:53317", "device-1", "192.168.1.77"))
            .await
            .unwrap();

        let pairs = magasin.lister().await;
        assert_eq!(pairs.len(), 1, "pas de doublon sur device_id");
        assert_eq!(pairs[0].ip, "192.168.1.77");
        assert_eq!(magasin.par_adresse("192.168.1.5", 53317).await, None);
    }

    #[tokio::test]
    async fn oublier_un_pair_le_retire_du_disque() {
        let dossier = tempfile::tempdir().unwrap();
        let magasin = TrustStore::charger(dossier.path());
        magasin
            .enregistrer(pair("peer-192.168.1.5:53317", "device-1", "192.168.1.5"))
            .await
            .unwrap();

        assert!(magasin.oublier("peer-192.168.1.5:53317").await.unwrap());
        assert!(!magasin.oublier("peer-192.168.1.5:53317").await.unwrap());
        assert!(magasin.lister().await.is_empty());

        let reloaded = TrustStore::charger(dossier.path());
        assert!(reloaded.lister().await.is_empty(), "l'oubli est persiste");
    }

    #[tokio::test]
    async fn tout_oublier_vide_le_magasin() {
        let dossier = tempfile::tempdir().unwrap();
        let magasin = TrustStore::charger(dossier.path());
        for i in 0..3 {
            magasin
                .enregistrer(pair(
                    &format!("peer-10.0.0.{i}:53317"),
                    &format!("device-{i}"),
                    &format!("10.0.0.{i}"),
                ))
                .await
                .unwrap();
        }
        assert_eq!(magasin.oublier_tout().await.unwrap(), 3);
        assert!(magasin.lister().await.is_empty());
    }

    #[tokio::test]
    async fn un_magasin_corrompu_demarre_vide_sans_paniquer() {
        let dossier = tempfile::tempdir().unwrap();
        std::fs::write(dossier.path().join("trusted-peers.json"), b"{ pas du json").unwrap();
        let magasin = TrustStore::charger(dossier.path());
        assert!(magasin.lister().await.is_empty());
        // Et il reste utilisable.
        magasin
            .enregistrer(pair("peer-10.0.0.1:53317", "device-1", "10.0.0.1"))
            .await
            .unwrap();
        assert_eq!(magasin.lister().await.len(), 1);
    }

    #[tokio::test]
    async fn le_magasin_ne_depasse_pas_la_borne() {
        let dossier = tempfile::tempdir().unwrap();
        let magasin = TrustStore::charger(dossier.path());
        for i in 0..(MAX_PAIRS + 10) {
            magasin
                .enregistrer(pair(
                    &format!("peer-10.0.{i}.1:53317"),
                    &format!("device-{i}"),
                    &format!("10.0.{i}.1"),
                ))
                .await
                .unwrap();
        }
        assert_eq!(magasin.lister().await.len(), MAX_PAIRS);
    }
}
