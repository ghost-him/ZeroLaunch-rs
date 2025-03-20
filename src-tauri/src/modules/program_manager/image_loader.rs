use super::unit::Program;
use crate::core::image_processor::ImageProcessor;
use dashmap::DashMap;
use dashmap::Entry::{Occupied, Vacant};
use std::sync::Arc;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;
#[derive(Debug)]
pub struct ImageLoader {
    default_app_icon_path: String,
    icon_path_cache: DashMap<String, String>,
}

impl ImageLoader {
    /// 新建一个
    pub fn new(default_icon_path: String) -> ImageLoader {
        let mut result = ImageLoader {
            default_app_icon_path: default_icon_path,
            icon_path_cache: DashMap::new(),
        };
        result.icon_path_cache = result.get_installed_programs();
        result
    }
    /// 加载一个图片
    pub async fn load_image(&self, program: Arc<Program>) -> Vec<u8> {
        let mut icon_path = program.icon_path.clone();
        if icon_path.ends_with("url") {
            let show_name = program.show_name.clone();
            icon_path = match self.icon_path_cache.entry(show_name) {
                Occupied(entry) => {
                    let value = entry.get();
                    value.to_string()
                }
                Vacant(_) => program.icon_path.clone(),
            };
        }
        let mut pic_bytes: Vec<u8> = self.load_image_from_path(&icon_path).await;
        if pic_bytes.is_empty() {
            pic_bytes = self.load_image_from_path(&self.default_app_icon_path).await;
        }
        pic_bytes
    }
    /// 使用路径加载一个图片
    async fn load_image_from_path(&self, icon_path: &str) -> Vec<u8> {
        ImageProcessor::load_image_from_path(icon_path.to_string()).await
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
                for result in uninstall_key.enum_keys() {
                    if let Ok(subkey_name) = result {
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
