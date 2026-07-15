//! 内置执行器模板 — 复制并自定义。
//!
//! 使用步骤:
//! 1. 复制此文件到 `src-tauri/src/builtin_plugin/executor/`
//! 2. 重命名 struct 和所有引用
//! 3. 实现 Configurable trait (component_id 必须 kebab-case，全局唯一)
//! 4. 实现 ActionExecutor trait
//! 5. 底部的 inventory::submit! 块会自动注册

use zerolaunch_plugin_api::config::{ComponentCore, ComponentType, Configurable};
use zerolaunch_plugin_api::{
    ActionExecutor, ExecutionContext, ExecutionError, ResultAction, TargetType,
};
use async_trait::async_trait;
use std::sync::Arc;
use zerolaunch_plugin_api::host::PluginHandle;

pub struct TemplateExecutor {
    core: ComponentCore,
    plugin_handle: Arc<PluginHandle>,
}

impl TemplateExecutor {
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        Self {
            core: ComponentCore::new(
                "template-executor".to_string(), // ← 改为你的唯一 ID (kebab-case)
                "模板执行器".to_string(),           // ← 改为你的显示名称
                "模板执行器的功能描述".to_string(),  // ← 改为你的功能描述
                ComponentType::ActionExecutor,
                60, // ← 数字越小越优先 (留间隔 10 方便插入)
            ),
            plugin_handle,
        }
    }
}

#[async_trait]
impl Configurable for TemplateExecutor {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<crate::plugin_framework::SettingDefinition> {
        vec![]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn apply_settings(
        &self,
        _settings: serde_json::Value,
    ) -> Result<(), crate::plugin_framework::ConfigError> {
        Ok(())
    }
}

#[async_trait]
impl ActionExecutor for TemplateExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::Path]
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: "execute".to_string(),
            label: "执行".to_string(),
            icon: zerolaunch_plugin_api::services::IconRequest::Path(String::new()),
            is_default: true,
            shortcut_key: String::new(),
        }]
    }

    async fn execute(
        &self,
        _ctx: &ExecutionContext,
        action_id: &str,
    ) -> Result<(), ExecutionError> {
        match action_id {
            "execute" => {
                // TODO: 实现你的执行逻辑
                Ok(())
            }
            _ => Err(ExecutionError::UnsupportedAction(
                TargetType::Path,
                action_id.to_string(),
            )),
        }
    }
}

// ---- inventory 自动注册 ----
// 写完上面的 impl 后，更新这里 factory 中的 handle_key 和 priority。

use crate::plugin_framework::builtin_registry::{ExecutorEntry, InventoryContext};

fn build_template_executor(
    ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn ActionExecutor>) {
    let handle = ctx.get_handle("shell-executor"); // ← 选择合适的 handle_key
    let exec: Arc<dyn ActionExecutor> = Arc::new(TemplateExecutor::new(handle));
    let configurable: Arc<dyn Configurable> = exec.clone();
    (configurable, exec)
}

::inventory::submit! {
    ExecutorEntry {
        component_id: "template-executor", // ← 与 ComponentCore::new() 的第一个参数一致
        handle_key: "shell-executor",
        priority: 60, // ← 数字越小越优先注册 (留间隔 10 方便插入)
        factory: build_template_executor,
    }
}
