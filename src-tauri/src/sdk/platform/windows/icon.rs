use crate::sdk::common::image_utils::ImageUtils;
use crate::sdk::host_api::HostApiError;
use crate::sdk::icon::icon_extractor::IconExtractor;
use crate::utils::defer::defer;
use crate::utils::windows::get_u16_vec;
use async_trait::async_trait;
use base64::prelude::*;
use core::mem::MaybeUninit;
use image::RgbaImage;
use parking_lot::RwLock;
use scraper::{Html, Selector};
use std::ffi::c_void;
use std::mem;
use std::path::Path;
use tracing::{debug, info, warn};
use url::Url;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetBitmapBits, GetObjectW, BITMAP, BITMAPINFOHEADER, BI_RGB, HBITMAP, HGDIOBJ,
};
use windows::Win32::Networking::WinInet::{InternetGetConnectedState, INTERNET_CONNECTION};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::Shell::SHFILEINFOW;
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHGFI_ADDOVERLAYS, SHGFI_ICON, SHGFI_LARGEICON};
use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};
use windows::Win32::UI::WindowsAndMessaging::{LoadImageW, IMAGE_ICON, LR_LOADFROMFILE};
use windows_core::BOOL;
use windows_core::PCWSTR;

/// 文件类型枚举
#[derive(Debug, PartialEq)]
enum FileType {
    Program,
    UrlShortcut,
    Ico,
    Image,
    Other,
}

fn get_file_type(path: &str) -> FileType {
    let path_lower = path.to_lowercase();
    if path_lower.ends_with(".url") {
        return FileType::UrlShortcut;
    }
    if path_lower.ends_with(".exe") || path_lower.ends_with(".lnk") {
        return FileType::Program;
    }
    if path_lower.ends_with(".ico") {
        return FileType::Ico;
    }
    if path_lower.ends_with(".png")
        || path_lower.ends_with(".jpg")
        || path_lower.ends_with(".jpeg")
        || path_lower.ends_with(".gif")
        || path_lower.ends_with(".bmp")
        || path_lower.ends_with(".webp")
        || path_lower.ends_with(".tiff")
        || path_lower.ends_with(".tif")
        || path_lower.ends_with(".svg")
    {
        return FileType::Image;
    }
    FileType::Other
}

/// Windows 平台图标提取器。
/// 实现了 IconExtractor trait 的 6 个平台原语方法。
/// 图标提取逻辑从 core::image_processor 搬迁而来，直接使用 IconRequest（不再有 ImageIdentity 转换）。
pub struct WindowsIconExtractor {
    /// 默认应用图标路径
    default_app_icon_path: String,
    /// 默认网址图标路径
    default_web_icon_path: String,
    /// 是否启用在线图标获取
    enable_online: RwLock<bool>,
}

impl WindowsIconExtractor {
    /// 创建 Windows 平台图标提取器。
    /// 参数：default_app_icon_path - 默认应用图标路径；default_web_icon_path - 默认网址图标路径。
    /// 返回：初始化后的 WindowsIconExtractor。
    pub fn new(default_app_icon_path: String, default_web_icon_path: String) -> Self {
        Self {
            default_app_icon_path,
            default_web_icon_path,
            enable_online: RwLock::new(true),
        }
    }

    /// 更新在线图标获取配置。
    /// 参数：enable_online - 是否启用在线图标获取。
    pub fn set_enable_online(&self, enable_online: bool) {
        *self.enable_online.write() = enable_online;
    }

    // ===== Windows API 图标提取 =====

    /// 从路径加载图标 PNG 数据。
    async fn load_image_from_path(&self, icon_path: &str) -> Result<Vec<u8>, HostApiError> {
        match get_file_type(icon_path) {
            FileType::Ico => {
                debug!("Detected ICO file, extracting largest icon: {}", icon_path);
                let rgba_image = Self::extract_largest_icon_from_ico_file(icon_path)
                    .await
                    .ok_or_else(|| HostApiError::IconExtractionFailed {
                        request: icon_path.to_string(),
                        reason: format!(
                            "Failed to extract largest icon from ICO file: {}",
                            icon_path
                        ),
                    })?;
                ImageUtils::rgba_image_to_png(&rgba_image).map_err(|e| {
                    HostApiError::IconExtractionFailed {
                        request: icon_path.to_string(),
                        reason: e.to_string(),
                    }
                })
            }
            FileType::Image => {
                debug!("Detected image file, loading directly: {}", icon_path);
                let file_content = tokio::fs::read(icon_path).await.map_err(|e| {
                    HostApiError::IconExtractionFailed {
                        request: icon_path.to_string(),
                        reason: format!("Failed to read image file: {}", e),
                    }
                })?;
                ImageUtils::convert_image_to_png(file_content)
                    .await
                    .map_err(|e| HostApiError::IconExtractionFailed {
                        request: icon_path.to_string(),
                        reason: e.to_string(),
                    })
            }
            FileType::UrlShortcut => {
                debug!(
                    "Detected URL shortcut file, parsing icon info: {}",
                    icon_path
                );
                let rgba_image = Self::extract_icon_from_url_file(icon_path)
                    .await
                    .ok_or_else(|| HostApiError::IconExtractionFailed {
                        request: icon_path.to_string(),
                        reason: format!(
                            "Failed to extract icon from URL shortcut file: {}",
                            icon_path
                        ),
                    })?;
                ImageUtils::rgba_image_to_png(&rgba_image).map_err(|e| {
                    HostApiError::IconExtractionFailed {
                        request: icon_path.to_string(),
                        reason: e.to_string(),
                    }
                })
            }
            FileType::Program | FileType::Other => {
                debug!(
                    "Detected {} file, extracting system icon: {}",
                    if matches!(get_file_type(icon_path), FileType::Program) {
                        "program"
                    } else {
                        "non-image"
                    },
                    icon_path
                );
                let rgba_image =
                    Self::extract_icon_from_file(icon_path)
                        .await
                        .ok_or_else(|| HostApiError::IconExtractionFailed {
                            request: icon_path.to_string(),
                            reason: format!(
                                "Failed to extract system icon from file: {}",
                                icon_path
                            ),
                        })?;
                ImageUtils::rgba_image_to_png(&rgba_image).map_err(|e| {
                    HostApiError::IconExtractionFailed {
                        request: icon_path.to_string(),
                        reason: e.to_string(),
                    }
                })
            }
        }
    }

    /// 从 ICO 文件中提取最大尺寸的图标。
    async fn extract_largest_icon_from_ico_file(file_path: &str) -> Option<RgbaImage> {
        const MAX_RETRIES: u32 = 3;
        let file_path = file_path.to_string();
        for attempt in 0..=MAX_RETRIES {
            let result = tauri::async_runtime::spawn_blocking({
                let file_path = file_path.clone();
                move || {
                    let com_init = unsafe { windows::Win32::System::Com::CoInitialize(None) };
                    if com_init.is_err() {
                        warn!("初始化com库失败：{:?}", com_init);
                    }
                    defer(move || unsafe {
                        if com_init.is_ok() {
                            windows::Win32::System::Com::CoUninitialize();
                        }
                    });

                    let wide_file_path = get_u16_vec(file_path.clone());

                    let hicon_result = unsafe {
                        LoadImageW(
                            None,
                            PCWSTR::from_raw(wide_file_path.as_ptr()),
                            IMAGE_ICON,
                            256,
                            256,
                            LR_LOADFROMFILE,
                        )
                    };

                    let hicon_handle = match hicon_result {
                        Ok(handle) => handle,
                        Err(e) => {
                            warn!(
                                "LoadImageW failed for ICO file: {}, error: {:?} (attempt {})",
                                file_path, e, attempt + 1
                            );
                            return None;
                        }
                    };

                    if hicon_handle.is_invalid() {
                        let last_error = unsafe { GetLastError() };
                        warn!(
                            "LoadImageW returned invalid handle for ICO file: {}, error: {:?} (attempt {})",
                            file_path, last_error, attempt + 1
                        );
                        return None;
                    }

                    let hicon = HICON(hicon_handle.0);

                    unsafe {
                        let image_result = Self::convert_icon_to_image(hicon);
                        if let Err(e) = DestroyIcon(hicon) {
                            warn!("Failed to destroy icon: {:?}", e);
                        }
                        image_result.ok()
                    }
                }
            })
            .await
            .expect("Tokio spawn should not fail");

            if result.is_some() {
                return result;
            }

            if attempt < MAX_RETRIES {
                use rand::{rng, Rng};
                let delay_ms = rng().random_range(50..=250);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                info!(
                    "Retrying largest icon extraction for ICO file: {} (attempt {}/{})",
                    file_path,
                    attempt + 1,
                    MAX_RETRIES + 1
                );
            }
        }

        warn!(
            "All {} attempts to extract largest icon failed for ICO file: {}",
            MAX_RETRIES + 1,
            file_path
        );
        None
    }

    /// 从文件提取 HICON。
    async fn extract_icon_from_file(file_path: &str) -> Option<RgbaImage> {
        const MAX_RETRIES: u32 = 3;
        let file_path = file_path.to_string();
        for attempt in 0..=MAX_RETRIES {
            let result = tauri::async_runtime::spawn_blocking({
                let file_path = file_path.clone();
                move || {
                    let com_init = unsafe { windows::Win32::System::Com::CoInitialize(None) };
                    if com_init.is_err() {
                        warn!("初始化com库失败：{:?}", com_init);
                    }
                    defer(move || unsafe {
                        if com_init.is_ok() {
                            windows::Win32::System::Com::CoUninitialize();
                        }
                    });

                    let wide_file_path = get_u16_vec(file_path.clone());
                    let mut sh_file_info: SHFILEINFOW = unsafe { std::mem::zeroed() };
                    let result = unsafe {
                        SHGetFileInfoW(
                            PCWSTR::from_raw(wide_file_path.as_ptr()),
                            FILE_ATTRIBUTE_NORMAL,
                            Some(&mut sh_file_info),
                            std::mem::size_of::<SHFILEINFOW>() as u32,
                            SHGFI_ICON | SHGFI_LARGEICON | SHGFI_ADDOVERLAYS,
                        )
                    };

                    if result == 0 {
                        let last_error = unsafe { GetLastError() };
                        warn!(
                            "SHGetFileInfoW failed for: {}, error: {:?}",
                            file_path, last_error
                        );
                        return None;
                    }

                    if sh_file_info.hIcon.is_invalid() {
                        warn!(
                            "Invalid icon handle for: {} (attempt {})",
                            file_path,
                            attempt + 1
                        );
                        return None;
                    }

                    let hicon = sh_file_info.hIcon;

                    unsafe {
                        let image_result = Self::convert_icon_to_image(hicon);
                        if let Err(e) = DestroyIcon(hicon) {
                            warn!("Failed to destroy icon: {:?}", e);
                        }
                        image_result.ok()
                    }
                }
            })
            .await
            .expect("Tokio spawn should not fail");

            if result.is_some() {
                return result;
            }

            if attempt < MAX_RETRIES {
                use rand::{rng, Rng};
                let delay_ms = rng().random_range(50..=250);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                info!(
                    "Retrying icon extraction for: {} (attempt {}/{})",
                    file_path,
                    attempt + 1,
                    MAX_RETRIES + 1
                );
            }
        }

        warn!(
            "All {} attempts to extract icon failed for: {}",
            MAX_RETRIES + 1,
            file_path
        );
        None
    }

    /// 从 .url 文件提取图标。
    async fn extract_icon_from_url_file(file_path: &str) -> Option<RgbaImage> {
        let file_path = file_path.to_string();
        let content = tokio::fs::read_to_string(&file_path).await.ok()?;

        let mut icon_file: Option<String> = None;
        let mut icon_index: i32 = 0;

        for line in content.lines() {
            let line = line.trim();
            if let Some(value) = line.strip_prefix("IconFile=") {
                icon_file = Some(value.to_string());
            } else if let Some(value) = line.strip_prefix("IconIndex=") {
                icon_index = value.parse().unwrap_or(0);
            }
        }

        if let Some(icon_path) = icon_file {
            debug!(
                "Found icon info in .url file: IconFile={}, IconIndex={}",
                icon_path, icon_index
            );

            if !Path::new(&icon_path).exists() {
                warn!("Icon file does not exist: {}", icon_path);
                return Self::extract_icon_from_file(&file_path).await;
            }

            let icon_path_lower = icon_path.to_lowercase();

            if icon_path_lower.ends_with(".ico") {
                return Self::extract_largest_icon_from_ico_file(&icon_path).await;
            } else if icon_path_lower.ends_with(".exe")
                || icon_path_lower.ends_with(".dll")
                || icon_path_lower.ends_with(".icl")
            {
                return Self::extract_icon_by_index(&icon_path, icon_index).await;
            } else if let Ok(data) = tokio::fs::read(&icon_path).await {
                if let Ok(png_data) = ImageUtils::convert_image_to_png(data).await {
                    if let Ok(img) = image::load_from_memory(&png_data) {
                        return Some(img.to_rgba8());
                    }
                }
            }
        }

        debug!(
            "No IconFile found in .url file or extraction failed, falling back to SHGetFileInfoW: {}",
            file_path
        );
        Self::extract_icon_from_file(&file_path).await
    }

    /// 从 exe/dll 文件中提取指定索引的图标。
    async fn extract_icon_by_index(file_path: &str, icon_index: i32) -> Option<RgbaImage> {
        const MAX_RETRIES: u32 = 3;
        let file_path = file_path.to_string();

        for attempt in 0..=MAX_RETRIES {
            let result = tauri::async_runtime::spawn_blocking({
                let file_path = file_path.clone();
                move || {
                    let com_init = unsafe { windows::Win32::System::Com::CoInitialize(None) };
                    if com_init.is_err() {
                        warn!("初始化com库失败：{:?}", com_init);
                    }
                    defer(move || unsafe {
                        if com_init.is_ok() {
                            windows::Win32::System::Com::CoUninitialize();
                        }
                    });

                    let wide_file_path = get_u16_vec(file_path.clone());
                    let mut large_icon: HICON = HICON::default();

                    let extracted_count = unsafe {
                        ExtractIconExW(
                            PCWSTR::from_raw(wide_file_path.as_ptr()),
                            icon_index,
                            Some(&mut large_icon),
                            None,
                            1,
                        )
                    };

                    if extracted_count == 0 || large_icon.is_invalid() {
                        warn!(
                            "ExtractIconExW failed for: {}, index: {} (attempt {})",
                            file_path,
                            icon_index,
                            attempt + 1
                        );
                        return None;
                    }

                    unsafe {
                        let image_result = Self::convert_icon_to_image(large_icon);
                        if let Err(e) = DestroyIcon(large_icon) {
                            warn!("Failed to destroy icon: {:?}", e);
                        }
                        image_result.ok()
                    }
                }
            })
            .await
            .expect("Tokio spawn should not fail");

            if result.is_some() {
                return result;
            }

            if attempt < MAX_RETRIES {
                use rand::{rng, Rng};
                let delay_ms = rng().random_range(50..=250);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                info!(
                    "Retrying icon extraction by index for: {} (attempt {}/{})",
                    file_path,
                    attempt + 1,
                    MAX_RETRIES + 1
                );
            }
        }

        warn!(
            "All {} attempts to extract icon by index failed for: {}",
            MAX_RETRIES + 1,
            file_path
        );
        None
    }

    /// 从扩展名提取系统关联图标。
    async fn extract_icon_from_extension(ext: &str) -> Result<Vec<u8>, HostApiError> {
        use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY;
        use windows::Win32::UI::Shell::SHGFI_USEFILEATTRIBUTES;

        let ext = ext.to_string();
        let ext_for_error = ext.clone();
        let image_buffer = tauri::async_runtime::spawn_blocking(move || {
            let com_init = unsafe { windows::Win32::System::Com::CoInitialize(None) };
            defer(move || unsafe {
                if com_init.is_ok() {
                    windows::Win32::System::Com::CoUninitialize();
                }
            });

            let mut sh_file_info: SHFILEINFOW = unsafe { std::mem::zeroed() };
            let flags = SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES;
            let mut file_attributes = FILE_ATTRIBUTE_NORMAL;

            let wide_path;
            if ext == "folder" {
                file_attributes = FILE_ATTRIBUTE_DIRECTORY;
                wide_path = get_u16_vec("dummy_folder");
            } else {
                wide_path = get_u16_vec(ext.clone());
            }
            let path_ptr = wide_path.as_ptr();

            let result = unsafe {
                SHGetFileInfoW(
                    PCWSTR::from_raw(path_ptr),
                    file_attributes,
                    Some(&mut sh_file_info),
                    std::mem::size_of::<SHFILEINFOW>() as u32,
                    flags,
                )
            };

            if result == 0 || sh_file_info.hIcon.is_invalid() {
                return None;
            }

            let hicon = sh_file_info.hIcon;

            unsafe {
                let res = Self::convert_icon_to_image(hicon);
                let _ = DestroyIcon(hicon);
                res.ok()
            }
        })
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

        ImageUtils::rgba_image_to_png(&image_buffer).map_err(|e| {
            HostApiError::IconExtractionFailed {
                request: ext_for_error,
                reason: e.to_string(),
            }
        })
    }

    /// 将 HICON 转换为 RGBA 图像。
    unsafe fn convert_icon_to_image(icon: HICON) -> Result<RgbaImage, HostApiError> {
        let bitmap_size_i32 = i32::try_from(mem::size_of::<BITMAP>()).map_err(|_| {
            HostApiError::IconExtractionFailed {
                request: "convert_icon_to_image".to_string(),
                reason: "BITMAP size conversion failed".to_string(),
            }
        })?;
        let biheader_size_u32 =
            u32::try_from(mem::size_of::<BITMAPINFOHEADER>()).map_err(|_| {
                HostApiError::IconExtractionFailed {
                    request: "convert_icon_to_image".to_string(),
                    reason: "BITMAPINFOHEADER size conversion failed".to_string(),
                }
            })?;

        let mut info = ICONINFO {
            fIcon: BOOL(0),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: std::mem::zeroed::<HBITMAP>() as HBITMAP,
            hbmColor: std::mem::zeroed::<HBITMAP>() as HBITMAP,
        };
        GetIconInfo(icon, &mut info).map_err(|_| HostApiError::IconExtractionFailed {
            request: "convert_icon_to_image".to_string(),
            reason: "Failed to get icon information".to_string(),
        })?;
        DeleteObject(HGDIOBJ(info.hbmMask.0)).expect("Failed to delete mask object");
        let mut bitmap: MaybeUninit<BITMAP> = MaybeUninit::uninit();

        let result = GetObjectW(
            HGDIOBJ(info.hbmColor.0),
            bitmap_size_i32,
            Some(bitmap.as_mut_ptr() as *mut c_void),
        );

        if result != bitmap_size_i32 {
            return Err(HostApiError::IconExtractionFailed {
                request: "convert_icon_to_image".to_string(),
                reason: "Failed to get bitmap object information".to_string(),
            });
        }
        let bitmap = bitmap.assume_init_ref();

        let width_u32 =
            u32::try_from(bitmap.bmWidth).map_err(|_| HostApiError::IconExtractionFailed {
                request: "convert_icon_to_image".to_string(),
                reason: format!("Invalid bitmap width: {}", bitmap.bmWidth),
            })?;
        let height_u32 =
            u32::try_from(bitmap.bmHeight).map_err(|_| HostApiError::IconExtractionFailed {
                request: "convert_icon_to_image".to_string(),
                reason: format!("Invalid bitmap height: {}", bitmap.bmHeight),
            })?;
        let width_usize =
            usize::try_from(bitmap.bmWidth).map_err(|_| HostApiError::IconExtractionFailed {
                request: "convert_icon_to_image".to_string(),
                reason: format!("Invalid bitmap width for usize: {}", bitmap.bmWidth),
            })?;
        let height_usize =
            usize::try_from(bitmap.bmHeight).map_err(|_| HostApiError::IconExtractionFailed {
                request: "convert_icon_to_image".to_string(),
                reason: format!("Invalid bitmap height for usize: {}", bitmap.bmHeight),
            })?;
        let buf_size = width_usize
            .checked_mul(height_usize)
            .and_then(|size| size.checked_mul(4))
            .ok_or_else(|| HostApiError::IconExtractionFailed {
                request: "convert_icon_to_image".to_string(),
                reason: format!(
                    "Integer overflow calculating buffer size: {}x{}x4",
                    width_usize, height_usize
                ),
            })?;

        let dc: windows::Win32::Graphics::Gdi::HDC = windows::Win32::Graphics::Gdi::GetDC(None);
        if dc == windows::Win32::Graphics::Gdi::HDC(std::ptr::null_mut()) {
            return Err(HostApiError::IconExtractionFailed {
                request: "convert_icon_to_image".to_string(),
                reason: "Failed to get device context".to_string(),
            });
        }

        let _bitmap_info = BITMAPINFOHEADER {
            biSize: biheader_size_u32,
            biWidth: bitmap.bmWidth,
            biHeight: -bitmap.bmHeight.abs(),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };

        let mut bmp: Vec<u8> = vec![0; buf_size];
        let _mr_right = GetBitmapBits(
            info.hbmColor,
            buf_size as i32,
            bmp.as_mut_ptr() as *mut c_void,
        );
        let result = windows::Win32::Graphics::Gdi::ReleaseDC(None, dc);
        if result != 1 {
            return Err(HostApiError::IconExtractionFailed {
                request: "convert_icon_to_image".to_string(),
                reason: "Failed to release device context".to_string(),
            });
        }
        DeleteObject(HGDIOBJ(info.hbmColor.0)).expect("Failed to delete color object");

        for chunk in bmp.chunks_exact_mut(4) {
            let [b, _, r, _] = chunk else {
                return Err(HostApiError::IconExtractionFailed {
                    request: "convert_icon_to_image".to_string(),
                    reason: "Unexpected chunk size in pixel data".to_string(),
                });
            };
            mem::swap(b, r);
        }

        RgbaImage::from_vec(width_u32, height_u32, bmp).ok_or_else(|| {
            HostApiError::IconExtractionFailed {
                request: "convert_icon_to_image".to_string(),
                reason: "Failed to create RgbaImage from pixel data".to_string(),
            }
        })
    }

    // ===== 网站图标获取（favicon）=====

    /// 获取网站的 PNG 格式图标 (favicon)。
    async fn fetch_website_favicon_data(&self, url: &str) -> Result<Vec<u8>, HostApiError> {
        if !self.is_network_available() {
            debug!("No network connection available");
            return Err(HostApiError::IconExtractionFailed {
                request: url.to_string(),
                reason: "No network connection available".to_string(),
            });
        }

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| HostApiError::IconExtractionFailed {
                request: url.to_string(),
                reason: format!("Failed to build reqwest client: {}", e),
            })?;

        // 定义候选 URL列表：首先尝试原始 URL
        // 然后尝试根域名及逐级向上的父域名
        // 例如输入: https://sub.example.com/foo/bar
        // 候选列表将包含:
        // 1. https://sub.example.com/foo/bar (原始)
        // 2. https://sub.example.com/ (当前域名根)
        // 3. https://example.com/ (父域名根)
        let mut candidate_urls = vec![url.to_string()];

        if let Ok(parsed_url) = Url::parse(url) {
            if let Some(host) = parsed_url.host_str() {
                let segments: Vec<&str> = host.split('.').collect();
                let scheme = parsed_url.scheme();

                for i in 0..(segments.len().saturating_sub(1)) {
                    let current_host = segments[i..].join(".");
                    let root_url = format!("{}://{}/", scheme, current_host);

                    let is_duplicate = candidate_urls
                        .iter()
                        .any(|c| c.trim_end_matches('/') == root_url.trim_end_matches('/'));

                    if !is_duplicate {
                        candidate_urls.push(root_url);
                    }
                }
            }
        }

        for (index, candidate_url) in candidate_urls.iter().enumerate() {
            debug!(
                "Attempting favicon fetch from candidate {}/{}: {}",
                index + 1,
                candidate_urls.len(),
                candidate_url
            );

            match Self::fetch_favicon_primary(&client, candidate_url).await {
                Ok(data) => match ImageUtils::convert_image_to_png(data).await {
                    Ok(png_data) => {
                        if index > 0 {
                            info!(
                                "Successfully fetched favicon from fallback URL: {}",
                                candidate_url
                            );
                        }
                        return Ok(png_data);
                    }
                    Err(e) => {
                        warn!(
                            "Favicon found at {} but validation failed. Error: {}",
                            candidate_url, e
                        );
                    }
                },
                Err(e) => {
                    debug!("Fetch failed for candidate {}: {}", candidate_url, e);
                }
            }
        }

        Err(HostApiError::IconExtractionFailed {
            request: url.to_string(),
            reason: format!("Failed to fetch valid favicon for {}", url),
        })
    }

    /// 获取网站的主图标。
    async fn fetch_favicon_primary(
        client: &reqwest::Client,
        url: &str,
    ) -> Result<Vec<u8>, HostApiError> {
        let response =
            client
                .get(url)
                .send()
                .await
                .map_err(|e| HostApiError::IconExtractionFailed {
                    request: url.to_string(),
                    reason: format!("Failed to fetch website: {}", e),
                })?;

        if let Some(ct) = response.headers().get(reqwest::header::CONTENT_TYPE) {
            if ct.to_str().unwrap_or("").starts_with("image/") {
                return Ok(response
                    .bytes()
                    .await
                    .map_err(|e| HostApiError::IconExtractionFailed {
                        request: url.to_string(),
                        reason: format!("Failed to read image bytes: {}", e),
                    })?
                    .to_vec());
            }
        }

        let html_content =
            response
                .text()
                .await
                .map_err(|e| HostApiError::IconExtractionFailed {
                    request: url.to_string(),
                    reason: format!("Failed to read response text: {}", e),
                })?;

        let icon_url = {
            let document = Html::parse_document(&html_content);
            let base_url = Url::parse(url).map_err(|e| HostApiError::IconExtractionFailed {
                request: url.to_string(),
                reason: format!("Invalid URL format: {}", e),
            })?;

            let icon_selectors = [
                r#"link[rel="apple-touch-icon"]"#,
                r#"link[rel="apple-touch-icon-precomposed"]"#,
                r#"link[rel="icon"]"#,
                r#"link[rel="shortcut icon"]"#,
            ];

            #[derive(Debug)]
            struct IconCandidate {
                url: String,
                size: u32,
                priority: u8,
            }

            let mut candidates = Vec::new();

            for (index, selector_str) in icon_selectors.iter().enumerate() {
                let base_priority = (icon_selectors.len() - index) as u8;

                let selector = Selector::parse(selector_str).map_err(|e| {
                    HostApiError::IconExtractionFailed {
                        request: url.to_string(),
                        reason: format!("Failed to parse CSS selector: {}", e),
                    }
                })?;

                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        if let Ok(icon_url) = base_url.join(href) {
                            let size = if let Some(sizes_attr) = element.value().attr("sizes") {
                                Self::parse_icon_size(sizes_attr)
                            } else if selector_str.contains("apple-touch-icon") {
                                180
                            } else {
                                32
                            };

                            candidates.push(IconCandidate {
                                url: icon_url.to_string(),
                                size,
                                priority: base_priority,
                            });
                        }
                    }
                }
            }

            if let Some(best_candidate) =
                candidates
                    .iter()
                    .max_by(|a, b| match a.priority.cmp(&b.priority) {
                        std::cmp::Ordering::Equal => a.size.cmp(&b.size),
                        other => other,
                    })
            {
                info!(
                    "Selected best icon: {} (size: {}x{})",
                    best_candidate.url, best_candidate.size, best_candidate.size
                );
                best_candidate.url.clone()
            } else {
                let default_url = base_url
                    .join("/favicon.ico")
                    .expect("/favicon.ico should always be joinable");
                info!("No icon found in HTML, falling back to: {}", default_url);
                default_url.to_string()
            }
        };

        info!("Downloading favicon from: {}", icon_url);

        if icon_url.starts_with("data:") {
            if let Some(comma_pos) = icon_url.find(',') {
                let meta = &icon_url[5..comma_pos];
                let data = &icon_url[comma_pos + 1..];

                if meta.ends_with(";base64") {
                    let decoded = BASE64_STANDARD.decode(data).map_err(|e| {
                        HostApiError::IconExtractionFailed {
                            request: url.to_string(),
                            reason: format!("Failed to decode base64 data URI: {}", e),
                        }
                    })?;
                    return Ok(decoded);
                } else {
                    return Ok(data.as_bytes().to_vec());
                }
            }
        }

        let icon_response =
            client
                .get(&icon_url)
                .send()
                .await
                .map_err(|e| HostApiError::IconExtractionFailed {
                    request: url.to_string(),
                    reason: format!("Failed to fetch favicon from: {}: {}", icon_url, e),
                })?;

        let icon_data =
            icon_response
                .bytes()
                .await
                .map_err(|e| HostApiError::IconExtractionFailed {
                    request: url.to_string(),
                    reason: format!("Failed to read favicon bytes: {}", e),
                })?;

        Ok(icon_data.to_vec())
    }

    /// 解析图标尺寸字符串。
    fn parse_icon_size(sizes: &str) -> u32 {
        if sizes.eq_ignore_ascii_case("any") {
            return 512;
        }

        sizes
            .split_whitespace()
            .filter_map(|size_str| {
                let parts: Vec<&str> = size_str.split('x').collect();
                if parts.len() == 2 {
                    let width = parts[0].parse::<u32>().ok()?;
                    let height = parts[1].parse::<u32>().ok()?;
                    Some(width.max(height))
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(32)
    }
}

#[async_trait]
impl IconExtractor for WindowsIconExtractor {
    async fn extract_from_path(&self, path: &str) -> Result<Vec<u8>, HostApiError> {
        self.load_image_from_path(path).await
    }

    async fn extract_from_url(&self, url: &str) -> Result<Vec<u8>, HostApiError> {
        let enable_online = *self.enable_online.read();
        if !enable_online {
            return Err(HostApiError::IconExtractionFailed {
                request: url.to_string(),
                reason: "Online icon fetching is disabled".to_string(),
            });
        }
        self.fetch_website_favicon_data(url).await
    }

    async fn extract_from_extension(&self, ext: &str) -> Result<Vec<u8>, HostApiError> {
        Self::extract_icon_from_extension(ext).await
    }

    fn default_app_icon_path(&self) -> &str {
        &self.default_app_icon_path
    }

    fn default_web_icon_path(&self) -> &str {
        &self.default_web_icon_path
    }

    fn is_network_available(&self) -> bool {
        unsafe {
            let mut flags = INTERNET_CONNECTION::default();
            InternetGetConnectedState(&mut flags, None).is_ok()
        }
    }
}
