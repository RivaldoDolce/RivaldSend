use std::path::PathBuf;
pub struct DesktopHooks;
impl DesktopHooks {
    pub fn download_dir() -> PathBuf { dirs::download_dir().unwrap_or_else(std::env::temp_dir) }
}
