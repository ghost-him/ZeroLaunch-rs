use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::{
    ArrayItem, ArrayUiHint, DataSource, ExecutionTarget, FieldDefinition, SearchCandidate,
    SettingType,
};
use crate::plugin_system::{ComponentType, ConfigError, Configurable, SettingDefinition};
use crate::sdk::host_api::PluginHandle;
use crate::sdk::IconRequest;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::debug;

/// 网页数据源插件，负责从用户配置的网页列表中加载数据源候选项。
pub struct UrlSource {
    settings: RwLock<serde_json::Value>,
    #[allow(dead_code)]
    handle: Arc<PluginHandle>,
}

impl UrlSource {
    pub fn new(handle: Arc<PluginHandle>) -> Self {
        UrlSource {
            settings: RwLock::new(serde_json::Value::Null),
            handle,
        }
    }

    /// 从 settings 中解析网页列表配置
    fn parse_web_pages(&self) -> Vec<(String, String)> {
        self.settings
            .read()
            .get("web_pages")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let name = item.get("name")?.as_str()?.to_string();
                        let url = item.get("url")?.as_str()?.to_string();
                        Some((name, url))
                    })
                    .collect()
            })
            .unwrap_or_default()
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
        vec![SettingDefinition {
            field: FieldDefinition {
                key: "web_pages".to_string(),
                label: "索引网页".to_string(),
                description: "配置要索引的网页快捷方式".to_string(),
                setting_type: SettingType::Array {
                    item: ArrayItem::Object(vec![
                        FieldDefinition {
                            key: "name".to_string(),
                            label: "名称".to_string(),
                            description: "网页的显示名称".to_string(),
                            setting_type: SettingType::Text,
                            default_value: serde_json::json!(""),
                            visible: true,
                            editable: true,
                        },
                        FieldDefinition {
                            key: "url".to_string(),
                            label: "URL".to_string(),
                            description: "网页的完整网址".to_string(),
                            setting_type: SettingType::Text,
                            default_value: serde_json::json!(""),
                            visible: true,
                            editable: true,
                        },
                    ]),
                    min_items: None,
                    max_items: None,
                    ui_hint: ArrayUiHint::Table,
                },
                default_value: serde_json::json!([]),
                visible: true,
                editable: true,
            },
            group: Some("网页索引".to_string()),
            order: 1,
            config_action: None,
        }]
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        *self.settings.write() = settings;
        Ok(())
    }
}

impl DataSource for UrlSource {
    fn fetch_candidates(&self) -> CachedCandidateData {
        let mut result = CachedCandidateData::new();
        let web_pages = self.parse_web_pages();

        for (name, url) in &web_pages {
            if name.is_empty() || url.is_empty() {
                continue;
            }

            let candidate = SearchCandidate {
                id: 0,
                name: name.clone(),
                icon: IconRequest::Url(url.clone()),
                target: ExecutionTarget::Url(url.clone()),
                keywords: Vec::new(),
                bias: 0.0,
            };

            debug!("UrlSource: 加载网页候选项: {} -> {}", name, url);
            result.add_candidate(candidate);
        }

        result
    }
}
