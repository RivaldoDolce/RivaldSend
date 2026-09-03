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
    semaphore: Arc<Semaphore>,
    resume_dir: std::path::PathBuf,
}
impl TransferManager {
    pub fn new(resume_dir: std::path::PathBuf) -> Self {
        Self { queue: tokio::sync::Mutex::new(Queue::new()), statuses: tokio::sync::Mutex::new(HashMap::new()), semaphore: Arc::new(Semaphore::new(2)), resume_dir }
    }
    pub async fn enqueue(&self, path: std::path::PathBuf) -> Uuid {
        let id = Uuid::new_v4();
        let mut q = self.queue.lock().await;
        q.push(QueuedTransfer { id, path });
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
        let partial_dir = std::path::PathBuf::from(format!("/tmp/rivaldsend-partial/{id}"));
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
}
