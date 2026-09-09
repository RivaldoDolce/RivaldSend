use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use uuid::Uuid;
use crate::error::CoreError;
use crate::queue::{Queue, QueuedTransfer};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferStatus { Queued, Running, Completed, Failed(String), Paused }
pub struct TransferManager {
    queue: tokio::sync::Mutex<Queue>,
    statuses: tokio::sync::Mutex<HashMap<Uuid, TransferStatus>>,
    targets: tokio::sync::Mutex<HashMap<Uuid, String>>,
    semaphore: Arc<Semaphore>,
    resume_dir: std::path::PathBuf,
    dossier_telechargement: tokio::sync::RwLock<std::path::PathBuf>,
}
impl TransferManager {
    pub fn new(resume_dir: std::path::PathBuf) -> Self {
        let dossier_defaut = Self::default_download_dir();
        Self { queue: tokio::sync::Mutex::new(Queue::new()), statuses: tokio::sync::Mutex::new(HashMap::new()), targets: tokio::sync::Mutex::new(HashMap::new()), semaphore: Arc::new(Semaphore::new(2)), resume_dir, dossier_telechargement: tokio::sync::RwLock::new(dossier_defaut) }
    }
    pub fn default_resume_dir() -> std::path::PathBuf {
        dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp")).join("rivaldsend-resume")
    }
    pub fn default_partial_dir(id: Uuid) -> std::path::PathBuf {
        dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp")).join(format!("rivaldsend-partial/{id}"))
    }
    pub fn resume_path(&self, id: Uuid) -> std::path::PathBuf {
        self.resume_dir.join(format!("{id}.json"))
    }
    pub async fn enqueue(&self, path: std::path::PathBuf) -> Uuid {
        let id = Uuid::new_v4();
        let mut q = self.queue.lock().await;
        q.push(QueuedTransfer { id, path, priority: 0 });
        let mut s = self.statuses.lock().await;
        s.insert(id, TransferStatus::Queued);
        id
    }
    pub async fn start_transfer(&self, id: Uuid) -> Result<(), CoreError> {
        let permit = self.semaphore.clone().try_acquire_owned().map_err(|_| CoreError::QueueFull)?;
        {
            let mut s = self.statuses.lock().await;
            s.insert(id, TransferStatus::Running);
        }
        tokio::spawn(async move { let _p = permit; });
        Ok(())
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
    pub async fn resume(&self, id: Uuid) -> Result<Option<crate::resume::ResumeState>, CoreError> {
        let path = self.resume_dir.join(format!("{id}.json"));
        crate::resume::load(&path).await
    }
    pub async fn status(&self, id: &Uuid) -> Option<TransferStatus> {
        self.statuses.lock().await.get(id).cloned()
    }
    pub async fn cancel(&self, id: Uuid) -> Result<(), CoreError> {
        let mut s = self.statuses.lock().await;
        if s.remove(&id).is_none() {
            return Err(CoreError::NotFound(id.to_string()));
        }
        s.insert(id, TransferStatus::Failed("cancelled".into()));
        let path = self.resume_dir.join(format!("{id}.json"));
        let _ = tokio::fs::remove_file(&path).await;
        let partial_dir = Self::default_partial_dir(id);
        let _ = tokio::fs::remove_dir_all(&partial_dir).await;
        Ok(())
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
            if let Ok(state) = crate::resume::load(&path).await {
                if let Some(s) = state {
                    if s.last_updated < cutoff {
                        let _ = tokio::fs::remove_file(&path).await;
                        removed += 1;
                    }
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

        // Met à jour le statut.
        let mut s = self.statuses.lock().await;
        if let Some(statut) = s.get_mut(&id) {
            *statut = TransferStatus::Completed;
        }

        Ok(destination)
    }

    pub async fn set_default_download_dir(&self, dir: std::path::PathBuf) -> Result<(), CoreError> {
        // Crée le dossier puis le mémorise pour les prochaines réceptions.
        let _ = tokio::fs::create_dir_all(&dir).await?;
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
