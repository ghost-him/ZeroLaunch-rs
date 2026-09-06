use crate::core::config::setting_builders::SchemaBuilder;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{KeywordInputSource, KeywordOptimizer};

use super::input_source::{
    resolve_input_source, SOURCE_NORMALIZED_BASE, SOURCE_OPTIMIZER_OUTPUT, SOURCE_ORIGINAL_NAME,
    SOURCE_REFINED,
};

/// 空格移除器的可持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpaceRemoverSettings {
    #[serde(rename = "priority", default = "default_priority_60")]
    priority: u32,
    /// 输入来源（KeywordInputSource 的扁平 snake_case 值，见 input_source 模块常量）。
    /// 决定本优化器消费哪一层关键词产物。
    #[serde(rename = "input_source", default = "default_input_source")]
    input_source: String,
    /// 被引用生产者优化器的 component_id；仅当 input_source = optimizer_output 时生效。
    #[serde(rename = "producer_id", default)]
    producer_id: String,
}

fn default_input_source() -> String {
    SOURCE_REFINED.to_string()
}

fn default_priority_60() -> u32 {
    60
}

impl Default for SpaceRemoverSettings {
    fn default() -> Self {
        Self {
            priority: default_priority_60(),
            input_source: SOURCE_REFINED.to_string(),
            producer_id: String::new(),
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
                t_key!("space-remover", "name").to_string(),
                t_key!("space-remover", "description").to_string(),
                ComponentType::KeywordOptimizer,
                20,
            ),
            inner: RwLock::new(SpaceRemoverSettings::default()),
        }
    }
}

#[async_trait]
impl Configurable for SpaceRemover {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number(
                "priority",
                t_key!("space-remover", "fields.priority.label"),
                t_key!("space-remover", "fields.priority.desc"),
            )
            .order(0)
            .default(60.0)
            .min(1.0)
            .max(100.0)
            .step(1.0)
            .build(),
            SchemaBuilder::select(
                "input_source",
                t_key!("space-remover", "fields.input_source.label"),
                t_key!("space-remover", "fields.input_source.desc"),
            )
            .options_with_labels(&[
                (
                    SOURCE_ORIGINAL_NAME,
                    t_key!("space-remover", "options.input_source.original_name"),
                ),
                (
                    SOURCE_NORMALIZED_BASE,
                    t_key!("space-remover", "options.input_source.normalized_base"),
                ),
                (
                    SOURCE_REFINED,
                    t_key!("space-remover", "options.input_source.refined"),
                ),
                (
                    SOURCE_OPTIMIZER_OUTPUT,
                    t_key!("space-remover", "options.input_source.optimizer_output"),
                ),
            ])
            .default(SOURCE_REFINED)
            .order(1)
            .build(),
            SchemaBuilder::text(
                "producer_id",
                t_key!("space-remover", "fields.producer_id.label"),
                t_key!("space-remover", "fields.producer_id.desc"),
            )
            .default("")
            .order(2)
            .visible_when("input_source", SOURCE_OPTIMIZER_OUTPUT)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: SpaceRemoverSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }
}

#[async_trait]
impl KeywordOptimizer for SpaceRemover {
    async fn optimize(&self, keyword: &str) -> Vec<String> {
        self.inner.read().optimize(keyword)
    }

    fn input_source(&self) -> KeywordInputSource {
        let settings = self.inner.read();
        resolve_input_source(&settings.input_source, &settings.producer_id)
    }

    fn get_priority(&self) -> u32 {
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
