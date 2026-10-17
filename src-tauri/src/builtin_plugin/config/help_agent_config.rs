//! AI 助手配置组件：声明助手使用哪个模型。
//!
//! 配置 id 与设置结构由消费侧（`core/agent/settings.rs`）拥有，本组件只承载
//! schema、下拉选项加载与持久化；模型选项经 `list_chat_models` 配置动作从宿主
//! 模型清单动态获取（与 ai-search 的模型选择同机制），用户无需手写模型 id。

use crate::core::agent::{HelpAgentSettings, HELP_AGENT_CONFIG_ID};
use crate::core::config::setting_builders::SchemaBuilder;
use crate::plugin_framework::builtin_registry::{ConfigEntry, InventoryContext};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigActionDef, ConfigError, Configurable, DataActionBinding,
    SettingDefinition,
};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::model::{ModelInfo, ModelKind};

/// 列出可选聊天模型的配置动作标识（字段级 data action）。
const ACTION_LIST_CHAT_MODELS: &str = "list_chat_models";

/// AI 助手配置组件。
pub struct HelpAgentConfigComponent {
    /// 提供组件 ID、名称、类型等基础元数据。
    core: ComponentCore,
    /// 仅 apply_settings 写入。
    settings: RwLock<HelpAgentSettings>,
    /// 宿主能力入口：读取模型清单，用于生成下拉选项。
    plugin_handle: Arc<PluginHandle>,
}

impl HelpAgentConfigComponent {
    /// 创建组件。
    /// 参数：plugin_handle - 宿主能力句柄（读取模型清单）。
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        Self {
            core: ComponentCore::new(
                HELP_AGENT_CONFIG_ID.to_string(),
                t_key!("help-agent-config", "name").to_string(),
                t_key!("help-agent-config", "description").to_string(),
                ComponentType::Core,
                55,
            ),
            settings: RwLock::new(HelpAgentSettings::default()),
            plugin_handle,
        }
    }
}

/// 从宿主模型清单筛出可作助手的聊天模型。
///
/// 参数：models - `host/model.list` 的聚合清单。
/// 返回：下拉选项条目 `{ name, modelId }`（显示名 + 全局模型 id，前端按这两个字段映射）。
fn chat_model_options(models: Vec<ModelInfo>) -> Vec<serde_json::Value> {
    models
        .into_iter()
        .filter(|info| matches!(info.kind, ModelKind::Chat { .. }))
        .map(|info| {
            serde_json::json!({
                "name": info.name,
                "modelId": info.model_id,
            })
        })
        .collect()
}

#[async_trait]
impl Configurable for HelpAgentConfigComponent {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![SchemaBuilder::select(
            "model_id",
            t_key!("help-agent-config", "fields.model_id.label"),
            t_key!("help-agent-config", "fields.model_id.desc"),
        )
        .group(t_key!("help-agent-config", "groups.assistant"))
        .order(0)
        .default("")
        // 选项由 list_chat_models 动态返回（labelField/valueField 映射，随模型配置变化刷新）
        .data_action(DataActionBinding {
            action: ACTION_LIST_CHAT_MODELS.to_string(),
            component: None,
            label_field: "name".to_string(),
            label_field_label: String::new(),
            value_field: "modelId".to_string(),
            merge_key: None,
            field_mapping: Vec::new(),
        })
        .build()]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        // 解析失败返回错误拒绝保存：静默回退默认会清空已保存的助手模型。
        let parsed: HelpAgentSettings = serde_json::from_value(settings)
            .map_err(|e| ConfigError::ValidationFailed(format!("AI 助手配置解析失败: {e}")))?;
        *self.settings.write() = parsed;
        Ok(())
    }

    async fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
        if let Some(model_id) = settings.get("model_id").and_then(|v| v.as_str()) {
            let model_id = model_id.trim();
            if !model_id.is_empty() && !model_id.contains('/') {
                return Err(ConfigError::ValidationFailed(
                    "模型 id 需为 {provider}/{model} 形式".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn config_actions(&self) -> Vec<ConfigActionDef> {
        vec![ConfigActionDef {
            action: ACTION_LIST_CHAT_MODELS.to_string(),
            label: t_key!("help-agent-config", "actions.list_chat_models.label").to_string(),
            description: t_key!("help-agent-config", "actions.list_chat_models.description")
                .to_string(),
        }]
    }

    /// 只读动作：返回当前可选的聊天模型，不修改任何状态（供下拉选项加载/刷新）。
    async fn execute_config_action(
        &self,
        action: &str,
        _params: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match action {
            ACTION_LIST_CHAT_MODELS => Ok(serde_json::Value::Array(chat_model_options(
                self.plugin_handle.model_list(),
            ))),
            other => Err(format!("Unknown config action: {other}")),
        }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

fn build_help_agent_config(ctx: &InventoryContext) -> Arc<dyn Configurable> {
    Arc::new(HelpAgentConfigComponent::new(
        ctx.get_handle(HELP_AGENT_CONFIG_ID),
    ))
}

::inventory::submit! {
    ConfigEntry {
        component_id: HELP_AGENT_CONFIG_ID,
        priority: 55,
        factory: build_help_agent_config,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造模型元数据。
    fn model(model_id: &str, name: &str, kind: ModelKind) -> ModelInfo {
        ModelInfo {
            model_id: model_id.to_string(),
            name: name.to_string(),
            kind,
            provider: model_id
                .split_once('/')
                .map(|(provider, _)| provider.to_string())
                .unwrap_or_default(),
        }
    }

    /// 聊天模型元数据（能力不参与下拉选项生成）。
    fn chat_kind() -> ModelKind {
        ModelKind::Chat {
            context_window: None,
            max_output: None,
            supports_stream: true,
            capabilities: Vec::new(),
        }
    }

    #[test]
    fn chat_model_options_keeps_only_chat_models() {
        let options = chat_model_options(vec![
            model("ollama/ornith-1.5:9b", "Ornith 1.5 9B", chat_kind()),
            model(
                "ollama/qwen3-embedding:0.6b",
                "Qwen3 Embedding",
                ModelKind::Embedding {
                    context_window: None,
                    dimensions: Some(1024),
                    similarity: Default::default(),
                    capabilities: Vec::new(),
                },
            ),
            model("openai/gpt-4o-mini", "GPT-4o mini", chat_kind()),
        ]);
        assert_eq!(options.len(), 2);
        // 显示名给用户看，值用全局 model id（与 help-agent-config 的语义一致）
        assert_eq!(options[0]["name"], "Ornith 1.5 9B");
        assert_eq!(options[0]["modelId"], "ollama/ornith-1.5:9b");
        assert_eq!(options[1]["modelId"], "openai/gpt-4o-mini");
    }

    #[test]
    fn chat_model_options_is_empty_without_chat_models() {
        assert!(chat_model_options(Vec::new()).is_empty());
    }
}
