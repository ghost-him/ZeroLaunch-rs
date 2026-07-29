use crate::core::config::setting_builders::SchemaBuilder;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigActionDef, ConfigError, Configurable, DataActionBinding,
    EffectActionBinding, SettingDefinition,
};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::IconRequest;

// ============================================================================
// 配置数据结构
// ============================================================================

/// 图标覆盖配置的根结构（仅存储元信息，图标数据写入缓存）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct IconOverrideSettings {
    /// 图标覆盖条目列表
    #[serde(rename = "entries", default)]
    entries: Vec<IconOverrideEntry>,
}

/// 单条图标覆盖条目（仅记录覆盖了哪些候选项，不含图标路径）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IconOverrideEntry {
    /// 目标程序路径/URL，匹配 candidate.target.payload()
    #[serde(rename = "target", default)]
    target: String,
    /// 目标类型（Path, Url, App 等），用于重建 IconRequest
    #[serde(rename = "targetType", default)]
    target_type: String,
    /// 备注
    #[serde(rename = "note", default)]
    note: String,
}

// ============================================================================
// IconOverrideConfig — 纯配置组件（ConfigEntry）
// ============================================================================

/// 图标覆盖配置组件，允许用户为指定候选项替换图标。
///
/// 使用与别名同款的 SearchTable UI，通过候选项注册表搜索程序。
/// - 使用 PathField + ConfigAction 实现：选择图标文件后立即通过 ConfigAction 处理
/// - 配置中不持久化图标文件路径（图标数据直接写入缓存）
pub struct IconOverrideConfig {
    core: ComponentCore,
    settings: RwLock<IconOverrideSettings>,
    plugin_handle: Arc<PluginHandle>,
}

impl IconOverrideConfig {
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        Self {
            core: ComponentCore::new(
                "icon-override-config".to_string(),
                "图标覆盖".to_string(),
                "为指定程序替换图标".to_string(),
                ComponentType::Core,
                10,
            ),
            settings: RwLock::new(IconOverrideSettings::default()),
            plugin_handle,
        }
    }
}

#[async_trait]
impl Configurable for IconOverrideConfig {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![SchemaBuilder::array(
            "entries",
            "图标覆盖",
            "为指定程序自定义图标，支持选择 .exe、.png、.ico 等文件",
        )
        .group("图标覆盖")
        .order(1)
        .object_items(vec![
            SchemaBuilder::text("target", "程序", "目标程序路径")
                .visible(false)
                .editable(false)
                .default("")
                .build(),
            SchemaBuilder::text("target_type", "目标类型", "目标类型（Path, Url 等）")
                .visible(false)
                .editable(false)
                .default("")
                .build(),
            SchemaBuilder::text(
                "icon_request_json",
                "图标请求",
                "原始图标请求的 JSON 序列化",
            )
            .visible(false)
            .editable(false)
            .default("")
            .build(),
            SchemaBuilder::path(
                "custom_icon_path",
                "图标文件",
                "选择新的图标文件（支持 .exe、.lnk、.png、.ico 等格式）",
            )
            .file()
            .default("")
            .effect_action(EffectActionBinding {
                action: "apply_override".into(),
                component: None,
                field_mapping: vec![
                    ("custom_icon_path".into(), "custom_icon_path".into()),
                    ("icon_request_json".into(), "icon_request_json".into()),
                ],
                transient: true,
            })
            .build(),
            // note 字段：可选备注
            SchemaBuilder::text("note", "备注", "可选备注信息")
                .default("")
                .build_field(),
        ])
        .search_table_ui()
        .data_action(DataActionBinding {
            action: "search_candidates".into(),
            component: Some("candidate-registry".into()),
            label_field: "name".into(),
            value_field: "target".into(),
            field_mapping: vec![
                ("iconRequestJson".into(), "icon_request_json".into()),
                ("targetType".into(), "target_type".into()),
            ],
        })
        .min_items(0)
        .default(serde_json::json!([]))
        .build()]
    }

    fn config_actions(&self) -> Vec<ConfigActionDef> {
        vec![ConfigActionDef {
            action: "apply_override".to_string(),
            label: "应用图标".to_string(),
            description: "立即将选择的图标文件处理后写入缓存，不保存文件路径".to_string(),
        }]
    }

    async fn execute_config_action(
        &self,
        action: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match action {
            "apply_override" => {
                let custom_icon_path = params
                    .get("custom_icon_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let icon_request_json = params
                    .get("icon_request_json")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if custom_icon_path.is_empty() || icon_request_json.is_empty() {
                    return Err("缺少必要参数".to_string());
                }

                let icon_request: IconRequest = serde_json::from_str(icon_request_json)
                    .map_err(|e| format!("解析 IconRequest 失败: {}", e))?;

                self.plugin_handle
                    .override_icon_cache(&icon_request, custom_icon_path)
                    .await
                    .map_err(|e| format!("图标覆盖失败: {}", e))?;

                Ok(serde_json::json!({
                    "success": true,
                    "custom_icon_path": "",
                }))
            }
            _ => Err(format!("未知动作: {}", action)),
        }
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: IconOverrideSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
        Ok(())
    }

    fn default_enabled(&self) -> bool {
        false
    }
}

// ============================================================================
// 注册到 inventory（作为 ConfigEntry）
// ============================================================================

use crate::plugin_framework::builtin_registry::{ConfigEntry, InventoryContext};

fn build_icon_override(ctx: &InventoryContext) -> Arc<dyn Configurable> {
    let handle = ctx.get_handle("icon-override-config");
    Arc::new(IconOverrideConfig::new(handle))
}

inventory::submit! {
    ConfigEntry {
        component_id: "icon-override-config",
        priority: 10,
        factory: build_icon_override,
    }
}
