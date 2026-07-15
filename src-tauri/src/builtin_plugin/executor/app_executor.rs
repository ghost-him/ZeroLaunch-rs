use async_trait::async_trait;
use std::sync::Arc;
use zerolaunch_plugin_api::config::{ComponentCore, ComponentType, Configurable};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};

/// 应用执行器 - 负责通过 PluginHandle 启动系统应用（UWP 等）。
/// 不再直接调用 Win32 API，而是委托 PluginHandle::launch_app() 由 SDK 层处理平台差异。
pub struct AppExecutor {
    core: ComponentCore,
    plugin_handle: Arc<PluginHandle>,
}

impl AppExecutor {
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        Self {
            core: ComponentCore::new(
                "app-executor".to_string(),
                "应用执行器".to_string(),
                "启动搜索到的应用程序".to_string(),
                ComponentType::ActionExecutor,
                30,
            ),
            plugin_handle,
        }
    }
}

#[async_trait]
impl Configurable for AppExecutor {
    fn core(&self) -> &ComponentCore {
        &self.core
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

use crate::plugin_framework::builtin_registry::{ExecutorEntry, InventoryContext};

pub(crate) fn build_app_executor(
    ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn ActionExecutor>) {
    let handle = ctx.get_handle("app-executor");
    let exec: Arc<dyn ActionExecutor> = Arc::new(AppExecutor::new(handle));
    let configurable: Arc<dyn Configurable> = exec.clone();
    (configurable, exec)
}

::inventory::submit! {
    ExecutorEntry {
        component_id: "app-executor",
        handle_key: "app-executor",
        priority: 30,
        factory: build_app_executor,
    }
}
