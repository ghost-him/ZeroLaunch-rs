use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};
use crate::sdk::host_api::PluginHandle;
use crate::sdk::IconRequest;
use async_trait::async_trait;
use std::sync::Arc;

/// 应用执行器 - 负责通过 PluginHandle 启动系统应用（UWP 等）。
/// 不再直接调用 Win32 API，而是委托 PluginHandle::launch_app() 由 SDK 层处理平台差异。
pub struct AppExecutor {
    plugin_handle: Arc<PluginHandle>,
}

impl AppExecutor {
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        Self { plugin_handle }
    }
}

impl Configurable for AppExecutor {
    fn component_id(&self) -> &str {
        "app-executor"
    }

    fn component_name(&self) -> &str {
        "应用执行器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::ActionExecutor
    }
}

#[async_trait]
impl ActionExecutor for AppExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::App]
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: "execute".to_string(),
            label: "打开".to_string(),
            icon: IconRequest::Path(String::new()),
            is_default: true,
            shortcut_key: String::new(),
        }]
    }

    async fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
        let app_id = match &ctx.target {
            ExecutionTarget::App(id) => id,
            _ => {
                return Err(ExecutionError::Failed(
                    "Invalid target type for AppExecutor".into(),
                ))
            }
        };

        match action_id {
            "execute" => {
                let handle = self.plugin_handle.clone();
                let app_id = app_id.to_string();
                match handle.launch_app(&app_id, None).await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(ExecutionError::Failed(format!("应用启动失败: {}", e))),
                }
            }
            _ => Err(ExecutionError::UnsupportedAction(
                TargetType::App,
                action_id.to_string(),
            )),
        }
    }
}
