// use super::config::PartialLocalConfig;
// use crate::storage::storage_manager::StorageClient;
// use crate::storage::storage_manager::TEST_CONFIG_FILE_DATA;
// use crate::storage::storage_manager::TEST_CONFIG_FILE_NAME;
// use crate::utils::service_locator::ServiceLocator;
// use async_trait::async_trait;
// use onedrive_api::{Auth, ItemLocation, OneDrive, Permission, Tenant};
// use onedrive_api::{ClientCredential, DriveLocation};
// use parking_lot::RwLock;
// use serde::{Deserialize, Serialize};
// use std::path::PathBuf;
// use std::path::MAIN_SEPARATOR;
// use std::sync::Arc;
// use tauri::Emitter;
// use tauri::Runtime;
// const ZEROLAUNCH_CLIENT_ID: &str = "d5679831-e737-4ce8-8eca-23da2dd31e64";
// const REDIRECT_URL: &str = "zerolaunch-rs://onedrive_auth_callback";

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct PartialOneDriveConfig {
//     pub refresh_token: Option<String>,
//     pub destination_dir: Option<String>,
// }

// #[derive(Serialize, Deserialize, Debug, Clone)]
// #[serde(default)]
// pub struct OneDriveConfigInner {
//     #[serde(default = "OneDriveConfigInner::default_refresh_token")]
//     pub refresh_token: String,
//     #[serde(default = "OneDriveConfigInner::default_destination_dir")]
//     pub destination_dir: String,
// }

// impl Default for OneDriveConfigInner {
//     fn default() -> Self {
//         Self {
//             refresh_token: Self::default_refresh_token(),
//             destination_dir: Self::default_destination_dir(),
//         }
//     }
// }

// impl OneDriveConfigInner {
//     pub(crate) fn default_refresh_token() -> String {
//         "".to_string()
//     }

//     pub(crate) fn default_destination_dir() -> String {
//         "zerolaunch-rs".to_string()
//     }

//     pub fn update(&mut self, partial_onedrive_config: PartialOneDriveConfig) {
//         if let Some(refresh_token) = partial_onedrive_config.refresh_token {
//             self.refresh_token = refresh_token;
//         }
//         if let Some(destination_dir) = partial_onedrive_config.destination_dir {
//             self.destination_dir = destination_dir;
//         }
//     }

//     pub fn to_partial(&self) -> PartialOneDriveConfig {
//         PartialOneDriveConfig {
//             refresh_token: Some(self.refresh_token.clone()),
//             destination_dir: Some(self.destination_dir.clone()),
//         }
//     }

//     pub fn get_refresh_token(&self) -> String {
//         self.refresh_token.clone()
//     }

//     pub fn get_destination_dir(&self) -> String {
//         self.destination_dir.clone()
//     }
// }

// #[derive(Debug)]
// pub struct OneDriveConfig {
//     inner: RwLock<OneDriveConfigInner>,
// }

// impl Default for OneDriveConfig {
//     fn default() -> Self {
//         Self {
//             inner: RwLock::new(OneDriveConfigInner::default()),
//         }
//     }
// }

// impl OneDriveConfig {
//     pub fn update(&self, partial_config: PartialOneDriveConfig) {
//         let mut inner = self.inner.write();
//         inner.update(partial_config);
//     }

//     pub fn get_refresh_token(&self) -> String {
//         let inner = self.inner.read();
//         inner.get_refresh_token()
//     }

//     pub fn get_destination_dir(&self) -> String {
//         let inner = self.inner.read();
//         inner.get_destination_dir()
//     }

//     pub fn to_partial(&self) -> PartialOneDriveConfig {
//         let inner = self.inner.read();
//         inner.to_partial()
//     }
// }

// pub struct OneDriveStorageInner {
//     pub destination_dir: PathBuf,
//     pub refresh_token: String,
//     pub access_token: String,
//     pub auth: Auth,
//     pub client: Option<OneDrive>,
// }

// impl OneDriveStorageInner {
//     pub async fn new(local_save_config: Arc<OneDriveConfig>) -> Self {
//         println!("inner创建");
//         let permission = Permission::new_read()
//             .offline_access(true)
//             .access_shared(false)
//             .write(true);
//         let auth = Auth::new(
//             ZEROLAUNCH_CLIENT_ID,
//             permission,
//             REDIRECT_URL,
//             Tenant::Consumers,
//         );

//         let client_credential = ClientCredential::None;
//         println!("123");
//         let ret = auth
//             .login_with_refresh_token(&local_save_config.get_refresh_token(), &client_credential)
//             .await
//             .unwrap();
//         println!("123");
//         let refresh_token = ret.refresh_token.unwrap();
//         // 更新存储管理器的refresh_token
//         local_save_config.update(PartialOneDriveConfig {
//             refresh_token: Some(refresh_token),
//             destination_dir: None,
//         });
//         println!("123");
//         let access_token = ret.access_token;
//         println!("创建完成");

//         // 获取目标路径
//         let mut destination_dir: PathBuf = local_save_config.get_destination_dir().into();

//         // 将 PathBuf 转换为字符串以检查是否以 '/' 开头
//         if let Some(path_str) = destination_dir.to_str() {
//             if !path_str.starts_with('\\') {
//                 // 如果不是以 '/' 开头，创建一个新的路径
//                 let new_path = format!("\\{}", path_str);
//                 destination_dir = PathBuf::from(new_path);
//             }
//         }

//         OneDriveStorageInner {
//             destination_dir: destination_dir,
//             refresh_token: local_save_config.get_refresh_token(),
//             access_token: access_token.clone(),
//             auth: auth,
//             client: Some(OneDrive::new(access_token, DriveLocation::me())),
//         }
//     }

//     async fn upload_small(&self, file_name: String, data: Vec<u8>) -> Result<(), String> {
//         let target_path = self.destination_dir.join(file_name);
//         let target_path = target_path.to_string_lossy().replace(MAIN_SEPARATOR, "/");
//         println!("目标路径：{}", target_path.clone());
//         if let Some(client) = self.client.as_ref() {
//             let item_loc = ItemLocation::from_path(&target_path);
//             if item_loc.is_none() {
//                 return Err("解析路径失败".to_string());
//             }
//             let item_loc = item_loc.unwrap();

//             println!("路径创建完成");
//             if let Err(e) = client.upload_small(item_loc, data).await {
//                 println!("{}", e.to_string());
//                 return Err(e.to_string());
//             }
//             println!("上传完成:{:?}", item_loc);
//         } else {
//             return Err("无法完成上传文件".to_string());
//         }
//         return Ok(());
//     }
// }

// #[async_trait]
// impl StorageClient for OneDriveStorageInner {
//     // 要可以上传文件
//     async fn upload(&self, file_name: String, data: Vec<u8>) -> Result<(), String> {
//         self.upload_small(file_name, data).await
//     }
//     // 要可以下载文件
//     async fn download(&self, file_name: String) -> Result<Vec<u8>, String> {
//         let target_path = self.destination_dir.join(file_name);
//         let target_path = target_path.to_string_lossy().replace(MAIN_SEPARATOR, "/");
//         println!("目标下载路径");
//         if let Some(client) = self.client.as_ref() {
//             let target_item = ItemLocation::from_path(&target_path).unwrap();
//             let download_url = client.get_item_download_url(target_item).await.unwrap();
//             println!("{} 的下载地址: {}", target_path, download_url);
//             let content = reqwest::get(download_url)
//                 .await
//                 .expect("Failed to request for downloading file")
//                 .bytes()
//                 .await
//                 .expect("Failed to download file")
//                 .to_vec();

//             // 保存到本地
//             return Ok(content.to_vec());
//         }
//         Err("当前未设置OneDrive客户端".to_string())
//     }
//     // 要可以获得当前文件的目标路径
//     async fn get_target_dir_path(&self) -> String {
//         self.destination_dir.to_str().unwrap().to_string()
//     }
//     // 判断是否有效(true: 有效，false: 无效)
//     async fn validate_config(&self) -> bool {
//         println!("开始内部验证");
//         if let Err(e) = self
//             .upload(
//                 TEST_CONFIG_FILE_NAME.to_string(),
//                 TEST_CONFIG_FILE_DATA.to_string().as_bytes().to_vec(),
//             )
//             .await
//         {
//             println!("{}", e);
//             return false;
//         }

//         if let Err(e) = self.download(TEST_CONFIG_FILE_NAME.to_string()).await {
//             println!("{}", e);
//             return false;
//         }

//         true
//     }
// }

// pub struct OneDriveStorage {
//     pub inner: tokio::sync::RwLock<OneDriveStorageInner>,
// }

// impl OneDriveStorage {
//     /// 创建一个新的 OneDriveStorage 实例（异步初始化）
//     pub async fn new(local_save_config: Arc<OneDriveConfig>) -> Self {
//         OneDriveStorage {
//             inner: tokio::sync::RwLock::new(OneDriveStorageInner::new(local_save_config).await),
//         }
//     }
// }

// #[async_trait]
// impl StorageClient for OneDriveStorage {
//     async fn download(&self, file_name: String) -> Result<Vec<u8>, String> {
//         let inner = self.inner.read().await;
//         inner.download(file_name).await
//     }

//     async fn upload(&self, file_name: String, data: Vec<u8>) -> Result<(), String> {
//         let inner = self.inner.read().await;
//         inner.upload(file_name, data).await
//     }

//     async fn get_target_dir_path(&self) -> String {
//         let inner = self.inner.read().await;
//         inner.get_target_dir_path().await
//     }

//     async fn validate_config(&self) -> bool {
//         let inner = self.inner.read().await;
//         inner.validate_config().await
//     }
// }

// pub async fn get_onedrive_refresh_token<R: Runtime>(
//     window: tauri::Window<R>,
// ) -> Result<String, String> {
//     let permission = Permission::new_read()
//         .offline_access(true)
//         .access_shared(false)
//         .write(true);
//     let auth = Auth::new(
//         ZEROLAUNCH_CLIENT_ID,
//         permission,
//         REDIRECT_URL,
//         Tenant::Consumers,
//     );
//     // 获取授权URL
//     let auth_url = auth.code_auth_url().as_str().to_string();
//     println!("请访问此URL并授权: {}", auth_url);
//     if let Err(e) = window.emit("emit_update_auth_link", auth_url) {
//         return Err(e.to_string());
//     }
//     let state = ServiceLocator::get_state();
//     let waiting_hashmap = state.get_waiting_hashmap();
//     let pairs = waiting_hashmap
//         .get_or_wait("onedrive_auth_callback".to_string())
//         .await;
//     let mut code = String::new();
//     for item in pairs {
//         if item.0 == "code" {
//             code = item.1;
//             break;
//         }
//     }
//     if code.is_empty() {
//         return Err("无法找到校验码".to_string());
//     }
//     let client_credential = ClientCredential::None;

//     // 使用授权码获取访问令牌
//     let token_response: onedrive_api::TokenResponse = auth
//         .login_with_code(&code, &client_credential)
//         .await
//         .unwrap();
//     Ok(token_response.refresh_token.unwrap())
// }
