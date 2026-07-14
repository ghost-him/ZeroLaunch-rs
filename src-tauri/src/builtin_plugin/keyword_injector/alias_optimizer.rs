use crate::core::config::setting_builders::SchemaBuilder;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::KeywordInjector;
use zerolaunch_plugin_api::SearchCandidate;

// ============================================================================
// 配置数据结构
// ============================================================================

/// 别名配置的根结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AliasSettings {
    /// 别名条目列表
    #[serde(rename = "entries", default)]
    entries: Vec<AliasEntry>,
}

/// 单个别名条目
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AliasEntry {
    /// 目标程序路径/URL，匹配 candidate.target.payload()
    /// apply_settings 时归一化为 to_ascii_lowercase()
    #[serde(rename = "target", default)]
    target: String,
    /// 别名列表
    #[serde(rename = "aliases", default)]
    aliases: Vec<String>,
    /// 备注（可选）
    #[serde(rename = "note", default)]
    note: String,
}

// ============================================================================
// AliasOptimizer — KeywordInjector + Configurable
// ============================================================================

/// 别名优化器，根据候选项的目标路径注入用户定义的别名关键词。
/// 作为 KeywordInjector 在 CandidatePipeline 中运行，在 KeywordOptimizer 链之后执行。
pub struct AliasOptimizer {
    core: ComponentCore,
    settings: RwLock<AliasSettings>,
}

impl AliasOptimizer {
    /// 创建 AliasOptimizer 实例
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "alias-optimizer".to_string(),
                "别名".to_string(),
                "为程序设置别名，增加搜索匹配路径".to_string(),
                ComponentType::KeywordInjector,
                50,
            ),
            settings: RwLock::new(AliasSettings::default()),
        }
    }
}

impl Default for AliasOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Configurable for AliasOptimizer {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::array("entries", "别名配置", "为程序设置别名，增加搜索匹配路径")
                .group("别名")
                .order(1)
                .object_items(vec![
                    // target 字段：程序标识，由搜索栏自动关联，不在弹窗中编辑
                    zerolaunch_plugin_api::config::FieldDefinition {
                        key: "target".to_string(),
                        label: "程序".to_string(),
                        description: "目标程序路径".to_string(),
                        setting_type: zerolaunch_plugin_api::config::SettingType::Text,
                        default_value: serde_json::json!(""),
                        visible: false,
                        editable: false,
                    },
                    SchemaBuilder::array("aliases", "别名", "别名列表，输入回车添加")
                        .primitive_item(zerolaunch_plugin_api::config::PrimitiveType::Text)
                        .tags_ui()
                        .min_items(1)
                        .default(serde_json::json!([]))
                        .build_field(),
                    SchemaBuilder::text("note", "备注", "可选备注信息")
                        .default("")
                        .build_field(),
                ])
                .search_table_ui("candidate-registry", "search_candidates")
                .min_items(0)
                .default(serde_json::json!([]))
                .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let mut parsed: AliasSettings = serde_json::from_value(settings).unwrap_or_default();
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

impl KeywordInjector for AliasOptimizer {
    /// 根据候选项的目标路径注入别名关键词。
    /// 匹配方式：精确匹配 candidate.target.payload().to_ascii_lowercase() 与配置中的 target 字段。
    fn inject_keywords(&self, candidate: &SearchCandidate) -> Vec<String> {
        let target = candidate.target.payload().to_ascii_lowercase();
        let settings = self.settings.read();
        settings
            .entries
            .iter()
            .filter(|entry| entry.target == target)
            .flat_map(|entry| entry.aliases.iter().cloned())
            .collect()
    }
}

// ============================================================================
// 注册到 inventory
// ============================================================================

use crate::plugin_framework::builtin_registry::InventoryContext;
use crate::plugin_framework::builtin_registry::KeywordInjectorEntry;

fn build_alias_optimizer(
    _ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn KeywordInjector>) {
    let opt = Arc::new(AliasOptimizer::new());
    (
        opt.clone() as Arc<dyn Configurable>,
        opt as Arc<dyn KeywordInjector>,
    )
}

inventory::submit! {
    KeywordInjectorEntry {
        component_id: "alias-optimizer",
        priority: 50,
        factory: build_alias_optimizer,
    }
}
