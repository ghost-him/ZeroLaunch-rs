use std::path::{Path, PathBuf};

use super::storage_manager::StorageClient;
use crate::core::storage::windows_utils::get_default_remote_data_dir_path;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialLocalSaveConfig {
    pub remote_config_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct LocalSaveConfigInner {
    #[serde(default = "LocalSaveConfigInner::default_remote_config_path")]
    pub remote_config_path: String,
}

impl Default for LocalSaveConfigInner {
    fn default() -> Self {
        Self {
            remote_config_path: Self::default_remote_config_path(),
        }
    }
}

impl LocalSaveConfigInner {
    pub(crate) fn default_remote_config_path() -> String {
        get_default_remote_data_dir_path()
    }

    pub fn update(&mut self, partial_local_config: PartialLocalSaveConfig) {
        if let Some(remote_config_path) = partial_local_config.remote_config_path {
            self.remote_config_path = remote_config_path;
        }
    }

    pub fn to_partial(&self) -> PartialLocalSaveConfig {
        PartialLocalSaveConfig {
            remote_config_path: Some(self.remote_config_path.clone()),
        }
    }

    pub fn get_remote_config_path(&self) -> String {
        self.remote_config_path.clone()
    }
}

#[derive(Debug)]
pub struct LocalSaveConfig {
    inner: parking_lot::RwLock<LocalSaveConfigInner>,
}

impl Default for LocalSaveConfig {
    fn default() -> Self {
        Self {
            inner: parking_lot::RwLock::new(LocalSaveConfigInner::default()),
        }
    }
}

impl LocalSaveConfig {
    pub fn update(&self, partial_config: PartialLocalSaveConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }

    pub fn get_remote_config_path(&self) -> String {
        let inner = self.inner.read();
        inner.get_remote_config_path()
    }

    pub fn to_partial(&self) -> PartialLocalSaveConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }
}

pub struct LocalStorageInner {
    pub remote_config_dir: PathBuf,
}

impl LocalStorageInner {
    pub fn new(local_save_config: Arc<LocalSaveConfig>) -> Self {
        LocalStorageInner {
            remote_config_dir: local_save_config.get_remote_config_path().into(),
        }
    }
}

#[async_trait]
impl StorageClient for LocalStorageInner {
    async fn download(&self, file_path: &str) -> Result<Vec<u8>, String> {
        let target_path = self.remote_config_dir.join(file_path);
        println!(
            "client: 开始下载：{}",
            target_path.to_str().unwrap().to_string()
        );
        tokio::fs::read(&target_path)
            .await
            .map_err(|e| format!("下载失败 {}: {}", target_path.display(), e))
    }

    async fn upload(&self, file_path: &str, data: &[u8]) -> Result<(), String> {
        let target_path = self.remote_config_dir.join(file_path);

        tokio::fs::create_dir_all(target_path.parent().unwrap())
            .await
            .map_err(|e| format!("创建目录失败: {}", e))?;
        println!(
            "client: 上传成功{}",
            target_path.to_str().unwrap().to_string()
        );
        tokio::fs::write(&target_path, data)
            .await
            .map_err(|e| format!("上传失败 {}: {}", target_path.display(), e))
    }

    async fn get_target_dir_path(&self) -> String {
        self.remote_config_dir.to_str().unwrap().to_string()
    }
}

pub struct LocalStorage {
    pub inner: tokio::sync::RwLock<LocalStorageInner>,
}

impl LocalStorage {
    /// 创建一个新的 LocalStorage 实例
    pub fn new(local_save_config: Arc<LocalSaveConfig>) -> Self {
        LocalStorage {
            inner: tokio::sync::RwLock::new(LocalStorageInner::new(local_save_config)),
        }
    }
}

#[async_trait]
impl StorageClient for LocalStorage {
    async fn download(&self, file_path: &str) -> Result<Vec<u8>, String> {
        let inner = self.inner.read().await;
        inner.download(file_path).await
    }

    async fn upload(&self, file_path: &str, data: &[u8]) -> Result<(), String> {
        let inner = self.inner.read().await;
        inner.upload(file_path, data).await
    }

    async fn get_target_dir_path(&self) -> String {
        let inner = self.inner.read().await;
        inner.get_target_dir_path().await
    }
}
