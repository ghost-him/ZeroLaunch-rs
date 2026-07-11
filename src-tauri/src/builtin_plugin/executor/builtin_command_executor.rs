use async_trait::async_trait;
use std::sync::Arc;
use zerolaunch_plugin_api::config::{ComponentCore, ComponentType, Configurable};
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};

use crate::core::app_command;

/// 内置命令执行器 - 将 BuiltinCommand 候选项的确认操作为 AppCommand，通过全局通道发送给消费者。
///
/// 不持有 HostApi（特权对象），也不持有任何通道发送端。
/// 真正的执行逻辑在 bootstrap.rs 中 spawn 的消费者 task 中完成。
/// 使用 `app_command::send()` 全局函数（OnceLock 保护的写一次基础设施）
/// 而非依赖注入的通道，因为：
/// - `InventoryContext` 的工厂签名是函数指针（非闭包），无法捕获外部变量；
/// - 通过 `InventoryContext` 层层传递会让 PluginManager 等中间人承载不相关的依赖；
/// - 命令通道是应用基础设施，有且仅有一个消费者，`OnceLock` 是最小耦合面。
///   详细设计讨论见 `core/app_command.rs` 顶部注释。
pub struct BuiltinCommandExecutor {
    core: ComponentCore,
}

impl BuiltinCommandExecutor {
    /// 创建内置命令执行器。无参数——命令通道通过全局 `app_command::send()` 访问。
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "builtin-command-executor".to_string(),
                "内置命令执行器".to_string(),
                "执行 ZeroLaunch 内置命令".to_string(),
                ComponentType::ActionExecutor,
                10,
            ),
        }
    }
}

impl Default for BuiltinCommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Configurable for BuiltinCommandExecutor {
    fn core(&self) -> &ComponentCore {
        &self.core
    }
}

#[async_trait]
impl ActionExecutor for BuiltinCommandExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::BuiltinCommand]
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
        let command = match &ctx.target {
            ExecutionTarget::BuiltinCommand(cmd) => cmd.as_str(),
            _ => {
                return Err(ExecutionError::Failed(
                    "Invalid target type for BuiltinCommandExecutor".into(),
                ))
            }
        };

        match action_id {
            "execute" => {
                let app_cmd = match command {
                    "ShowSettings" => app_command::AppCommand::ShowSettings,
                    "RefreshDatabase" => app_command::AppCommand::RefreshCandidates,
                    "ReregisterHotkeys" => app_command::AppCommand::ReregisterHotkeys,
                    "ToggleGameMode" => app_command::AppCommand::ToggleGameMode,
                    "ExitProgram" => app_command::AppCommand::ExitProgram,
                    _ => {
                        return Err(ExecutionError::Failed(format!(
                            "Unknown builtin command: {}",
                            command
                        )))
                    }
                };

                // 通过全局通道发送命令（OnceLock 保护的写一次基础设施）
                app_command::send(app_cmd);
                Ok(())
            }
            _ => Err(ExecutionError::UnsupportedAction(
                TargetType::BuiltinCommand,
                action_id.to_string(),
            )),
        }
    }
}

use crate::plugin_framework::builtin_registry::{ExecutorEntry, InventoryContext};

pub(crate) fn build_builtin_command_executor(
    _ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn ActionExecutor>) {
    let exec: Arc<dyn ActionExecutor> = Arc::new(BuiltinCommandExecutor::new());
    let configurable: Arc<dyn Configurable> = exec.clone();
    (configurable, exec)
}

::inventory::submit! {
    ExecutorEntry {
        component_id: "builtin-command-executor",
        handle_key: "builtin-command-executor",
        priority: 10,
        factory: build_builtin_command_executor,
    }
}
