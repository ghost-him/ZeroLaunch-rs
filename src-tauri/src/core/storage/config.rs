use crate::core::storage::local_save::LocalSaveConfig;
use crate::core::storage::local_save::PartialLocalSaveConfig;
// use crate::core::storage::onedrive::OneDriveConfig;
// use crate::core::storage::onedrive::PartialOneDriveConfig;
use crate::core::storage::webdav::PartialWebDAVConfig;
use crate::core::storage::webdav::WebDAVConfig;
use crate::modules::config::default::APP_VERSION;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StorageDestination {
    WebDAV,
    Local,
    OneDrive,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PartialLocalConfig {
    pub storage_destination: Option<StorageDestination>,
    pub local_save_config: Option<PartialLocalSaveConfig>,
    pub webdav_save_config: Option<PartialWebDAVConfig>,
    //pub onedrive_save_config: Option<PartialOneDriveConfig>,
    pub save_to_local_per_update: Option<u32>,
    pub version: Option<String>,
    pub welcome_page_version: Option<String>,
}

#[derive(Debug)]
pub struct LocalConfig {
    // 软件的版本，用于判断当前的用户是不是更新了，默认值为空
    version: Arc<String>,
    // 表示配置信息的存储目标
    storage_destination: Arc<StorageDestination>,
    // 表示配置信息如果要保存在本地，则其使用的配置
    local_save_config: Arc<LocalSaveConfig>,
    // 表示配置信息如果要保存到 WebDAV 服务器，则其使用的配置
    webdav_save_config: Arc<WebDAVConfig>,
    //onedrive_save_config: Arc<OneDriveConfig>,
    // 表示缓冲区的大小，保存几次后会更新到存储目标。如果为0，则每次都直接上传。
    save_to_local_per_update: Arc<u32>,
    // 欢迎页面的版本号，用于判断是否需要显示欢迎页面
    welcome_page_version: Arc<String>,
}

impl Default for LocalConfig {
    fn default() -> Self {
        LocalConfig {
            version: Arc::new(String::new()),
            storage_destination: Arc::new(StorageDestination::Local),
            local_save_config: Arc::new(LocalSaveConfig::default()),
            webdav_save_config: Arc::new(WebDAVConfig::default()),
            //onedrive_save_config: Arc::new(OneDriveConfig::default()),
            save_to_local_per_update: Arc::new(0),
            welcome_page_version: Arc::new(String::new()),
        }
    }
}

impl LocalConfig {
    pub fn get_version(&self) -> Arc<String> {
        self.version.clone()
    }

    pub fn get_storage_destination(&self) -> Arc<StorageDestination> {
        self.storage_destination.clone()
    }

    pub fn get_local_save_config(&self) -> Arc<LocalSaveConfig> {
        self.local_save_config.clone()
    }

    pub fn get_webdav_save_config(&self) -> Arc<WebDAVConfig> {
        self.webdav_save_config.clone()
    }

    // pub fn get_onedrive_save_config(&self) -> Arc<OneDriveConfig> {
    //     self.onedrive_save_config.clone()
    // }

    pub fn get_save_to_local_per_update(&self) -> Arc<u32> {
        self.save_to_local_per_update.clone()
    }

    pub fn get_welcome_page_version(&self) -> Arc<String> {
        self.welcome_page_version.clone()
    }

    pub fn update(&mut self, partial_local_config: PartialLocalConfig) {
        self.version = Arc::new(APP_VERSION.to_string());
        if let Some(sd) = partial_local_config.storage_destination {
            self.storage_destination = Arc::new(sd);
        }
        if let Some(local_save_config) = partial_local_config.local_save_config {
            self.local_save_config.update(local_save_config);
        }
        if let Some(webdav_save_config) = partial_local_config.webdav_save_config {
            self.webdav_save_config.update(webdav_save_config);
        }
        // if let Some(onedrive_save_config) = partial_local_config.onedrive_save_config {
        //     self.onedrive_save_config.update(onedrive_save_config);
        // }
        if let Some(count) = partial_local_config.save_to_local_per_update {
            self.save_to_local_per_update = Arc::new(count);
        }
        if let Some(welcome_version) = partial_local_config.welcome_page_version {
            self.welcome_page_version = Arc::new(welcome_version);
        }
    }

    pub fn to_partial(&self) -> PartialLocalConfig {
        PartialLocalConfig {
            storage_destination: Some((*self.storage_destination).clone()),
            local_save_config: Some(self.local_save_config.to_partial()),
            webdav_save_config: Some(self.webdav_save_config.to_partial()),
            //onedrive_save_config: Some(self.onedrive_save_config.to_partial()),
            save_to_local_per_update: Some(*self.save_to_local_per_update),
            version: Some((*self.version).clone()),
            welcome_page_version: Some((*self.welcome_page_version).clone()),
        }
    }
}
