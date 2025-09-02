use crate::error::{AppError, AppResult, OptionExt, ResultExt};
use crate::utils::defer::defer;
use crate::utils::windows::get_u16_vec;
use core::mem::MaybeUninit;
use fnv::FnvHasher;
use image::GenericImageView;
use image::ImageBuffer;
use image::ImageEncoder;
use image::ImageFormat;
use image::Rgba;
use image::RgbaImage;
use kmeans_colors::get_kmeans;
use palette::{IntoColor, Lab, Srgb};
use rand::Rng;
use rayon::prelude::*;
use resvg;
use scraper::{Html, Selector};
use std::ffi::c_void;
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::mem;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tiny_skia;
use tracing::{debug, error, info, warn};
use url::Url;
use usvg;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Graphics::Gdi::BITMAP;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetBitmapBits, GetObjectW, BITMAPINFOHEADER, BI_RGB, HBITMAP, HGDIOBJ,
};
use windows::Win32::Networking::WinInet::INTERNET_CONNECTION;
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::UI::Shell::SHFILEINFOW;
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHGFI_ADDOVERLAYS, SHGFI_ICON, SHGFI_LARGEICON};
use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};
use windows_core::BOOL;
use windows_core::PCWSTR;

/// 图片的身份标识，根据不同的身份，使用不同的函数来获取
#[derive(Debug, Clone)]
pub enum ImageIdentity {
    /// 普通的文件类型 => 获取路径的图片，如果是图片则返回png格式，如果是普通文件，则获取图标的png格式
    File(String),
    /// 网页类型 => 获取目标网站的图标
    Web(String),
}

impl ImageIdentity {
    pub fn get_text(&self) -> String {
        match self {
            ImageIdentity::File(path) => path.clone(),
            ImageIdentity::Web(path) => path.clone(),
        }
    }

    pub fn get_hash(&self) -> String {
        let mut hasher = FnvHasher::default();
        self.get_text().hash(&mut hasher);
        hasher.finish().to_string()
    }
}

pub struct ImageProcessor {}

impl ImageProcessor {
    /// 传入图片标识，返回png格式的数据
    /// 如果加载失败，返回空的Vec<u8>
    pub async fn load_image(icon_identity: &ImageIdentity) -> Vec<u8> {
        let result = match icon_identity {
            ImageIdentity::File(path) => Self::load_image_from_path_internal(path).await,
            ImageIdentity::Web(url) => Self::load_web_icon_internal(url).await,
        };

        match result {
            Ok(data) => {
                debug!(
                    "Successfully loaded image for: {:?}",
                    icon_identity.get_text()
                );
                data
            }
            Err(err) => {
                error!(
                    "Failed to load image for {:?}: {}",
                    icon_identity.get_text(),
                    err
                );
                vec![]
            }
        }
    }

    /// 内部函数：加载网页图标，返回Result类型
    async fn load_web_icon_internal(url: &str) -> AppResult<Vec<u8>> {
        debug!("Loading web icon from: {}", url);

        let icon_data = Self::fetch_website_favicon_png(url).await?;

        debug!("Fetched {} bytes of favicon data", icon_data.len());

        let png_data = Self::convert_image_to_png(icon_data).await?;

        debug!(
            "Successfully converted web icon to PNG ({} bytes)",
            png_data.len()
        );
        Ok(png_data)
    }

    /// 获取网站的 PNG 格式图标（favicon）
    async fn fetch_website_favicon_png(url: &str) -> AppResult<Vec<u8>> {
        debug!("Checking network availability for favicon fetch");
        if !Self::is_network_available() {
            debug!("No network connection available");
            return Err(AppError::NetworkError {
                message: "No network connection available".to_string(),
                source: None,
            });
        }

        debug!("Fetching website content from: {}", url);
        // 获取网页内容
        let response = reqwest::get(url).await.map_err(|e| {
            AppError::network_error_with_source(
                format!("Failed to fetch website: {}", url),
                Box::new(e),
            )
        })?;

        let html_content = response.text().await.map_err(|e| {
            AppError::network_error_with_source(
                "Failed to read response text".to_string(),
                Box::new(e),
            )
        })?;

        debug!("Parsing HTML content ({} bytes)", html_content.len());
        // 将 HTML 解析和图标 URL 提取放在一个同步代码块中
        let icon_url = {
            let document = Html::parse_document(&html_content);

            let icon_selector = Selector::parse(r#"link[rel="icon"], link[rel="shortcut icon"]"#)
                .map_err(|e| AppError::ImageProcessingError {
                message: format!("Failed to parse CSS selector: {}", e),
            })?;

            let base_url = Url::parse(url).map_err(|e| AppError::NetworkError {
                message: format!("Invalid URL format: {}", e),
                source: None,
            })?;

            let favicon_url = document
                .select(&icon_selector)
                .next()
                .and_then(|e| e.value().attr("href"))
                .and_then(|href| {
                    debug!("Found favicon href: {}", href);
                    base_url.join(href).ok()
                })
                .unwrap_or_else(|| {
                    debug!("No favicon link found, using default /favicon.ico");
                    base_url.join("/favicon.ico").expect_programming(
                        "Failed to create default favicon URL - this should never happen",
                    )
                });

            favicon_url.to_string()
        };

        info!("Downloading favicon from: {}", icon_url);
        // 下载图标
        let icon_response = reqwest::get(&icon_url).await.map_err(|e| {
            AppError::network_error_with_source(
                format!("Failed to fetch favicon from: {}", icon_url),
                Box::new(e),
            )
        })?;

        let icon_data = icon_response.bytes().await.map_err(|e| {
            AppError::network_error_with_source(
                "Failed to read favicon bytes".to_string(),
                Box::new(e),
            )
        })?;

        debug!(
            "Successfully downloaded favicon ({} bytes)",
            icon_data.len()
        );
        Ok(icon_data.to_vec())
    }

    fn is_network_available() -> bool {
        use windows::Win32::Networking::WinInet::InternetGetConnectedState;

        unsafe {
            let mut flags = INTERNET_CONNECTION::default();
            InternetGetConnectedState(&mut flags, None).is_ok()
        }
    }
    /// 内部函数：从路径加载图像，返回Result类型
    async fn load_image_from_path_internal(icon_path: &str) -> AppResult<Vec<u8>> {
        debug!("Loading image from path: {}", icon_path);

        if Self::is_program(icon_path) {
            debug!("Detected program file, extracting icon: {}", icon_path);
            // 使用Windows系统调用读取程序图标
            let rgba_image = Self::extract_icon_from_file(icon_path)
                .await
                .ok_or_else(|| AppError::ImageProcessingError {
                    message: format!("Failed to extract icon from program: {}", icon_path),
                })?;

            let png_data = Self::rgba_image_to_png(&rgba_image)?;

            debug!(
                "Successfully extracted and converted program icon ({} bytes)",
                png_data.len()
            );
            Ok(png_data)
        } else {
            debug!("Loading regular image file: {}", icon_path);
            // 直接使用库来读取图像文件
            let png_data = Self::load_and_convert_to_png(icon_path).await?;

            debug!(
                "Successfully loaded and converted image ({} bytes)",
                png_data.len()
            );
            Ok(png_data)
        }
    }

    async fn load_and_convert_to_png<P: AsRef<Path>>(file_path: P) -> AppResult<Vec<u8>> {
        let path_str = file_path.as_ref().to_string_lossy();
        debug!("Reading file content from: {}", path_str);

        // 异步读取文件内容
        let file_content = tokio::fs::read(file_path.as_ref()).await.map_err(|e| {
            AppError::filesystem_error_with_io(
                "Failed to read image file".to_string(),
                Some(path_str.to_string()),
                e,
            )
        })?;

        debug!("Read {} bytes from file", file_content.len());
        Self::convert_image_to_png(file_content).await
    }

    async fn convert_image_to_png(image_data: Vec<u8>) -> AppResult<Vec<u8>> {
        // 在阻塞线程中处理图像
        tauri::async_runtime::spawn_blocking(move || -> AppResult<Vec<u8>> {
            debug!("Converting {} bytes of image data to PNG", image_data.len());

            // 尝试将数据解析为 SVG
            match usvg::Tree::from_data(&image_data, &usvg::Options::default()) {
                Ok(tree) => {
                    debug!("Successfully parsed as SVG, rendering to PNG");
                    // 成功解析为 SVG，现在进行渲染
                    let pixmap_size = tree.size().to_int_size();

                    // 检查 SVG 尺寸是否有效
                    if pixmap_size.width() == 0 || pixmap_size.height() == 0 {
                        return Err(AppError::ImageProcessingError {
                            message: format!(
                                "Invalid SVG dimensions (width: {}px, height: {}px)",
                                pixmap_size.width(),
                                pixmap_size.height()
                            ),
                        });
                    }

                    // 创建一个 Pixmap 来渲染 SVG
                    let mut pixmap =
                        tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
                            .ok_or_else(|| AppError::ImageProcessingError {
                                message: "Failed to create Pixmap for SVG rendering".to_string(),
                            })?;

                    // 渲染 SVG 到 Pixmap
                    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

                    // 将 Pixmap 编码为 PNG 数据
                    let png_data =
                        pixmap
                            .encode_png()
                            .map_err(|e| AppError::ImageProcessingError {
                                message: format!("Failed to encode SVG as PNG: {}", e),
                            })?;

                    debug!(
                        "Successfully converted SVG to PNG ({} bytes)",
                        png_data.len()
                    );
                    Ok(png_data)
                }
                Err(svg_error) => {
                    debug!(
                        "Not an SVG or SVG parsing failed: {}, trying as regular image",
                        svg_error
                    );
                    // 解析 SVG 失败，或它不是 SVG。回退到原始的 image crate 逻辑。
                    let img_reader = image::ImageReader::new(Cursor::new(image_data))
                        .with_guessed_format()
                        .map_err(|e| AppError::ImageProcessingError {
                            message: format!("Failed to create image reader: {}", e),
                        })?;

                    // 读取图像格式
                    let format =
                        img_reader
                            .format()
                            .ok_or_else(|| AppError::ImageProcessingError {
                                message: "Unable to detect image format".to_string(),
                            })?;

                    debug!("Detected image format: {:?}", format);

                    // 解码图像
                    let mut img =
                        img_reader
                            .decode()
                            .map_err(|e| AppError::ImageProcessingError {
                                message: format!("Failed to decode image: {}", e),
                            })?;

                    // 如果不是 PNG 格式，则转换
                    if format != ImageFormat::Png {
                        debug!("Converting from {:?} to PNG format", format);
                        // 转换为 RGBA8 以便编码为 PNG
                        img = image::DynamicImage::ImageRgba8(img.to_rgba8());
                    }

                    // 使用 PNG 编码器将图像写入 Vec<u8>
                    let mut png_data = Vec::new();
                    let encoder = image::codecs::png::PngEncoder::new(&mut png_data);
                    img.write_with_encoder(encoder).map_err(|e| {
                        AppError::ImageProcessingError {
                            message: format!("Failed to encode image as PNG: {}", e),
                        }
                    })?;

                    debug!(
                        "Successfully converted image to PNG ({} bytes)",
                        png_data.len()
                    );
                    Ok(png_data)
                }
            }
        })
        .await
        .map_err(|e| {
            AppError::programming_error(format!("Task join error in image conversion: {}", e))
        })?
    }

    /// 判断是不是一个程序的图标
    fn is_program(path: &str) -> bool {
        if path.ends_with(".lnk") {
            return true;
        }
        if path.ends_with(".exe") {
            return true;
        }
        if path.ends_with(".url") {
            return true;
        }
        false
    }

    /// 从文件提取hicon
    /// 这个很奇怪啊，不知道为什么我的这个函数在运行的时候就会出现提取图标失败的问题，有可能是因为我开的并发太大了。
    /// 但是如果是一个一个的获取图片，速度又会比较慢，所以我这里就搞一个重试机制，目前测下来也没啥问题。
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

                    // 提前返回 None 如果无法获取图标
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

                    let image_buffer = unsafe {
                        // 将 HICON 传递给转换函数，并确保在转换完成后才销毁
                        let image_result = Self::convert_icon_to_image(hicon);
                        if let Err(e) = DestroyIcon(hicon) {
                            warn!("Failed to destroy icon: {:?}", e);
                        }
                        image_result.expect_programming("图标转换为图像失败")
                    };

                    Some(image_buffer)
                }
            })
            .await
            .expect_programming("Tokio spawn should not fail - this is a programming error");

            if result.is_some() {
                return result;
            }

            // 如果不是最后一次尝试，则等待随机时间后重试
            if attempt < MAX_RETRIES {
                // 生成100-200ms之间的随机延迟
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

        // 所有重试都失败
        warn!(
            "All {} attempts to extract icon failed for: {}",
            MAX_RETRIES + 1,
            file_path
        );
        None
    }
    /// 将icon图像变成raga图像
    unsafe fn convert_icon_to_image(icon: HICON) -> Result<RgbaImage, AppError> {
        let bitmap_size_i32 = i32::try_from(mem::size_of::<BITMAP>()).map_err(|_| {
            AppError::programming_error("BITMAP size conversion failed - this should never happen")
        })?;
        let biheader_size_u32 =
            u32::try_from(mem::size_of::<BITMAPINFOHEADER>()).map_err(|_| {
                AppError::programming_error(
                    "BITMAPINFOHEADER size conversion failed - this should never happen",
                )
            })?;
        let mut info = ICONINFO {
            fIcon: BOOL(0),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: std::mem::zeroed::<HBITMAP>() as HBITMAP,
            hbmColor: std::mem::zeroed::<HBITMAP>() as HBITMAP,
        };
        GetIconInfo(icon, &mut info).map_err(|_| AppError::ImageProcessingError {
            message: "Failed to get icon information".to_string(),
        })?;
        DeleteObject(HGDIOBJ(info.hbmMask.0)).expect("Failed to delete mask object");
        let mut bitmap: MaybeUninit<BITMAP> = MaybeUninit::uninit();

        let result = GetObjectW(
            HGDIOBJ(info.hbmColor.0),
            bitmap_size_i32,
            Some(bitmap.as_mut_ptr() as *mut c_void),
        );

        if result != bitmap_size_i32 {
            return Err(AppError::ImageProcessingError {
                message: "Failed to get bitmap object information".to_string(),
            });
        }
        let bitmap = bitmap.assume_init_ref();

        let width_u32 = u32::try_from(bitmap.bmWidth).map_err(|_| {
            AppError::programming_error(format!("Invalid bitmap width: {}", bitmap.bmWidth))
        })?;
        let height_u32 = u32::try_from(bitmap.bmHeight).map_err(|_| {
            AppError::programming_error(format!("Invalid bitmap height: {}", bitmap.bmHeight))
        })?;
        let width_usize = usize::try_from(bitmap.bmWidth).map_err(|_| {
            AppError::programming_error(format!(
                "Invalid bitmap width for usize: {}",
                bitmap.bmWidth
            ))
        })?;
        let height_usize = usize::try_from(bitmap.bmHeight).map_err(|_| {
            AppError::programming_error(format!(
                "Invalid bitmap height for usize: {}",
                bitmap.bmHeight
            ))
        })?;
        let buf_size = width_usize
            .checked_mul(height_usize)
            .and_then(|size| size.checked_mul(4))
            .ok_or_else(|| {
                AppError::programming_error(format!(
                    "Integer overflow calculating buffer size: {}x{}x4",
                    width_usize, height_usize
                ))
            })?;
        let mut buf: Vec<u8> = Vec::with_capacity(buf_size);

        let dc: windows::Win32::Graphics::Gdi::HDC = windows::Win32::Graphics::Gdi::GetDC(None);
        if dc == windows::Win32::Graphics::Gdi::HDC(std::ptr::null_mut()) {
            return Err(AppError::ImageProcessingError {
                message: "Failed to get device context".to_string(),
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
        buf.set_len(bmp.capacity());
        let result = windows::Win32::Graphics::Gdi::ReleaseDC(None, dc);
        if result != 1 {
            return Err(AppError::ImageProcessingError {
                message: "Failed to release device context".to_string(),
            });
        }
        DeleteObject(HGDIOBJ(info.hbmColor.0)).expect("Failed to delete color object");

        for chunk in bmp.chunks_exact_mut(4) {
            let [b, _, r, _] = chunk else {
                return Err(AppError::programming_error(
                    "Unexpected chunk size in pixel data - this should never happen",
                ));
            };
            mem::swap(b, r);
        }
        RgbaImage::from_vec(width_u32, height_u32, bmp).ok_or_else(|| {
            AppError::programming_error(
                "Failed to create RgbaImage from pixel data - this indicates a programming error",
            )
        })
    }

    /// 从 PNG 图像数据中裁剪掉外围的白色或透明像素
    pub fn trim_transparent_white_border(png_data: Vec<u8>) -> AppResult<Vec<u8>> {
        debug!(
            "Trimming transparent/white border from {} bytes of PNG data",
            png_data.len()
        );

        // 解析 PNG 数据
        let img =
            image::load_from_memory(&png_data).map_err(|e| AppError::ImageProcessingError {
                message: format!("Failed to load image from memory: {}", e),
            })?;

        let width = img.width();
        let height = img.height();

        debug!("Image dimensions: {}x{}", width, height);

        // 确保图像是正方形
        if width != height {
            return Err(AppError::ImageProcessingError {
                message: format!("Input image is not square: {}x{}", width, height),
            });
        }

        let mut border_width = 0;
        let size = width;

        // 从外到内一圈一圈检查
        'outer: for layer in 0..size / 2 {
            // 检查当前圈的四条边

            // 上边
            for x in layer..size - layer {
                let pixel = img.get_pixel(x, layer);
                if !Self::is_white_or_transparent(pixel) {
                    break 'outer;
                }
            }

            // 右边
            for y in layer..size - layer {
                let pixel = img.get_pixel(size - 1 - layer, y);
                if !Self::is_white_or_transparent(pixel) {
                    break 'outer;
                }
            }

            // 下边
            for x in layer..size - layer {
                let pixel = img.get_pixel(x, size - 1 - layer);
                if !Self::is_white_or_transparent(pixel) {
                    break 'outer;
                }
            }

            // 左边
            for y in layer..size - layer {
                let pixel = img.get_pixel(layer, y);
                if !Self::is_white_or_transparent(pixel) {
                    break 'outer;
                }
            }

            // 如果整个圈都是白色或透明的，增加边界宽度
            border_width = layer + 1;
        }

        // 如果整个图像都是白色或透明的，返回原图
        if border_width >= size / 2 {
            debug!("Image is mostly transparent/white, returning original");
            return Ok(png_data);
        }

        debug!(
            "Trimming border of {} pixels, new size will be {}",
            border_width,
            size - 2 * border_width
        );

        // 裁剪图像
        let new_size = size - 2 * border_width;
        let mut new_img = ImageBuffer::new(new_size, new_size);

        for y in 0..new_size {
            for x in 0..new_size {
                let pixel = img.get_pixel(x + border_width, y + border_width);
                new_img.put_pixel(x, y, pixel);
            }
        }

        // 将新图像编码为 PNG
        let mut output = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut output);
        encoder
            .write_image(
                &new_img.into_raw(),
                new_size,
                new_size,
                image::ColorType::Rgba8.into(),
            )
            .map_err(|e| AppError::ImageProcessingError {
                message: format!("Failed to encode trimmed image: {}", e),
            })?;

        debug!(
            "Successfully trimmed image to {}x{} ({} bytes)",
            new_size,
            new_size,
            output.len()
        );
        Ok(output)
    }

    /// 判断像素是否为白色或透明
    fn is_white_or_transparent(pixel: Rgba<u8>) -> bool {
        // 透明: alpha 通道接近 0
        // 白色: RGB 都接近 255 且 alpha 不透明
        pixel[3] < 10 || (pixel[0] > 245 && pixel[1] > 245 && pixel[2] > 245)
    }

    /// 将RGBA转换为PNG图像数据
    fn rgba_image_to_png(rgba_image: &RgbaImage) -> AppResult<Vec<u8>> {
        debug!(
            "Converting RgbaImage ({}x{}) to PNG",
            rgba_image.width(),
            rgba_image.height()
        );

        // 创建一个缓冲区来存储PNG数据
        let mut buffer = Vec::new();

        // 使用 Cursor 作为写入目标
        let mut cursor = Cursor::new(&mut buffer);

        // 尝试将RGBA图像编码为PNG格式
        rgba_image
            .write_to(&mut cursor, ImageFormat::Png)
            .map_err(|e| AppError::ImageProcessingError {
                message: format!("Failed to encode RGBA image as PNG: {}", e),
            })?;

        debug!(
            "Successfully converted RgbaImage to PNG ({} bytes)",
            buffer.len()
        );
        Ok(buffer)
    }

    pub async fn get_dominant_color(image_data: Vec<u8>) -> AppResult<(u8, u8, u8)> {
        debug!(
            "Analyzing dominant color from {} bytes of image data",
            image_data.len()
        );
        // 使用 spawn_blocking 将 CPU 密集型任务移到单独的线程
        tauri::async_runtime::spawn_blocking(move || -> AppResult<(u8, u8, u8)> {
            // 加载并解码PNG图片
            let img = image::load_from_memory(&image_data).map_err(|e| {
                AppError::ImageProcessingError {
                    message: format!("Failed to load image for color analysis: {}", e),
                }
            })?;
            let rgba_img = img.to_rgba8();

            debug!(
                "Loaded image for color analysis: {}x{}",
                rgba_img.width(),
                rgba_img.height()
            );

            // 提取非透明像素的RGB值 - 使用并行迭代器收集
            let pixels: Vec<[u8; 3]> = rgba_img
                .pixels()
                .par_bridge() // 并行迭代
                .filter_map(|pixel| {
                    if pixel[3] != 0 {
                        // 过滤透明像素
                        Some([pixel[0], pixel[1], pixel[2]])
                    } else {
                        None
                    }
                })
                .collect();

            if pixels.is_empty() {
                return Err(AppError::ImageProcessingError {
                    message: "No visible pixels found in image for color analysis".to_string(),
                });
            }

            debug!("Found {} visible pixels for color analysis", pixels.len());

            // 转换为Lab颜色空间 - 并行处理
            let lab_samples: Vec<Lab> = pixels
                .par_iter() // 并行迭代
                .map(|&rgb| Srgb::from(rgb).into_format::<f32>().into_color())
                .collect();

            // 参数配置
            let cluster_count = 5; // 颜色簇数量
            let max_iterations = 20; // 最大迭代次数
            let tolerance = 1.0; // 收敛阈值
            let runs = 3; // 多次运行取最佳结果
            let verbose = false; // 不输出迭代信息

            // 多次运行取最佳结果 - 并行运行
            let lab_samples_arc = Arc::new(lab_samples);
            let best_result = (0..runs)
                .into_par_iter() // 并行迭代
                .map(|_| {
                    let seed = rand::rng().random::<u64>(); // 使用线程安全的随机数生成器
                    let samples = Arc::clone(&lab_samples_arc);
                    get_kmeans(
                        cluster_count,
                        max_iterations,
                        tolerance,
                        verbose,
                        &samples,
                        seed,
                    )
                })
                .min_by(|a, b| {
                    a.score
                        .partial_cmp(&b.score)
                        .expect_programming("Score comparison should not fail")
                })
                .expect_programming("Should have at least one clustering result");

            // 统计簇分布 - 并行计数
            let cluster_counts = {
                let counts = vec![0; cluster_count];
                let counts_mutex = Arc::new(Mutex::new(counts));

                best_result.indices.par_iter().for_each(|&i| {
                    let mut counts = counts_mutex
                        .lock()
                        .expect_programming("Mutex should not be poisoned");
                    counts[i as usize] += 1;
                });

                Arc::try_unwrap(counts_mutex)
                    .expect_programming("Arc should have only one reference")
                    .into_inner()
                    .expect_programming("Mutex should not be poisoned")
            };

            // 获取最大簇的质心
            let (dominant_idx, _) = cluster_counts
                .iter()
                .enumerate()
                .max_by_key(|&(_, count)| count)
                .expect_programming("Should have at least one cluster count");

            // 转换回RGB
            let dominant_lab = best_result.centroids[dominant_idx];
            let srgb: Srgb = dominant_lab.into_color();
            let rgb = srgb.into_format::<u8>();
            let (r, g, b) = rgb.into_components();

            info!("Dominant color analysis complete: RGB({}, {}, {})", r, g, b);
            Ok((r, g, b))
        })
        .await
        .map_err(|e| {
            AppError::programming_error(format!("Task join error in color analysis: {}", e))
        })?
    }
}
