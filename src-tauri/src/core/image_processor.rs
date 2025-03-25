use crate::utils::defer::defer;
use crate::utils::windows::get_u16_vec;
use core::mem::MaybeUninit;
use image::codecs::png::PngEncoder;
use image::DynamicImage;
use image::GenericImageView;
use image::ImageBuffer;
use image::ImageEncoder;
use image::ImageFormat;
use image::ImageReader;
use image::Rgba;
use image::RgbaImage;
use kmeans_colors::get_kmeans;
use palette::{IntoColor, Lab, Srgb};
use rand::Rng;
use rayon::prelude::*;
use std::ffi::c_void;
use std::io::Cursor;
use std::mem;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::warn;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Graphics::Gdi::BITMAP;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetBitmapBits, GetObjectW, BITMAPINFOHEADER, BI_RGB, HBITMAP, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::UI::Shell::SHFILEINFOW;
use windows::Win32::UI::Shell::{
    SHGetFileInfoW, SHGFI_ADDOVERLAYS, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES,
};
use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};
use windows_core::BOOL;
use windows_core::PCWSTR;
pub struct ImageProcessor {}

impl ImageProcessor {
    pub async fn load_image_from_path(icon_path: String) -> Vec<u8> {
        // 读取程序图标
        let mut img: Option<Vec<u8>> = None;
        if Self::is_program(&icon_path) {
            //使用windows 系统调用读
            if let Some(raba) = Self::extract_icon_from_file(icon_path).await {
                img = Self::rgba_image_to_png(&raba);
            }
        } else {
            // 直接使用库来读
            img = if let Ok(result) = Self::load_and_convert_to_png(icon_path).await {
                Some(result)
            } else {
                None
            };
        }
        // 如果有内容，就返回
        if img.is_some() {
            return img.unwrap();
        }
        // 如果没有内容，就使用默认的编码
        return vec![];
    }

    // 读取本地图片，并将其转化为png图片
    async fn load_and_convert_to_png<P: AsRef<Path>>(
        file_path: P,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // 异步读取文件内容
        let file_content = tokio::fs::read(file_path.as_ref()).await?;

        // 在阻塞线程中处理图像
        tauri::async_runtime::spawn_blocking(move || {
            // 创建带有格式猜测的图像读取器
            let img_reader = ImageReader::new(Cursor::new(file_content)).with_guessed_format()?;

            // 读取图像格式
            let format = img_reader.format().ok_or("无法检测格式")?;
            let mut img = img_reader.decode()?;

            // 如果不是 PNG 格式，则转换
            if format != ImageFormat::Png {
                img = DynamicImage::ImageRgba8(img.to_rgba8());
            }

            // 使用 PNG 编码器将图像写入 Vec<u8>
            let mut png_data = Vec::new();
            let encoder = PngEncoder::new(&mut png_data);
            img.write_with_encoder(encoder)?;

            Ok(png_data)
        })
        .await?
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
        return false;
    }

    /// 从文件提取hicon
    async fn extract_icon_from_file(file_path: String) -> Option<RgbaImage> {
        tauri::async_runtime::spawn_blocking(move || {
            let com_init = unsafe {
                windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_MULTITHREADED
                        | windows::Win32::System::Com::COINIT_DISABLE_OLE1DDE
                        | windows::Win32::System::Com::COINIT_SPEED_OVER_MEMORY,
                )
            };
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
                    SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES | SHGFI_ADDOVERLAYS,
                )
            };
            // 提前返回 None 如果无法获取图标
            if result == 0 || sh_file_info.hIcon.is_invalid() {
                let last_error = unsafe { GetLastError() };
                if result == 0 {
                    warn!("SHGetFileInfoW failed for: {:?}", last_error);
                } else {
                    warn!(
                        "Failed to extract valid icon from: {:?}, error: {:?}",
                        file_path, last_error
                    );
                }
                return None;
            }
            let hicon = sh_file_info.hIcon;

            let result = unsafe {
                // 将 HICON 传递给转换函数，并确保在转换完成后才销毁
                let image = Self::convert_icon_to_image(hicon);
                DestroyIcon(hicon).unwrap(); // 显式销毁
                image
            };
            Some(result)
        })
        .await
        .unwrap()
    }
    /// 将icon图像变成raga图像
    unsafe fn convert_icon_to_image(icon: HICON) -> RgbaImage {
        let bitmap_size_i32 = i32::try_from(mem::size_of::<BITMAP>()).unwrap();
        let biheader_size_u32 = u32::try_from(mem::size_of::<BITMAPINFOHEADER>()).unwrap();
        let mut info = ICONINFO {
            fIcon: BOOL(0),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: std::mem::zeroed::<HBITMAP>() as HBITMAP,
            hbmColor: std::mem::zeroed::<HBITMAP>() as HBITMAP,
        };
        GetIconInfo(icon, &mut info).unwrap();
        DeleteObject(HGDIOBJ(info.hbmMask.0)).unwrap();
        let mut bitmap: MaybeUninit<BITMAP> = MaybeUninit::uninit();

        let result = GetObjectW(
            HGDIOBJ(info.hbmColor.0),
            bitmap_size_i32,
            Some(bitmap.as_mut_ptr() as *mut c_void),
        );

        assert!(result == bitmap_size_i32);
        let bitmap = bitmap.assume_init_ref();

        let width_u32 = u32::try_from(bitmap.bmWidth).unwrap();
        let height_u32 = u32::try_from(bitmap.bmHeight).unwrap();
        let width_usize = usize::try_from(bitmap.bmWidth).unwrap();
        let height_usize = usize::try_from(bitmap.bmHeight).unwrap();
        let buf_size = width_usize
            .checked_mul(height_usize)
            .and_then(|size| size.checked_mul(4))
            .unwrap();
        let mut buf: Vec<u8> = Vec::with_capacity(buf_size);

        let dc: windows::Win32::Graphics::Gdi::HDC = windows::Win32::Graphics::Gdi::GetDC(None);
        assert!(dc != windows::Win32::Graphics::Gdi::HDC(std::ptr::null_mut()));

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
        assert!(result == 1);
        DeleteObject(HGDIOBJ(info.hbmColor.0)).unwrap();

        for chunk in bmp.chunks_exact_mut(4) {
            let [b, _, r, _] = chunk else { unreachable!() };
            mem::swap(b, r);
        }
        RgbaImage::from_vec(width_u32, height_u32, bmp).unwrap()
    }

    /// 从 PNG 图像数据中裁剪掉外围的白色或透明像素
    pub fn trim_transparent_white_border(png_data: Vec<u8>) -> Result<Vec<u8>, String> {
        // 解析 PNG 数据
        let img = image::load_from_memory(&png_data).map_err(|e| format!("无法加载图像: {}", e))?;

        let width = img.width();
        let height = img.height();

        // 确保图像是正方形
        if width != height {
            return Err("输入图像不是正方形".to_string());
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
            return Ok(png_data);
        }

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
            .map_err(|e| format!("无法编码图像: {}", e))?;

        Ok(output)
    }

    /// 判断像素是否为白色或透明
    fn is_white_or_transparent(pixel: Rgba<u8>) -> bool {
        // 透明: alpha 通道接近 0
        // 白色: RGB 都接近 255 且 alpha 不透明
        pixel[3] < 10 || (pixel[0] > 245 && pixel[1] > 245 && pixel[2] > 245)
    }

    /// 将RGBA转换为PNG图像数据
    fn rgba_image_to_png(rgba_image: &RgbaImage) -> Option<Vec<u8>> {
        // 创建一个缓冲区来存储PNG数据
        let mut buffer = Vec::new();

        // 使用 Cursor 作为写入目标
        let mut cursor = Cursor::new(&mut buffer);

        // 尝试将RGBA图像编码为PNG格式
        match rgba_image.write_to(&mut cursor, ImageFormat::Png) {
            Ok(_) => Some(buffer),
            Err(e) => {
                warn!("PNG编码失败: {}", e);
                None
            }
        }
    }

    pub async fn get_dominant_color(
        image_data: Vec<u8>,
    ) -> Result<(u8, u8, u8), Box<dyn std::error::Error + Send + Sync>> {
        // 使用 spawn_blocking 将 CPU 密集型任务移到单独的线程
        tauri::async_runtime::spawn_blocking(move || {
            // 加载并解码PNG图片
            let img = image::load_from_memory(&image_data)?;
            let rgba_img = img.to_rgba8();

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
                return Err("No visible pixels in image".into());
            }

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
                .min_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
                .unwrap();

            // 统计簇分布 - 并行计数
            let cluster_counts = {
                let counts = vec![0; cluster_count];
                let counts_mutex = Arc::new(Mutex::new(counts));

                best_result.indices.par_iter().for_each(|&i| {
                    let mut counts = counts_mutex.lock().unwrap();
                    counts[i as usize] += 1;
                });

                Arc::try_unwrap(counts_mutex).unwrap().into_inner().unwrap()
            };

            // 获取最大簇的质心
            let (dominant_idx, _) = cluster_counts
                .iter()
                .enumerate()
                .max_by_key(|&(_, count)| count)
                .unwrap();

            // 转换回RGB
            let dominant_lab = best_result.centroids[dominant_idx];
            let srgb: Srgb = dominant_lab.into_color();
            let rgb = srgb.into_format::<u8>();

            Ok(rgb.into_components())
        })
        .await?
    }
}
