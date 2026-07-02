use crate::core::config::setting_builders::SchemaBuilder;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;
use zerolaunch_plugin_api::config::{ComponentType, ConfigError, Configurable, SettingDefinition};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{CachedCandidateData, DataSource, ExecutionTarget, SearchCandidate};

/// 单个网页快捷方式的配置项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlEntry {
    #[serde(rename = "name", default)]
    pub name: String,
    #[serde(rename = "url", default)]
    pub url: String,
    /// 触发关键词，逗号分隔。为空时使用 name 作为默认触发词
    #[serde(rename = "triggerKeywords", default)]
    pub trigger_keywords: String,
}

/// 网页数据源的强类型配置结构。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UrlSourceSettings {
    #[serde(rename = "web_pages", default)]
    pub web_pages: Vec<UrlEntry>,
}

/// 网页数据源插件，负责从用户配置的网页列表中加载数据源候选项。
pub struct UrlSource {
    settings: RwLock<UrlSourceSettings>,
    #[allow(dead_code)]
    handle: Arc<PluginHandle>,
}

impl UrlSource {
    pub fn new(handle: Arc<PluginHandle>) -> Self {
        UrlSource {
            settings: RwLock::new(UrlSourceSettings::default()),
            handle,
        }
    }
}

impl Configurable for UrlSource {
    fn component_id(&self) -> &str {
        "url-source"
    }

    fn component_name(&self) -> &str {
        "网页数据源"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::DataSource
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::array("web_pages", "索引网页", "配置要索引的网页快捷方式")
                .group("网页索引")
                .order(1)
                .object_items(vec![
                    SchemaBuilder::text("name", "名称", "网页的显示名称")
                        .default("")
                        .build_field(),
                    SchemaBuilder::text("url", "URL", "网页的完整网址")
                        .default("")
                        .build_field(),
                    SchemaBuilder::text(
                        "triggerKeywords",
                        "触发关键词",
                        "逗号分隔的触发词列表。输入触发词+空格进入参数模式。为空时默认使用名称。",
                    )
                    .default("")
                    .build_field(),
                ])
                .table_ui()
                .default(serde_json::json!([]))
                .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: UrlSourceSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
        Ok(())
    }
}

#[async_trait]
impl DataSource for UrlSource {
    async fn fetch_candidates(&self) -> CachedCandidateData {
        let mut result = CachedCandidateData::new();
        let s = self.settings.read();

        for entry in &s.web_pages {
            if entry.name.is_empty() || entry.url.is_empty() {
                continue;
            }

            // 解析触发关键词：逗号分隔，去空白，过滤空值；为空时默认使用名称
            let trigger_keywords: Vec<String> = if entry.trigger_keywords.is_empty() {
                vec![entry.name.to_lowercase()]
            } else {
                entry
                    .trigger_keywords
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            };
            let candidate = SearchCandidate {
                id: 0,
                name: entry.name.clone(),
                icon: IconRequest::Url(entry.url.clone()),
                target: ExecutionTarget::Url(entry.url.clone()),
                keywords: Vec::new(),
                bias: 0.0,
                trigger_keywords,
            };

            debug!("UrlSource: 加载网页候选项: {} -> {}", entry.name, entry.url);
            result.add_candidate(candidate);
        }

        result
    }
}

use crate::plugin_framework::builtin_registry::{DataSourceEntry, InventoryContext};

pub(crate) fn build_url_source(
    ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn DataSource>) {
    let handle = ctx.get_handle("url-source");
    let source: Arc<dyn DataSource> = Arc::new(UrlSource::new(handle));
    let configurable: Arc<dyn Configurable> = source.clone();
    (configurable, source)
}

::inventory::submit! {
    DataSourceEntry {
        component_id: "url-source",
        handle_key: "url-source",
        priority: 20,
        factory: build_url_source,
    }
}
