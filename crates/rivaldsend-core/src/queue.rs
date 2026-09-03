use std::collections::VecDeque;
use uuid::Uuid;
#[derive(Debug, Clone)]
pub struct QueuedTransfer {
    pub id: Uuid,
    pub path: std::path::PathBuf,
}
#[derive(Debug, Default)]
pub struct Queue {
    inner: VecDeque<QueuedTransfer>,
}
impl Queue {
    pub fn new() -> Self { Self { inner: VecDeque::new() } }
    pub fn push(&mut self, item: QueuedTransfer) { self.inner.push_back(item); }
    pub fn pop(&mut self) -> Option<QueuedTransfer> { self.inner.pop_front() }
    pub fn len(&self) -> usize { self.inner.len() }
    pub fn is_empty(&self) -> bool { self.inner.is_empty() }
}
