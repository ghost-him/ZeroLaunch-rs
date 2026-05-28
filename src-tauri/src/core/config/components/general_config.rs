use crate::core::config::setting_builders::SchemaBuilder;
use crate::core::types::setting_def::SettingDefinition;
use crate::core::types::{ComponentType, ConfigError, Configurable};
use crate::sdk::host_api::HostApi;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

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
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            is_auto_start: false,
            is_debug_mode: false,
            log_level: "info".to_string(),
            reset_session_on_wake: true,
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}

/// 通用设置配置组件。
/// 管理开机自启动、调试模式和日志级别。
/// 配置变更时自动应用自启动设置和日志级别。
pub struct GeneralConfigComponent {
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
            host_api,
            settings: RwLock::new(GeneralSettings::default()),
        }
    }
}

impl Configurable for GeneralConfigComponent {
    fn component_id(&self) -> &str {
        "general"
    }

    fn component_name(&self) -> &str {
        "通用"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::boolean(
                "is_auto_start",
                "开机自启动",
                "启用后，系统启动时自动运行 ZeroLaunch",
            )
            .group("通用")
            .order(0)
            .default(false)
            .build(),
            SchemaBuilder::boolean(
                "is_debug_mode",
                "调试模式",
                "启用后，显示额外的调试信息和开发工具",
            )
            .group("通用")
            .order(1)
            .default(false)
            .build(),
            SchemaBuilder::select("log_level", "日志级别", "控制日志输出的详细程度")
                .group("通用")
                .order(2)
                .options(&["debug", "info", "warn", "error"])
                .default("info")
                .build(),
            SchemaBuilder::boolean(
                "reset_session_on_wake",
                "唤醒时重置会话",
                "启用后，每次显示启动器时恢复初始搜索界面（参数面板和行内参数模式始终恢复）。关闭后，插件面板状态可在隐藏/显示间保持。",
            )
            .group("通用")
            .order(3)
            .default(true)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: GeneralSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
        Ok(())
    }

    fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
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
