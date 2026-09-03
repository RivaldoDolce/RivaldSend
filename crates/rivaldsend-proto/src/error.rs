use thiserror::Error;
#[derive(Debug, Error)]
pub enum ProtoError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("limits exceeded: {0}")]
    Limits(String),
    #[error("version unsupported: {0}")]
    VersionUnsupported(String),
}
