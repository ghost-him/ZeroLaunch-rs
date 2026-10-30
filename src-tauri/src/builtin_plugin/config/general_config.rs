use crate::core::config::setting_builders::SchemaBuilder;
use crate::sdk::HostApi;
use crate::utils::locale::get_default_app_language;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};

/// 通用设置的强类型配置结构。
/// 每个字段标注 `#[serde(default)]`，确保老 JSON 缺失新字段时回退到业务默认值。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    #[serde(rename = "is_auto_start", default)]
    pub is_auto_start: bool,
    #[serde(rename = "is_debug_mode", default)]
    pub is_debug_mode: bool,
    #[serde(rename = "log_level", default = "default_log_level")]
    pub log_level: String,
    #[serde(rename = "reset_session_on_wake", default = "default_true")]
    pub reset_session_on_wake: bool,
    /// 显示系统托盘图标：启用后托盘显示图标（read-at-use，启动与配置变更时经 AppState 读取）。
    #[serde(rename = "is_show_tray_icon", default = "default_true")]
    pub is_show_tray_icon: bool,
    /// 界面语言，取值 zh-Hans / zh-Hant / en（由 validate_settings 校验）。
    /// 默认值跟随系统语言（未持久化该项时每次都按当前系统语言决定）。
    #[serde(rename = "language", default = "default_language")]
    pub language: String,
    /// 启用本机 CLI HTTP 服务（`zl` 只读接口），默认启用。
    #[serde(rename = "enable_cli_server", default = "default_true")]
    pub enable_cli_server: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            is_auto_start: false,
            is_debug_mode: false,
            log_level: "info".to_string(),
            reset_session_on_wake: true,
            is_show_tray_icon: true,
            language: default_language(),
            enable_cli_server: true,
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

/// 默认界面语言 = 系统语言映射出的支持语言。
fn default_language() -> String {
    get_default_app_language()
}

fn default_true() -> bool {
    true
}

/// 通用设置配置组件：语言、自启动、调试模式、日志级别、唤醒重置、托盘图标与本机 CLI 服务。
/// 自启动与日志级别在组件内即时应用；CLI 服务开关由 bootstrap 的事件订阅启停。
pub struct GeneralConfigComponent {
    /// 组件身份核心
    core: ComponentCore,
    /// HostApi 引用，用于应用自启动配置
    host_api: Arc<HostApi>,
    /// 当前配置状态
    settings: RwLock<GeneralSettings>,
}

impl GeneralConfigComponent {
    /// 创建 GeneralConfigComponent。
    /// 参数：host_api - HostApi 实例，用于应用自启动配置。
    pub fn new(host_api: Arc<HostApi>) -> Self {
        Self {
            core: ComponentCore::new(
                "general-config".to_string(),
                t_key!("general-config", "name").to_string(),
                t_key!("general-config", "description").to_string(),
                ComponentType::Core,
                10,
            ),
            host_api,
            settings: RwLock::new(GeneralSettings::default()),
        }
    }
}

#[async_trait]
impl Configurable for GeneralConfigComponent {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::select(
                "language",
                t_key!("general-config", "fields.language.label"),
                t_key!("general-config", "fields.language.desc"),
            )
            .note(t_key!("general-config", "fields.language.note"))
            .group(t_key!("general-config", "groups.general"))
            .order(0)
            // 语言选项固定显示各语言自身名称（符合语言选择器惯例，不随界面语言翻译）
            .options_with_labels(&[
                ("zh-Hans", "简体中文"),
                ("zh-Hant", "繁體中文"),
                ("en", "English"),
            ])
            .default(default_language())
            .build(),
            SchemaBuilder::boolean(
                "is_auto_start",
                t_key!("general-config", "fields.is_auto_start.label"),
                t_key!("general-config", "fields.is_auto_start.desc"),
            )
            .note(t_key!("general-config", "fields.is_auto_start.note"))
            .group(t_key!("general-config", "groups.general"))
            .order(1)
            .default(false)
            .build(),
            SchemaBuilder::boolean(
                "is_debug_mode",
                t_key!("general-config", "fields.is_debug_mode.label"),
                t_key!("general-config", "fields.is_debug_mode.desc"),
            )
            .note(t_key!("general-config", "fields.is_debug_mode.note"))
            .group(t_key!("general-config", "groups.general"))
            .order(2)
            .default(false)
            .build(),
            SchemaBuilder::select(
                "log_level",
                t_key!("general-config", "fields.log_level.label"),
                t_key!("general-config", "fields.log_level.desc"),
            )
            .note(t_key!("general-config", "fields.log_level.note"))
            .group(t_key!("general-config", "groups.general"))
            .order(3)
            .options(&["debug", "info", "warn", "error"])
            .default("info")
            .build(),
            SchemaBuilder::boolean(
                "reset_session_on_wake",
                t_key!("general-config", "fields.reset_session_on_wake.label"),
                t_key!("general-config", "fields.reset_session_on_wake.desc"),
            )
            .group(t_key!("general-config", "groups.general"))
            .order(4)
            .default(true)
            .build(),
            SchemaBuilder::boolean(
                "is_show_tray_icon",
                t_key!("general-config", "fields.is_show_tray_icon.label"),
                t_key!("general-config", "fields.is_show_tray_icon.desc"),
            )
            .group(t_key!("general-config", "groups.general"))
            .order(5)
            .default(true)
            .build(),
            SchemaBuilder::boolean(
                "enable_cli_server",
                t_key!("general-config", "fields.enable_cli_server.label"),
                t_key!("general-config", "fields.enable_cli_server.desc"),
            )
            .note(t_key!("general-config", "fields.enable_cli_server.note"))
            .group(t_key!("general-config", "groups.general"))
            .order(6)
            .default(true)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: GeneralSettings = serde_json::from_value(settings).unwrap_or_else(|e| {
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
        // 语言
        if let Some(lang) = settings.get("language").and_then(|v| v.as_str()) {
            if !["zh-Hans", "zh-Hant", "en"].contains(&lang) {
                return Err(ConfigError::ValidationFailed(format!(
                    "Invalid language value: {}",
                    lang
                )));
            }
        }
        if let Some(level) = settings.get("log_level").and_then(|v| v.as_str()) {
            if !["debug", "info", "warn", "error"].contains(&level) {
                return Err(ConfigError::ValidationFailed(format!(
                    "无效的日志级别: {}",
                    level
                )));
            }
        }
        Ok(())
    }

    fn on_settings_changed(&self) {
        let s = self.settings.read().clone();

        let is_auto_start = s.is_auto_start;
        let host_api = self.host_api.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = host_api.apply_autostart_setting(is_auto_start).await {
                warn!("应用自启动配置失败: {}", e);
            } else {
                info!(
                    "自启动配置已更新: {}",
                    if is_auto_start { "启用" } else { "禁用" }
                );
            }
        });

        let level: tracing::Level = match s.log_level.as_str() {
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => return,
        };
        if let Err(e) = crate::logging::update_log_level(level) {
            warn!("更新日志级别失败: {}", e);
        } else {
            info!("日志级别已更新为: {}", s.log_level);
        }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

use crate::plugin_framework::builtin_registry::{ConfigEntry, InventoryContext};

fn build_general_config(ctx: &InventoryContext) -> std::sync::Arc<dyn Configurable> {
    std::sync::Arc::new(GeneralConfigComponent::new(ctx.host_api().clone()))
}

::inventory::submit! {
    ConfigEntry {
        component_id: "general-config",
        priority: 30,
        factory: build_general_config,
    }
}
