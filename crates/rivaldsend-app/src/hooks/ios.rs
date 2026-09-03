use std::path::PathBuf;
pub struct IosHooks;
impl IosHooks {
    pub fn download_dir() -> PathBuf { PathBuf::from("/var/mobile/Containers/Data/Documents") }
    pub fn local_network_permission() -> bool { true }
}
