use crate::core::types::{ComponentType, Configurable};
use async_trait::async_trait;
use std::sync::Arc;
use zerolaunch_plugin_api::host::{OpenTarget, PluginHandle};
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};

/// URL 执行器 - 负责使用系统默认浏览器打开 URL
/// 委托 PluginHandle 的 shell_open 方法打开 URL
pub struct UrlExecutor {
    plugin_handle: Arc<PluginHandle>,
}

impl UrlExecutor {
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        Self { plugin_handle }
    }

    /// 使用系统默认浏览器打开 URL
    async fn execute_url(&self, url: &str) -> Result<(), ExecutionError> {
        let handle = self.plugin_handle.clone();
        let url = url.to_string();
        handle
            .shell_open(OpenTarget::Url(url))
            .await
            .map_err(|e| ExecutionError::Failed(format!("启动 URL 失败: {}", e)))
    }
}

impl Configurable for UrlExecutor {
    fn component_id(&self) -> &str {
        "url-executor"
    }

    fn component_name(&self) -> &str {
        "URL执行器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::ActionExecutor
    }
}

impl Default for UrlExecutor {
    fn default() -> Self {
        panic!("UrlExecutor 必须通过 new(plugin_handle) 创建，不支持 Default");
    }
}

#[async_trait]
impl ActionExecutor for UrlExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::Url]
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
        let url_template = match &ctx.target {
            ExecutionTarget::Url(u) => u.as_str(),
            _ => {
                return Err(ExecutionError::Failed(
                    "Invalid target type for UrlExecutor".into(),
                ))
            }
        };

        match action_id {
            "execute" => {
                // 解析模板，替换 {} 用户参数和 {clip}/{hwnd}/{selection} 系统参数
                let resolved = self
                    .plugin_handle
                    .resolve_parameters(url_template, &ctx.user_args, &ctx.parameter_snapshot)
                    .await
                    .map_err(|e| ExecutionError::Failed(format!("URL参数解析失败: {}", e)))?;

                self.execute_url(&resolved).await
            }
            _ => Err(ExecutionError::UnsupportedAction(
                TargetType::Url,
                action_id.to_string(),
            )),
        }
    }
}

use crate::plugin_framework::builtin_registry::{ExecutorEntry, InventoryContext};

pub(crate) fn build_url_executor(
    ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn ActionExecutor>) {
    let handle = ctx.get_handle("shell-executor");
    let exec: Arc<dyn ActionExecutor> = Arc::new(UrlExecutor::new(handle));
    let configurable: Arc<dyn Configurable> = exec.clone();
    (configurable, exec)
}

::inventory::submit! {
    ExecutorEntry {
        component_id: "url-executor",
        handle_key: "shell-executor",
        priority: 20,
        factory: build_url_executor,
    }
}
