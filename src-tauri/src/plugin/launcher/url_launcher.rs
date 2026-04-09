use crate::plugin_system::types::{
    LaunchError, LaunchMethod, LaunchMethodType, Launcher, ResultAction,
};

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

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: "launch".to_string(),
            label: "打开".to_string(),
            icon: String::new(),
            is_default: true,
        }]
    }

    fn execute(&self, _method: &LaunchMethod, action_id: &str) -> Result<(), LaunchError> {
        match action_id {
            "launch" => todo!("UrlLauncher::launch 尚未实现"),
            _ => Err(LaunchError::UnsupportedAction(action_id.to_string())),
        }
    }
}
