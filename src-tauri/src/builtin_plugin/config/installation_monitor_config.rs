use crate::core::config::setting_builders::SchemaBuilder;
use crate::sdk::HostApi;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};

/// 安装监控设置的强类型配置结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationMonitorSettings {
    #[serde(rename = "enable_installation_monitor", default)]
    pub enable_installation_monitor: bool,
    #[serde(
        rename = "monitor_debounce_secs",
        default = "default_monitor_debounce_secs"
    )]
    pub monitor_debounce_secs: f64,
    #[serde(rename = "monitor_watch_paths", default)]
    pub monitor_watch_paths: String,
}

impl Default for InstallationMonitorSettings {
    fn default() -> Self {
        Self {
            // 默认关闭：与 schema 默认值（false）及设计文档一致，由用户在设置页显式开启
            enable_installation_monitor: false,
            monitor_debounce_secs: default_monitor_debounce_secs(),
            monitor_watch_paths: String::new(),
        }
    }
}

fn default_monitor_debounce_secs() -> f64 {
    5.0
}

/// 安装监控配置组件。
/// 管理安装监控的启用/禁用、监控路径及 debounce 时间。
/// 配置变更时自动启动/停止 HostApi 的安装监控服务。
pub struct InstallationMonitorConfigComponent {
    core: ComponentCore,
    /// HostApi 引用，用于控制安装监控服务
    host_api: Arc<HostApi>,
    /// 当前配置状态
    settings: RwLock<InstallationMonitorSettings>,
}

impl InstallationMonitorConfigComponent {
    /// 创建 InstallationMonitorConfigComponent。
    /// 参数：host_api - HostApi 实例，用于控制安装监控服务。
    pub fn new(host_api: Arc<HostApi>) -> Self {
        Self {
            core: ComponentCore::new(
                "installation-monitor-config".to_string(),
                t_key!("installation-monitor-config", "name").to_string(),
                t_key!("installation-monitor-config", "description").to_string(),
                ComponentType::Core,
                50,
            ),
            host_api,
            settings: RwLock::new(InstallationMonitorSettings::default()),
        }
    }
}

#[async_trait]
impl Configurable for InstallationMonitorConfigComponent {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::boolean(
                "enable_installation_monitor",
                t_key!(
                    "installation-monitor-config",
                    "fields.enable_installation_monitor.label"
                ),
                t_key!(
                    "installation-monitor-config",
                    "fields.enable_installation_monitor.desc"
                ),
            )
            .group(t_key!(
                "installation-monitor-config",
                "groups.installationMonitor"
            ))
            .order(0)
            .default(false)
            .build(),
            SchemaBuilder::number(
                "monitor_debounce_secs",
                t_key!(
                    "installation-monitor-config",
                    "fields.monitor_debounce_secs.label"
                ),
                t_key!(
                    "installation-monitor-config",
                    "fields.monitor_debounce_secs.desc"
                ),
            )
            .group(t_key!(
                "installation-monitor-config",
                "groups.installationMonitor"
            ))
            .order(1)
            .default(5.0)
            .min(1.0)
            .max(60.0)
            .step(1.0)
            .build(),
            SchemaBuilder::text(
                "monitor_watch_paths",
                t_key!(
                    "installation-monitor-config",
                    "fields.monitor_watch_paths.label"
                ),
                t_key!(
                    "installation-monitor-config",
                    "fields.monitor_watch_paths.desc"
                ),
            )
            .group(t_key!(
                "installation-monitor-config",
                "groups.installationMonitor"
            ))
            .order(2)
            .default("")
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: InstallationMonitorSettings =
            serde_json::from_value(settings).unwrap_or_else(|e| {
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
        if let Some(debounce) = settings
            .get("monitor_debounce_secs")
            .and_then(|v| v.as_f64())
        {
            if !(1.0..=60.0).contains(&debounce) {
                return Err(ConfigError::ValidationFailed(
                    "去抖等待时间必须在 1-60 秒之间".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn on_settings_changed(&self) {
        let s = self.settings.read().clone();
        let enabled = s.enable_installation_monitor;

        // 解析监控路径（每行一个，空列表表示使用平台默认开始菜单路径）
        let paths: Vec<String> = s
            .monitor_watch_paths
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        let host_api = self.host_api.clone();

        tauri::async_runtime::spawn(async move {
            // 无条件下发路径与去抖配置：空路径让平台层回退默认开始菜单路径，
            // 清空配置也能恢复默认行为（此前仅在非空时更新，导致清空不生效）。
            host_api.update_installation_monitor_paths(paths.clone());
            host_api.update_installation_monitor_debounce(s.monitor_debounce_secs);

            if enabled {
                // 路径/去抖变更后重启 watcher 使配置生效（stop 幂等，未运行则直接启动）
                if host_api.is_installation_monitor_running() {
                    if let Err(e) = host_api.stop_installation_monitor().await {
                        warn!("停止安装监控失败: {}", e);
                        return;
                    }
                }
                match host_api.start_installation_monitor().await {
                    Ok(()) => info!(
                        "安装监控已启动（监控路径 {} 条，去抖 {}s）",
                        paths.len(),
                        s.monitor_debounce_secs
                    ),
                    Err(e) => warn!("启动安装监控失败: {}", e),
                }
            } else if host_api.is_installation_monitor_running() {
                if let Err(e) = host_api.stop_installation_monitor().await {
                    warn!("停止安装监控失败: {}", e);
                } else {
                    info!("安装监控已停止");
                }
            }
        });
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

use crate::plugin_framework::builtin_registry::{ConfigEntry, InventoryContext};

fn build_installation_monitor_config(ctx: &InventoryContext) -> std::sync::Arc<dyn Configurable> {
    std::sync::Arc::new(InstallationMonitorConfigComponent::new(
        ctx.host_api().clone(),
    ))
}

::inventory::submit! {
    ConfigEntry {
        component_id: "installation-monitor-config",
        priority: 50,
        factory: build_installation_monitor_config,
    }
}
