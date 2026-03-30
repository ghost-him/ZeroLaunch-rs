use crate::plugin_system::types::{LaunchError, LaunchMethod, LaunchMethodType, Launcher};

pub struct PathLauncher;

impl PathLauncher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PathLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl Launcher for PathLauncher {
    fn supported_method(&self) -> LaunchMethodType {
        LaunchMethodType::Path
    }

    fn launch(&self, _method: &LaunchMethod) -> Result<(), LaunchError> {
        todo!("PathLauncher::launch 尚未实现")
    }
}
