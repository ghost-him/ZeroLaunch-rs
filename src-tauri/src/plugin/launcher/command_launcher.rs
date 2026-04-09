use crate::plugin_system::types::{
    LaunchError, LaunchMethod, LaunchMethodType, Launcher, ResultAction,
};

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

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: "launch".to_string(),
            label: "执行".to_string(),
            icon: String::new(),
            is_default: true,
        }]
    }

    fn execute(&self, _method: &LaunchMethod, action_id: &str) -> Result<(), LaunchError> {
        match action_id {
            "launch" => todo!("CommandLauncher::launch 尚未实现"),
            _ => Err(LaunchError::UnsupportedAction(action_id.to_string())),
        }
    }
}
