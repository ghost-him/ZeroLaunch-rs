use crate::core::types::setting_def::{FieldDefinition, SettingDefinition, SettingType};
use crate::core::types::{ComponentType, ConfigError, Configurable};
use crate::sdk::host_api::HostApi;
use crate::sdk::storage::local_storage::LocalStorageService;
use crate::sdk::storage::storage_service::StorageService;
use crate::sdk::storage::webdav_storage::{WebDAVConfig, WebDAVStorageService};
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::info;

/// 存储配置组件。
/// 管理存储后端类型、自定义保存路径、WebDAV 连接配置。
/// 配置变更时自动重配置 HostApi 的 StorageService。
pub struct StorageConfigComponent {
    /// HostApi 引用，用于运行时重配置存储服务
    host_api: Arc<HostApi>,
    /// 当前配置状态
    settings: RwLock<serde_json::Value>,
}

impl StorageConfigComponent {
    /// 创建 StorageConfigComponent。
    /// 参数：host_api - HostApi 实例，用于重配置存储服务。
    pub fn new(host_api: Arc<HostApi>) -> Self {
        Self {
            host_api,
            settings: RwLock::new(serde_json::Value::Null),
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

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SettingDefinition {
                field: FieldDefinition {
                    key: "storage_destination".to_string(),
                    label: "存储后端".to_string(),
                    description: "选择配置文件的存储方式".to_string(),
                    setting_type: SettingType::Select {
                        options: vec!["Local".to_string(), "WebDAV".to_string()],
                    },
                    default_value: serde_json::json!("Local"),
                    visible: true,
                    editable: true,
                },
                group: Some("存储设置".to_string()),
                order: 0,
                config_action: None,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "custom_save_path".to_string(),
                    label: "自定义保存路径".to_string(),
                    description: "覆盖默认的应用数据目录（留空使用默认路径）".to_string(),
                    setting_type: SettingType::Path {
                        mode: crate::core::types::setting_def::PathMode::Directory,
                    },
                    default_value: serde_json::json!(""),
                    visible: true,
                    editable: true,
                },
                group: Some("存储设置".to_string()),
                order: 1,
                config_action: None,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "webdav_host_url".to_string(),
                    label: "WebDAV 服务器地址".to_string(),
                    description: "WebDAV 服务器的完整 URL".to_string(),
                    setting_type: SettingType::Text,
                    default_value: serde_json::json!(""),
                    visible: true,
                    editable: true,
                },
                group: Some("WebDAV 配置".to_string()),
                order: 2,
                config_action: None,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "webdav_account".to_string(),
                    label: "WebDAV 账号".to_string(),
                    description: "WebDAV 服务的认证账号".to_string(),
                    setting_type: SettingType::Text,
                    default_value: serde_json::json!(""),
                    visible: true,
                    editable: true,
                },
                group: Some("WebDAV 配置".to_string()),
                order: 3,
                config_action: None,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "webdav_password".to_string(),
                    label: "WebDAV 密码".to_string(),
                    description: "WebDAV 服务的认证密码".to_string(),
                    setting_type: SettingType::Text,
                    default_value: serde_json::json!(""),
                    visible: true,
                    editable: true,
                },
                group: Some("WebDAV 配置".to_string()),
                order: 4,
                config_action: None,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "webdav_destination_dir".to_string(),
                    label: "WebDAV 远程目录".to_string(),
                    description: "WebDAV 服务器上的目标存储目录".to_string(),
                    setting_type: SettingType::Text,
                    default_value: serde_json::json!("/ZeroLaunch-rs/"),
                    visible: true,
                    editable: true,
                },
                group: Some("WebDAV 配置".to_string()),
                order: 5,
                config_action: None,
            },
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let destination = settings
            .get("storage_destination")
            .and_then(|v| v.as_str())
            .unwrap_or("Local");

        let new_service: Arc<dyn StorageService> = match destination {
            "WebDAV" => {
                let host_url = settings
                    .get("webdav_host_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let account = settings
                    .get("webdav_account")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let password = settings
                    .get("webdav_password")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let destination_dir = settings
                    .get("webdav_destination_dir")
                    .and_then(|v| v.as_str())
                    .unwrap_or("/ZeroLaunch-rs/")
                    .to_string();

                info!("切换存储后端为 WebDAV: {}", host_url);
                let config = WebDAVConfig {
                    host_url,
                    account,
                    password,
                    destination_dir,
                };
                Arc::new(WebDAVStorageService::new(&config))
            }
            _ => {
                // Local 模式
                let custom_path = settings
                    .get("custom_save_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if custom_path.is_empty() {
                    // 使用当前 HostApi 中存储服务的目标路径（保持默认）
                    info!("切换存储后端为 Local（默认路径）");
                    let current_dir = self.host_api.storage().target_dir_path();
                    Arc::new(LocalStorageService::new(&current_dir))
                } else {
                    info!("切换存储后端为 Local（自定义路径: {}）", custom_path);
                    Arc::new(LocalStorageService::new(custom_path))
                }
            }
        };

        self.host_api.reconfigure_storage(new_service);
        *self.settings.write() = settings;
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
        let destination = self
            .settings
            .read()
            .get("storage_destination")
            .and_then(|v| v.as_str())
            .unwrap_or("Local")
            .to_string();
        info!("存储配置已变更，当前后端: {}", destination);
    }
}
