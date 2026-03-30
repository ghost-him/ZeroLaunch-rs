use crate::plugin_system::types::{LaunchError, LaunchMethod, LaunchMethodType, Launcher};

pub struct UrlLauncher;

impl UrlLauncher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UrlLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl Launcher for UrlLauncher {
    fn supported_method(&self) -> LaunchMethodType {
        LaunchMethodType::Url
    }

    fn launch(&self, _method: &LaunchMethod) -> Result<(), LaunchError> {
        todo!("UrlLauncher::launch 尚未实现")
    }
}
