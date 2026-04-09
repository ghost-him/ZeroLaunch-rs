use crate::plugin_system::types::{
    LaunchError, LaunchMethod, LaunchMethodType, Launcher, ResultAction,
};

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
            ResultAction {
                id: "open_folder".to_string(),
                label: "打开所在文件夹".to_string(),
                icon: String::new(),
                is_default: false,
            },
        ]
    }

    fn execute(&self, _method: &LaunchMethod, action_id: &str) -> Result<(), LaunchError> {
        match action_id {
            "launch" => todo!("PathLauncher::launch 尚未实现"),
            "launch_admin" => todo!("PathLauncher::launch_admin 尚未实现"),
            "open_folder" => todo!("PathLauncher::open_folder 尚未实现"),
            _ => Err(LaunchError::UnsupportedAction(action_id.to_string())),
        }
    }
}
