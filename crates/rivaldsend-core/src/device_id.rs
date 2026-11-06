use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn charger_ou_creer(dossier: &PathBuf) -> Self {
        let _ = std::fs::create_dir_all(dossier);
        let chemin = dossier.join("device_id.txt");
        if let Ok(contenu) = std::fs::read_to_string(&chemin) {
            let propre = contenu.trim().to_string();
            if Uuid::parse_str(&propre).is_ok() {
                return Self(propre);
            }
        }
        let identifiant = Uuid::new_v4().to_string();
        let _ = std::fs::write(&chemin, &identifiant);
        Self(identifiant)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn court(&self) -> String {
        self.0.chars().take(8).collect()
    }
}
