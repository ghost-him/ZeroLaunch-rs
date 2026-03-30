use crate::plugin_system::types::{LaunchError, LaunchMethod, LaunchMethodType, Launcher};

pub struct CommandLauncher;

impl CommandLauncher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CommandLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl Launcher for CommandLauncher {
    fn supported_method(&self) -> LaunchMethodType {
        LaunchMethodType::Command
    }

    fn launch(&self, _method: &LaunchMethod) -> Result<(), LaunchError> {
        todo!("CommandLauncher::launch 尚未实现")
    }
}
