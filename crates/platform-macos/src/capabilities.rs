use std::collections::HashSet;
use zerolaunch_plugin_api::{PlatformCapabilities, PlatformCapability};

pub fn macos_capabilities() -> PlatformCapabilities {
    PlatformCapabilities::new(HashSet::from([
        PlatformCapability::IconExtraction,
        PlatformCapability::ShellOpen,
        PlatformCapability::AppEnumeration,
        PlatformCapability::AppLaunch,
        PlatformCapability::WindowActivation,
        PlatformCapability::AutoStart,
        PlatformCapability::HotkeyListening,
        PlatformCapability::InstallationMonitoring,
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_supported_macos_services() {
        let caps = macos_capabilities();
        assert!(caps.has(PlatformCapability::AppEnumeration));
        assert!(caps.has(PlatformCapability::AutoStart));
        assert!(!caps.has(PlatformCapability::RunAsAdmin));
    }
}
