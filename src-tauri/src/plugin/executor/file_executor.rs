use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};
use async_trait::async_trait;
use std::sync::Arc;
use zerolaunch_plugin_api::host::{OpenTarget, PluginHandle};
use zerolaunch_plugin_api::services::IconRequest;

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
    async fn execute_file(&self, file_name: &str) -> Result<(), ExecutionError> {
        let handle = self.plugin_handle.clone();
        let file_name = file_name.to_string();
        handle
            .shell_open(OpenTarget::File(file_name))
            .await
            .map_err(|e| ExecutionError::Failed(format!("启动文件失败: {}", e)))
    }

    /// 打开目标文件所在的文件夹
    async fn open_folder(&self, file_name: &str) -> Result<(), ExecutionError> {
        let handle = self.plugin_handle.clone();
        let file_name = file_name.to_string();
        handle
            .shell_open_folder(&file_name)
            .await
            .map_err(|e| ExecutionError::Failed(format!("打开文件夹失败: {}", e)))
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

#[async_trait]
impl ActionExecutor for FileExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::File]
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![
            ResultAction {
                id: "execute".to_string(),
                label: "打开".to_string(),
                icon: IconRequest::Path(String::new()),
                is_default: true,
                shortcut_key: String::new(),
            },
            ResultAction {
                id: "open_folder".to_string(),
                label: "打开所在文件夹".to_string(),
                icon: IconRequest::Path(String::new()),
                is_default: false,
                shortcut_key: String::new(),
            },
        ]
    }

    async fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
        let file_name = match &ctx.target {
            ExecutionTarget::File(f) => f,
            _ => {
                return Err(ExecutionError::Failed(
                    "Invalid target type for FileExecutor".into(),
                ))
            }
        };

        match action_id {
            "execute" => self.execute_file(file_name).await,
            "open_folder" => self.open_folder(file_name).await,
            _ => Err(ExecutionError::UnsupportedAction(
                TargetType::File,
                action_id.to_string(),
            )),
        }
    }
}

use crate::plugin_system::builtin_registry::{ExecutorEntry, InventoryContext};

pub(crate) fn build_file_executor(
    ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn ActionExecutor>) {
    let handle = ctx.get_handle("shell-executor");
    let exec: Arc<dyn ActionExecutor> = Arc::new(FileExecutor::new(handle));
    let configurable: Arc<dyn Configurable> = exec.clone();
    (configurable, exec)
}

::inventory::submit! {
    ExecutorEntry {
        component_id: "file-executor",
        handle_key: "shell-executor",
        priority: 10,
        factory: build_file_executor,
    }
}
