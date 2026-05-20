use crate::core::config::setting_builders::SchemaBuilder;
use crate::plugin_system::types::{
    ComponentType, ConfigError, Configurable, KeywordOptimizer, SettingDefinition,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// 符号移除器的可持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SymbolRemoverSettings {
    #[serde(rename = "priority", default = "default_priority_70")]
    priority: i32,
    #[serde(rename = "uses_context", default = "default_uses_context_true")]
    uses_context: bool,
}

fn default_priority_70() -> i32 {
    70
}

fn default_uses_context_true() -> bool {
    true
}

impl Default for SymbolRemoverSettings {
    fn default() -> Self {
        Self {
            priority: default_priority_70(),
            uses_context: default_uses_context_true(),
        }
    }
}

impl SymbolRemoverSettings {
    /// 移除字符串中所有非字母数字的符号字符
    fn remove_symbols(&self, s: &str) -> String {
        s.chars().filter(|c| c.is_alphanumeric()).collect()
    }

    /// 对关键词执行优化：移除符号，生成去符号后的变体
    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = self.remove_symbols(keyword);
        if result.is_empty() || result == keyword {
            Vec::new()
        } else {
            vec![result]
        }
    }
}

pub struct SymbolRemover {
    inner: RwLock<SymbolRemoverSettings>,
}

impl Default for SymbolRemover {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolRemover {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(SymbolRemoverSettings::default()),
        }
    }
}

impl Configurable for SymbolRemover {
    fn component_id(&self) -> &str {
        "symbol-remover"
    }

    fn component_name(&self) -> &str {
        "符号移除器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::KeywordOptimizer
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number("priority", "优先级", "优化器执行优先级，数值越小越先执行")
                .order(0)
                .default(70.0)
                .min(1.0)
                .max(100.0)
                .step(1.0)
                .build(),
            SchemaBuilder::boolean(
                "uses_context",
                "使用上下文",
                "是否对所有已累积的关键词进行优化，而非仅对原始名称优化",
            )
            .order(1)
            .default(true)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: SymbolRemoverSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }
}

impl KeywordOptimizer for SymbolRemover {
    fn optimize(&self, keyword: &str) -> Vec<String> {
        self.inner.read().optimize(keyword)
    }

    fn uses_context(&self) -> bool {
        self.inner.read().uses_context
    }

    fn get_priority(&self) -> i32 {
        self.inner.read().priority
    }
}
