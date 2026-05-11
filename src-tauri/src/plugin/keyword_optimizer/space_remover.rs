use crate::core::config::setting_builders::SchemaBuilder;
use crate::plugin_system::types::{
    ComponentType, ConfigError, Configurable, KeywordOptimizer, SettingDefinition,
};
use parking_lot::RwLock;

struct SpaceRemoverInner {
    priority: i32,
    uses_context: bool,
}

impl SpaceRemoverInner {
    fn new() -> Self {
        Self {
            priority: 60,
            uses_context: true,
        }
    }

    fn remove_string_space(&self, input_text: &str) -> String {
        input_text.chars().filter(|&c| c != ' ').collect()
    }

    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = self.remove_string_space(keyword);
        if result.is_empty() || result == keyword {
            Vec::new()
        } else {
            vec![result]
        }
    }
}

pub struct SpaceRemover {
    inner: RwLock<SpaceRemoverInner>,
}

impl Default for SpaceRemover {
    fn default() -> Self {
        Self::new()
    }
}

impl SpaceRemover {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(SpaceRemoverInner::new()),
        }
    }
}

impl Configurable for SpaceRemover {
    fn component_id(&self) -> &str {
        "space-remover"
    }

    fn component_name(&self) -> &str {
        "空格移除器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::KeywordOptimizer
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number("priority", "优先级", "优化器执行优先级，数值越小越先执行")
                .order(0)
                .default(60.0)
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

impl KeywordOptimizer for SpaceRemover {
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
