//! OpenAI 兼容模型服务配置组件（宿主内置模型提供方之一）。

use crate::core::config::setting_builders::SchemaBuilder;
use crate::core::model::{ModelEntryConfig, ModelOpenAiSettings, MODEL_OPENAI_CONFIG_ID};
use async_trait::async_trait;
use parking_lot::RwLock;
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigActionDef, ConfigError, Configurable, DataActionBinding,
    EffectActionBinding, SettingDefinition,
};

/// OpenAI 兼容模型服务配置组件：管理 base_url / api_key / 模型条目清单。
pub struct ModelOpenAiConfigComponent {
    core: ComponentCore,
    settings: RwLock<ModelOpenAiSettings>,
}

impl ModelOpenAiConfigComponent {
    /// 创建组件实例。
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                MODEL_OPENAI_CONFIG_ID.to_string(),
                t_key!("model-openai-config", "name").to_string(),
                t_key!("model-openai-config", "description").to_string(),
                ComponentType::Core,
                40,
            ),
            settings: RwLock::new(ModelOpenAiSettings::default()),
        }
    }

    /// 解析动作使用的连接参数。
    /// 参数：params - 前端表单当前值（可缺省）。
    /// 返回：(base_url, api_key)；表单未提供的键回落到已保存配置。
    fn resolve_connection(&self, params: &Value) -> (String, String) {
        let saved = self.settings.read();
        let pick = |key: &str, fallback: &str| {
            params
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| fallback.to_string())
        };
        (
            pick("base_url", &saved.base_url),
            pick("api_key", &saved.api_key),
        )
    }

    /// 请求 `/models` 端点，验证地址与密钥是否可用。
    /// 参数：base_url - 端点根地址；api_key - 密钥，空串时不带认证头。
    /// 返回：服务端返回的非空模型 id 列表。
    /// 错误：地址非法、网络失败、认证被拒或响应无法解析时返回可读原因。
    async fn request_remote_models(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Result<Vec<String>, String> {
        let url = format!("{}/models", base_url.trim_end_matches('/'));
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .map_err(|e| format!("创建模型列表客户端失败: {e}"))?;
        let mut request = client.get(url);
        if !api_key.is_empty() {
            request = request.bearer_auth(api_key);
        }
        let response = request
            .send()
            .await
            .map_err(|e| format!("请求模型列表失败: {e}"))?
            .error_for_status()
            .map_err(|e| format!("模型列表响应失败: {e}"))?;
        let response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| format!("解析模型列表失败: {e}"))?;
        Ok(response
            .data
            .into_iter()
            .map(|model| model.id)
            .filter(|id| !id.is_empty())
            .collect())
    }
}
/// OpenAI 兼容 `/models` 响应的最小解析结构。
///
/// 仅在配置动作中使用，避免将服务端可选模型元数据绑定到 SDK 类型。
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    /// 服务端返回的模型条目列表。
    #[serde(rename = "data", default)]
    data: Vec<ModelItem>,
}

/// OpenAI 兼容模型列表中的必要字段。
///
/// 仅在配置动作中使用，id 为空的条目会被忽略。
#[derive(Debug, Deserialize)]
struct ModelItem {
    /// provider 内部模型名称。
    #[serde(rename = "id", default)]
    id: String,
}

/// 依据模型 id 判定是否为 embedding 模型。
///
/// OpenAI 兼容 API 的 `/models` 不返回能力标签，只能按命名约定启发式判断：
/// 主流 embedding 模型 id 普遍包含 `embedding`/`embed`/`ada` 等特征词。
/// 误判代价低：kind 可在模型条目里手动改回 chat。
fn is_embedding_model_id(id: &str) -> bool {
    let lowered = id.to_ascii_lowercase();
    lowered.contains("embedding")
        || lowered.contains("text-embed")
        || lowered.contains("-embed-")
        || lowered.contains("bge-")
        || lowered.contains("m3e")
        || lowered == "ada"
        || lowered.starts_with("ada-")
        || lowered.ends_with("-ada")
        || lowered.contains("similarity")
        || lowered.contains("rerank")
        || lowered.contains("retrieval")
}

impl Default for ModelOpenAiConfigComponent {
    /// 使用默认连接配置创建 OpenAI 兼容配置组件。
    fn default() -> Self {
        Self::new()
    }
}

/// 模型条目 schema：提供通用 MasterDetail 控件所需的字段定义。
///
/// chat 与 embedding 字段由 visibleWhen 在条目作用域内互斥显示。
pub(crate) fn model_entry_schema() -> Vec<SettingDefinition> {
    let mut fields = vec![
        SchemaBuilder::text(
            "name",
            t_key!("model-entry", "fields.name.label"),
            t_key!("model-entry", "fields.name.desc"),
        )
        .note(t_key!("model-entry", "fields.name.note"))
        .order(0)
        .build(),
        SchemaBuilder::select(
            "kind",
            t_key!("model-entry", "fields.kind.label"),
            t_key!("model-entry", "fields.kind.desc"),
        )
        .note(t_key!("model-entry", "fields.kind.note"))
        .options_with_labels(&[
            ("chat", t_key!("model-entry", "kinds.chat")),
            ("embedding", t_key!("model-entry", "kinds.embedding")),
        ])
        .default("chat")
        .order(1)
        .build(),
        SchemaBuilder::number(
            "temperature",
            t_key!("model-entry", "fields.temperature.label"),
            t_key!("model-entry", "fields.temperature.desc"),
        )
        .min(0.0)
        .max(2.0)
        .step(0.05)
        .default(0.7)
        .order(2)
        .visible_when("kind", "chat")
        .build(),
        SchemaBuilder::integer(
            "max_tokens",
            t_key!("model-entry", "fields.max_tokens.label"),
            t_key!("model-entry", "fields.max_tokens.desc"),
        )
        .default(2048)
        .min(1.0)
        .order(3)
        .visible_when("kind", "chat")
        .build(),
        SchemaBuilder::number(
            "top_p",
            t_key!("model-entry", "fields.top_p.label"),
            t_key!("model-entry", "fields.top_p.desc"),
        )
        .min(0.0)
        .max(1.0)
        .step(0.05)
        .default(1.0)
        .order(4)
        .visible_when("kind", "chat")
        .build(),
        SchemaBuilder::select(
            "reasoning_effort",
            t_key!("model-entry", "fields.reasoning_effort.label"),
            t_key!("model-entry", "fields.reasoning_effort.desc"),
        )
        .note(t_key!("model-entry", "fields.reasoning_effort.note"))
        .options_with_labels(&[
            ("none", t_key!("model-entry", "efforts.none")),
            ("low", t_key!("model-entry", "efforts.low")),
            ("medium", t_key!("model-entry", "efforts.medium")),
            ("high", t_key!("model-entry", "efforts.high")),
        ])
        .default("none")
        .order(5)
        .visible_when("kind", "chat")
        .build(),
        SchemaBuilder::multi_select(
            "chat_capabilities",
            t_key!("model-entry", "fields.chat_capabilities.label"),
            t_key!("model-entry", "fields.chat_capabilities.desc"),
        )
        .note(t_key!("model-entry", "fields.chat_capabilities.note"))
        .options_with_labels(&[
            ("reasoning", t_key!("model-entry", "capabilities.reasoning")),
            ("tools", t_key!("model-entry", "capabilities.tools")),
        ])
        .default(serde_json::json!(["reasoning", "tools"]))
        .order(6)
        .visible_when("kind", "chat")
        .build(),
    ];
    fields.push(
        SchemaBuilder::integer(
            "context_length",
            t_key!("model-entry", "fields.context_length.label"),
            t_key!("model-entry", "fields.context_length.desc"),
        )
        .note(t_key!("model-entry", "fields.context_length.note"))
        .editable(false)
        .default(0)
        .order(8)
        .build(),
    );
    fields.extend([
        SchemaBuilder::integer(
            "dimensions",
            t_key!("model-entry", "fields.dimensions.label"),
            t_key!("model-entry", "fields.dimensions.desc"),
        )
        .note(t_key!("model-entry", "fields.dimensions.note"))
        .min(1.0)
        .default(768)
        .order(9)
        .visible_when("kind", "embedding")
        .build(),
        SchemaBuilder::select(
            "similarity",
            t_key!("model-entry", "fields.similarity.label"),
            t_key!("model-entry", "fields.similarity.desc"),
        )
        .note(t_key!("model-entry", "fields.similarity.note"))
        .options_with_labels(&[
            ("cosine", t_key!("model-entry", "similarities.cosine")),
            (
                "dotProduct",
                t_key!("model-entry", "similarities.dot_product"),
            ),
            ("euclidean", t_key!("model-entry", "similarities.euclidean")),
        ])
        .default("cosine")
        .order(11)
        .visible_when("kind", "embedding")
        .build(),
        SchemaBuilder::multi_select(
            "embedding_capabilities",
            t_key!("model-entry", "fields.embedding_capabilities.label"),
            t_key!("model-entry", "fields.embedding_capabilities.desc"),
        )
        .note(t_key!("model-entry", "fields.embedding_capabilities.note"))
        .options_with_labels(&[(
            "outputDimensions",
            t_key!("model-entry", "capabilities.output_dimensions"),
        )])
        .default(serde_json::json!(["outputDimensions"]))
        .order(12)
        .visible_when("kind", "embedding")
        .build(),
    ]);
    fields
}
#[async_trait]
impl Configurable for ModelOpenAiConfigComponent {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::text(
                "base_url",
                t_key!("model-openai-config", "fields.base_url.label"),
                t_key!("model-openai-config", "fields.base_url.desc"),
            )
            .note(t_key!("model-openai-config", "fields.base_url.note"))
            .group(t_key!("model-openai-config", "groups.connection"))
            .order(0)
            .default("https://api.openai.com/v1")
            .build(),
            SchemaBuilder::text(
                "api_key",
                t_key!("model-openai-config", "fields.api_key.label"),
                t_key!("model-openai-config", "fields.api_key.desc"),
            )
            .note(t_key!("model-openai-config", "fields.api_key.note"))
            .group(t_key!("model-openai-config", "groups.connection"))
            .order(1)
            .default("")
            .sensitive(true)
            .effect_action(EffectActionBinding {
                action: "test_connection".to_string(),
                component: None,
                field_mapping: vec![],
                transient: false,
            })
            .build(),
            SchemaBuilder::array(
                "models",
                t_key!("model-openai-config", "fields.models.label"),
                t_key!("model-openai-config", "fields.models.desc"),
            )
            .note(t_key!("model-openai-config", "fields.models.note"))
            .group(t_key!("model-openai-config", "groups.models"))
            .order(2)
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
        // 解析失败返回错误拒绝保存：静默回退默认会清空已保存的模型清单与 api_key。
        let parsed: ModelOpenAiSettings = serde_json::from_value(settings)
            .map_err(|e| ConfigError::ValidationFailed(format!("模型 OpenAI 配置解析失败: {e}")))?;
        *self.settings.write() = parsed;
        Ok(())
    }

    fn config_actions(&self) -> Vec<ConfigActionDef> {
        vec![
            ConfigActionDef {
                action: "test_connection".to_string(),
                label: t_key!("model-openai-config", "actions.test_connection.label").to_string(),
                description: t_key!("model-openai-config", "actions.test_connection.description")
                    .to_string(),
            },
            ConfigActionDef {
                action: "fetch_models".to_string(),
                label: t_key!("model-openai-config", "actions.fetch_models.label").to_string(),
                description: t_key!("model-openai-config", "actions.fetch_models.description")
                    .to_string(),
            },
        ]
    }

    async fn execute_config_action(&self, action: &str, params: &Value) -> Result<Value, String> {
        match action {
            // 使用前端表单值探测，确保未保存的地址/密钥也能先验证再落盘。
            "test_connection" => {
                let (base_url, api_key) = self.resolve_connection(params);
                let models = self.request_remote_models(&base_url, &api_key).await?;
                Ok(serde_json::json!({
                    "success": true,
                    "modelCount": models.len(),
                }))
            }
            "fetch_models" => {
                // 只读取连接配置，释放同步锁后再执行网络请求。
                let (base_url, api_key) = {
                    let settings = self.settings.read();
                    (settings.base_url.clone(), settings.api_key.clone())
                };
                let models = self
                    .request_remote_models(&base_url, &api_key)
                    .await?
                    .into_iter()
                    .map(|id| {
                        if is_embedding_model_id(&id) {
                            ModelEntryConfig::embedding(&id)
                        } else {
                            ModelEntryConfig::chat(&id)
                        }
                    })
                    .collect::<Vec<_>>();
                Ok(serde_json::json!({ "models": models }))
            }
            _ => Err(format!("未知动作: {}", action)),
        }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

use crate::plugin_framework::builtin_registry::{ConfigEntry, InventoryContext};

fn build_model_openai_config(_ctx: &InventoryContext) -> std::sync::Arc<dyn Configurable> {
    std::sync::Arc::new(ModelOpenAiConfigComponent::new())
}

::inventory::submit! {
    ConfigEntry {
        component_id: MODEL_OPENAI_CONFIG_ID,
        priority: 40,
        factory: build_model_openai_config,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证模型条目输出两个按角色显隐的能力复选框组。
    #[test]
    fn model_entry_schema_exposes_capability_groups() {
        let fields = model_entry_schema();
        let chat = fields
            .iter()
            .find(|field| field.key == "chat_capabilities")
            .expect("chat capabilities field");
        let embedding = fields
            .iter()
            .find(|field| field.key == "embedding_capabilities")
            .expect("embedding capabilities field");

        let chat_json = serde_json::to_value(chat).unwrap();
        assert_eq!(chat_json["ui"]["widget"]["kind"], "multiselect");
        assert_eq!(chat_json["schema"]["type"], "array");
        assert_eq!(
            chat_json["schema"]["items"]["enum"],
            serde_json::json!(["reasoning", "tools"])
        );
        assert_eq!(
            chat_json["schema"]["default"],
            serde_json::json!(["reasoning", "tools"])
        );

        let embedding_json = serde_json::to_value(embedding).unwrap();
        assert_eq!(embedding_json["ui"]["widget"]["kind"], "multiselect");
        assert_eq!(
            embedding_json["schema"]["items"]["enum"],
            serde_json::json!(["outputDimensions"])
        );
        assert_eq!(
            embedding_json["schema"]["default"],
            serde_json::json!(["outputDimensions"])
        );
        let context = fields
            .iter()
            .find(|field| field.key == "context_length")
            .expect("embedding context length field");
        let context_json = serde_json::to_value(context).unwrap();
        assert_eq!(context_json["ui"]["readOnly"], true);
    }
    /// 验证兼容服务缺少 created 等可选字段时仍能解析模型 id。
    #[test]
    fn models_response_accepts_minimal_items() {
        let response: ModelsResponse = serde_json::from_str(
            r#"{"object":"list","data":[{"id":"deepseek-v4-flash","object":"model"}]}"#,
        )
        .expect("minimal model response should deserialize");
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].id, "deepseek-v4-flash");
    }

    /// 验证 embedding 模型 id 识别：常见命名归类为 embedding，chat 模型不误判。
    #[test]
    fn embedding_model_id_classification() {
        for embedding_id in [
            "text-embedding-3-small",
            "text-embedding-ada-002",
            "bge-m3",
            "m3e-large",
            "nomic-embed-text",
        ] {
            assert!(
                is_embedding_model_id(embedding_id),
                "{embedding_id} 应识别为 embedding"
            );
        }
        for chat_id in ["gpt-4o-mini", "deepseek-v4-flash", "qwen3:8b"] {
            assert!(
                !is_embedding_model_id(chat_id),
                "{chat_id} 不应识别为 embedding"
            );
        }
    }
}
