use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};
use crate::sdk::host_api::{OpenTarget, PluginHandle};
use crate::sdk::IconRequest;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::warn;

/// 路径执行器 - 负责通过文件路径启动程序
/// 支持普通启动、管理员启动和打开所在文件夹
pub struct PathExecutor {
    plugin_handle: Arc<PluginHandle>,
}

impl PathExecutor {
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        Self { plugin_handle }
    }

    /// 普通启动程序。
    async fn execute_normal(&self, path: &str) {
        let handle = self.plugin_handle.clone();
        let path = path.to_string();
        if let Err(e) = handle.shell_open(OpenTarget::File(path)).await {
            warn!("启动程序失败: {}", e);
        }
    }

    /// 以管理员权限启动程序
    async fn execute_elevation(&self, path: &str) {
        let handle = self.plugin_handle.clone();
        let path = path.to_string();
        if let Err(e) = handle.shell_execute_elevation(&path).await {
            warn!("管理员启动失败: {}", e);
        }
    }

    /// 打开目标文件所在的文件夹
    async fn open_folder(&self, path: &str) -> Result<(), ExecutionError> {
        let handle = self.plugin_handle.clone();
        let path = path.to_string();
        handle
            .shell_open_folder(&path)
            .await
            .map_err(|e| ExecutionError::Failed(format!("打开文件夹失败: {}", e)))
    }
}

impl Configurable for PathExecutor {
    fn component_id(&self) -> &str {
        "path-executor"
    }

    fn component_name(&self) -> &str {
        "路径执行器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::ActionExecutor
    }
}

impl Default for PathExecutor {
    fn default() -> Self {
        panic!("PathExecutor 必须通过 new(plugin_handle) 创建，不支持 Default");
    }
}

#[async_trait]
impl ActionExecutor for PathExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::Path]
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
                id: "execute_admin".to_string(),
                label: "以管理员身份运行".to_string(),
                icon: IconRequest::Path(String::new()),
                is_default: false,
                shortcut_key: "Ctrl+Enter".to_string(),
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
        let path = match &ctx.target {
            ExecutionTarget::Path(p) => p,
            _ => {
                return Err(ExecutionError::Failed(
                    "Invalid target type for PathExecutor".into(),
                ))
            }
        };

        match action_id {
            "execute" => {
                self.execute_normal(path).await;
                Ok(())
            }
            "execute_admin" => {
                self.execute_elevation(path).await;
                Ok(())
            }
            "open_folder" => self.open_folder(path).await,
            _ => Err(ExecutionError::UnsupportedAction(
                TargetType::Path,
                action_id.to_string(),
            )),
        }
    }
}
