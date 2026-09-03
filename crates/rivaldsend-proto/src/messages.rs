use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub fingerprint: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiateRequest {
    pub protocol_version: String,
    pub features: Vec<String>,
    pub device: DeviceInfo,
    pub session_id: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiateResponse {
    pub accepted: bool,
    pub error: Option<String>,
    pub min_supported: String,
    pub max_supported: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkAck {
    pub offset: u64,
    pub size: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferManifest {
    pub transfer_id: Uuid,
    pub protocol_version: crate::manifest::SemVerWrapper,
    pub files: Vec<crate::manifest::ManifestEntry>,
    pub total_bytes: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
