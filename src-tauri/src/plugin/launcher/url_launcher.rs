use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    LaunchError, LaunchMethod, LaunchMethodType, Launcher, ResultAction,
};
use crate::utils::windows::shell_execute_open;
use tracing::warn;

/// URL 启动器 - 负责使用系统默认浏览器打开 URL
/// 使用 ShellExecuteW 直接打开 URL，避免 cmd.exe 对特殊字符的错误解析
pub struct UrlLauncher;

impl UrlLauncher {
    pub fn new() -> Self {
        Self
    }

    /// 使用系统默认浏览器打开 URL
    fn launch_url(&self, url: &str) -> Result<(), LaunchError> {
        shell_execute_open(url).map_err(|e| {
            let msg = format!("启动 URL 失败：{:?}", e);
            warn!("{}", msg);
            LaunchError::Failed(msg)
        })
    }
}

impl Configurable for UrlLauncher {
    fn component_id(&self) -> &str {
        "url-launcher"
    }

    fn component_name(&self) -> &str {
        "URL启动器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Launcher
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

    fn execute(&self, method: &LaunchMethod, action_id: &str) -> Result<(), LaunchError> {
        let url = match method {
            LaunchMethod::Url(u) => u,
            _ => {
                return Err(LaunchError::Failed(
                    "Invalid launch method for UrlLauncher".into(),
                ))
            }
        };

        match action_id {
            "launch" => self.launch_url(url),
            _ => Err(LaunchError::UnsupportedAction(action_id.to_string())),
        }
    }
}
