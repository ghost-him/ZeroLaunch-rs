pub mod capabilities;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::WindowsAppEnumerator;

#[cfg(target_os = "windows")]
pub use windows::WindowsAutoStartManager;

#[cfg(target_os = "windows")]
pub use windows::WindowsHotkeyManager;

#[cfg(target_os = "windows")]
pub use windows::WindowsAppLauncher;

#[cfg(target_os = "windows")]
pub use windows::WindowsClipboardProvider;

#[cfg(target_os = "windows")]
pub use windows::WindowsIconExtractor;

#[cfg(target_os = "windows")]
pub use windows::WindowsInstallationMonitor;

#[cfg(target_os = "windows")]
pub use windows::WindowsLnkResolver;

#[cfg(target_os = "windows")]
pub use windows::WindowsPathResolver;

#[cfg(target_os = "windows")]
pub use windows::WindowsResourceLoader;

#[cfg(target_os = "windows")]
pub use windows::WindowsSelectionProvider;

#[cfg(target_os = "windows")]
pub use windows::WindowsShellExecutor;

#[cfg(target_os = "windows")]
pub use windows::WindowsWindowHandleProvider;

#[cfg(target_os = "windows")]
pub use windows::WindowsWindowManager;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
compile_error!("Plugin SDK 暂不支持当前平台");
