use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};
use crate::utils::windows::shell_execute_open;
use std::path::Path;
use tracing::warn;

/// 文件执行器 - 负责使用系统默认方式打开文件
/// 支持打开文件和打开所在文件夹
pub struct FileExecutor;

impl FileExecutor {
    pub fn new() -> Self {
        Self
    }

    /// 使用系统默认程序打开文件
    fn execute_file(&self, file_name: &str) -> Result<(), ExecutionError> {
        shell_execute_open(file_name).map_err(|e| {
            let msg = format!("启动文件失败：{:?}", e);
            warn!("{}", msg);
            ExecutionError::Failed(msg)
        })
    }

    /// 打开目标文件所在的文件夹
    fn open_folder(&self, file_name: &str) -> Result<(), ExecutionError> {
        let target_path = Path::new(file_name);

        let folder_to_open = if target_path.is_dir() {
            target_path
        } else {
            target_path.parent().unwrap_or_else(|| {
                warn!(
                    "Target path has no parent, fallback to original path: {}",
                    target_path.display()
                );
                target_path
            })
        };

        if !folder_to_open.exists() {
            let msg = format!(
                "Target folder does not exist and cannot be opened: {}",
                folder_to_open.display()
            );
            warn!("{}", msg);
            return Err(ExecutionError::Failed(msg));
        }

        shell_execute_open(folder_to_open).map_err(|e| {
            let msg = format!(
                "Failed to open folder with default file manager. Error code: {}",
                e.to_hresult()
            );
            warn!("{}", msg);
            ExecutionError::Failed(msg)
        })
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
        Self::new()
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
