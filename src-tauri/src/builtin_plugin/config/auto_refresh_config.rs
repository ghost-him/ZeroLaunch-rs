use crate::core::config::setting_builders::SchemaBuilder;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::warn;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};

/// 定时刷新设置的强类型配置结构。
/// 反序列化自配置文件中 "auto-refresh-config" 组件的 settings 段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoRefreshSettings {
    /// 定时自动刷新间隔（分钟），0 表示禁用定时刷新；范围 0-1440。
    #[serde(
        rename = "auto_refresh_interval_mins",
        default = "default_auto_refresh_interval_mins"
    )]
    pub auto_refresh_interval_mins: f64,
}

/// 默认定时刷新间隔（分钟）：30
fn default_auto_refresh_interval_mins() -> f64 {
    30.0
}

impl Default for AutoRefreshSettings {
    fn default() -> Self {
        Self {
            auto_refresh_interval_mins: default_auto_refresh_interval_mins(),
        }
    }
}

/// 定时刷新配置组件。
/// 管理自动刷新间隔（分钟，0 表示禁用）；实际定时调度由 bootstrap 的
/// 周期任务消费该配置（读 ConfigManager → 距上次刷新超间隔则触发 refresh_candidates）。
pub struct AutoRefreshConfigComponent {
    /// 提供组件 ID、名称、类型等基础元数据。
    core: ComponentCore,
    /// 通过 RwLock 提供内部可变性，仅在 apply_settings 时写入。
    settings: RwLock<AutoRefreshSettings>,
}

impl AutoRefreshConfigComponent {
    /// 创建 AutoRefreshConfigComponent。
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "auto-refresh-config".to_string(),
                t_key!("auto-refresh-config", "name").to_string(),
                t_key!("auto-refresh-config", "description").to_string(),
                ComponentType::Core,
                50,
            ),
            settings: RwLock::new(AutoRefreshSettings::default()),
        }
    }
}

impl Default for AutoRefreshConfigComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Configurable for AutoRefreshConfigComponent {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![SchemaBuilder::number(
            "auto_refresh_interval_mins",
            t_key!(
                "auto-refresh-config",
                "fields.auto_refresh_interval_mins.label"
            ),
            t_key!(
                "auto-refresh-config",
                "fields.auto_refresh_interval_mins.desc"
            ),
        )
        .group(t_key!("auto-refresh-config", "groups.scheduledRefresh"))
        .order(0)
        .default(30.0)
        .min(0.0)
        .max(1440.0)
        .step(1.0)
        .build()]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: AutoRefreshSettings = serde_json::from_value(settings).unwrap_or_else(|e| {
            warn!(
                "failed to parse settings for {}, using defaults: {e}",
                self.component_id()
            );
            Default::default()
        });
        *self.settings.write() = parsed;
        Ok(())
    }

    async fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
        if let Some(v) = settings
            .get("auto_refresh_interval_mins")
            .and_then(|v| v.as_f64())
        {
            if !(0.0..=1440.0).contains(&v) {
                return Err(ConfigError::ValidationFailed(
                    "自动刷新间隔必须在 0-1440 分钟之间".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

use crate::plugin_framework::builtin_registry::{ConfigEntry, InventoryContext};

fn build_auto_refresh_config(_ctx: &InventoryContext) -> std::sync::Arc<dyn Configurable> {
    std::sync::Arc::new(AutoRefreshConfigComponent::new())
}

::inventory::submit! {
    ConfigEntry {
        component_id: "auto-refresh-config",
        priority: 50,
        factory: build_auto_refresh_config,
    }
}
