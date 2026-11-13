//! 内置模型提供方的连接配置与模型条目。
//!
//! 连接配置属于宿主 provider 层；模型条目复用 plugin-api 的能力枚举，
//! 并按 chat / embedding 分支承载各自的配置字段。

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use zerolaunch_plugin_api::services::model::{
    ChatCapability, EmbeddingCapability, ModelSimilarity,
};

/// chat 模型的通用生成参数与模型元数据。
///
/// 仅由 `ModelEntryConfig::Chat` 使用，配置字段不绑定具体 provider。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatModelConfig {
    /// 采样温度（0-2）；未配置时使用 0.7。
    #[serde(default = "default_chat_temperature")]
    pub temperature: f32,
    /// 单次最大输出 token；未配置时使用 2048。
    #[serde(default = "default_chat_max_tokens")]
    pub max_tokens: u32,
    /// 核采样阈值（0-1）；未配置时使用 1.0。
    #[serde(default = "default_chat_top_p")]
    pub top_p: f32,
    /// 深度思考档位；未配置时使用 none。
    #[serde(default = "default_reasoning_effort")]
    pub reasoning_effort: String,
    /// provider 声明的最大上下文长度；0 表示尚未获取。
    #[serde(default = "default_context_length")]
    pub context_length: u32,
    /// 模型声明支持的 chat 能力清单。
    #[serde(default = "default_chat_capabilities", rename = "chat_capabilities")]
    pub capabilities: Vec<ChatCapability>,
}

impl Default for ChatModelConfig {
    fn default() -> Self {
        Self {
            temperature: default_chat_temperature(),
            max_tokens: default_chat_max_tokens(),
            top_p: default_chat_top_p(),
            reasoning_effort: default_reasoning_effort(),
            context_length: default_context_length(),
            capabilities: default_chat_capabilities(),
        }
    }
}
/// 返回 chat 温度默认值。
fn default_chat_temperature() -> f32 {
    0.7
}

/// 返回 chat 最大输出 token 默认值。
fn default_chat_max_tokens() -> u32 {
    2048
}

/// 返回 chat top-p 默认值。
fn default_chat_top_p() -> f32 {
    1.0
}

/// 返回 reasoning 档位默认值。
fn default_reasoning_effort() -> String {
    "none".to_string()
}

/// 返回未获取模型元数据时的上下文长度值。
fn default_context_length() -> u32 {
    0
}

/// embedding 模型的通用输出配置与模型元数据。
///
/// 仅由 `ModelEntryConfig::Embedding` 使用，配置字段不绑定具体 provider。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingModelConfig {
    /// embedding 输出维度；未配置时使用 768，拉取元数据后由 provider 覆盖。
    #[serde(default = "default_embedding_dimensions")]
    pub dimensions: u32,
    /// provider 声明的最大输入上下文长度；0 表示尚未获取。
    #[serde(default = "default_context_length")]
    pub context_length: u32,
    /// 模型推荐的向量相似度计算方式。
    #[serde(default)]
    pub similarity: ModelSimilarity,
    /// 模型声明支持的 embedding 能力清单。
    #[serde(
        default = "default_embedding_capabilities",
        rename = "embedding_capabilities"
    )]
    pub capabilities: Vec<EmbeddingCapability>,
}

impl Default for EmbeddingModelConfig {
    fn default() -> Self {
        Self {
            dimensions: default_embedding_dimensions(),
            context_length: default_context_length(),
            similarity: ModelSimilarity::default(),
            capabilities: default_embedding_capabilities(),
        }
    }
}

/// 返回 embedding 维度默认值。
fn default_embedding_dimensions() -> u32 {
    768
}

/// 单个模型条目：按模型种类隔离宿主配置字段。
///
/// 由 model-*-config 组件持久化并由 OpenAI/Ollama provider 消费；
/// Chat 与 Embedding 分支使用相同的 `kind` 平铺 JSON 形式。
#[derive(Debug, Clone, PartialEq)]
pub enum ModelEntryConfig {
    /// chat 模型条目；由文本生成 provider 清单与 chat 调用消费。
    Chat {
        /// provider 内部模型名；空串条目会被 provider 忽略。
        name: String,
        /// chat 模型默认参数与能力声明。
        config: ChatModelConfig,
    },
    /// embedding 模型条目；由向量 provider 清单与 embedding 调用消费。
    Embedding {
        /// provider 内部模型名；空串条目会被 provider 忽略。
        name: String,
        /// embedding 输出配置与能力声明。
        config: EmbeddingModelConfig,
    },
}

/// 模型条目的内部平铺序列化表示。
///
/// 仅限本文件内使用，用于保持设置 JSON 的 `kind` 标签与 snake_case 字段。
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
enum ModelEntryConfigRepr {
    /// chat 平铺配置，缺失字段使用 ChatModelConfig 默认值。
    #[serde(rename = "chat")]
    Chat {
        /// provider 内部模型名。
        #[serde(rename = "name", default)]
        name: String,
        /// chat 配置字段，平铺到条目对象。
        #[serde(flatten)]
        config: ChatModelConfig,
    },
    /// embedding 平铺配置，缺失字段使用 EmbeddingModelConfig 默认值。
    #[serde(rename = "embedding")]
    Embedding {
        /// provider 内部模型名。
        #[serde(rename = "name", default)]
        name: String,
        /// embedding 配置字段，平铺到条目对象。
        #[serde(flatten)]
        config: EmbeddingModelConfig,
    },
}

impl From<ModelEntryConfig> for ModelEntryConfigRepr {
    fn from(value: ModelEntryConfig) -> Self {
        match value {
            ModelEntryConfig::Chat { name, config } => Self::Chat { name, config },
            ModelEntryConfig::Embedding { name, config } => Self::Embedding { name, config },
        }
    }
}

impl From<ModelEntryConfigRepr> for ModelEntryConfig {
    fn from(value: ModelEntryConfigRepr) -> Self {
        match value {
            ModelEntryConfigRepr::Chat { name, config } => Self::Chat { name, config },
            ModelEntryConfigRepr::Embedding { name, config } => Self::Embedding { name, config },
        }
    }
}

impl Serialize for ModelEntryConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 下发形状的唯一构造是 ModelEntryConfigRepr 的 From 实现，此处只做转接。
        ModelEntryConfigRepr::from(self.clone()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ModelEntryConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut value = Value::deserialize(deserializer)?;
        if let Value::Object(object) = &mut value {
            object
                .entry("kind".to_string())
                .or_insert_with(|| Value::String("chat".to_string()));
        }
        ModelEntryConfigRepr::deserialize(value)
            .map(Into::into)
            .map_err(serde::de::Error::custom)
    }
}

impl ModelEntryConfig {
    /// 创建 chat 条目，供 fetch action 将远端模型归类为文本生成模型。
    pub fn chat(name: &str) -> Self {
        Self::Chat {
            name: name.to_string(),
            config: ChatModelConfig::default(),
        }
    }

    /// 创建 embedding 条目，供 Ollama 拉取动作按官方能力归类。
    pub fn embedding(name: &str) -> Self {
        Self::Embedding {
            name: name.to_string(),
            config: EmbeddingModelConfig::default(),
        }
    }

    /// 返回 provider 内部模型名，供清单和调用路由匹配。
    pub fn name(&self) -> &str {
        match self {
            Self::Chat { name, .. } | Self::Embedding { name, .. } => name,
        }
    }
}

/// chat 条目的默认能力声明。
///
/// 默认声明 `tools` 与 `reasoning`：主流 chat 模型均已支持工具调用，默认勾选免去逐条配置；
/// 能力清单仍是 provider 校验请求的依据，模型不支持时用户需在条目里取消勾选。
fn default_chat_capabilities() -> Vec<ChatCapability> {
    vec![ChatCapability::Reasoning, ChatCapability::Tools]
}

/// embedding 条目的默认能力声明。
fn default_embedding_capabilities() -> Vec<EmbeddingCapability> {
    vec![EmbeddingCapability::OutputDimensions]
}

/// OpenAI 兼容服务的连接配置。
///
/// 经 model-openai-config 组件持久化，OpenAiCompatibleProvider 消费。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOpenAiSettings {
    /// API 基础地址；空值不合法时由 provider 返回错误。
    #[serde(default = "default_openai_base_url")]
    pub base_url: String,
    /// API Key；空字符串表示未配置认证信息。
    #[serde(default)]
    pub api_key: String,
    /// 用户声明的模型条目清单。
    #[serde(default)]
    pub models: Vec<ModelEntryConfig>,
}

/// OpenAI 兼容服务的默认 API 地址。
fn default_openai_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

impl Default for ModelOpenAiSettings {
    fn default() -> Self {
        Self {
            base_url: default_openai_base_url(),
            api_key: String::new(),
            models: Vec::new(),
        }
    }
}

/// Ollama 服务的连接配置。
///
/// 经 model-ollama-config 组件持久化，OllamaProvider 消费。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOllamaSettings {
    /// Ollama 服务地址；默认指向本机 11434 端口。
    #[serde(default = "default_ollama_base_url")]
    pub base_url: String,
    /// 用户声明的模型条目清单；空清单表示本地模型全部按 chat 列出。
    #[serde(default)]
    pub models: Vec<ModelEntryConfig>,
}

/// Ollama 服务的默认地址。
fn default_ollama_base_url() -> String {
    "http://localhost:11434".to_string()
}

impl Default for ModelOllamaSettings {
    fn default() -> Self {
        Self {
            base_url: default_ollama_base_url(),
            models: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_settings_missing_fields_fall_back_to_defaults() {
        let s: ModelOpenAiSettings =
            serde_json::from_value(serde_json::json!({ "base_url": "http://x/v1" })).unwrap();
        assert_eq!(s.base_url, "http://x/v1");
        assert_eq!(s.api_key, "");
        assert!(s.models.is_empty());
    }

    #[test]
    fn entry_without_kind_defaults_to_chat() {
        let entry: ModelEntryConfig =
            serde_json::from_value(serde_json::json!({ "name": "qwen3:8b" })).unwrap();
        assert!(matches!(entry, ModelEntryConfig::Chat { .. }));
    }

    #[test]
    fn embedding_entry_uses_only_embedding_configuration() {
        let entry: ModelEntryConfig = serde_json::from_value(serde_json::json!({
            "name": "gemma-embedding",
            "kind": "embedding",
            "context_length": 8192,
            "similarity": "dotProduct",
            "embedding_capabilities": ["outputDimensions"]
        }))
        .unwrap();
        let ModelEntryConfig::Embedding { config, .. } = entry else {
            panic!("expected embedding entry");
        };
        assert_eq!(config.context_length, 8192);
        assert_eq!(config.similarity, ModelSimilarity::DotProduct);
    }

    #[test]
    fn entry_roundtrip_preserves_kind_specific_params() {
        let entry = ModelEntryConfig::Chat {
            name: "gpt-4o-mini".to_string(),
            config: ChatModelConfig {
                temperature: 0.7,
                max_tokens: 1024,
                top_p: 0.9,
                reasoning_effort: "medium".to_string(),
                context_length: 128000,
                ..Default::default()
            },
        };
        let json = serde_json::to_value(&entry).unwrap();
        let back: ModelEntryConfig = serde_json::from_value(json).unwrap();
        assert_eq!(back, entry);
    }
}
