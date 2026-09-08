use serde::Serialize;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub transfer_id: String,
    pub bytes_done: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub eta_secs: u64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerDiscoveredEvent {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub fingerprint_short: String,
    pub trusted: bool,
    pub platform: String,
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
