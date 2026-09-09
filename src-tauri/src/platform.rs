//! Compile-time platform selection for HostApi dependency injection.
//!
//! 宿主侧平台差异的唯一收敛点：两平台 crate 实现各自能力，本模块按
//! `cfg(target_os)` 重导出为统一别名。宿主其余代码只消费这里的
//! `Platform*` 类型与自由函数，不出现第二处条件编译。

#[cfg(target_os = "windows")]
pub use zerolaunch_platform_windows::{
    init_com, is_foreground_fullscreen, os_version, start_system_theme_monitor,
    windows_capabilities as platform_capabilities, WindowsAppEnumerator as PlatformAppEnumerator,
    WindowsAppLauncher as PlatformAppLauncher, WindowsAutoStartManager as PlatformAutoStartManager,
    WindowsClipboardManager as PlatformClipboardManager,
    WindowsClipboardProvider as PlatformClipboardProvider,
    WindowsFocusMonitor as PlatformFocusMonitor, WindowsHotkeyManager as PlatformHotkeyManager,
    WindowsIconExtractor as PlatformIconExtractor,
    WindowsInstallationMonitor as PlatformInstallationMonitor,
    WindowsLnkResolver as PlatformLnkResolver, WindowsPathResolver as PlatformPathResolver,
    WindowsResourceLoader as PlatformResourceLoader,
    WindowsSelectionProvider as PlatformSelectionProvider,
    WindowsShellExecutor as PlatformShellExecutor, WindowsThemeProvider as PlatformThemeProvider,
    WindowsWindowHandleProvider as PlatformWindowHandleProvider,
    WindowsWindowManager as PlatformWindowManager,
    WindowsWindowPositioner as PlatformWindowPositioner,
};

#[cfg(target_os = "macos")]
pub use zerolaunch_platform_macos::{
    init_com, is_foreground_fullscreen, macos_capabilities as platform_capabilities, os_version,
    start_system_theme_monitor, MacosAppEnumerator as PlatformAppEnumerator,
    MacosAppLauncher as PlatformAppLauncher, MacosAutoStartManager as PlatformAutoStartManager,
    MacosClipboardManager as PlatformClipboardManager,
    MacosClipboardProvider as PlatformClipboardProvider, MacosFocusMonitor as PlatformFocusMonitor,
    MacosHotkeyManager as PlatformHotkeyManager, MacosIconExtractor as PlatformIconExtractor,
    MacosInstallationMonitor as PlatformInstallationMonitor,
    MacosLnkResolver as PlatformLnkResolver, MacosPathResolver as PlatformPathResolver,
    MacosResourceLoader as PlatformResourceLoader,
    MacosSelectionProvider as PlatformSelectionProvider,
    MacosShellExecutor as PlatformShellExecutor, MacosThemeProvider as PlatformThemeProvider,
    MacosWindowHandleProvider as PlatformWindowHandleProvider,
    MacosWindowManager as PlatformWindowManager, MacosWindowPositioner as PlatformWindowPositioner,
};
