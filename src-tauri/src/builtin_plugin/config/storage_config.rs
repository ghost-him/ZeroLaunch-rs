use crate::core::config::setting_builders::SchemaBuilder;
use crate::sdk::HostApi;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use zerolaunch_plugin_api::config::{ComponentType, ConfigError, Configurable, SettingDefinition};
use zerolaunch_plugin_api::services::storage::local_storage::LocalStorageService;
use zerolaunch_plugin_api::services::storage::storage_service::StorageService;
use zerolaunch_plugin_api::services::storage::webdav_storage::{
    WebDAVConfig, WebDAVStorageService,
};

/// 存储设置的强类型配置结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    #[serde(
        rename = "storage_destination",
        default = "default_storage_destination"
    )]
    pub storage_destination: String,
    #[serde(rename = "custom_save_path", default)]
    pub custom_save_path: String,
    #[serde(rename = "webdav_host_url", default)]
    pub webdav_host_url: String,
    #[serde(rename = "webdav_account", default)]
    pub webdav_account: String,
    #[serde(rename = "webdav_password", default)]
    pub webdav_password: String,
    #[serde(
        rename = "webdav_destination_dir",
        default = "default_webdav_destination_dir"
    )]
    pub webdav_destination_dir: String,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            storage_destination: default_storage_destination(),
            custom_save_path: String::new(),
            webdav_host_url: String::new(),
            webdav_account: String::new(),
            webdav_password: String::new(),
            webdav_destination_dir: default_webdav_destination_dir(),
        }
    }
}

fn default_storage_destination() -> String {
    "Local".to_string()
}

fn default_webdav_destination_dir() -> String {
    "/ZeroLaunch-rs/".to_string()
}

/// 存储配置组件。
/// 管理存储后端类型、自定义保存路径、WebDAV 连接配置。
/// 配置变更时自动重配置 HostApi 的 StorageService。
pub struct StorageConfigComponent {
    /// HostApi 引用，用于运行时重配置存储服务
    host_api: Arc<HostApi>,
    /// 当前配置状态
    settings: RwLock<StorageSettings>,
}

impl StorageConfigComponent {
    /// 创建 StorageConfigComponent。
    /// 参数：host_api - HostApi 实例，用于重配置存储服务。
    pub fn new(host_api: Arc<HostApi>) -> Self {
        Self {
            host_api,
            settings: RwLock::new(StorageSettings::default()),
        }
    }
}

impl Configurable for StorageConfigComponent {
    fn component_id(&self) -> &str {
        "storage-config"
    }

    fn component_name(&self) -> &str {
        "存储配置"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Core
    }
    fn priority(&self) -> u32 {
        30
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::select("storage_destination", "存储后端", "选择配置文件的存储方式")
                .group("存储设置")
                .order(0)
                .options(&["Local", "WebDAV"])
                .default("Local")
                .build(),
            SchemaBuilder::path(
                "custom_save_path",
                "自定义保存路径",
                "覆盖默认的应用数据目录（留空使用默认路径）",
            )
            .group("存储设置")
            .order(1)
            .directory()
            .default("")
            .build(),
            SchemaBuilder::text(
                "webdav_host_url",
                "WebDAV 服务器地址",
                "WebDAV 服务器的完整 URL",
            )
            .group("WebDAV 配置")
            .order(2)
            .default("")
            .build(),
            SchemaBuilder::text("webdav_account", "WebDAV 账号", "WebDAV 服务的认证账号")
                .group("WebDAV 配置")
                .order(3)
                .default("")
                .build(),
            SchemaBuilder::text("webdav_password", "WebDAV 密码", "WebDAV 服务的认证密码")
                .group("WebDAV 配置")
                .order(4)
                .default("")
                .build(),
            SchemaBuilder::text(
                "webdav_destination_dir",
                "WebDAV 远程目录",
                "WebDAV 服务器上的目标存储目录",
            )
            .group("WebDAV 配置")
            .order(5)
            .default("/ZeroLaunch-rs/")
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: StorageSettings = serde_json::from_value(settings).unwrap_or_else(|e| {
            warn!(
                "failed to parse settings for {}, using defaults: {e}",
                self.component_id()
            );
            Default::default()
        });
        *self.settings.write() = parsed;
        Ok(())
    }

    fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
        let destination = settings
            .get("storage_destination")
            .and_then(|v| v.as_str())
            .unwrap_or("Local");

        if destination == "WebDAV" {
            let host_url = settings
                .get("webdav_host_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let account = settings
                .get("webdav_account")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if host_url.is_empty() {
                return Err(ConfigError::ValidationFailed(
                    "WebDAV 服务器地址不能为空".to_string(),
                ));
            }
            if account.is_empty() {
                return Err(ConfigError::ValidationFailed(
                    "WebDAV 账号不能为空".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn on_settings_changed(&self) {
        let s = self.settings.read().clone();

        let new_service: Arc<dyn StorageService> = match s.storage_destination.as_str() {
            "WebDAV" => {
                info!("切换存储后端为 WebDAV: {}", s.webdav_host_url);
                let config = WebDAVConfig {
                    host_url: s.webdav_host_url,
                    account: s.webdav_account,
                    password: s.webdav_password,
                    destination_dir: s.webdav_destination_dir,
                };
                Arc::new(WebDAVStorageService::new(&config))
            }
            _ => {
                if s.custom_save_path.is_empty() {
                    info!("切换存储后端为 Local（默认路径）");
                    let current_dir = self.host_api.storage().target_dir_path();
                    Arc::new(LocalStorageService::new(&current_dir))
                } else {
                    info!("切换存储后端为 Local（自定义路径: {}）", s.custom_save_path);
                    Arc::new(LocalStorageService::new(&s.custom_save_path))
                }
            }
        };

        self.host_api.reconfigure_storage(new_service);
        info!("存储配置已变更，当前后端: {}", s.storage_destination);
    }
}

use crate::plugin_framework::builtin_registry::{ConfigEntry, InventoryContext};

fn build_storage_config(ctx: &InventoryContext) -> std::sync::Arc<dyn Configurable> {
    std::sync::Arc::new(StorageConfigComponent::new(ctx.host_api().clone()))
}

::inventory::submit! {
    ConfigEntry {
        component_id: "storage-config",
        priority: 0,
        factory: build_storage_config,
    }
}
