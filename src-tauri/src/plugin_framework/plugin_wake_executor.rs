use async_trait::async_trait;
use std::sync::{Arc, Weak};
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};

use super::builtin_registry::{ExecutorEntry, InventoryContext};
use super::SessionDispatcher;

/// 插件面板唤醒动作 ID —— 本执行器私有动作（打开/唤醒插件面板）。
const ACTION_OPEN: &str = "open";

/// 宿主内置执行器：沉浸式插件面板候选（ExecutionTarget::Plugin(id)）的确认动作——
/// 由 `ExecutorRegistry::resolve` 解析到本执行器，执行即唤醒对应插件面板（wake_plugin）。
///
/// 经 inventory 的 `ExecutorEntry` 注册：进入 `ConfigManager`（无配置项）并注册到
/// SessionDispatcher；持 Weak 引用避免与 dispatcher 循环强引用。
pub struct PluginWakeExecutor {
    core: ComponentCore,
    dispatcher: Weak<SessionDispatcher>,
}

impl PluginWakeExecutor {
    /// 组件 id：经 `ExecutorEntry` 注册，并计入内置 id 冲突预检。
    pub const COMPONENT_ID: &'static str = "plugin-wake-executor";

    /// 创建插件唤醒执行器。
    /// 参数：dispatcher - SessionDispatcher 弱引用（AppState 生命周期内始终有效）。
    pub fn new(dispatcher: Weak<SessionDispatcher>) -> Self {
        Self {
            core: ComponentCore::new(
                Self::COMPONENT_ID.to_string(),
                "插件面板唤醒执行器".to_string(),
                "沉浸式插件候选项选中后唤醒其面板".to_string(),
                ComponentType::ActionExecutor,
                0,
            ),
            dispatcher,
        }
    }
}

#[async_trait]
impl Configurable for PluginWakeExecutor {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![]
    }
}

#[async_trait]
impl ActionExecutor for PluginWakeExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::Plugin]
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: ACTION_OPEN.to_string(),
            label: "common.open".to_string(),
            icon: IconRequest::Path(String::new()),
            is_default: true,
            shortcut_key: String::new(),
        }]
    }

    async fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
        let plugin_id = match &ctx.target {
            ExecutionTarget::Plugin(id) => id.as_str(),
            _ => {
                return Err(ExecutionError::Failed(
                    "Invalid target type for PluginWakeExecutor".into(),
                ))
            }
        };
        if action_id != ACTION_OPEN {
            return Err(ExecutionError::UnsupportedAction(
                TargetType::Plugin,
                action_id.to_string(),
            ));
        }

        let dispatcher = self.dispatcher.upgrade().ok_or_else(|| {
            ExecutionError::Failed("SessionDispatcher 已释放，无法唤醒插件".into())
        })?;
        dispatcher
            .wake_plugin(plugin_id)
            .await
            .map_err(|e| ExecutionError::Failed(e.to_string()))
    }
}

/// 构造插件唤醒执行器并注入当前 SessionDispatcher 的弱引用。
fn build_plugin_wake_executor(
    ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn ActionExecutor>) {
    let component = Arc::new(PluginWakeExecutor::new(Arc::downgrade(
        ctx.session_dispatcher(),
    )));
    let configurable: Arc<dyn Configurable> = component.clone();
    let executor: Arc<dyn ActionExecutor> = component;
    (configurable, executor)
}

::inventory::submit! {
    ExecutorEntry {
        component_id: PluginWakeExecutor::COMPONENT_ID,
        handle_key: "plugin-wake-executor",
        priority: 0,
        factory: build_plugin_wake_executor,
    }
}
