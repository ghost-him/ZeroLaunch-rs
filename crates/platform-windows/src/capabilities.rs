use std::collections::HashSet;
pub use zerolaunch_plugin_api::{PlatformCapabilities, PlatformCapability};

/// Windows 平台的完整能力集构造函数。
/// 返回包含所有 Windows 平台能力的 PlatformCapabilities。
#[cfg(target_os = "windows")]
pub fn windows_capabilities() -> PlatformCapabilities {
    PlatformCapabilities::new(HashSet::from([
        PlatformCapability::IconExtraction,
        PlatformCapability::ShellOpen,
        PlatformCapability::RunAsAdmin,
        PlatformCapability::AppEnumeration,
        PlatformCapability::AppLaunch,
        PlatformCapability::WindowActivation,
        PlatformCapability::AutoStart,
        PlatformCapability::HotkeyListening,
        PlatformCapability::InstallationMonitoring,
    ]))
}
