use thiserror::Error;
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("proto: {0}")]
    Proto(#[from] rivaldsend_proto::ProtoError),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("transfer not found: {0}")]
    NotFound(String),
    #[error("validation: {0}")]
    Validation(String),
    #[error("pairing: {0}")]
    Pairing(String),
    #[error("queue full")]
    QueueFull,
}
