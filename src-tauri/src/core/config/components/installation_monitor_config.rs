use crate::core::config::setting_builders::SchemaBuilder;
use crate::core::types::setting_def::SettingDefinition;
use crate::core::types::{ComponentType, ConfigError, Configurable};
use crate::sdk::host_api::HostApi;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{info, warn};

/// 安装监控配置组件。
/// 管理安装监控的启用/禁用、监控路径及 debounce 时间。
/// 配置变更时自动启动/停止 HostApi 的安装监控服务。
pub struct InstallationMonitorConfigComponent {
    /// HostApi 引用，用于控制安装监控服务
    host_api: Arc<HostApi>,
    /// 当前配置状态
    settings: RwLock<serde_json::Value>,
}

impl InstallationMonitorConfigComponent {
    /// 创建 InstallationMonitorConfigComponent。
    /// 参数：host_api - HostApi 实例，用于控制安装监控服务。
    pub fn new(host_api: Arc<HostApi>) -> Self {
        Self {
            host_api,
            settings: RwLock::new(serde_json::Value::Null),
        }
    }
}

impl Configurable for InstallationMonitorConfigComponent {
    fn component_id(&self) -> &str {
        "installation-monitor-config"
    }

    fn component_name(&self) -> &str {
        "安装监控配置"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::boolean(
                "enable_installation_monitor",
                "启用安装监控",
                "启用后，自动监控文件系统变化（如开始菜单），检测程序的安装和卸载",
            )
            .group("安装监控")
            .order(0)
            .default(false)
            .build(),
            SchemaBuilder::number(
                "monitor_debounce_secs",
                "去抖等待时间（秒）",
                "检测到文件变化后等待的时间，避免频繁触发刷新",
            )
            .group("安装监控")
            .order(1)
            .default(5.0)
            .min(1.0)
            .max(60.0)
            .step(1.0)
            .build(),
            SchemaBuilder::text(
                "monitor_watch_paths",
                "监控路径",
                "要监控的目录路径列表（每行一个），留空使用平台默认路径（Windows 开始菜单）",
            )
            .group("安装监控")
            .order(2)
            .default("")
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        *self.settings.write() = settings;
        Ok(())
    }

    fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
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
        let settings = self.settings.read().clone();
        let enabled = settings
            .get("enable_installation_monitor")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // 解析监控路径
        let paths: Vec<String> = settings
            .get("monitor_watch_paths")
            .and_then(|v| v.as_str())
            .map(|s| {
                s.lines()
                    .map(|line| line.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let host_api = self.host_api.clone();

        tokio::spawn(async move {
            if enabled {
                // 更新监控路径
                if !paths.is_empty() {
                    host_api.update_installation_monitor_paths(paths);
                }

                // 启动监控（已启动则忽略）
                if !host_api.is_installation_monitor_running() {
                    if let Err(e) = host_api.start_installation_monitor().await {
                        warn!("启动安装监控失败: {}", e);
                    } else {
                        info!("安装监控已启动");
                    }
                } else {
                    info!("安装监控已在运行中，配置已更新（需重启监控以应用路径变更）");
                }
            } else {
                // 停止监控
                if host_api.is_installation_monitor_running() {
                    if let Err(e) = host_api.stop_installation_monitor().await {
                        warn!("停止安装监控失败: {}", e);
                    } else {
                        info!("安装监控已停止");
                    }
                }
            }
        });
    }

    fn default_enabled(&self) -> bool {
        true
    }
}
