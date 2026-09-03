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
        Self { queue: tokio::sync::Mutex::new(Queue::new()), statuses: tokio::sync::Mutex::new(HashMap::new()), semaphore: Arc::new(Semaphore::new(2)), resume_dir: resume_dir }
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
}
