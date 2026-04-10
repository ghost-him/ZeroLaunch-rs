use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    LaunchError, LaunchMethod, LaunchMethodType, Launcher, ResultAction,
};
use std::os::windows::process::CommandExt;
use tracing::{debug, warn};

/// 命令启动器 - 负责执行自定义命令
/// 使用 cmd.exe 执行用户配置的命令字符串
pub struct CommandLauncher;

impl CommandLauncher {
    pub fn new() -> Self {
        Self
    }

    /// 执行命令
    /// 使用 cmd /D /S /C 执行命令字符串
    /// /D: 禁用 AutoRun 注册表键
    /// /S: 修改 /C 后字符串的处理方式
    /// /C: 执行字符串指定的命令后终止
    fn execute_command(&self, command: &str) -> Result<(), LaunchError> {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;

        let command = command.trim();
        if command.is_empty() {
            return Err(LaunchError::Failed("命令为空".to_string()));
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
                Err(LaunchError::Failed(msg))
            }
        }
    }
}

impl Configurable for CommandLauncher {
    fn component_id(&self) -> &str {
        "command-launcher"
    }

    fn component_name(&self) -> &str {
        "命令启动器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Launcher
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

    fn execute(&self, method: &LaunchMethod, action_id: &str) -> Result<(), LaunchError> {
        let command = match method {
            LaunchMethod::Command(cmd) => cmd,
            _ => {
                return Err(LaunchError::Failed(
                    "Invalid launch method for CommandLauncher".into(),
                ))
            }
        };

        match action_id {
            "launch" => self.execute_command(command),
            _ => Err(LaunchError::UnsupportedAction(action_id.to_string())),
        }
    }
}
