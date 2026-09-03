use std::net::IpAddr;
pub trait PlatformHooks: Send + Sync {
    fn store_secret(&self, service: &str, key: &str, value: &[u8]) -> Result<(), String>;
    fn load_secret(&self, service: &str, key: &str) -> Result<Option<Vec<u8>>, String>;
    fn notify(&self, title: &str, body: &str);
    fn network_interfaces(&self) -> Vec<IpAddr>;
}
pub struct NoopHooks;
impl PlatformHooks for NoopHooks {
    fn store_secret(&self, _s: &str, _k: &str, _v: &[u8]) -> Result<(), String> { Ok(()) }
    fn load_secret(&self, _s: &str, _k: &str) -> Result<Option<Vec<u8>>, String> { Ok(None) }
    fn notify(&self, _t: &str, _b: &str) {}
    fn network_interfaces(&self) -> Vec<IpAddr> { Vec::new() }
}
