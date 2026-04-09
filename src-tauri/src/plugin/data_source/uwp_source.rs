use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::{DataSource, LaunchMethod, SearchCandidate};
use crate::plugin_system::{ComponentType, ConfigError, Configurable};
use crate::utils::defer::defer;
use crate::utils::windows::get_u16_vec;
use image::ImageReader;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PROPVARIANT};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PSGetPropertyKeyFromName};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, BHID_PropertyStore, IEnumShellItems, IShellItem, SHCreateItemFromParsingName,
    SIGDN_NORMALDISPLAY,
};
use windows_core::PCWSTR;

pub struct UwpSource {
    settings: serde_json::Value,
}

impl Default for UwpSource {
    fn default() -> Self {
        Self::new()
    }
}

impl UwpSource {
    pub fn new() -> Self {
        UwpSource {
            settings: serde_json::Value::Null,
        }
    }

    /// 将 PROPVARIANT 转换为字符串
    fn prop_variant_to_string(pv: &PROPVARIANT) -> String {
        pv.to_string()
    }

    /// 验证图标路径并返回分辨率最大的图标
    fn validate_icon_path(icon_path: String) -> String {
        let scales = [
            ".scale-400.",
            ".scale-300.",
            ".targetsize-256.",
            ".scale-200.",
            ".targetsize-48.",
            ".scale-100.",
            ".targetsize-24.",
            ".targetsize-16.",
        ];

        let path = Path::new(&icon_path);

        let extension = match path.extension().and_then(OsStr::to_str) {
            Some(ext) => ext,
            None => return String::new(),
        };

        let stem = match path.file_stem().and_then(OsStr::to_str) {
            Some(s) => s,
            None => return String::new(),
        };

        let parent = match path.parent() {
            Some(p) => p,
            None => return String::new(),
        };

        // 按照预设的分辨率顺序检查缩放后的图标文件
        for scale in &scales {
            let new_stem = format!("{}{}.", stem, scale);
            let mut new_path = PathBuf::from(parent);
            new_path.push(format!("{}.{}", new_stem, extension));

            if new_path.exists() {
                return new_path.to_string_lossy().into_owned();
            }
        }

        // 如果没有匹配的缩放图标，寻找所有匹配的图标文件并比较实际分辨率
        let icon_prefix = stem;

        let entries = match fs::read_dir(parent) {
            Ok(entries) => entries,
            Err(e) => {
                warn!("Failed to read directory for icon validation: {}", e);
                return String::new();
            }
        };

        let mut matching_icons: Vec<(PathBuf, u64)> = Vec::new();

        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_file() {
                if let Some(ext) = entry_path.extension().and_then(OsStr::to_str) {
                    if ext.eq_ignore_ascii_case("png") {
                        if let Some(file_stem) = entry_path.file_stem().and_then(OsStr::to_str) {
                            if file_stem.starts_with(icon_prefix) {
                                if let Some(resolution) = Self::get_image_resolution(&entry_path) {
                                    matching_icons.push((entry_path, resolution));
                                }
                            }
                        }
                    }
                }
            }
        }

        // 按分辨率从高到低排序
        matching_icons.sort_by(|a, b| b.1.cmp(&a.1));

        if let Some((highest_res_path, _)) = matching_icons.first() {
            return highest_res_path.to_string_lossy().into_owned();
        }

        String::new()
    }

    /// 获取图像的分辨率（宽 x 高）
    fn get_image_resolution(path: &Path) -> Option<u64> {
        match ImageReader::open(path) {
            Ok(reader) => match reader.with_guessed_format() {
                Ok(format_reader) => match format_reader.into_dimensions() {
                    Ok((width, height)) => Some(width as u64 * height as u64),
                    Err(e) => {
                        warn!("Failed to get image dimensions: {}", e);
                        None
                    }
                },
                Err(e) => {
                    warn!("Failed to guess image format: {}", e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to open image file: {}", e);
                None
            }
        }
    }
}

impl Configurable for UwpSource {
    fn component_id(&self) -> &str {
        "uwp-source"
    }

    fn component_name(&self) -> &str {
        "UWP 应用数据源"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::DataSource
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.clone()
    }

    fn apply_settings(&mut self, settings: serde_json::Value) -> Result<(), ConfigError> {
        self.settings = settings;
        Ok(())
    }
}

impl DataSource for UwpSource {
    fn fetch_candidates(&self) -> CachedCandidateData {
        let mut result = CachedCandidateData::new();

        unsafe {
            let com_init = windows::Win32::System::Com::CoInitialize(None);
            if com_init.is_err() {
                warn!("初始化com库失败：{:?}", com_init);
            }

            let _defer = defer(move || {
                if com_init.is_ok() {
                    windows::Win32::System::Com::CoUninitialize();
                }
            });

            // Create Shell item for AppsFolder
            let tmp = get_u16_vec("shell:AppsFolder");
            let app_folder: IShellItem =
                match SHCreateItemFromParsingName(PCWSTR::from_raw(tmp.as_ptr()), None) {
                    Ok(item) => item,
                    Err(e) => {
                        warn!("UwpSource: fail to open shell:AppsFolder {}", e);
                        return result;
                    }
                };

            // Bind to IEnumShellItems
            let enum_shell_items: IEnumShellItems =
                match app_folder.BindToHandler(None, &BHID_EnumItems) {
                    Ok(enumerator) => enumerator,
                    Err(e) => {
                        warn!("UwpSource: fail to bind to handler {}", e);
                        return result;
                    }
                };

            // Define PROPERTYKEYs
            let tmp = get_u16_vec("System.Launcher.AppState");
            let mut pk_launcher_app_state = PROPERTYKEY::default();
            if let Err(e) =
                PSGetPropertyKeyFromName(PCWSTR::from_raw(tmp.as_ptr()), &mut pk_launcher_app_state)
            {
                warn!(
                    "Failed to get PROPERTYKEY for System.Launcher.AppState {}",
                    e
                );
                return result;
            };

            let tmp = get_u16_vec("System.Tile.SmallLogoPath");
            let mut pk_small_logo_path = PROPERTYKEY::default();
            if let Err(e) =
                PSGetPropertyKeyFromName(PCWSTR::from_raw(tmp.as_ptr()), &mut pk_small_logo_path)
            {
                warn!(
                    "Failed to get PROPERTYKEY for System.Tile.SmallLogoPath {}",
                    e
                );
                return result;
            };

            let tmp = get_u16_vec("System.AppUserModel.ID");
            let mut pk_app_user_model_id = PROPERTYKEY::default();
            if let Err(e) =
                PSGetPropertyKeyFromName(PCWSTR::from_raw(tmp.as_ptr()), &mut pk_app_user_model_id)
            {
                warn!("Failed to get PROPERTYKEY for System.AppUserModel.ID {}", e);
                return result;
            };

            let tmp = get_u16_vec("System.AppUserModel.PackageInstallPath");
            let mut pk_install_path = PROPERTYKEY::default();
            if let Err(e) =
                PSGetPropertyKeyFromName(PCWSTR::from_raw(tmp.as_ptr()), &mut pk_install_path)
            {
                warn!(
                    "Failed to get PROPERTYKEY for System.AppUserModel.PackageInstallPath {}",
                    e
                );
                return result;
            };

            // Enumerate Shell Items
            let mut items: Vec<Option<IShellItem>> = Vec::new();
            items.resize(300, None);

            let mut fetched: u32 = 0;
            if let Err(e) = enum_shell_items.Next(&mut items, Some(&mut fetched as *mut u32)) {
                warn!("UwpSource: error enumerating shell items: {}", e);
                return result;
            }

            for shell_item in &items {
                if shell_item.is_none() {
                    continue;
                }
                let shell_item = match shell_item.clone() {
                    Some(item) => item,
                    None => continue,
                };

                // Bind to IPropertyStore
                let property_store: IPropertyStore =
                    match shell_item.BindToHandler(None, &BHID_PropertyStore) {
                        Ok(store) => store,
                        Err(e) => {
                            warn!("UwpSource: error binding to property store: {}", e);
                            continue;
                        }
                    };

                // Get Launcher.AppState
                let mut pv_launcher = PROPVARIANT::default();
                if let Ok(value) = property_store.GetValue(&pk_launcher_app_state) {
                    pv_launcher = value.clone();
                }
                if let Err(e) = PropVariantClear(&mut pv_launcher) {
                    warn!("清理PropVariant失败: {}", e);
                }

                // Get Display Name
                let short_name = match shell_item.GetDisplayName(SIGDN_NORMALDISPLAY) {
                    Ok(name) => match name.to_string() {
                        Ok(s) => s,
                        Err(e) => {
                            warn!("转换显示名称失败: {}", e);
                            String::new()
                        }
                    },
                    Err(e) => {
                        warn!("获取显示名称失败: {}", e);
                        String::new()
                    }
                };

                // Get AppUserModel.ID
                let mut pv_app_id = PROPVARIANT::default();
                if let Ok(value) = property_store.GetValue(&pk_app_user_model_id) {
                    pv_app_id = value.clone();
                }
                let app_id = Self::prop_variant_to_string(&pv_app_id);
                if let Err(e) = PropVariantClear(&mut pv_app_id) {
                    warn!("清理PropVariant失败: {}", e);
                }

                // Get PackageInstallPath
                let mut pv_install = PROPVARIANT::default();
                if let Ok(value) = property_store.GetValue(&pk_install_path) {
                    pv_install = value.clone();
                }
                let install_path = Self::prop_variant_to_string(&pv_install);
                if install_path.is_empty() {
                    continue;
                }
                if let Err(e) = PropVariantClear(&mut pv_install) {
                    warn!("清理PropVariant失败: {}", e);
                }

                // Get SmallLogoPath
                let mut pv_icon = PROPVARIANT::default();
                if let Ok(value) = property_store.GetValue(&pk_small_logo_path) {
                    pv_icon = value.clone();
                }
                let logo_path = Self::prop_variant_to_string(&pv_icon);
                if let Err(e) = PropVariantClear(&mut pv_icon) {
                    warn!("清理PropVariant失败: {}", e);
                }

                let mut full_icon_path = PathBuf::from(&install_path);
                full_icon_path.push(&logo_path);
                let icon_path =
                    Self::validate_icon_path(full_icon_path.to_string_lossy().into_owned());

                let candidate = SearchCandidate {
                    id: 0,
                    name: short_name,
                    icon: icon_path,
                    launch_method: LaunchMethod::PackageFamilyName(app_id),
                    keywords: Vec::new(),
                    bias: 0.0,
                };

                result.add_candidate(candidate);
            }
        }

        result
    }
}
