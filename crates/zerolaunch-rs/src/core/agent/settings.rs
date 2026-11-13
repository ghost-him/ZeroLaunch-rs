//! 助手配置：组件 id 与强类型设置。
//!
//! 与 `core/model/settings.rs` 同构：消费侧（本模块）拥有配置 id 与结构，
//! 配置组件（`builtin_plugin/config/help_agent_config.rs`）引用它们，
//! 字段名只声明一次。

use serde::{Deserialize, Serialize};

/// 助手配置组件 id。
pub const HELP_AGENT_CONFIG_ID: &str = "help-agent-config";

/// 助手设置。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HelpAgentSettings {
    /// 助手使用的模型全局 id（`{provider}/{model}`，如 `ollama/qwen3:8b`）；
    /// 空串表示未配置，助手不可用。
    #[serde(rename = "model_id", default)]
    pub model_id: String,
}
