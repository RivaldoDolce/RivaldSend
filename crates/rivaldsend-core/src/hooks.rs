use std::path::PathBuf;
pub trait PlatformHooks: Send + Sync {
    fn keystore_get(&self, _key: &str) -> Option<Vec<u8>>;
    fn keystore_set(&self, _key: &str, _value: &[u8]) -> Result<(), String>;
    fn notify(&self, title: &str, body: &str);
    fn download_dir(&self) -> PathBuf;
    fn network_interfaces(&self) -> Vec<(String, String)>;
}
pub struct NoopHooks;
impl PlatformHooks for NoopHooks {
    fn keystore_get(&self, _k: &str) -> Option<Vec<u8>> { None }
    fn keystore_set(&self, _k: &str, _v: &[u8]) -> Result<(), String> { Ok(()) }
    fn notify(&self, _t: &str, _b: &str) {}
    fn download_dir(&self) -> PathBuf { std::env::temp_dir() }
    fn network_interfaces(&self) -> Vec<(String, String)> { vec![] }
}
pub struct DesktopHooks;
impl PlatformHooks for DesktopHooks {
    fn keystore_get(&self, key: &str) -> Option<Vec<u8>> {
        keyring::Entry::new("rivaldsend", key).ok()?.get_password().ok().map(|s| s.into_bytes())
    }
    fn keystore_set(&self, key: &str, value: &[u8]) -> Result<(), String> {
        let s = String::from_utf8_lossy(value).to_string();
        keyring::Entry::new("rivaldsend", key).map_err(|e| e.to_string())?.set_password(&s).map_err(|e| e.to_string())
    }
    fn notify(&self, title: &str, body: &str) { let _ = (title, body); }
    fn download_dir(&self) -> PathBuf { dirs::download_dir().unwrap_or_else(std::env::temp_dir) }
    fn network_interfaces(&self) -> Vec<(String, String)> { crate::discovery::list_interfaces().into_iter().map(|(n,ip)| (n, ip.to_string())).collect() }
}
