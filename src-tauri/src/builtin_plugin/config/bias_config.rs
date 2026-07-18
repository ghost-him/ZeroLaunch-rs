use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, FieldDefinition, SettingDefinition,
    SettingType,
};

use crate::core::config::setting_builders::SchemaBuilder;

// ============================================================================
// 配置数据结构
// ============================================================================

/// 固定偏移量配置的根结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BiasSettings {
    /// 偏移量规则条目列表
    #[serde(rename = "entries", default)]
    entries: Vec<BiasEntry>,
}

/// 单条固定偏移量规则
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BiasEntry {
    /// 目标程序标识，匹配 candidate.target.payload()
    /// 由前端 SearchTable UI 自动填充，visible: false
    /// apply_settings 时归一化为 to_ascii_lowercase()
    #[serde(rename = "target", default)]
    target: String,
    /// 权重偏移值，正值提升搜索结果位置，负值降低
    #[serde(rename = "bias", default = "BiasEntry::default_bias")]
    bias: f64,
    /// 备注信息（可选）
    #[serde(rename = "note", default)]
    note: String,
}

impl BiasEntry {
    fn default_bias() -> f64 {
        0.0
    }
}

// ============================================================================
// BiasConfig — 纯配置组件（ConfigEntry）
// ============================================================================

/// 固定偏移量配置组件。
///
/// 作为纯 Configurable 组件注册到 ConfigManager，提供搜索式表格 UI
/// 供用户为特定程序设置固定的权重偏移量。
/// 偏移量规则在 CandidatePipeline::collect() 中注入到候选项的 bias 字段，
/// 在关键字注入之后、检索引擎之前生效。
pub struct BiasConfig {
    core: ComponentCore,
    settings: RwLock<BiasSettings>,
}

impl BiasConfig {
    /// 创建 BiasConfig 实例
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "bias-config".to_string(),
                "固定偏移量".to_string(),
                "为程序设置固定权重偏移，调整其在搜索结果中的位置".to_string(),
                ComponentType::BiasRule,
                45,
            ),
            settings: RwLock::new(BiasSettings::default()),
        }
    }
}

impl Default for BiasConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Configurable for BiasConfig {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![SchemaBuilder::array(
            "entries",
            "固定偏移量",
            "为程序设置固定权重偏移，正值提升排名，负值降低排名",
        )
        .group("固定偏移量")
        .order(1)
        .object_items(vec![
            // target 字段：程序标识，由搜索栏自动关联，不在弹窗中编辑
            FieldDefinition {
                key: "target".to_string(),
                label: "程序".to_string(),
                description: "目标程序标识".to_string(),
                setting_type: SettingType::Text,
                default_value: serde_json::json!(""),
                visible: false,
                editable: false,
            },
            SchemaBuilder::number("bias", "偏移量", "正值提升排名，负值降低排名")
                .default(0.0)
                .min(-10.0)
                .max(10.0)
                .step(0.1)
                .build_field(),
            SchemaBuilder::text("note", "备注", "可选备注信息")
                .default("")
                .build_field(),
        ])
        .search_table_ui("candidate-registry", "search_candidates")
        .min_items(0)
        .default(serde_json::json!([]))
        .build()]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let mut parsed: BiasSettings = serde_json::from_value(settings).unwrap_or_default();
        // 归一化 target 为小写，避免 Windows 路径大小写不一致导致匹配失败
        for entry in &mut parsed.entries {
            entry.target = entry.target.to_ascii_lowercase();
        }
        *self.settings.write() = parsed;
        Ok(())
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

// ============================================================================
// 注册到 inventory（ConfigEntry）
// ============================================================================

use crate::plugin_framework::builtin_registry::{ConfigEntry, InventoryContext};

fn build_bias_config(_ctx: &InventoryContext) -> Arc<dyn Configurable> {
    Arc::new(BiasConfig::new())
}

inventory::submit! {
    ConfigEntry {
        component_id: "bias-config",
        priority: 45,
        factory: build_bias_config,
    }
}
