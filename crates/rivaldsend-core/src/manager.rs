use crate::error::CoreError;
use crate::queue::{Queue, QueuedTransfer};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferStatus {
    Queued,
    Running,
    /// Transfert annonce par un pair distant : en attente de morceaux.
    Receiving,
    Completed,
    Failed(String),
    Paused,
}

impl TransferStatus {
    /// Un transfert est-il encore actif (en cours ou en attente) ?
    pub fn est_actif(&self) -> bool {
        matches!(
            self,
            TransferStatus::Queued
                | TransferStatus::Running
                | TransferStatus::Receiving
                | TransferStatus::Paused
        )
    }

    /// Un transfert a-t-il abouti ?
    pub fn est_reussi(&self) -> bool {
        matches!(self, TransferStatus::Completed)
    }
}

pub struct TransferManager {
    queue: tokio::sync::Mutex<Queue>,
    statuses: tokio::sync::Mutex<HashMap<Uuid, TransferStatus>>,
    targets: tokio::sync::Mutex<HashMap<Uuid, String>>,
    codes: tokio::sync::Mutex<HashMap<Uuid, String>>,
    /// Permis maintenus vivants aussi longtemps que le transfert, pour que la
    /// limite de concurrence porte enfin sur les transferts et pas sur
    /// l'instant de leur demarrage.
    permis: tokio::sync::Mutex<HashMap<Uuid, OwnedSemaphorePermit>>,
    semaphore: Arc<Semaphore>,
    resume_dir: std::path::PathBuf,
    dossier_telechargement: tokio::sync::RwLock<std::path::PathBuf>,
}
impl TransferManager {
    pub fn new(resume_dir: std::path::PathBuf) -> Self {
        let dossier_defaut = Self::default_download_dir();
        Self {
            queue: tokio::sync::Mutex::new(Queue::new()),
            statuses: tokio::sync::Mutex::new(HashMap::new()),
            targets: tokio::sync::Mutex::new(HashMap::new()),
            codes: tokio::sync::Mutex::new(HashMap::new()),
            permis: tokio::sync::Mutex::new(HashMap::new()),
            semaphore: Arc::new(Semaphore::new(2)),
            resume_dir,
            dossier_telechargement: tokio::sync::RwLock::new(dossier_defaut),
        }
    }
    /// Repertoire de l'application : un seul point de depart pour le TLS,
    /// l'historique, la reprise et les pairs de confiance.
    pub fn dossier_app() -> std::path::PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("rivaldsend")
    }
    pub fn default_resume_dir() -> std::path::PathBuf {
        Self::dossier_app().join("resume")
    }
    pub fn default_partial_dir(id: Uuid) -> std::path::PathBuf {
        Self::dossier_app().join("partial").join(id.to_string())
    }
    pub fn resume_path(&self, id: Uuid) -> std::path::PathBuf {
        self.resume_dir.join(format!("{id}.json"))
    }
    pub async fn enqueue(&self, path: std::path::PathBuf) -> Uuid {
        let id = Uuid::new_v4();
        let mut q = self.queue.lock().await;
        q.push(QueuedTransfer {
            id,
            path,
            priority: 0,
        });
        drop(q);
        let mut s = self.statuses.lock().await;
        s.insert(id, TransferStatus::Queued);
        drop(s);
        // Essayer de demarrer immediatement si un slot de concurrence est libre.
        let _ = self.start_transfer(id).await;
        id
    }

    /// Demarre le prochain transfert en attente dans la queue, si un slot est
    /// libre. Appele par [`marquer_termine`](Self::marquer_termine) et
    /// [`cancel`](Self::cancel) quand un transfert se termine.
    pub async fn process_queue(&self) {
        let next = {
            let mut q = self.queue.lock().await;
            q.pop()
        };
        let Some(queued) = next else {
            return;
        };
        let id = queued.id;
        // Remettre le statut en Queued (il etait peut-etre en Running si
        // preemptif) puis essayer de demarrer.
        {
            let mut s = self.statuses.lock().await;
            s.insert(id, TransferStatus::Queued);
        }
        let _ = self.start_transfer(id).await;
        if self.status(&id).await != Some(TransferStatus::Running) {
            // Echec (semaphore plein ou autre) : remettre en queue pour
            // une tentative ultérieure.
            let mut q = self.queue.lock().await;
            q.push(queued);
        }
    }
    pub async fn start_transfer(&self, id: Uuid) -> Result<(), CoreError> {
        // Le permis est stocke et rendu quand le transfert se termine : sans
        // cela la tache le lachait aussitot et la limite ne valait rien.
        let permit = self
            .semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|_| CoreError::QueueFull)?;
        {
            let mut s = self.statuses.lock().await;
            s.insert(id, TransferStatus::Running);
        }
        self.permis.lock().await.insert(id, permit);
        Ok(())
    }

    /// Enregistre un transfert **entrant** annonce par un pair distant.
    ///
    /// Sans cet enregistrement, l'application receptrice ne connaitait pas le
    /// transfert : `put_chunk` le rejetait systematiquement en « transfert
    /// introuvable », donc aucun octet n'arrivait jamais. L'appel se fait
    /// depuis `POST /v1/transfers`, sur le processus qui recoit.
    pub async fn enregistrer_entrant(&self, id: Uuid, pair_id: Option<String>) {
        let mut s = self.statuses.lock().await;
        s.insert(id, TransferStatus::Receiving);
        drop(s);
        if let Some(pair) = pair_id {
            self.targets.lock().await.insert(id, pair);
        }
    }

    /// Met en pause un transfert en cours
    pub async fn pause_transfer(&self, id: Uuid) -> Result<(), CoreError> {
        let mut statuses = self.statuses.lock().await;
        match statuses.get_mut(&id) {
            Some(TransferStatus::Running) => {
                statuses.insert(id, TransferStatus::Paused);
                tracing::info!("Transfer {} paused", id);
                Ok(())
            }
            Some(_) => Err(CoreError::Validation("transfer not running".into())),
            None => Err(CoreError::NotFound(id.to_string())),
        }
    }
    /// Reprend un transfert en pause
    pub async fn resume_transfer(&self, id: Uuid) -> Result<(), CoreError> {
        let mut statuses = self.statuses.lock().await;
        match statuses.get_mut(&id) {
            Some(TransferStatus::Paused) => {
                statuses.insert(id, TransferStatus::Running);
                tracing::info!("Transfer {} resumed", id);
                // Relancer le pipeline sera fait par le worker
                Ok(())
            }
            Some(_) => Err(CoreError::Validation("transfer not paused".into())),
            None => Err(CoreError::NotFound(id.to_string())),
        }
    }
    /// Associe un peer cible au transfert
    pub async fn set_target_peer(&self, id: Uuid, peer_id: String) {
        self.targets.lock().await.insert(id, peer_id);
    }
    /// Retourne le peer cible associé au transfert, le cas échéant
    pub async fn target_peer(&self, id: &Uuid) -> Option<String> {
        self.targets.lock().await.get(id).cloned()
    }
    /// Mémorise le code d'appairage saisi pour un transfert sortant.
    /// Utilisé par le futur travailleur d'envoi pour dériver la même PSK que le receveur.
    pub async fn set_pairing_code(&self, id: Uuid, code: String) {
        self.codes.lock().await.insert(id, code);
    }
    /// Retourne le code d'appairage mémorisé pour un transfert, le cas échéant.
    pub async fn pairing_code(&self, id: &Uuid) -> Option<String> {
        self.codes.lock().await.get(id).cloned()
    }
    /// Marque un transfert sortant comme terminé (succès côté expéditeur).
    pub async fn marquer_termine(&self, id: Uuid) {
        {
            let mut s = self.statuses.lock().await;
            if let Some(statut) = s.get_mut(&id) {
                *statut = TransferStatus::Completed;
            }
        }
        // Libere le slot de concurrence.
        self.permis.lock().await.remove(&id);
        // Relancer le prochain transfert en attente, si aucun.
        self.process_queue().await;
    }
    pub async fn resume(&self, id: Uuid) -> Result<Option<crate::resume::ResumeState>, CoreError> {
        let path = self.resume_dir.join(format!("{id}.json"));
        crate::resume::load(&path).await
    }
    pub async fn status(&self, id: &Uuid) -> Option<TransferStatus> {
        self.statuses.lock().await.get(id).cloned()
    }
    /// Annule un transfert actif.
    ///
    /// Refuse un transfert deja termine, deja annule ou inconnu : sans cette
    /// garde, un second appel trouvait l'entree `Failed("cancelled")` posee par
    /// le premier, Jaffichait de nouveau les donnees et renvoyait un succes
    /// trompeur a l'interface.
    pub async fn cancel(&self, id: Uuid) -> Result<(), CoreError> {
        {
            let mut s = self.statuses.lock().await;
            match s.get(&id) {
                None => return Err(CoreError::NotFound(id.to_string())),
                Some(statut) if !statut.est_actif() => {
                    return Err(CoreError::Validation(format!(
                        "transfert {id} deja termine ({statut:?})"
                    )))
                }
                Some(_) => {}
            }
            s.insert(id, TransferStatus::Failed("cancelled".into()));
        }
        self.permis.lock().await.remove(&id);
        self.codes.lock().await.remove(&id);
        self.targets.lock().await.remove(&id);
        // Relancer le prochain transfert en attente, si aucun.
        self.process_queue().await;
        let path = self.resume_dir.join(format!("{id}.json"));
        let _ = tokio::fs::remove_file(&path).await;
        let partial_dir = Self::default_partial_dir(id);
        let _ = tokio::fs::remove_dir_all(&partial_dir).await;
        Ok(())
    }

    /// Oublie un transfert : efface statut, code, cible et donnees partielles.
    ///
    /// Distinct de [`Self::cancel`] qui marque un echec : ici on veut effacer
    /// proprement, typiquement apres une annulation de l'utilisateur, sans
    /// laisser d'entree fantome en memoire.
    pub async fn oublier(&self, id: Uuid) {
        {
            let mut s = self.statuses.lock().await;
            s.remove(&id);
        }
        self.permis.lock().await.remove(&id);
        self.codes.lock().await.remove(&id);
        self.targets.lock().await.remove(&id);
        let path = self.resume_dir.join(format!("{id}.json"));
        let _ = tokio::fs::remove_file(&path).await;
        let partial_dir = Self::default_partial_dir(id);
        let _ = tokio::fs::remove_dir_all(&partial_dir).await;
    }

    /// Transfers encore actifs. Utilise par l'interface et les tests pour
    /// savoir si un redemarrage doit proposer une reprise.
    pub async fn transferts_actifs(&self) -> Vec<(Uuid, TransferStatus)> {
        self.statuses
            .lock()
            .await
            .iter()
            .filter(|(_, statut)| statut.est_actif())
            .map(|(id, statut)| (*id, statut.clone()))
            .collect()
    }

    pub async fn cleanup_stale(&self, max_age_days: u64) -> Result<usize, CoreError> {
        let mut removed = 0;
        let mut dir = match tokio::fs::read_dir(&self.resume_dir).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e.into()),
        };
        let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if let Ok(Some(s)) = crate::resume::load(&path).await {
                if s.last_updated < cutoff {
                    let _ = tokio::fs::remove_file(&path).await;
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }

    pub async fn finalize_transfer(
        &self,
        id: Uuid,
        destination: std::path::PathBuf,
    ) -> Result<std::path::PathBuf, CoreError> {
        // Livre le contenu reçu vers la destination finale, puis nettoie.
        let dossier_partiel = Self::default_partial_dir(id);
        let source = dossier_partiel.join("data.bin");
        if !source.exists() {
            return Err(CoreError::NotFound(format!("données manquantes pour {id}")));
        }
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        // Copie puis suppression : simple et robuste entre volumes.
        tokio::fs::copy(&source, &destination).await?;
        let _ = tokio::fs::remove_dir_all(&dossier_partiel).await;
        let chemin_reprise = self.resume_path(id);
        let _ = tokio::fs::remove_file(&chemin_reprise).await;

        // Met à jour le statut et libere le slot de concurrence.
        {
            let mut s = self.statuses.lock().await;
            if let Some(statut) = s.get_mut(&id) {
                *statut = TransferStatus::Completed;
            }
        }
        self.permis.lock().await.remove(&id);

        Ok(destination)
    }

    pub async fn set_default_download_dir(&self, dir: std::path::PathBuf) -> Result<(), CoreError> {
        // Crée le dossier puis le mémorise pour les prochaines réceptions.
        tokio::fs::create_dir_all(&dir).await?;
        let mut garde = self.dossier_telechargement.write().await;
        *garde = dir;
        Ok(())
    }

    pub async fn get_download_dir(&self) -> std::path::PathBuf {
        self.dossier_telechargement.read().await.clone()
    }

    pub fn default_download_dir() -> std::path::PathBuf {
        dirs::download_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp")))
    }

    /// Résout un chemin relatif par rapport au dossier de téléchargement configuré.
    pub async fn resolve_download_path(&self, relatif: &str) -> std::path::PathBuf {
        let base = self.get_download_dir().await;
        if let Some(reste) = relatif.strip_prefix("~/") {
            base.join(reste)
        } else if relatif.starts_with('/') {
            std::path::PathBuf::from(relatif)
        } else {
            base.join(relatif)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn le_code_appairage_est_memorise_par_transfert() {
        let dossier = tempfile::tempdir().unwrap();
        let gestionnaire = TransferManager::new(dossier.path().to_path_buf());
        let id = Uuid::new_v4();
        assert!(gestionnaire.pairing_code(&id).await.is_none());
        gestionnaire
            .set_pairing_code(id, "AB39XZ".to_string())
            .await;
        assert_eq!(
            gestionnaire.pairing_code(&id).await.as_deref(),
            Some("AB39XZ")
        );
    }

    #[tokio::test]
    async fn le_dossier_telechargement_est_memorise() {
        let dossier = tempfile::tempdir().unwrap();
        let gestionnaire = TransferManager::new(dossier.path().to_path_buf());
        let cible = dossier.path().join("telechargements");
        gestionnaire
            .set_default_download_dir(cible.clone())
            .await
            .unwrap();
        assert_eq!(gestionnaire.get_download_dir().await, cible);
        assert!(cible.exists());
    }

    #[tokio::test]
    async fn un_transfert_entrant_est_visible_pour_put_chunk() {
        // Regression : `put_chunk` rejetait tout transfert absent du manager,
        // or `create_transfer` ne l'y inscrivait pas. Aucun octet n'arrivait
        // donc jamais. L'enregistrement doit rendre le statut visible.
        let dossier = tempfile::tempdir().unwrap();
        let gestionnaire = TransferManager::new(dossier.path().to_path_buf());
        let id = Uuid::new_v4();
        assert!(
            gestionnaire.status(&id).await.is_none(),
            "inconnu avant l'enregistrement"
        );

        gestionnaire
            .enregistrer_entrant(id, Some("peer-192.168.1.5:53317".into()))
            .await;

        assert_eq!(
            gestionnaire.status(&id).await,
            Some(TransferStatus::Receiving),
            "put_chunk doit trouver le transfert"
        );
        assert_eq!(
            gestionnaire.target_peer(&id).await.as_deref(),
            Some("peer-192.168.1.5:53317")
        );
        assert!(gestionnaire
            .transferts_actifs()
            .await
            .contains(&(id, TransferStatus::Receiving)));
    }

    #[tokio::test]
    async fn la_concurrence_est_reellement_bornee() {
        // Le permis doit vivre aussi longtemps que le transfert. Avant, la
        // tache le lachait immediatement et la limite ne valait rien.
        let dossier = tempfile::tempdir().unwrap();
        let gestionnaire = TransferManager::new(dossier.path().to_path_buf());
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        gestionnaire.start_transfer(a).await.unwrap();
        gestionnaire.start_transfer(b).await.unwrap();
        assert!(
            gestionnaire.start_transfer(c).await.is_err(),
            "le troisieme transfert doit etre refuse (semaphore = 2)"
        );

        // Liberer un slot redonne une place.
        gestionnaire.marquer_termine(a).await;
        assert!(
            gestionnaire.start_transfer(c).await.is_ok(),
            "un slot libere doit etre reutilisable"
        );
    }

    #[tokio::test]
    async fn pauser_reprendre_annuler_ont_un_cycle_de_vie_realiste() {
        let dossier = tempfile::tempdir().unwrap();
        let gestionnaire = TransferManager::new(dossier.path().to_path_buf());
        let id = Uuid::new_v4();
        gestionnaire.start_transfer(id).await.unwrap();

        gestionnaire.pause_transfer(id).await.unwrap();
        assert_eq!(gestionnaire.status(&id).await, Some(TransferStatus::Paused));
        // Un second `pause` sur un transfert deja en pause doit etre refuse
        // plutot que silencieusement accepte.
        assert!(gestionnaire.pause_transfer(id).await.is_err());

        gestionnaire.resume_transfer(id).await.unwrap();
        assert_eq!(
            gestionnaire.status(&id).await,
            Some(TransferStatus::Running)
        );
        // Reprendre un transfert deja actif doit echouer aussi.
        assert!(gestionnaire.resume_transfer(id).await.is_err());
        // Un transfert inconnu n'existe pas.
        assert!(gestionnaire.pause_transfer(Uuid::new_v4()).await.is_err());
        assert!(gestionnaire.resume_transfer(Uuid::new_v4()).await.is_err());

        gestionnaire.cancel(id).await.unwrap();
        assert!(matches!(
            gestionnaire.status(&id).await,
            Some(TransferStatus::Failed(_))
        ));
        // Annuler deux fois doit echouer proprement.
        assert!(gestionnaire.cancel(id).await.is_err());
    }

    #[tokio::test]
    async fn oublier_efface_le_transfert_sans_laisser_de_fantome() {
        let dossier = tempfile::tempdir().unwrap();
        let gestionnaire = TransferManager::new(dossier.path().to_path_buf());
        let id = Uuid::new_v4();
        gestionnaire.enregistrer_entrant(id, None).await;
        gestionnaire.set_pairing_code(id, "AB39XZ".into()).await;

        gestionnaire.oublier(id).await;

        assert_eq!(gestionnaire.status(&id).await, None);
        assert_eq!(gestionnaire.pairing_code(&id).await, None);
        assert!(gestionnaire.transferts_actifs().await.is_empty());
    }
}
