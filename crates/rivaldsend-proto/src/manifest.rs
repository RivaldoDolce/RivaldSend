use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::messages::SemVer;
pub use crate::messages::SemVer as SemVerWrapper;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileMode {
    File,
    Directory,
    Symlink,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub relative_path: String,
    pub size: u64,
    pub blake3: String,
    pub mode: FileMode,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferManifest {
    pub transfer_id: Uuid,
    pub protocol_version: SemVer,
    pub files: Vec<ManifestEntry>,
    pub total_bytes: u64,
    pub created_at: DateTime<Utc>,
}
impl TransferManifest {
    pub fn validate(&self) -> Result<(), crate::error::ProtoError> {
        crate::validation::validate_manifest(self)
    }
}
