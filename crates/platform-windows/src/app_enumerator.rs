use crate::com_guard::ComGuard;
use crate::utils::get_u16_vec;
use async_trait::async_trait;
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
use zerolaunch_plugin_api::services::app::{AppEnumerator, AppInfo};
use zerolaunch_plugin_api::services::IconRequest;

/// Windows 应用枚举器实现。
/// 通过 shell:AppsFolder 枚举 UWP 应用，通过 IPropertyStore 读取应用属性。
pub struct WindowsAppEnumerator;

impl Default for WindowsAppEnumerator {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsAppEnumerator {
    pub fn new() -> Self {
        Self
    }

    /// 将 PROPVARIANT 转换为字符串
    fn prop_variant_to_string(pv: &PROPVARIANT) -> String {
        pv.to_string()
    }

    /// 验证图标路径并返回分辨率最大的图标。
    /// 参数：icon_path - 原始图标路径。
    /// 返回：验证后的图标路径，验证失败返回空字符串。
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
        matching_icons.sort_by_key(|b| std::cmp::Reverse(b.1));

        if let Some((highest_res_path, _)) = matching_icons.first() {
            return highest_res_path.to_string_lossy().into_owned();
        }

        String::new()
    }

    /// 获取图像的分辨率（宽 x 高）。
    /// 参数：path - 图像文件路径。
    /// 返回：像素总数（宽*高），失败返回 None。
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

    /// 从 AppUserModelID 派生英文检索关键词。
    ///
    /// AUMID 形如 `Microsoft.WindowsTerminal_8wekyb3d8bbwe!App`：`!` 前是包族名，
    /// 其 `_` 前是包名。包名按 `.` 与驼峰切成小写词，去掉开头的发布者（Microsoft），
    /// 用剩余词生成连写与空格分隔两种别名（`windowsterminal`、`windows terminal`）。
    /// 参数：app_id - 应用标识；display_name - 展示名，用于跳过重复别名。
    /// 返回：小写关键词；路径型、占位型或无法切词的标识返回空。
    fn app_id_keywords(app_id: &str, display_name: &str) -> Vec<String> {
        let app_id = app_id.trim();
        // 路径与 `{...}` 占位 AUMID 不含包名，派生不出英文词
        if app_id.is_empty()
            || app_id.contains('{')
            || app_id.contains('\\')
            || app_id.contains('/')
        {
            return Vec::new();
        }

        let package_name = app_id
            .split('!')
            .next()
            .unwrap_or("")
            .split('_')
            .next()
            .unwrap_or("");
        let mut tokens = Self::split_package_tokens(package_name);
        while tokens.len() > 1 && matches!(tokens[0].as_str(), "microsoft" | "ms") {
            tokens.remove(0);
        }
        if tokens.is_empty() {
            return Vec::new();
        }

        let display_name_lower = display_name.to_lowercase();
        let mut keywords: Vec<String> = Vec::with_capacity(2);
        for keyword in [tokens.concat(), tokens.join(" ")] {
            if keyword.is_empty() || keyword == display_name_lower || keywords.contains(&keyword) {
                continue;
            }
            keywords.push(keyword);
        }
        keywords
    }

    /// 把包名按非字母数字边界与驼峰边界切成小写词。
    /// 参数：package_name - 形如 `Microsoft.WindowsTerminal` 的包名。
    /// 返回：小写词序列；连续大写按一个词处理（`WSL` → `wsl`）。
    fn split_package_tokens(package_name: &str) -> Vec<String> {
        let chars: Vec<char> = package_name.chars().collect();
        let mut tokens: Vec<String> = Vec::new();
        let mut current = String::new();

        for (index, &character) in chars.iter().enumerate() {
            if !character.is_alphanumeric() {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current).to_lowercase());
                }
                continue;
            }

            // 当前字符开启新词：前一字符是小写/数字，或前一段全大写且下一个字符是小写
            let starts_new_token =
                character.is_uppercase() && !current.is_empty() && index > 0 && {
                    let previous = chars[index - 1];
                    let next_is_lower =
                        chars.get(index + 1).is_some_and(|next| next.is_lowercase());
                    previous.is_lowercase()
                        || previous.is_ascii_digit()
                        || (previous.is_uppercase() && next_is_lower)
                };
            if starts_new_token {
                tokens.push(std::mem::take(&mut current).to_lowercase());
            }
            current.push(character);
        }
        if !current.is_empty() {
            tokens.push(current.to_lowercase());
        }
        tokens
    }
}

#[async_trait]
impl AppEnumerator for WindowsAppEnumerator {
    /// 枚举所有已安装应用。
    /// 通过 shell:AppsFolder 枚举 UWP 应用，读取 IPropertyStore 属性构建 AppInfo。
    /// 参数：无。
    /// 返回：应用信息列表。
    async fn enumerate_apps(&self) -> Vec<AppInfo> {
        let mut result = Vec::new();

        unsafe {
            let _com_guard = ComGuard::init();

            // Create Shell item for AppsFolder
            let tmp = get_u16_vec("shell:AppsFolder");
            let app_folder: IShellItem =
                match SHCreateItemFromParsingName(PCWSTR::from_raw(tmp.as_ptr()), None) {
                    Ok(item) => item,
                    Err(e) => {
                        warn!("WindowsAppEnumerator: fail to open shell:AppsFolder {}", e);
                        return result;
                    }
                };

            // Bind to IEnumShellItems
            let enum_shell_items: IEnumShellItems =
                match app_folder.BindToHandler(None, &BHID_EnumItems) {
                    Ok(enumerator) => enumerator,
                    Err(e) => {
                        warn!("WindowsAppEnumerator: fail to bind to handler {}", e);
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
            // 每次 Next 最多填充缓冲区长度，必须循环取完，否则条目数超过单批上限时后续应用被静默丢弃
            const BATCH_SIZE: usize = 256;
            let mut shell_items: Vec<IShellItem> = Vec::new();
            loop {
                let mut batch: Vec<Option<IShellItem>> = vec![None; BATCH_SIZE];
                let mut fetched: u32 = 0;
                if let Err(e) = enum_shell_items.Next(&mut batch, Some(&mut fetched as *mut u32)) {
                    warn!("WindowsAppEnumerator: error enumerating shell items: {}", e);
                    break;
                }
                // 本次返回 0 条表示枚举已结束（不足一批也可能是末尾，继续调用无副作用）
                if fetched == 0 {
                    break;
                }
                let filled = (fetched as usize).min(batch.len());
                for item in batch.into_iter().take(filled).flatten() {
                    shell_items.push(item);
                }
            }

            for shell_item in &shell_items {
                // Bind to IPropertyStore
                let property_store: IPropertyStore =
                    match shell_item.BindToHandler(None, &BHID_PropertyStore) {
                        Ok(store) => store,
                        Err(e) => {
                            warn!(
                                "WindowsAppEnumerator: error binding to property store: {}",
                                e
                            );
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

                let search_keywords = Self::app_id_keywords(&app_id, &short_name);
                result.push(AppInfo {
                    app_id,
                    display_name: short_name,
                    icon: IconRequest::Path(icon_path),
                    install_path: Some(install_path),
                    search_keywords,
                });
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 包名派生的英文别名：去掉发布者后生成连写与空格分隔两种形式。
    #[test]
    fn app_id_keywords_follow_package_name() {
        let cases = [
            (
                "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App",
                "终端",
                vec!["windowsterminal", "windows terminal"],
            ),
            (
                "Microsoft.ScreenSketch_8wekyb3d8bbwe!App",
                "截图工具",
                vec!["screensketch", "screen sketch"],
            ),
            ("Microsoft.Paint_8wekyb3d8bbwe!App", "画图", vec!["paint"]),
            (
                "Microsoft.MicrosoftEdge.Stable_8wekyb3d8bbwe!App",
                "Microsoft Edge",
                vec!["edgestable", "edge stable"],
            ),
            (
                "Microsoft.Windows.Photos_8wekyb3d8bbwe!App",
                "照片",
                vec!["windowsphotos", "windows photos"],
            ),
        ];
        for (app_id, display_name, expected) in cases {
            assert_eq!(
                WindowsAppEnumerator::app_id_keywords(app_id, display_name),
                expected,
                "{app_id}"
            );
        }
    }

    /// 路径/占位 AUMID 不派生别名，与展示名重复的形式不注入。
    #[test]
    fn app_id_keywords_skip_unusable_ids() {
        assert!(WindowsAppEnumerator::app_id_keywords("", "终端").is_empty());
        assert!(
            WindowsAppEnumerator::app_id_keywords("C:\\Program Files\\App\\App.exe", "App")
                .is_empty()
        );
        assert!(WindowsAppEnumerator::app_id_keywords(
            "{6D809377-6AF0-444B-8957-A3773F02200E}\\App.exe",
            "App"
        )
        .is_empty());
        // 英文系统上展示名已覆盖空格形式，只保留连写别名
        assert_eq!(
            WindowsAppEnumerator::app_id_keywords(
                "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App",
                "Windows Terminal"
            ),
            vec!["windowsterminal"]
        );
    }
}
