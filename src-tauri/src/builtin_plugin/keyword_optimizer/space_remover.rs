use crate::core::config::setting_builders::SchemaBuilder;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::KeywordOptimizer;

/// 空格移除器的可持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpaceRemoverSettings {
    #[serde(rename = "priority", default = "default_priority_60")]
    priority: i32,
    #[serde(rename = "uses_context", default = "default_uses_context_true")]
    uses_context: bool,
}

fn default_priority_60() -> i32 {
    60
}

fn default_uses_context_true() -> bool {
    true
}

impl Default for SpaceRemoverSettings {
    fn default() -> Self {
        Self {
            priority: default_priority_60(),
            uses_context: default_uses_context_true(),
        }
    }
}

impl SpaceRemoverSettings {
    /// 移除输入字符串中的所有空格字符
    fn remove_string_space(&self, input_text: &str) -> String {
        input_text.chars().filter(|&c| c != ' ').collect()
    }

    /// 对关键词执行优化：移除空格，生成去空格后的变体
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
    core: ComponentCore,
    inner: RwLock<SpaceRemoverSettings>,
}

impl Default for SpaceRemover {
    fn default() -> Self {
        Self::new()
    }
}

impl SpaceRemover {
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "space-remover".to_string(),
                "空格移除器".to_string(),
                "移除关键词中的空格以支持紧凑匹配".to_string(),
                ComponentType::KeywordOptimizer,
                20,
            ),
            inner: RwLock::new(SpaceRemoverSettings::default()),
        }
    }
}

impl Configurable for SpaceRemover {
    fn core(&self) -> &ComponentCore {
        &self.core
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
        serde_json::to_value(self.inner.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: SpaceRemoverSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
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

use crate::plugin_framework::builtin_registry::KeywordOptimizerEntry;
use std::sync::Arc;

pub(crate) fn build_space_remover() -> (Arc<dyn Configurable>, Arc<dyn KeywordOptimizer>) {
    let opt: Arc<dyn KeywordOptimizer> = Arc::new(SpaceRemover::new());
    let configurable: Arc<dyn Configurable> = opt.clone();
    (configurable, opt)
}

::inventory::submit! {
    KeywordOptimizerEntry {
        component_id: "space-remover",
        priority: 20,
        factory: build_space_remover,
    }
}
