use std::path::PathBuf;

use crate::storage::storage_manager::StorageClient;
use crate::storage::storage_manager::{TEST_CONFIG_FILE_DATA, TEST_CONFIG_FILE_NAME};
use async_trait::async_trait;
use parking_lot::RwLock;
use reqwest_dav::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialWebDAVConfig {
    pub host_url: Option<String>,
    pub account: Option<String>,
    pub password: Option<String>,
    pub destination_dir: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct WebDAVConfigInner {
    #[serde(default = "WebDAVConfigInner::default_host_url")]
    pub host_url: String,
    #[serde(default = "WebDAVConfigInner::default_account")]
    pub account: String,
    #[serde(default = "WebDAVConfigInner::default_password")]
    pub password: String,
    #[serde(default = "WebDAVConfigInner::default_destination_dir")]
    pub destination_dir: String,
}

impl Default for WebDAVConfigInner {
    fn default() -> Self {
        Self {
            host_url: Self::default_host_url(),
            account: Self::default_account(),
            password: Self::default_password(),
            destination_dir: Self::default_destination_dir(),
        }
    }
}

impl WebDAVConfigInner {
    // 默认值方法
    pub(crate) fn default_host_url() -> String {
        String::new()
    }

    pub(crate) fn default_account() -> String {
        String::new()
    }

    pub(crate) fn default_password() -> String {
        String::new()
    }

    pub(crate) fn default_destination_dir() -> String {
        "".into() // 可根据需求修改默认路径
    }

    // 更新方法
    pub fn update(&mut self, partial_config: PartialWebDAVConfig) {
        if let Some(host_url) = partial_config.host_url {
            self.host_url = host_url;
        }
        if let Some(account) = partial_config.account {
            self.account = account;
        }
        if let Some(password) = partial_config.password {
            self.password = password;
        }
        if let Some(dir) = partial_config.destination_dir {
            self.destination_dir = dir;
        }
    }

    // 转换方法
    pub fn to_partial(&self) -> PartialWebDAVConfig {
        PartialWebDAVConfig {
            host_url: Some(self.host_url.clone()),
            account: Some(self.account.clone()),
            password: Some(self.password.clone()),
            destination_dir: Some(self.destination_dir.clone()),
        }
    }

    // 访问方法
    pub fn get_host_url(&self) -> &str {
        &self.host_url
    }

    pub fn get_account(&self) -> &str {
        &self.account
    }

    pub fn get_password(&self) -> &str {
        &self.password
    }

    pub fn get_destination_dir(&self) -> &str {
        &self.destination_dir
    }
}

#[derive(Debug)]
pub struct WebDAVConfig {
    inner: RwLock<WebDAVConfigInner>,
}

impl Default for WebDAVConfig {
    fn default() -> Self {
        Self {
            inner: RwLock::new(WebDAVConfigInner::default()),
        }
    }
}

impl WebDAVConfig {
    pub fn update(&self, partial_config: PartialWebDAVConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }

    pub fn to_partial(&self) -> PartialWebDAVConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }

    pub fn get_destination_dir(&self) -> String {
        let inner = self.inner.read();
        inner.destination_dir.clone()
    }

    pub fn get_host_url(&self) -> String {
        let inner = self.inner.read();
        inner.host_url.clone()
    }

    pub fn get_account(&self) -> String {
        let inner = self.inner.read();
        inner.account.clone()
    }

    pub fn get_password(&self) -> String {
        let inner = self.inner.read();
        inner.password.clone()
    }
}

pub struct WebDAVStorageInner {
    pub destination_dir: PathBuf,
    pub host_url: String,
    pub account: String,
    pub password: String,
    pub client: Option<Client>,
}

impl WebDAVStorageInner {
    pub fn new(webdav_config: Arc<WebDAVConfig>) -> Self {
        let mut inner = WebDAVStorageInner {
            destination_dir: webdav_config.get_destination_dir().clone().into(),
            host_url: webdav_config.get_host_url(),
            account: webdav_config.get_account(),
            password: webdav_config.get_password(),
            client: None,
        };
        inner.client = {
            if let Ok(client) = ClientBuilder::new()
                .set_host(inner.host_url.clone())
                .set_auth(reqwest_dav::Auth::Basic(
                    inner.account.clone(),
                    inner.password.clone(),
                ))
                .build()
            {
                Some(client)
            } else {
                None
            }
        };
        inner
    }
}

#[async_trait]
impl StorageClient for WebDAVStorageInner {
    // 要可以上传文件
    async fn upload(&self, file_name: String, data: Vec<u8>) -> Result<(), String> {
        let target_path = self.destination_dir.join(file_name);
        let target_path = target_path.to_str().unwrap().to_string();
        if let Some(client) = self.client.as_ref() {
            if let Err(e) = client.put(&target_path, data).await {
                return Err(e.to_string());
            }
        } else {
            return Err("当前无客户端连接".to_string());
        }
        Ok(())
    }
    // 要可以下载文件
    async fn download(&self, file_name: String) -> Result<Option<Vec<u8>>, String> {
        let target_path = self.destination_dir.join(file_name);
        let target_path = target_path.to_str().unwrap().to_string();
        if let Some(client) = self.client.as_ref() {
            // 直接尝试下载文件
            match client.get(&target_path).await {
                Ok(response) => response
                    .bytes()
                    .await
                    .map(|bytes| Some(bytes.to_vec()))
                    .map_err(|e| format!("读取文件流失败: {}", e)),

                Err(e) => {
                    if let reqwest_dav::Error::Decode(decode_error) = e {
                        if let reqwest_dav::DecodeError::Server(server_error) = decode_error {
                            if server_error.response_code == 404 {
                                println!("收到404");
                                return Ok(None);
                            } else {
                                return Err(format!("{:?}", server_error));
                            }
                        } else {
                            return Err(format!("{:?}", decode_error));
                        }
                    } else {
                        return Err(format!("{:?}", e));
                    }
                }
            }
        } else {
            return Err("当前无客户端连接".to_string());
        }
    }
    // 要可以获得当前文件的目标路径
    async fn get_target_dir_path(&self) -> String {
        self.destination_dir.to_str().unwrap().to_string()
    }

    async fn validate_config(&self) -> bool {
        if self
            .upload(
                TEST_CONFIG_FILE_NAME.to_string(),
                TEST_CONFIG_FILE_DATA.to_string().as_bytes().to_vec(),
            )
            .await.is_err()
        {
            return false;
        }

        if self.download(TEST_CONFIG_FILE_NAME.to_string()).await.is_err() {
            return false;
        }

        true
    }
}

pub struct WebDAVStorage {
    pub inner: tokio::sync::RwLock<WebDAVStorageInner>,
}

impl WebDAVStorage {
    /// 创建一个新的 LocalStorage 实例
    pub fn new(local_save_config: Arc<WebDAVConfig>) -> Self {
        WebDAVStorage {
            inner: tokio::sync::RwLock::new(WebDAVStorageInner::new(local_save_config)),
        }
    }
}

#[async_trait]
impl StorageClient for WebDAVStorage {
    async fn download(&self, file_path: String) -> Result<Option<Vec<u8>>, String> {
        let inner = self.inner.read().await;
        inner.download(file_path).await
    }

    async fn upload(&self, file_path: String, data: Vec<u8>) -> Result<(), String> {
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
