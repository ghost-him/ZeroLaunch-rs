use super::storage_manager::{StorageClient, TEST_CONFIG_FILE_DATA, TEST_CONFIG_FILE_NAME};
use crate::core::storage::windows_utils::get_default_remote_data_dir_path;
use crate::error::{AppResult, OptionExt};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialLocalSaveConfig {
    pub destination_dir: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct LocalSaveConfigInner {
    #[serde(default = "LocalSaveConfigInner::default_destination_dir")]
    pub destination_dir: String,
}

impl Default for LocalSaveConfigInner {
    fn default() -> Self {
        Self {
            destination_dir: Self::default_destination_dir(),
        }
    }
}

impl LocalSaveConfigInner {
    pub(crate) fn default_destination_dir() -> String {
        get_default_remote_data_dir_path()
    }

    pub fn update(&mut self, partial_local_config: PartialLocalSaveConfig) {
        if let Some(destination_dir) = partial_local_config.destination_dir {
            self.destination_dir = destination_dir;
        }
    }

    pub fn to_partial(&self) -> PartialLocalSaveConfig {
        PartialLocalSaveConfig {
            destination_dir: Some(self.destination_dir.clone()),
        }
    }

    pub fn get_destination_dir(&self) -> String {
        self.destination_dir.clone()
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
        inner.get_destination_dir()
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
            remote_config_dir: local_save_config.get_remote_config_path().clone().into(),
        }
    }
}

#[async_trait]
impl StorageClient for LocalStorageInner {
    async fn download(&self, file_path: String) -> AppResult<Option<Vec<u8>>> {
        let target_path = self.remote_config_dir.join(file_path);
        // 如果没有，则直接返回空
        let path = Path::new(&target_path);
        if !path.exists() {
            return Ok(None);
        }

        match tokio::fs::read(&target_path).await {
            Ok(data) => Ok(Some(data)),
            Err(e) => Err(crate::error::AppError::StorageError {
                message: format!("{}", e),
            }),
        }
    }

    async fn upload(&self, file_path: String, data: Vec<u8>) -> AppResult<()> {
        let target_path = self.remote_config_dir.join(file_path);

        tokio::fs::create_dir_all(
            target_path
                .parent()
                .expect_programming("Target path should have a parent directory"),
        )
        .await
        .map_err(|e| crate::error::AppError::StorageError {
            message: format!("创建目录失败: {}", e),
        })?;
        tokio::fs::write(&target_path, data).await.map_err(|e| {
            crate::error::AppError::StorageError {
                message: format!("上传失败 {}: {}", target_path.display(), e),
            }
        })
    }

    async fn get_target_dir_path(&self) -> String {
        self.remote_config_dir
            .to_str()
            .expect_programming("Path should be valid UTF-8")
            .to_string()
    }

    async fn validate_config(&self) -> bool {
        if self
            .upload(
                TEST_CONFIG_FILE_NAME.to_string(),
                TEST_CONFIG_FILE_DATA.to_string().as_bytes().to_vec(),
            )
            .await
            .is_err()
        {
            return false;
        }

        if self
            .download(TEST_CONFIG_FILE_NAME.to_string())
            .await
            .is_err()
        {
            return false;
        }

        true
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
    async fn download(&self, file_path: String) -> AppResult<Option<Vec<u8>>> {
        let inner = self.inner.read().await;
        inner.download(file_path).await
    }

    async fn upload(&self, file_path: String, data: Vec<u8>) -> AppResult<()> {
        let inner = self.inner.read().await;
        inner.upload(file_path, data).await
    }

    async fn get_target_dir_path(&self) -> String {
        let inner = self.inner.read().await;
        inner.get_target_dir_path().await
    }

    async fn validate_config(&self) -> bool {
        let inner = self.inner.read().await;
        inner.validate_config().await
    }
}
