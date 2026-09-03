use serde::Serialize;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    pub transfer_id: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
}
pub struct ProgressEmitter {
    last: Mutex<Option<Instant>>,
    interval: Duration,
}
impl ProgressEmitter {
    pub fn new() -> Self {
        Self { last: Mutex::new(None), interval: Duration::from_millis(250) }
    }
    pub async fn should_emit(&self) -> bool {
        let mut g = self.last.lock().await;
        let now = Instant::now();
        if let Some(prev) = *g {
            if now.duration_since(prev) < self.interval {
                return false;
            }
        }
        *g = Some(now);
        true
    }
}
impl Default for ProgressEmitter {
    fn default() -> Self { Self::new() }
}
