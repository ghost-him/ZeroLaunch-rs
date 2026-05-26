use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};
use crate::sdk::host_api::PluginHandle;
use crate::sdk::IconRequest;
use async_trait::async_trait;
use std::sync::Arc;

/// 命令执行器 - 负责执行自定义命令
/// 通过 PluginHandle::shell_execute_command 委托 SDK 层执行，不直接调用平台 API
pub struct CommandExecutor {
    plugin_handle: Arc<PluginHandle>,
}

impl CommandExecutor {
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        Self { plugin_handle }
    }
}

impl Configurable for CommandExecutor {
    fn component_id(&self) -> &str {
        "command-executor"
    }

    fn component_name(&self) -> &str {
        "命令执行器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::ActionExecutor
    }
}

#[async_trait]
impl ActionExecutor for CommandExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::Command]
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: "execute".to_string(),
            label: "执行".to_string(),
            icon: IconRequest::Path(String::new()),
            is_default: true,
            shortcut_key: String::new(),
        }]
    }

    async fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
        let template = match &ctx.target {
            ExecutionTarget::Command(cmd) => cmd.as_str(),
            _ => {
                return Err(ExecutionError::Failed(
                    "Invalid target type for CommandExecutor".into(),
                ))
            }
        };

        match action_id {
            "execute" => {
                // 解析模板，替换 {} 用户参数和 {clip}/{hwnd}/{selection} 系统参数
                let resolved = self
                    .plugin_handle
                    .resolve_parameters(template, &ctx.user_args, &ctx.parameter_snapshot)
                    .await
                    .map_err(|e| ExecutionError::Failed(format!("命令参数解析失败: {}", e)))?;

                let handle = self.plugin_handle.clone();
                handle
                    .shell_execute_command(&resolved)
                    .await
                    .map_err(|e| ExecutionError::Failed(format!("命令执行失败: {}", e)))
            }
            _ => Err(ExecutionError::UnsupportedAction(
                TargetType::Command,
                action_id.to_string(),
            )),
        }
    }
}
