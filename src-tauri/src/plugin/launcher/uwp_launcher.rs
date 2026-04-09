use crate::plugin_system::types::{
    LaunchError, LaunchMethod, LaunchMethodType, Launcher, ResultAction,
};

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

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![
            ResultAction {
                id: "launch".to_string(),
                label: "打开".to_string(),
                icon: String::new(),
                is_default: true,
            },
            ResultAction {
                id: "launch_admin".to_string(),
                label: "以管理员身份运行".to_string(),
                icon: String::new(),
                is_default: false,
            },
        ]
    }

    fn execute(&self, _method: &LaunchMethod, action_id: &str) -> Result<(), LaunchError> {
        match action_id {
            "launch" => todo!("UwpLauncher::launch 尚未实现"),
            "launch_admin" => todo!("UwpLauncher::launch_admin 尚未实现"),
            _ => Err(LaunchError::UnsupportedAction(action_id.to_string())),
        }
    }
}
