use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialOneDriveConfig {
    pub folder_path: Option<String>,
    pub sync_enabled: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct OneDriveConfigInner {
    #[serde(default = "OneDriveConfigInner::default_folder_path")]
    pub folder_path: String,
    #[serde(default = "OneDriveConfigInner::default_sync_enabled")]
    pub sync_enabled: bool,
}

impl Default for OneDriveConfigInner {
    fn default() -> Self {
        Self {
            folder_path: Self::default_folder_path(),
            sync_enabled: Self::default_sync_enabled(),
        }
    }
}

impl OneDriveConfigInner {
    pub(crate) fn default_folder_path() -> String {
        "zerolaunch-rs".to_string()
    }

    pub(crate) fn default_sync_enabled() -> bool {
        false
    }

    pub fn update(&mut self, partial_onedrive_config: PartialOneDriveConfig) {
        if let Some(folder_path) = partial_onedrive_config.folder_path {
            self.folder_path = folder_path;
        }
        if let Some(sync_enabled) = partial_onedrive_config.sync_enabled {
            self.sync_enabled = sync_enabled;
        }
    }

    pub fn to_partial(&self) -> PartialOneDriveConfig {
        PartialOneDriveConfig {
            folder_path: Some(self.folder_path.clone()),
            sync_enabled: Some(self.sync_enabled),
        }
    }

    pub fn get_folder_path(&self) -> String {
        self.folder_path.clone()
    }

    pub fn is_sync_enabled(&self) -> bool {
        self.sync_enabled
    }
}

#[derive(Debug)]
pub struct OneDriveConfig {
    inner: RwLock<OneDriveConfigInner>,
}

impl Default for OneDriveConfig {
    fn default() -> Self {
        Self {
            inner: RwLock::new(OneDriveConfigInner::default()),
        }
    }
}

impl OneDriveConfig {
    pub fn update(&self, partial_config: PartialOneDriveConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }

    pub fn get_folder_path(&self) -> String {
        let inner = self.inner.read();
        inner.get_folder_path()
    }

    pub fn is_sync_enabled(&self) -> bool {
        let inner = self.inner.read();
        inner.is_sync_enabled()
    }

    pub fn to_partial(&self) -> PartialOneDriveConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }
}
