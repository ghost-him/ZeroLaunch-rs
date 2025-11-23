use crate::core::image_processor::{ImageIdentity, ImageProcessor};
use crate::core::storage::utils::read_dir_or_create;
use crate::error::OptionExt;
use crate::modules::config::default::ICON_CACHE_DIR;
use crate::modules::icon_manager::config::{IconManagerConfig, RuntimeIconManagerConfig};
use dashmap::{DashMap, DashSet};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;
use tauri::async_runtime::RwLock;
use tracing::warn;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;
pub mod config;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum IconRequest {
    /// 本地文件路径 (exe, lnk, ico, png) -> 提取文件图标
    /// 对于 .url 文件，可以提供 app_name 用于注册表查找
    Path {
        path: String,
        app_name: Option<String>,
    },
    /// 网址 -> 下载或查找本地域名图标库
    Url(String),
    /// 文件扩展名 (.txt, .doc) -> 获取系统关联图标
    Extension(String),
    /// 应用程序名称 -> 用于从注册表查找图标
    AppId(String),
}

impl IconRequest {
    pub fn get_hash_string(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish().to_string()
    }
}

#[derive(Debug)]
struct IconManagerInner {
    /// 默认的应用图标的路径
    default_app_icon_path: String,
    /// 默认的网址图片路径
    default_web_icon_path: String,
    /// 注册表图标缓存 (AppId -> IconPath)
    registry_icon_cache: DashMap<String, String>,
    /// 已缓存的图标哈希集合 (文件名)
    cached_icon_hashes: DashSet<String>,
    /// 要不要开启图片缓存
    enable_icon_cache: bool,
    /// 要不要联网来获取网址的图标
    enable_online: bool,
}

impl IconManagerInner {
    pub fn new(runtime_config: RuntimeIconManagerConfig) -> Self {
        let mut inner = Self {
            default_app_icon_path: runtime_config.default_app_icon_path,
            default_web_icon_path: runtime_config.default_web_icon_path,
            enable_icon_cache: true,
            enable_online: true,
            registry_icon_cache: DashMap::new(),
            cached_icon_hashes: DashSet::new(),
        };
        inner.init();
        inner
    }

    fn init(&mut self) {
        self.registry_icon_cache = self.scan_registry_programs();
        self.cached_icon_hashes = self.scan_cached_icons();
    }

    pub fn load_from_config(&mut self, config: Arc<IconManagerConfig>) {
        self.enable_icon_cache = config.get_enable_icon_cache();
        self.enable_online = config.get_enable_online();

        if self.enable_icon_cache && self.cached_icon_hashes.is_empty() {
            self.cached_icon_hashes = self.scan_cached_icons();
        }
    }

    pub async fn get_icon(&self, request: IconRequest) -> Vec<u8> {
        let hash_name = request.get_hash_string() + ".png";

        // 1. 缓存策略
        if self.enable_icon_cache && self.cached_icon_hashes.contains(&hash_name) {
            let cached_icon_dir = ICON_CACHE_DIR.clone();
            let icon_path = Path::new(&cached_icon_dir).join(&hash_name);
            let identity = ImageIdentity::File(
                icon_path
                    .to_str()
                    .expect_programming("图标路径转换为字符串失败")
                    .to_string(),
            );
            return ImageProcessor::load_image(&identity).await;
        }

        // 2. 处理不同类型的请求
        let (mut icon_data, is_default) = match request {
            IconRequest::Path { path, app_name } => self.handle_path_request(path, app_name).await,
            IconRequest::Url(url) => self.handle_url_request(url).await,
            IconRequest::Extension(ext) => self.handle_extension_request(ext).await,
            IconRequest::AppId(app_id) => self.handle_appid_request(app_id).await,
        };

        // 裁剪透明白边
        if !icon_data.is_empty() {
            if let Ok(output) = ImageProcessor::trim_transparent_white_border(icon_data.clone()) {
                icon_data = output;
            }
        }

        // 3. 写入缓存
        if self.enable_icon_cache && !is_default && !icon_data.is_empty() {
            let icon_data_clone = icon_data.clone();
            let hash_name_clone = hash_name.clone();
            tauri::async_runtime::spawn(async move {
                let cached_icon_dir = ICON_CACHE_DIR.clone();
                let icon_path = Path::new(&cached_icon_dir).join(hash_name_clone);
                let _ = tokio::fs::write(
                    icon_path
                        .to_str()
                        .expect_programming("缓存路径转换为字符串失败")
                        .to_string(),
                    icon_data_clone,
                )
                .await;
            });
            self.cached_icon_hashes.insert(hash_name);
        }

        icon_data
    }

    async fn handle_path_request(&self, path: String, app_name: Option<String>) -> (Vec<u8>, bool) {
        // 特殊处理 .url 文件
        if path.ends_with(".url") {
            // 优先使用提供的 app_name 查找注册表图标（与 ImageLoader 逻辑一致）
            if let Some(name) = app_name {
                if let Some(registry_icon) = self.registry_icon_cache.get(&name) {
                    let data =
                        ImageProcessor::load_image(&ImageIdentity::File(registry_icon.clone()))
                            .await;
                    if !data.is_empty() {
                        return (data, false);
                    }
                }
            }
        }

        let data = ImageProcessor::load_image(&ImageIdentity::File(path)).await;
        if data.is_empty() {
            let default_data = ImageProcessor::load_image(&ImageIdentity::File(
                self.default_app_icon_path.clone(),
            ))
            .await;
            (default_data, true)
        } else {
            (data, false)
        }
    }

    async fn handle_url_request(&self, url: String) -> (Vec<u8>, bool) {
        if !self.enable_online {
            let default_data = ImageProcessor::load_image(&ImageIdentity::File(
                self.default_web_icon_path.clone(),
            ))
            .await;
            return (default_data, true);
        }

        // TODO: 这里可以添加 DomainStrategy，查找 Icons/url/{domain}.png

        let data = ImageProcessor::load_image(&ImageIdentity::Web(url)).await;
        if data.is_empty() {
            let default_data = ImageProcessor::load_image(&ImageIdentity::File(
                self.default_web_icon_path.clone(),
            ))
            .await;
            (default_data, true)
        } else {
            (data, false)
        }
    }

    async fn handle_extension_request(&self, _ext: String) -> (Vec<u8>, bool) {
        // TODO: 实现获取扩展名关联图标的逻辑
        // 目前暂时返回默认图标
        let default_data =
            ImageProcessor::load_image(&ImageIdentity::File(self.default_app_icon_path.clone()))
                .await;
        (default_data, true)
    }

    async fn handle_appid_request(&self, app_id: String) -> (Vec<u8>, bool) {
        if let Some(icon_path) = self.registry_icon_cache.get(&app_id) {
            let data = ImageProcessor::load_image(&ImageIdentity::File(icon_path.clone())).await;
            if !data.is_empty() {
                return (data, false);
            }
        }

        let default_data =
            ImageProcessor::load_image(&ImageIdentity::File(self.default_app_icon_path.clone()))
                .await;
        (default_data, true)
    }

    fn scan_cached_icons(&self) -> DashSet<String> {
        let result = DashSet::new();
        if !self.enable_icon_cache {
            return result;
        }

        let icon_cache_dir_clone = ICON_CACHE_DIR.clone();
        match read_dir_or_create(icon_cache_dir_clone) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let file_name = file_name.to_string_lossy();
                    result.insert(file_name.into_owned());
                }
            }
            Err(e) => warn!("Error reading icon cache directory: {}", e),
        }
        result
    }

    fn scan_registry_programs(&self) -> DashMap<String, String> {
        let programs = DashMap::new();
        let paths = [
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ];
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        for path in paths.iter() {
            if let Ok(uninstall_key) = hklm.open_subkey_with_flags(path, KEY_READ) {
                for subkey_name in uninstall_key.enum_keys().flatten() {
                    if let Ok(subkey) = uninstall_key.open_subkey_with_flags(&subkey_name, KEY_READ)
                    {
                        let display_name: Result<String, _> = subkey.get_value("DisplayName");
                        let display_icon: Result<String, _> = subkey.get_value("DisplayIcon");

                        if let (Ok(name), Ok(icon)) = (display_name, display_icon) {
                            if !name.trim().is_empty() && !icon.trim().is_empty() {
                                let name = name.trim().to_string();
                                let icon = self.normalized_icon_path(icon.trim());
                                programs.insert(name, icon);
                            }
                        }
                    }
                }
            }
        }
        programs
    }

    fn normalized_icon_path(&self, icon_path: &str) -> String {
        let mut result = icon_path.to_string();
        if let Some(pos) = result.rfind(',') {
            result = result[..pos].to_string();
        }
        if result.starts_with('"') && result.ends_with('"') {
            result = result[1..result.len() - 1].to_string()
        }
        result
    }
}

#[derive(Debug)]
pub struct IconManager {
    inner: RwLock<IconManagerInner>,
}

impl IconManager {
    pub fn new(config: RuntimeIconManagerConfig) -> Self {
        Self {
            inner: RwLock::new(IconManagerInner::new(config)),
        }
    }

    pub async fn load_from_config(&self, config: Arc<IconManagerConfig>) {
        let mut inner = self.inner.write().await;
        inner.load_from_config(config);
    }

    pub async fn get_icon(&self, request: IconRequest) -> Vec<u8> {
        let inner = self.inner.read().await;
        inner.get_icon(request).await
    }
}
