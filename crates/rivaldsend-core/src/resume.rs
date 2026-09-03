use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::pipeline::writer::atomic_write;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeState {
    pub transfer_id: Uuid,
    pub validated_offsets: Vec<rivaldsend_proto::ChunkAck>,
    pub last_updated: DateTime<Utc>,
}
impl ResumeState {
    pub fn new(transfer_id: Uuid) -> Self {
        Self { transfer_id, validated_offsets: Vec::new(), last_updated: Utc::now() }
    }
    pub fn add_ack(&mut self, ack: rivaldsend_proto::ChunkAck) {
        self.validated_offsets.push(ack);
        self.last_updated = Utc::now();
    }
}
pub async fn load(path: &std::path::Path) -> Result<Option<ResumeState>, crate::error::CoreError> {
    match tokio::fs::read(path).await {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
pub async fn save(path: &std::path::Path, state: &ResumeState) -> Result<(), crate::error::CoreError> {
    let data = serde_json::to_vec(state)?;
    atomic_write(path, &data).await?;
    Ok(())
}
