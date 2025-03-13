use crate::core::storage::local_save::LocalSaveConfig;
use crate::core::storage::local_save::PartialLocalSaveConfig;
// use crate::core::storage::onedrive::OneDriveConfig;
// use crate::core::storage::onedrive::PartialOneDriveConfig;
use crate::core::storage::webdav::PartialWebDAVConfig;
use crate::core::storage::webdav::WebDAVConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StorageDestination {
    WebDAV,
    Local,
    OneDrive,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialLocalConfig {
    pub storage_destination: Option<StorageDestination>,
    pub local_save_config: Option<PartialLocalSaveConfig>,
    pub webdav_save_config: Option<PartialWebDAVConfig>,
    //pub onedrive_save_config: Option<PartialOneDriveConfig>,
    pub save_to_local_per_update: Option<u32>,
}

impl Default for PartialLocalConfig {
    /// 默认为全空
    fn default() -> Self {
        PartialLocalConfig {
            storage_destination: None,
            local_save_config: None,
            webdav_save_config: None,
            //onedrive_save_config: None,
            save_to_local_per_update: None,
        }
    }
}

#[derive(Debug)]
pub struct LocalConfig {
    storage_destination: Arc<StorageDestination>,
    local_save_config: Arc<LocalSaveConfig>,
    webdav_save_config: Arc<WebDAVConfig>,
    //onedrive_save_config: Arc<OneDriveConfig>,
    save_to_local_per_update: Arc<u32>,
}

impl Default for LocalConfig {
    fn default() -> Self {
        LocalConfig {
            storage_destination: Arc::new(StorageDestination::Local),
            local_save_config: Arc::new(LocalSaveConfig::default()),
            webdav_save_config: Arc::new(WebDAVConfig::default()),
            //onedrive_save_config: Arc::new(OneDriveConfig::default()),
            save_to_local_per_update: Arc::new(4),
        }
    }
}

impl LocalConfig {
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

    pub fn update(&mut self, partial_local_config: PartialLocalConfig) {
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
    }

    pub fn to_partial(&self) -> PartialLocalConfig {
        PartialLocalConfig {
            storage_destination: Some((*self.storage_destination).clone()),
            local_save_config: Some(self.local_save_config.to_partial()),
            webdav_save_config: Some(self.webdav_save_config.to_partial()),
            //onedrive_save_config: Some(self.onedrive_save_config.to_partial()),
            save_to_local_per_update: Some(*self.save_to_local_per_update),
        }
    }
}
