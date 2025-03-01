use crate::utils::windows::get_u16_vec;
use core::mem::MaybeUninit;
use image::codecs::png::PngEncoder;
use image::ImageError;
use image::ImageFormat;
use image::ImageReader;
use image::RgbaImage;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

use image::{DynamicImage, GenericImageView};
use kmeans_colors::{get_kmeans, Calculate, Kmeans};
use palette::{IntoColor, Lab, Srgb};
use rand::Rng;

use std::ffi::c_void;
use std::io::Cursor;
use std::mem;
use std::path::Path;
use tracing::debug;
use tracing::warn;
use windows::Win32::Graphics::Gdi::BITMAP;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetBitmapBits, GetObjectW, BITMAPINFOHEADER, BI_RGB, HBITMAP, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::UI::Shell::SHFILEINFOW;
use windows::Win32::UI::Shell::{
    SHGetFileInfoW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES,
};
use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};
use windows_core::PCWSTR;
pub struct ImageProcessor;

impl ImageProcessor {
    pub fn load_image_from_path(icon_path: &str) -> Vec<u8> {
        // 读取程序图标
        let mut img: Option<Vec<u8>> = None;
        if Self::is_program(icon_path) {
            //使用windows 系统调用读
            let hicon = Self::extract_icon_from_file(icon_path);
            if hicon.is_some() {
                // 将其变成png图片
                let raba = unsafe { Self::convert_icon_to_image(hicon.unwrap()) };
                unsafe { DestroyIcon(hicon.unwrap()).unwrap() };
                img = Self::rgba_image_to_png(&raba);
            }
        } else {
            // 直接使用库来读
            img = if let Ok(result) = Self::load_and_convert_to_png(icon_path) {
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
    fn load_and_convert_to_png<P: AsRef<Path>>(
        file_path: P,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // 打开图像文件
        let img_reader = ImageReader::open(file_path.as_ref())?;

        // 读取图像格式
        let format = img_reader.format().ok_or("无法检测格式")?;
        let mut img = img_reader.decode()?;

        let mut png_data = Vec::new();

        // 如果不是 PNG 格式，则转换
        if format != ImageFormat::Png {
            img = DynamicImage::ImageRgba8(img.to_rgba8());
        }

        // 使用 PNG 编码器将图像写入 Vec<u8>
        let encoder = PngEncoder::new(&mut png_data);
        img.write_with_encoder(encoder).unwrap();

        Ok(png_data)
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
    fn extract_icon_from_file(file_path: &str) -> Option<HICON> {
        let wide_file_path = get_u16_vec(file_path);
        let mut sh_file_info: SHFILEINFOW = unsafe { std::mem::zeroed() };
        let result = unsafe {
            SHGetFileInfoW(
                PCWSTR::from_raw(wide_file_path.as_ptr()),
                FILE_ATTRIBUTE_NORMAL,
                Some(&mut sh_file_info),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES,
            )
        };
        if result != 0 {
            if !sh_file_info.hIcon.is_invalid() {
                // 检查 hIcon 是否有效
                debug!("Successfully extracted icon from: {}", file_path);
                return Some(sh_file_info.hIcon);
            } else {
                warn!("Failed to extract valid icon from: {}", file_path);
            }
        } else {
            warn!("SHGetFileInfoW failed for: {}", file_path);
        }
        None
    }
    /// 将icon图像变成raga图像
    unsafe fn convert_icon_to_image(icon: HICON) -> RgbaImage {
        let bitmap_size_i32 = i32::try_from(mem::size_of::<BITMAP>()).unwrap();
        let biheader_size_u32 = u32::try_from(mem::size_of::<BITMAPINFOHEADER>()).unwrap();
        let mut info = ICONINFO {
            fIcon: windows::Win32::Foundation::BOOL(0),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: std::mem::zeroed::<HBITMAP>() as HBITMAP,
            hbmColor: std::mem::zeroed::<HBITMAP>() as HBITMAP,
        };
        debug!("{:?} is valid: {}", icon.0, !icon.is_invalid());
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

    pub fn get_dominant_color(
        image_data: Vec<u8>,
    ) -> Result<(u8, u8, u8), Box<dyn std::error::Error>> {
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
    }
}
