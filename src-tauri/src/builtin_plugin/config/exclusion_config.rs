use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};

use crate::core::config::exclusion_settings::{ExclusionSettings, EXCLUSION_CONFIG_ID};
use crate::core::config::setting_builders::SchemaBuilder;

// ============================================================================
// ExclusionConfig — 纯配置组件（ConfigEntry）
// ============================================================================

/// 结果排除配置组件。
///
/// 纯配置组件（`ConfigEntry`，仅实现 `Configurable`）：提供规则表格 UI，
/// 规则由 `SessionDispatcher` 在全量重建候选项时经
/// `plugin_framework::candidate_filter` 统一应用，覆盖全部数据源与插件候选
/// （区别于单个数据源内部的 forbidden_paths，后者只能剪枝自己的扫描）。
pub struct ExclusionConfig {
    /// 组件 ID、名称、类型等基础元数据。
    core: ComponentCore,
    /// 规则配置。使用 `RwLock` 提供内部可变性，仅在 `apply_settings` 时写入。
    settings: RwLock<ExclusionSettings>,
}

impl ExclusionConfig {
    /// 创建 ExclusionConfig 实例（空规则表）。
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                EXCLUSION_CONFIG_ID.to_string(),
                t_key!("exclusion-config", "name").to_string(),
                t_key!("exclusion-config", "description").to_string(),
                ComponentType::ExclusionRule,
                46,
            ),
            settings: RwLock::new(ExclusionSettings::default()),
        }
    }
}

impl Default for ExclusionConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Configurable for ExclusionConfig {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![SchemaBuilder::array(
            "rules",
            t_key!("exclusion-config", "fields.rules.label"),
            t_key!("exclusion-config", "fields.rules.desc"),
        )
        .group(t_key!("exclusion-config", "groups.exclusion"))
        .order(1)
        .object_items(vec![
            // 下拉选项值必须与 ExclusionKind 的 serde rename 一致（exact / path_prefix / contains）
            SchemaBuilder::select(
                "kind",
                t_key!("exclusion-config", "fields.kind.label"),
                t_key!("exclusion-config", "fields.kind.desc"),
            )
            .options_with_labels(&[
                ("exact", t_key!("exclusion-config", "options.kind.exact")),
                (
                    "path_prefix",
                    t_key!("exclusion-config", "options.kind.path_prefix"),
                ),
                (
                    "contains",
                    t_key!("exclusion-config", "options.kind.contains"),
                ),
            ])
            .default("exact")
            .build_field(),
            SchemaBuilder::text(
                "target",
                t_key!("exclusion-config", "fields.target.label"),
                t_key!("exclusion-config", "fields.target.desc"),
            )
            .default("")
            .build_field(),
            SchemaBuilder::text(
                "note",
                t_key!("exclusion-config", "fields.note.label"),
                t_key!("exclusion-config", "fields.note.desc"),
            )
            .default("")
            .build_field(),
        ])
        // 表格 UI（可见条目 + 增删改）：搜索结果行内联 picker（searchTable）无法展示/恢复
        // 已保存规则，而“查看并恢复已隐藏项”是命令的硬需求；条目主要来自结果页右键
        //「不再显示」，也可在此手动补录路径/关键字。
        .table_ui()
        .min_items(0)
        .default(serde_json::json!([]))
        .build()]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(&*self.settings.read()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        // 规则值不在落盘时归一化：匹配阶段统一归一化，落盘保留用户原始输入便于编辑。
        let parsed: ExclusionSettings = serde_json::from_value(settings).unwrap_or_default();
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

fn build_exclusion_config(_ctx: &InventoryContext) -> Arc<dyn Configurable> {
    Arc::new(ExclusionConfig::new())
}

::inventory::submit! {
    ConfigEntry {
        component_id: "exclusion-config",
        priority: 46,
        factory: build_exclusion_config,
    }
}
