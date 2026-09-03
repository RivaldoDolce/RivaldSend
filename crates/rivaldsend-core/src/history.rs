use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub transfer_id: Uuid,
    pub file_count: usize,
    pub total_bytes: u64,
    pub peer: String,
    pub direction: String,
    pub status: String,
    pub timestamp: DateTime<Utc>,
}
pub async fn append(path: &std::path::Path, entry: &HistoryEntry) -> Result<(), crate::error::CoreError> {
    let mut line = serde_json::to_vec(entry)?;
    line.push(b'\n');
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut file = tokio::fs::OpenOptions::new().create(true).append(true).open(path).await?;
    use tokio::io::AsyncWriteExt;
    file.write_all(&line).await?;
    file.sync_data().await?;
    Ok(())
}
pub async fn load_all(path: &std::path::Path) -> Result<Vec<HistoryEntry>, crate::error::CoreError> {
    let data = match tokio::fs::read_to_string(path).await {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut out = Vec::new();
    for line in data.lines() {
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str(line)?);
    }
    Ok(out)
}
