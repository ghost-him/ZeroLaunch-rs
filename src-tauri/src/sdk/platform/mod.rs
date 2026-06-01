pub mod capabilities;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsAppEnumerator;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsAppLauncher;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsAutoStartManager;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsClipboardProvider;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsFocusMonitor;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsHotkeyManager;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsIconExtractor;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsInstallationMonitor;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsLnkResolver;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsPathResolver;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsResourceLoader;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsSelectionProvider;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsShellExecutor;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsWindowHandleProvider;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsWindowManager;

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::WindowsWindowPositioner;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
compile_error!("Plugin SDK 暂不支持当前平台");
