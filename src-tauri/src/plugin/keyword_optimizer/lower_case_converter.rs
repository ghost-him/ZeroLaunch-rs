use crate::core::config::setting_builders::SchemaBuilder;
use crate::plugin_system::types::{
    ComponentType, ConfigError, Configurable, KeywordOptimizer, SettingDefinition,
};
use parking_lot::RwLock;

struct LowerCaseConverterInner {
    priority: i32,
    uses_context: bool,
}

impl LowerCaseConverterInner {
    fn new() -> Self {
        Self {
            priority: 30,
            uses_context: false,
        }
    }

    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = keyword.to_lowercase();
        if result == keyword {
            Vec::new()
        } else {
            vec![result]
        }
    }
}

pub struct LowerCaseConverter {
    inner: RwLock<LowerCaseConverterInner>,
}

impl Default for LowerCaseConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl LowerCaseConverter {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(LowerCaseConverterInner::new()),
        }
    }
}

impl Configurable for LowerCaseConverter {
    fn component_id(&self) -> &str {
        "lower-case-converter"
    }

    fn component_name(&self) -> &str {
        "小写转换器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::KeywordOptimizer
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number("priority", "优先级", "优化器执行优先级，数值越小越先执行")
                .order(0)
                .default(30.0)
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
            .default(false)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        let inner = self.inner.read();
        serde_json::json!({
            "priority": inner.priority,
            "uses_context": inner.uses_context
        })
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let mut inner = self.inner.write();
        if let Some(priority) = settings.get("priority").and_then(|v| v.as_f64()) {
            inner.priority = priority as i32;
        }
        if let Some(uses_context) = settings.get("uses_context").and_then(|v| v.as_bool()) {
            inner.uses_context = uses_context;
        }
        Ok(())
    }
}

impl KeywordOptimizer for LowerCaseConverter {
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
