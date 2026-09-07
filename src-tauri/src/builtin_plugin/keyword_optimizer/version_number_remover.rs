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

/// 版本号移除器的可持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionNumberRemoverSettings {
    #[serde(rename = "priority", default = "default_priority_10")]
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
    SOURCE_NORMALIZED_BASE.to_string()
}

fn default_priority_10() -> u32 {
    10
}

impl Default for VersionNumberRemoverSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionNumberRemoverSettings {
    fn new() -> Self {
        Self {
            priority: 10,
            input_source: SOURCE_NORMALIZED_BASE.to_string(),
            producer_id: String::new(),
        }
    }

    /// 对关键词执行版本号移除优化。
    /// 若结果与原文相同则返回空 Vec。
    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = crate::utils::remove_version_number(keyword);
        if result.is_empty() || result == keyword {
            Vec::new()
        } else {
            vec![result]
        }
    }
}

pub struct VersionNumberRemover {
    core: ComponentCore,
    inner: RwLock<VersionNumberRemoverSettings>,
}

impl Default for VersionNumberRemover {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionNumberRemover {
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "version-number-remover".to_string(),
                t_key!("version-number-remover", "name").to_string(),
                t_key!("version-number-remover", "description").to_string(),
                ComponentType::KeywordOptimizer,
                0,
            ),
            inner: RwLock::new(VersionNumberRemoverSettings::new()),
        }
    }
}

#[async_trait]
impl Configurable for VersionNumberRemover {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number(
                "priority",
                t_key!("version-number-remover", "fields.priority.label"),
                t_key!("version-number-remover", "fields.priority.desc"),
            )
            .order(0)
            .default(10.0)
            .min(1.0)
            .max(100.0)
            .step(1.0)
            .build(),
            SchemaBuilder::select(
                "input_source",
                t_key!("version-number-remover", "fields.input_source.label"),
                t_key!("version-number-remover", "fields.input_source.desc"),
            )
            .options_with_labels(&[
                (
                    SOURCE_ORIGINAL_NAME,
                    t_key!(
                        "version-number-remover",
                        "options.input_source.original_name"
                    ),
                ),
                (
                    SOURCE_NORMALIZED_BASE,
                    t_key!(
                        "version-number-remover",
                        "options.input_source.normalized_base"
                    ),
                ),
                (
                    SOURCE_REFINED,
                    t_key!("version-number-remover", "options.input_source.refined"),
                ),
                (
                    SOURCE_OPTIMIZER_OUTPUT,
                    t_key!(
                        "version-number-remover",
                        "options.input_source.optimizer_output"
                    ),
                ),
            ])
            .default(SOURCE_NORMALIZED_BASE)
            .order(1)
            .build(),
            SchemaBuilder::text(
                "producer_id",
                t_key!("version-number-remover", "fields.producer_id.label"),
                t_key!("version-number-remover", "fields.producer_id.desc"),
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
        let parsed: VersionNumberRemoverSettings =
            serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }
}

#[async_trait]
impl KeywordOptimizer for VersionNumberRemover {
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

pub(crate) fn build_version_number_remover() -> (Arc<dyn Configurable>, Arc<dyn KeywordOptimizer>) {
    let opt: Arc<dyn KeywordOptimizer> = Arc::new(VersionNumberRemover::new());
    let configurable: Arc<dyn Configurable> = opt.clone();
    (configurable, opt)
}

::inventory::submit! {
    KeywordOptimizerEntry {
        component_id: "version-number-remover",
        priority: 0,
        factory: build_version_number_remover,
    }
}
