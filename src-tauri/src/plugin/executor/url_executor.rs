use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};
use crate::utils::windows::shell_execute_open;
use tracing::warn;

/// URL 执行器 - 负责使用系统默认浏览器打开 URL
/// 使用 ShellExecuteW 直接打开 URL，避免 cmd.exe 对特殊字符的错误解析
pub struct UrlExecutor;

impl UrlExecutor {
    pub fn new() -> Self {
        Self
    }

    /// 使用系统默认浏览器打开 URL
    fn execute_url(&self, url: &str) -> Result<(), ExecutionError> {
        shell_execute_open(url).map_err(|e| {
            let msg = format!("启动 URL 失败：{:?}", e);
            warn!("{}", msg);
            ExecutionError::Failed(msg)
        })
    }
}

impl Configurable for UrlExecutor {
    fn component_id(&self) -> &str {
        "url-executor"
    }

    fn component_name(&self) -> &str {
        "URL执行器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Launcher
    }
}

impl Default for UrlExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionExecutor for UrlExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::Url]
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: "execute".to_string(),
            label: "打开".to_string(),
            icon: String::new(),
            is_default: true,
            shortcut_key: String::new(),
        }]
    }

    fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
        let url = match &ctx.target {
            ExecutionTarget::Url(u) => u,
            _ => {
                return Err(ExecutionError::Failed(
                    "Invalid target type for UrlExecutor".into(),
                ))
            }
        };

        match action_id {
            "execute" => self.execute_url(url),
            _ => Err(ExecutionError::UnsupportedAction(
                TargetType::Url,
                action_id.to_string(),
            )),
        }
    }
}
