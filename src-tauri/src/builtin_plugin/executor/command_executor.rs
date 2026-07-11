use async_trait::async_trait;
use std::sync::Arc;
use zerolaunch_plugin_api::config::{ComponentCore, ComponentType, Configurable};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};

/// 命令执行器 - 负责执行自定义命令
/// 通过 PluginHandle::shell_execute_command 委托 SDK 层执行，不直接调用平台 API
pub struct CommandExecutor {
    core: ComponentCore,
    plugin_handle: Arc<PluginHandle>,
}

impl CommandExecutor {
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        Self {
            core: ComponentCore::new(
                "command-executor".to_string(),
                "命令执行器".to_string(),
                "执行用户自定义的 Shell 命令".to_string(),
                ComponentType::ActionExecutor,
                40,
            ),
            plugin_handle,
        }
    }
}

impl Configurable for CommandExecutor {
    fn core(&self) -> &ComponentCore {
        &self.core
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

use crate::plugin_framework::builtin_registry::{ExecutorEntry, InventoryContext};

pub(crate) fn build_command_executor(
    ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn ActionExecutor>) {
    let handle = ctx.get_handle("command-executor");
    let exec: Arc<dyn ActionExecutor> = Arc::new(CommandExecutor::new(handle));
    let configurable: Arc<dyn Configurable> = exec.clone();
    (configurable, exec)
}

::inventory::submit! {
    ExecutorEntry {
        component_id: "command-executor",
        handle_key: "command-executor",
        priority: 40,
        factory: build_command_executor,
    }
}
