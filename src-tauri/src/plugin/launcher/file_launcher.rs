use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    LaunchError, LaunchMethod, LaunchMethodType, Launcher, ResultAction,
};
use crate::utils::windows::shell_execute_open;
use std::path::Path;
use tracing::warn;

/// 文件启动器 - 负责使用系统默认方式打开文件
/// 支持打开文件和打开所在文件夹
pub struct FileLauncher;

impl FileLauncher {
    pub fn new() -> Self {
        Self
    }

    /// 使用系统默认程序打开文件
    fn launch_file(&self, file_name: &str) -> Result<(), LaunchError> {
        shell_execute_open(file_name).map_err(|e| {
            let msg = format!("启动文件失败：{:?}", e);
            warn!("{}", msg);
            LaunchError::Failed(msg)
        })
    }

    /// 打开目标文件所在的文件夹
    fn open_folder(&self, file_name: &str) -> Result<(), LaunchError> {
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
            return Err(LaunchError::Failed(msg));
        }

        shell_execute_open(folder_to_open).map_err(|e| {
            let msg = format!(
                "Failed to open folder with default file manager. Error code: {}",
                e.to_hresult()
            );
            warn!("{}", msg);
            LaunchError::Failed(msg)
        })
    }
}

impl Configurable for FileLauncher {
    fn component_id(&self) -> &str {
        "file-launcher"
    }

    fn component_name(&self) -> &str {
        "文件启动器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Launcher
    }
}

impl Default for FileLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl Launcher for FileLauncher {
    fn supported_method(&self) -> LaunchMethodType {
        LaunchMethodType::File
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![
            ResultAction {
                id: "launch".to_string(),
                label: "打开".to_string(),
                icon: String::new(),
                is_default: true,
            },
            ResultAction {
                id: "open_folder".to_string(),
                label: "打开所在文件夹".to_string(),
                icon: String::new(),
                is_default: false,
            },
        ]
    }

    fn execute(&self, method: &LaunchMethod, action_id: &str) -> Result<(), LaunchError> {
        let file_name = match method {
            LaunchMethod::File(f) => f,
            _ => {
                return Err(LaunchError::Failed(
                    "Invalid launch method for FileLauncher".into(),
                ))
            }
        };

        match action_id {
            "launch" => self.launch_file(file_name),
            "open_folder" => self.open_folder(file_name),
            _ => Err(LaunchError::UnsupportedAction(action_id.to_string())),
        }
    }
}
