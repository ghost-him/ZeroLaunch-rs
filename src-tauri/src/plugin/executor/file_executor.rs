use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};
use crate::sdk::host_api::{OpenTarget, PluginHandle};
use std::sync::Arc;
use tracing::warn;

/// 文件执行器 - 负责使用系统默认方式打开文件
/// 支持打开文件和打开所在文件夹
pub struct FileExecutor {
    plugin_handle: Arc<PluginHandle>,
}

impl FileExecutor {
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        Self { plugin_handle }
    }

    /// 使用系统默认程序打开文件
    fn execute_file(&self, file_name: &str) -> Result<(), ExecutionError> {
        let handle = self.plugin_handle.clone();
        let file_name = file_name.to_string();
        tokio::spawn(async move {
            if let Err(e) = handle.shell_open(OpenTarget::File(file_name)).await {
                warn!("启动文件失败: {}", e);
            }
        });
        Ok(())
    }

    /// 打开目标文件所在的文件夹
    fn open_folder(&self, file_name: &str) -> Result<(), ExecutionError> {
        let handle = self.plugin_handle.clone();
        let file_name = file_name.to_string();
        tokio::spawn(async move {
            if let Err(e) = handle.shell_open_folder(&file_name).await {
                warn!("打开文件夹失败: {}", e);
            }
        });
        Ok(())
    }
}

impl Configurable for FileExecutor {
    fn component_id(&self) -> &str {
        "file-executor"
    }

    fn component_name(&self) -> &str {
        "文件执行器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::ActionExecutor
    }
}

impl Default for FileExecutor {
    fn default() -> Self {
        panic!("FileExecutor 必须通过 new(plugin_handle) 创建，不支持 Default");
    }
}

impl ActionExecutor for FileExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::File]
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![
            ResultAction {
                id: "execute".to_string(),
                label: "打开".to_string(),
                icon: String::new(),
                is_default: true,
                shortcut_key: String::new(),
            },
            ResultAction {
                id: "open_folder".to_string(),
                label: "打开所在文件夹".to_string(),
                icon: String::new(),
                is_default: false,
                shortcut_key: String::new(),
            },
        ]
    }

    fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
        let file_name = match &ctx.target {
            ExecutionTarget::File(f) => f,
            _ => {
                return Err(ExecutionError::Failed(
                    "Invalid target type for FileExecutor".into(),
                ))
            }
        };

        match action_id {
            "execute" => self.execute_file(file_name),
            "open_folder" => self.open_folder(file_name),
            _ => Err(ExecutionError::UnsupportedAction(
                TargetType::File,
                action_id.to_string(),
            )),
        }
    }
}
