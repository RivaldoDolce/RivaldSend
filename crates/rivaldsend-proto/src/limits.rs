use std::time::Duration;
pub const MAX_MANIFEST_FILES: usize = 100_000;
pub const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_TOTAL_TRANSFER_BYTES: u64 = 1 << 40;
pub const MAX_SINGLE_FILE_NAME_LEN: usize = 255;
pub const MAX_RELATIVE_PATH_DEPTH: usize = 32;
pub const MAX_CHUNK_SIZE: usize = 16 * 1024 * 1024;
pub const NEGOTIATE_TIMEOUT: Duration = Duration::from_secs(5);
pub const TRANSFER_IDLE_TIMEOUT: Duration = Duration::from_secs(60);
