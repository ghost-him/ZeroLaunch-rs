//! Ollama 模型服务配置组件（宿主内置模型提供方之一）。

use super::model_openai_config::model_entry_schema;
use crate::core::config::setting_builders::SchemaBuilder;
use crate::core::model::{ModelEntryConfig, ModelOllamaSettings, MODEL_OLLAMA_CONFIG_ID};
use async_trait::async_trait;
use ollama_rs::models::ModelInfo;
use ollama_rs::Ollama;
use parking_lot::RwLock;
use serde_json::Value;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigActionDef, ConfigError, Configurable, DataActionBinding,
    SettingDefinition,
};

/// 从 Ollama `/api/show` 元数据中提取 u32 数值字段。
fn extract_model_info_u32(
    model_info: &serde_json::Map<String, Value>,
    suffix: &str,
) -> Option<u32> {
    model_info
        .iter()
        .find(|(key, _)| key.ends_with(suffix))
        .and_then(|(_, value)| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
}

/// 根据 Ollama `/api/show` 的 capabilities 判断模型类型。
fn model_is_embedding(info: &ModelInfo) -> bool {
    info.capabilities
        .iter()
        .any(|capability| capability == "embedding")
}

/// Ollama 模型服务配置组件：管理 base_url 与模型条目清单。
pub struct ModelOllamaConfigComponent {
    core: ComponentCore,
    settings: RwLock<ModelOllamaSettings>,
}

impl ModelOllamaConfigComponent {
    /// 创建组件实例。
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                MODEL_OLLAMA_CONFIG_ID.to_string(),
                t_key!("model-ollama-config", "name").to_string(),
                t_key!("model-ollama-config", "description").to_string(),
                ComponentType::Core,
                45,
            ),
            settings: RwLock::new(ModelOllamaSettings::default()),
        }
    }
}

impl Default for ModelOllamaConfigComponent {
    /// 使用默认连接配置创建 Ollama 配置组件。
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Configurable for ModelOllamaConfigComponent {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::text(
                "base_url",
                t_key!("model-ollama-config", "fields.base_url.label"),
                t_key!("model-ollama-config", "fields.base_url.desc"),
            )
            .note(t_key!("model-ollama-config", "fields.base_url.note"))
            .group(t_key!("model-ollama-config", "groups.connection"))
            .order(0)
            .default("http://localhost:11434")
            .build(),
            SchemaBuilder::array(
                "models",
                t_key!("model-ollama-config", "fields.models.label"),
                t_key!("model-ollama-config", "fields.models.desc"),
            )
            .group(t_key!("model-ollama-config", "groups.models"))
            .order(1)
            .object_items(model_entry_schema())
            .master_detail_ui()
            .default(serde_json::json!([]))
            .data_action(DataActionBinding {
                action: "fetch_models".to_string(),
                component: None,
                label_field: "name".to_string(),
                label_field_label: String::new(),
                value_field: "models".to_string(),
                merge_key: Some("name".to_string()),
                field_mapping: vec![],
            })
            .build(),
        ]
    }

    fn get_settings(&self) -> Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: Value) -> Result<(), ConfigError> {
        // 解析失败返回错误拒绝保存：静默回退默认会清空已保存的模型配置。
        let parsed: ModelOllamaSettings = serde_json::from_value(settings)
            .map_err(|e| ConfigError::ValidationFailed(format!("模型 Ollama 配置解析失败: {e}")))?;
        *self.settings.write() = parsed;
        Ok(())
    }

    fn config_actions(&self) -> Vec<ConfigActionDef> {
        vec![ConfigActionDef {
            action: "fetch_models".to_string(),
            label: t_key!("model-ollama-config", "actions.fetch_models.label").to_string(),
            description: t_key!("model-ollama-config", "actions.fetch_models.description")
                .to_string(),
        }]
    }

    async fn execute_config_action(&self, action: &str, _params: &Value) -> Result<Value, String> {
        match action {
            "fetch_models" => {
                // `/api/tags` 获取名称，`/api/show` 提供 capabilities、上下文和维度。
                let client = Ollama::try_new(self.settings.read().base_url.trim())
                    .map_err(|e| e.to_string())?;
                let local = client
                    .list_local_models()
                    .await
                    .map_err(|e| e.to_string())?;
                let configured = self.settings.read().models.clone();
                let mut entries = Vec::with_capacity(local.len());
                for model in &local {
                    let configured_entry = configured
                        .iter()
                        .find(|entry| entry.name() == model.name)
                        .cloned();
                    let info = client.show_model_info(model.name.clone()).await.ok();
                    let mut entry = match (info.as_ref().map(model_is_embedding), configured_entry)
                    {
                        (Some(true), Some(ModelEntryConfig::Embedding { config, .. })) => {
                            ModelEntryConfig::Embedding {
                                name: model.name.clone(),
                                config,
                            }
                        }
                        (Some(true), _) => ModelEntryConfig::embedding(&model.name),
                        (Some(false), Some(ModelEntryConfig::Chat { config, .. })) => {
                            ModelEntryConfig::Chat {
                                name: model.name.clone(),
                                config,
                            }
                        }
                        (Some(false), _) => ModelEntryConfig::chat(&model.name),
                        (None, Some(entry)) => entry,
                        (None, None) => ModelEntryConfig::chat(&model.name),
                    };
                    if let Some(info) = info {
                        let context_length =
                            extract_model_info_u32(&info.model_info, ".context_length")
                                .unwrap_or_default();
                        match &mut entry {
                            ModelEntryConfig::Chat { config, .. } => {
                                config.context_length = context_length;
                            }
                            ModelEntryConfig::Embedding { config, .. } => {
                                config.context_length = context_length;
                                config.dimensions =
                                    extract_model_info_u32(&info.model_info, ".embedding_length")
                                        .unwrap_or(config.dimensions);
                            }
                        }
                    }
                    entries.push(entry);
                }
                Ok(serde_json::json!({ "models": entries }))
            }
            _ => Err(format!("未知动作: {}", action)),
        }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

use crate::plugin_framework::builtin_registry::{ConfigEntry, InventoryContext};

fn build_model_ollama_config(_ctx: &InventoryContext) -> std::sync::Arc<dyn Configurable> {
    std::sync::Arc::new(ModelOllamaConfigComponent::new())
}

::inventory::submit! {
    ConfigEntry {
        component_id: MODEL_OLLAMA_CONFIG_ID,
        priority: 45,
        factory: build_model_ollama_config,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zerolaunch_plugin_api::config::{Configurable, SchemaKind};

    /// 验证 Ollama 模型 schema 可生成且上下文长度字段只出现一次。
    #[test]
    fn setting_schema_has_one_context_length_field() {
        let component = ModelOllamaConfigComponent::new();
        let contribution = component
            .settings_contribution()
            .expect("Ollama schema should be valid");
        let models = contribution
            .properties
            .get("models")
            .expect("Ollama schema should define models");
        let items = match &models.kind {
            SchemaKind::Array { items, .. } => items,
            _ => panic!("Ollama models should be an array"),
        };
        let properties = match &items.kind {
            SchemaKind::Object { properties, .. } => properties,
            _ => panic!("Ollama model entries should be objects"),
        };

        assert!(properties.contains_key("context_length"));
        assert_eq!(
            properties
                .keys()
                .filter(|key| *key == "context_length")
                .count(),
            1
        );
    }
}
