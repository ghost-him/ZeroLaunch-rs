use std::fmt::Debug;

use super::config::PartialLocalConfig;
use super::config::StorageDestination;
use super::utils::read_or_create_str;
use crate::core::storage::config::LocalConfig;
use crate::core::storage::local_save::LocalStorage;
use crate::LOCAL_CONFIG_PATH;
use async_trait::async_trait;
use dashmap::DashMap;
use dashmap::Entry;
use std::sync::Arc;
use tokio::sync::RwLock;
/// 存储管理器的配置文件为 appdata下的目录，这个决定了远程配置文件保存的位置
#[async_trait]
pub trait StorageClient: Send + Sync {
    // 要可以上传文件
    async fn upload(&self, file_path: &str, data: &[u8]) -> Result<(), String>;
    // 要可以下载文件
    async fn download(&self, file_path: &str) -> Result<Vec<u8>, String>;
    // 要可以获得当前文件的目标路径
    async fn get_target_dir_path(&self) -> String;
}

pub struct StorageManagerInner {
    /// 当前的存储信息
    pub local_config: LocalConfig,
    /// 缓存的数据(文件名, (剩余更新次数, 要上传的内容))
    pub cached_content: DashMap<String, (u32, Vec<u8>)>,
    /// 上传文件与下载文件的对象
    pub client: Option<Arc<RwLock<dyn StorageClient>>>,
}

impl std::fmt::Debug for StorageManagerInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageManagerInner")
            .field("local_config", &self.local_config)
            .field("cached_content", &self.cached_content)
            .finish()
    }
}

impl StorageManagerInner {
    // 创建一个存储管理器
    pub fn new() -> StorageManagerInner {
        let mut inner = StorageManagerInner {
            local_config: LocalConfig::default(),
            cached_content: DashMap::new(),
            client: None,
        };
        // 从本地读取配置信息
        let default_content = serde_json::to_string(&inner.local_config.to_partial()).unwrap();
        let local_config_data =
            read_or_create_str(&LOCAL_CONFIG_PATH, Some(default_content)).unwrap();
        let partial_local_config: PartialLocalConfig =
            serde_json::from_str(&local_config_data).unwrap();
        inner.update(partial_local_config);
        inner
    }
    // 更新
    pub fn update(&mut self, partial_local_config: PartialLocalConfig) {
        self.local_config.update(partial_local_config);
        self.save_to_local_disk();
        // 根据配置信息选择合理的后端
        match *self.local_config.get_storage_destination() {
            StorageDestination::Local => {
                self.client = Some(Arc::new(RwLock::new(LocalStorage::new(
                    self.local_config.get_local_save_config(),
                ))));
                println!("已成功赋值");
            }
            StorageDestination::WebDAV => {}
            _ => {}
        }
    }
    // 将自己的信息保存到本地
    fn save_to_local_disk(&self) {
        let partial_local_config = self.local_config.to_partial();
        let contents = serde_json::to_string(&partial_local_config).unwrap();
        let path = LOCAL_CONFIG_PATH.clone();
        std::fs::write(path, contents).unwrap();
    }

    /// 上传文件
    /// file_name: 工作目录下的相对地址
    /// contents: 内容
    pub async fn upload_file_str(&self, file_name: String, contents: String) -> bool {
        self.upload_file_bytes(file_name, contents.into_bytes())
            .await
    }

    /// 下载文件
    /// file_name: 工作目录下的相对地址
    pub async fn download_file_str(&mut self, file_name: String) -> String {
        let bytes = self.download_file_bytes(file_name).await;
        String::from_utf8_lossy(&bytes).into_owned()
    }
    /// 上传文件
    /// file_name: 工作目录下的相对地址
    /// contents: 内容
    pub async fn upload_file_bytes(&self, file_name: String, contents: Vec<u8>) -> bool {
        println!("收到上传文件请求：{:?}", file_name);
        let save_count = *self.local_config.get_save_to_local_per_update();
        // 若配置为0，直接上传
        if save_count == 0 {
            return self
                .upload_file_bytes_force(file_name, Some(contents))
                .await;
        }

        match self.cached_content.entry(file_name.clone()) {
            Entry::Occupied(mut entry) => {
                let (counter, data) = entry.get_mut();
                *counter -= 1;
                *data = contents.clone();

                if *counter == 0 {
                    // 如果减成了0，则上传文件，同时删除当前的文件

                    let client = self.client.as_ref().unwrap().read().await;
                    client.upload(&file_name, &contents).await;
                    println!("成功上传文件：{}", file_name);

                    entry.remove();
                }
            }
            Entry::Vacant(entry) => {
                let save_count = *self.local_config.get_save_to_local_per_update();
                entry.insert((save_count, contents));
            }
        }
        true
    }

    /// 强制上传文件, 忽略之前的文件
    /// 如果contents有内容，则直接发送该内容，否则，直接发送缓存的内容
    pub async fn upload_file_bytes_force(
        &self,
        file_name: String,
        mut contents: Option<Vec<u8>>,
    ) -> bool {
        match self.cached_content.entry(file_name.clone()) {
            Entry::Occupied(entry) => {
                if contents.is_none() {
                    let (_, data) = entry.get();
                    contents = Some(data.clone())
                }
                entry.remove();
            }
            Entry::Vacant(_) => {
                // 如果没有内容，则忽略
            }
        }
        if contents.is_some() {
            let client = self.client.as_ref().unwrap().read().await;
            client.upload(&file_name, &contents.unwrap()).await;
            println!("成功强制上传文件：{}", file_name);

            return true;
        }
        return false;
    }

    /// 将当前缓存中所有的文件都上传，只能在程序结束时调用
    pub async fn upload_all_file_force(&self) {
        let client = self.client.as_ref().unwrap().read().await;

        for item in self.cached_content.iter() {
            client.upload(item.key(), &item.value().1).await;
        }
    }

    /// 下载文件
    /// file_name: 工作目录下的相对地址
    pub async fn download_file_bytes(&mut self, file_name: String) -> Vec<u8> {
        println!("开始下载文件：{:?}", file_name);
        let cached_data = self
            .cached_content
            .get(&file_name)
            .map(|entry| entry.value().1.clone());

        if let Some(content) = cached_data {
            // 这里默认用户只会同时开一个应用，所以本机的配置一定是最新的，云端的配置一定不是最新的
            self.upload_file_bytes_force(file_name.clone(), Some(content.clone()))
                .await;
            return content;
        }

        let client = self.client.as_ref().unwrap().read().await;
        client.download(&file_name).await.unwrap_or(Vec::new())
    }

    /// 获得目标文件夹的地址
    pub async fn get_target_dir_path(&self) -> String {
        let client = self.client.as_ref().unwrap().read().await;
        client.get_target_dir_path().await
    }
}
#[derive(Debug)]
pub struct StorageManager {
    pub inner: RwLock<StorageManagerInner>,
}

impl StorageManager {
    /// 创建一个新的 StorageManager 实例
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(StorageManagerInner::new()),
        }
    }

    /// 更新存储管理器配置
    pub async fn update(&self, partial_local_config: PartialLocalConfig) {
        println!("{:?}", partial_local_config);
        let mut inner = self.inner.write().await;
        inner.update(partial_local_config);
    }

    /// 上传字符串内容到指定文件（带缓存策略）
    pub async fn upload_file_str(&self, file_name: String, contents: String) -> bool {
        let inner = self.inner.read().await;
        inner.upload_file_str(file_name, contents).await
    }

    /// 下载文件内容为字符串（优先使用缓存）
    pub async fn download_file_str(&self, file_name: String) -> String {
        let mut inner = self.inner.write().await;
        inner.download_file_str(file_name).await
    }

    /// 上传二进制内容到指定文件（带缓存策略）
    pub async fn upload_file_bytes(&self, file_name: String, contents: Vec<u8>) -> bool {
        let inner = self.inner.read().await;
        inner.upload_file_bytes(file_name, contents).await
    }

    /// 下载文件内容为二进制（优先使用缓存）
    pub async fn download_file_bytes(&self, file_name: String) -> Vec<u8> {
        let mut inner = self.inner.write().await;
        inner.download_file_bytes(file_name).await
    }

    /// 强制上传文件内容（绕过缓存策略）
    pub async fn upload_file_bytes_force(
        &self,
        file_name: String,
        contents: Option<Vec<u8>>,
    ) -> bool {
        let inner = self.inner.read().await;
        inner.upload_file_bytes_force(file_name, contents).await
    }

    /// 强制上传所有缓存中的内容
    pub async fn upload_all_file_force(&self) {
        let inner = self.inner.read().await;
        inner.upload_all_file_force().await;
    }

    /// 获得目标文件夹的路径
    pub async fn get_target_dir_path(&self) -> String {
        let inner = self.inner.read().await;
        inner.get_target_dir_path().await
    }
}

/// 为 StorageManager 实现默认构造
impl Default for StorageManager {
    fn default() -> Self {
        Self::new()
    }
}
