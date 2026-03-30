use crate::plugin_system::types::{LaunchError, LaunchMethod, LaunchMethodType, Launcher};

pub struct UwpLauncher;

impl UwpLauncher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UwpLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl Launcher for UwpLauncher {
    fn supported_method(&self) -> LaunchMethodType {
        LaunchMethodType::PackageFamilyName
    }

    fn launch(&self, _method: &LaunchMethod) -> Result<(), LaunchError> {
        todo!("UwpLauncher::launch 尚未实现")
    }
}
