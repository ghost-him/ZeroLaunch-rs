use crate::sdk::storage::storage_error::StorageError;
use crate::sdk::storage::storage_service::StorageService;
use async_trait::async_trait;
use reqwest_dav::{Client, ClientBuilder};
use std::path::PathBuf;
use tracing::{debug, warn};

/// WebDAV 连接配置。
/// 用于创建 WebDAVStorageService 实例。
pub struct WebDAVConfig {
    /// WebDAV 服务器地址
    pub host_url: String,
    /// 认证账号
    pub account: String,
    /// 认证密码
    pub password: String,
    /// 远程目标目录
    pub destination_dir: String,
}

/// WebDAV 远程存储服务。
/// 通过 WebDAV 协议将文件存储到远程服务器，使用 reqwest_dav 实现（跨平台）。
pub struct WebDAVStorageService {
    /// 远程目标目录
    destination_dir: PathBuf,
    /// WebDAV 客户端
    client: Option<Client>,
}

impl WebDAVStorageService {
    /// 创建 WebDAVStorageService。
    /// 参数：config - WebDAV 连接配置。
    pub fn new(config: &WebDAVConfig) -> Self {
        let client = ClientBuilder::new()
            .set_host(config.host_url.clone())
            .set_auth(reqwest_dav::Auth::Basic(
                config.account.clone(),
                config.password.clone(),
            ))
            .build()
            .ok();

        Self {
            destination_dir: PathBuf::from(&config.destination_dir),
            client,
        }
    }
}

#[async_trait]
impl StorageService for WebDAVStorageService {
    /// 将数据上传到 WebDAV 服务器。
    async fn upload(&self, file_name: &str, data: &[u8]) -> Result<(), StorageError> {
        let target_path = self.destination_dir.join(file_name);
        let target_path_str = target_path
            .to_str()
            .ok_or_else(|| StorageError::InvalidPath(file_name.to_string()))?
            .to_string();

        let client = self
            .client
            .as_ref()
            .ok_or(StorageError::ClientNotInitialized)?;

        client
            .put(&target_path_str, data.to_vec())
            .await
            .map_err(|e| StorageError::UploadFailed {
                file: file_name.to_string(),
                reason: e.to_string(),
            })?;

        debug!("WebDAV 上传完成: {}", file_name);
        Ok(())
    }

    /// 从 WebDAV 服务器下载数据。
    /// 文件不存在（404）时返回 Ok(None)。
    async fn download(&self, file_name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let target_path = self.destination_dir.join(file_name);
        let target_path_str = target_path
            .to_str()
            .ok_or_else(|| StorageError::InvalidPath(file_name.to_string()))?
            .to_string();

        let client = self
            .client
            .as_ref()
            .ok_or(StorageError::ClientNotInitialized)?;

        match client.get(&target_path_str).await {
            Ok(response) => {
                let bytes = response
                    .bytes()
                    .await
                    .map_err(|e| StorageError::DownloadFailed {
                        file: file_name.to_string(),
                        reason: format!("读取文件流失败: {}", e),
                    })?;
                debug!("WebDAV 下载完成: {}, {} bytes", file_name, bytes.len());
                Ok(Some(bytes.to_vec()))
            }
            Err(e) => {
                // 404 表示文件不存在，返回 None
                if let reqwest_dav::Error::Decode(reqwest_dav::DecodeError::Server(server_error)) =
                    &e
                {
                    if server_error.response_code == 404 {
                        debug!("WebDAV 文件不存在: {}", file_name);
                        return Ok(None);
                    }
                }
                Err(StorageError::DownloadFailed {
                    file: file_name.to_string(),
                    reason: format!("{:?}", e),
                })
            }
        }
    }

    /// 获取 WebDAV 存储的目标目录路径。
    fn target_dir_path(&self) -> String {
        self.destination_dir.to_str().unwrap_or("").to_string()
    }

    /// 验证 WebDAV 存储配置是否有效。
    /// 尝试写入并读取测试文件来验证。
    async fn validate(&self) -> bool {
        let test_file = "__zerolaunch_storage_test__.txt";
        let test_data = b"ZeroLaunch storage validation test";

        if self.upload(test_file, test_data).await.is_err() {
            warn!("WebDAV 验证上传失败");
            return false;
        }

        if self.download(test_file).await.is_err() {
            warn!("WebDAV 验证下载失败");
            return false;
        }

        true
    }
}
