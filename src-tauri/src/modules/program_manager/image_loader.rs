use super::config::image_loader_config::RuntimeImageLoaderConfig;
use super::unit::Program;
use crate::core::image_processor::{ImageIdentity, ImageProcessor};
use crate::core::storage::utils::read_dir_or_create;
use crate::modules::config::default::ICON_CACHE_DIR;
use crate::modules::program_manager::config::image_loader_config::ImageLoaderConfig;
use dashmap::Entry::{Occupied, Vacant};
use dashmap::{DashMap, DashSet};
use std::path::Path;
use std::sync::Arc;
use tauri::async_runtime::RwLock;
use tracing::warn;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;

#[derive(Debug)]
struct ImageLoaderInner {
    /// 默认的应用图标的路径
    default_app_icon_path: String,
    /// 默认的网址图片路径
    default_web_icon_path: String,
    /// 是不是要开启缓存
    enable_icon_cache: bool,
    /// 是不是要联网
    enable_online: bool,
    /// 对 .url文件 的特殊缓存(应用的名字，应用的图标路径)
    icon_path_cache: DashMap<String, String>,
    /// 目前已缓存的图片文件(当前在缓存文件夹中的文件)
    cached_icon_name: DashSet<String>,
}

impl ImageLoaderInner {
    /// 新建一个
    pub fn new(image_loader_config: RuntimeImageLoaderConfig) -> ImageLoaderInner {
        let mut result = ImageLoaderInner {
            default_app_icon_path: image_loader_config.default_app_icon_path,
            default_web_icon_path: image_loader_config.default_web_icon_path,
            enable_online: true,
            enable_icon_cache: true,
            icon_path_cache: DashMap::new(),
            cached_icon_name: DashSet::new(),
        };
        result.icon_path_cache = result.get_installed_programs();
        result.cached_icon_name = result.get_cached_icon_name();
        result
    }

    /// 从配置中加载程序
    pub fn load_from_config(&mut self, image_loader_config: &ImageLoaderConfig) {
        self.enable_icon_cache = image_loader_config.get_enable_icon_cache();
        self.enable_online = image_loader_config.get_enable_online();
        self.icon_path_cache = self.get_installed_programs();
        self.cached_icon_name = self.get_cached_icon_name();
    }

    /// 加载一个图片
    pub async fn load_image(&self, program: Arc<Program>) -> Vec<u8> {
        let mut icon_identity = program.icon_path.clone();
        let mut hash_name = icon_identity.get_hash();
        hash_name.push_str(".png");
        // 如果这个图片已经在缓存中了，则直接返回缓存中的图片
        if self.enable_icon_cache && self.cached_icon_name.contains(&hash_name) {
            let cached_icon_dir = ICON_CACHE_DIR.clone();
            let icon_path = Path::new(&cached_icon_dir).join(&hash_name);
            // 如果存在，则直接将目标的路径（网页url）更换成本地的地址
            icon_identity = ImageIdentity::File(icon_path.to_str().unwrap().to_string());
            return ImageProcessor::load_image(&icon_identity).await;
        }

        let (icon_data, is_default_pic) = match icon_identity {
            ImageIdentity::File(mut icon_path) => {
                // 如果是以.url结尾的,比如 steam.url ，则优先看看能不能找到其对应的图标，如果有，则使用这个图标来获得程序图标，如果没有，则使用默认的文件地址获得
                if icon_path.ends_with("url") {
                    let show_name = program.show_name.clone();
                    icon_path = match self.icon_path_cache.entry(show_name) {
                        Occupied(entry) => {
                            let value = entry.get();
                            value.to_string()
                        }
                        Vacant(_) => program.icon_path.get_text().clone(),
                    };
                }
                // 现在 icon_path 就是
                let mut pic_bytes: Vec<u8> =
                    ImageProcessor::load_image(&ImageIdentity::File(icon_path)).await;
                let is_empty = pic_bytes.is_empty();
                // 如果是空的，则说明返回的是默认的图标
                if is_empty {
                    pic_bytes = ImageProcessor::load_image(&ImageIdentity::File(
                        self.default_app_icon_path.clone(),
                    ))
                    .await;
                }
                (pic_bytes, is_empty)
            }
            ImageIdentity::Web(_) => {
                let mut web_image = if self.enable_online {
                    // 只有启用在线时，才能使用网络
                    ImageProcessor::load_image(&program.icon_path).await
                } else {
                    Vec::new()
                };
                let is_empty = web_image.is_empty();
                // 如果是空的，则说明返回的是默认的图标
                if is_empty {
                    web_image = ImageProcessor::load_image(&ImageIdentity::File(
                        self.default_web_icon_path.clone(),
                    ))
                    .await;
                }
                (web_image, is_empty)
            }
        };
        // 如果要缓存到本地，则需要判断是不是默认的图标，如果不是默认的图标，则将其保存到缓存中
        if self.enable_icon_cache && !is_default_pic {
            let icon_cache_clone = icon_data.clone();
            tauri::async_runtime::spawn(async move {
                let cached_icon_dir = ICON_CACHE_DIR.clone();
                let icon_path = Path::new(&cached_icon_dir).join(&hash_name);
                let _ = tokio::fs::write(icon_path.to_str().unwrap().to_string(), icon_cache_clone)
                    .await;
            });
        }
        icon_data
    }

    /// 获取缓存文件夹中的所有的文件
    fn get_cached_icon_name(&self) -> DashSet<String> {
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
            Err(e) => warn!("Error reading directory: {}", e),
        }
        result
    }

    /// 获取当前安装程序的图标
    fn get_installed_programs(&self) -> DashMap<String, String> {
        let programs = DashMap::new();

        // 定义要遍历的两个注册表路径
        let paths = [
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ];

        // 获取HKEY_LOCAL_MACHINE根键
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        for path in paths.iter() {
            // 尝试打开注册表路径
            if let Ok(uninstall_key) = hklm.open_subkey_with_flags(path, KEY_READ) {
                // 遍历所有子键
                for subkey_name in uninstall_key.enum_keys().flatten() {
                    // 打开子键
                    if let Ok(subkey) =
                        uninstall_key.open_subkey_with_flags(&subkey_name, KEY_READ)
                    {
                        // 尝试读取DisplayName和DisplayIcon值
                        let display_name: Result<String, _> = subkey.get_value("DisplayName");
                        let display_icon: Result<String, _> = subkey.get_value("DisplayIcon");

                        // 如果两个值都存在，则添加到HashMap中
                        if let (Ok(name), Ok(icon)) = (display_name, display_icon) {
                            // 过滤掉空值
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
    /// 规范化图标的路径
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

/// 线程安全的图片加载器外壳
#[derive(Debug)]
pub struct ImageLoader {
    inner: RwLock<ImageLoaderInner>,
}

impl ImageLoader {
    /// 创建一个新的 ImageLoader 实例
    pub fn new(image_loader_config: RuntimeImageLoaderConfig) -> Self {
        Self {
            inner: RwLock::new(ImageLoaderInner::new(image_loader_config)),
        }
    }

    /// 从配置中重新加载设置
    pub async fn load_from_config(&self, image_loader_config: &ImageLoaderConfig) {
        let mut inner = self.inner.write().await;
        inner.load_from_config(image_loader_config);
    }

    /// 异步加载图片
    pub async fn load_image(&self, program: Arc<Program>) -> Vec<u8> {
        let inner = self.inner.read().await;
        inner.load_image(program).await
    }
}
