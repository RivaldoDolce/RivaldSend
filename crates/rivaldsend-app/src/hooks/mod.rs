#[cfg(target_os = "android")]
pub mod android;
pub mod desktop;
pub use desktop::DesktopHooks;
