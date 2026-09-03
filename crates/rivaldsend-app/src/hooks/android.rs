use rivaldsend_core::hooks::PlatformHooks;
use std::path::PathBuf;

pub struct AndroidHooks;

impl PlatformHooks for AndroidHooks {
    fn keystore_get(&self, _key: &str) -> Option<Vec<u8>> { None }
    fn keystore_set(&self, _key: &str, _value: &[u8]) -> Result<(), String> { Ok(()) }
    fn notify(&self, title: &str, body: &str) {
        println!("[Android notification] {}: {}", title, body);
    }
    fn download_dir(&self) -> PathBuf { PathBuf::from("/storage/emulated/0/Download") }
    fn network_interfaces(&self) -> Vec<(String, String)> { rivaldsend_core::discovery::list_interfaces().into_iter().map(|(n,ip)| (n, ip.to_string())).collect() }
}
