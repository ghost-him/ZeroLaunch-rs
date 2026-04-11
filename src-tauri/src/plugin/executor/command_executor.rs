use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};
use std::os::windows::process::CommandExt;
use tracing::{debug, warn};

/// 命令执行器 - 负责执行自定义命令
/// 使用 cmd.exe 执行用户配置的命令字符串
pub struct CommandExecutor;

impl CommandExecutor {
    pub fn new() -> Self {
        Self
    }

    /// 执行命令
    /// 使用 cmd /D /S /C 执行命令字符串
    /// /D: 禁用 AutoRun 注册表键
    /// /S: 修改 /C 后字符串的处理方式
    /// /C: 执行字符串指定的命令后终止
    fn execute_command(&self, command: &str) -> Result<(), ExecutionError> {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;

        let command = command.trim();
        if command.is_empty() {
            return Err(ExecutionError::Failed("命令为空".to_string()));
        }

        let result = std::process::Command::new("cmd")
            .args(["/D", "/S", "/C"])
            .raw_arg(command)
            .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
            .spawn();

        match result {
            Ok(_) => {
                debug!("命令启动成功: {}", command);
                Ok(())
            }
            Err(e) => {
                let msg = format!("命令启动失败: {:?}", e);
                warn!("{}", msg);
                Err(ExecutionError::Failed(msg))
            }
        }
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
        ComponentType::Launcher
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionExecutor for CommandExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::Command]
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: "execute".to_string(),
            label: "执行".to_string(),
            icon: String::new(),
            is_default: true,
            shortcut_key: String::new(),
        }]
    }

    fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
        let command = match &ctx.target {
            ExecutionTarget::Command(cmd) => cmd,
            _ => {
                return Err(ExecutionError::Failed(
                    "Invalid target type for CommandExecutor".into(),
                ))
            }
        };

        match action_id {
            "execute" => self.execute_command(command),
            _ => Err(ExecutionError::UnsupportedAction(
                TargetType::Command,
                action_id.to_string(),
            )),
        }
    }
}
