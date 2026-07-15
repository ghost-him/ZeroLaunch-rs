use async_trait::async_trait;
use std::sync::Arc;
use tracing::warn;
use zerolaunch_plugin_api::config::{ComponentCore, ComponentType, Configurable};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};

/// 窗口激活执行器 - 负责唤醒已存在的程序窗口。
/// 通过 PluginHandle 委托给 SDK 层的 WindowManager 实现窗口激活。
pub struct WindowActivateExecutor {
    core: ComponentCore,
    plugin_handle: Arc<PluginHandle>,
}

impl WindowActivateExecutor {
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        Self {
            core: ComponentCore::new(
                "window-activate-executor".to_string(),
                "窗口唤醒执行器".to_string(),
                "激活或切换到已打开的窗口".to_string(),
                ComponentType::ActionExecutor,
                50,
            ),
            plugin_handle,
        }
    }

    /// 尝试激活已存在的程序窗口。
    /// 根据 ExecutionTarget 类型选择不同的激活策略：
    /// - Path(.url): 按标题激活
    /// - Path(.exe/.lnk): 按进程名激活
    /// - App: 按标题激活
    async fn try_activate(&self, target: &ExecutionTarget, name: &str) -> bool {
        match target {
            ExecutionTarget::Path(path) => {
                if path.ends_with(".url") {
                    self.activate_by_title(name).await
                } else {
                    let exe_path = if path.ends_with(".exe") {
                        path.clone()
                    } else {
                        self.plugin_handle
                            .resolve_lnk_target(path)
                            .unwrap_or_default()
                    };
                    if exe_path.is_empty() {
                        return false;
                    }
                    self.activate_by_exe(&exe_path).await
                }
            }
            ExecutionTarget::App(_) => self.activate_by_title(name).await,
            _ => false,
        }
    }

    /// 直接使用标题来激活窗口。
    async fn activate_by_title(&self, program_name: &str) -> bool {
        let handle = self.plugin_handle.clone();
        let program_name = program_name.to_string();
        match handle.activate_window_by_title(&program_name).await {
            Ok(activated) => activated,
            Err(e) => {
                warn!("按标题激活窗口失败: {}", e);
                false
            }
        }
    }

    /// 激活 .exe 程序的窗口，传入绝对路径。
    /// 先按进程名查找，未找到则按文件名（不含扩展名）按标题查找。
    async fn activate_by_exe(&self, path: &str) -> bool {
        let abs_path = std::path::Path::new(path);
        let program_name = match abs_path.file_name() {
            Some(name) => match name.to_str() {
                Some(s) => s.to_string(),
                None => return false,
            },
            None => return false,
        };
        let program_stem = match abs_path.file_stem() {
            Some(stem) => match stem.to_str() {
                Some(s) => s.to_string(),
                None => return false,
            },
            None => return false,
        };

        let handle = self.plugin_handle.clone();

        // 先按进程名查找窗口
        let activated = match handle.activate_window_by_process(&program_name).await {
            Ok(result) => result,
            Err(e) => {
                warn!("按进程名激活窗口失败: {}", e);
                false
            }
        };

        if activated {
            return true;
        }

        // 未找到则按文件主名按标题查找
        let handle2 = self.plugin_handle.clone();
        match handle2.activate_window_by_title(&program_stem).await {
            Ok(result) => result,
            Err(e) => {
                warn!("按标题激活窗口失败: {}", e);
                false
            }
        }
    }
}

#[async_trait]
impl Configurable for WindowActivateExecutor {
    fn core(&self) -> &ComponentCore {
        &self.core
    }
}

#[async_trait]
impl ActionExecutor for WindowActivateExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::Path, TargetType::App]
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: "activate_window".to_string(),
            label: "唤醒窗口".to_string(),
            icon: IconRequest::Path(String::new()),
            is_default: false,
            shortcut_key: "Shift+Enter".to_string(),
        }]
    }

    async fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
        match action_id {
            "activate_window" => {
                if self.try_activate(&ctx.target, &ctx.display_name).await {
                    Ok(())
                } else {
                    Err(ExecutionError::ActivationFailed {
                        fallback_action: "execute".to_string(),
                    })
                }
            }
            _ => Err(ExecutionError::UnsupportedAction(
                ctx.target.target_type(),
                action_id.to_string(),
            )),
        }
    }
}

use crate::plugin_framework::builtin_registry::{ExecutorEntry, InventoryContext};

pub(crate) fn build_window_activate_executor(
    ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn ActionExecutor>) {
    let handle = ctx.get_handle("window-activator");
    let exec: Arc<dyn ActionExecutor> = Arc::new(WindowActivateExecutor::new(handle));
    let configurable: Arc<dyn Configurable> = exec.clone();
    (configurable, exec)
}

::inventory::submit! {
    ExecutorEntry {
        component_id: "window-activate-executor",
        handle_key: "window-activator",
        priority: 50,
        factory: build_window_activate_executor,
    }
}
