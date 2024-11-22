use crate::utils::get_u16_vec;
use base64::prelude::*;
use core::mem::MaybeUninit;
/// 这个类主要用于加载程序的图片，支持并发查询
use dashmap::DashMap;
use image::codecs::png::PngEncoder;
use image::DynamicImage;
use image::ImageFormat;
use image::ImageReader;
use image::RgbaImage;
use std::ffi::c_void;
use std::io::Cursor;
use std::mem;
use std::path::Path;
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
pub struct ImageLoader {
    dash_map: DashMap<u64, String>,
}

impl ImageLoader {
    /// 新建一个
    pub fn new() -> ImageLoader {
        ImageLoader {
            dash_map: DashMap::new(),
        }
    }
    /// 加载一个图片
    pub fn load_image(&self, program_guid: &u64, icon_path: &str) -> String {
        if let Some(cached_image) = self.dash_map.get(program_guid) {
            return cached_image.clone();
        }
        let pic_base64 = self.load_image_from_path(icon_path);
        self.dash_map.insert(*program_guid, pic_base64.clone());
        pic_base64
    }
    /// 使用路径加载一个图片
    fn load_image_from_path(&self, icon_path: &str) -> String {
        // 读取程序图标
        let mut img: Option<Vec<u8>> = None;
        if self.is_program(icon_path) {
            //使用windows 系统调用读
            let hicon = self.extract_icon_from_file(icon_path);
            if hicon.is_some() {
                // 将其变成png图片
                let raba = unsafe { self.convert_icon_to_image(hicon.unwrap()) };
                unsafe { DestroyIcon(hicon.unwrap()).unwrap() };
                img = self.rgba_image_to_png(&raba);
            }
        } else {
            // 直接使用库来读
            img = if let Ok(result) = self.load_and_convert_to_png(icon_path) {
                Some(result)
            } else {
                None
            };
        }
        // 如果有内容，就编码成base64
        if let Some(image) = img {
            return BASE64_STANDARD.encode(image);
        }
        // 如果没有内容，就使用默认的编码
        "".to_string()
    }

    /// 判断是不是一个程序的图标
    fn is_program(&self, path: &str) -> bool {
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
    fn extract_icon_from_file(&self, file_path: &str) -> Option<HICON> {
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
                println!("Successfully extracted icon from: {}", file_path);
                return Some(sh_file_info.hIcon);
            } else {
                println!("Failed to extract valid icon from: {}", file_path);
            }
        } else {
            println!("SHGetFileInfoW failed for: {}", file_path);
        }
        None
    }
    /// 将icon图像变成raga图像
    unsafe fn convert_icon_to_image(&self, icon: HICON) -> RgbaImage {
        let bitmap_size_i32 = i32::try_from(mem::size_of::<BITMAP>()).unwrap();
        let biheader_size_u32 = u32::try_from(mem::size_of::<BITMAPINFOHEADER>()).unwrap();
        let mut info = ICONINFO {
            fIcon: windows::Win32::Foundation::BOOL(0),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: std::mem::zeroed::<HBITMAP>() as HBITMAP,
            hbmColor: std::mem::zeroed::<HBITMAP>() as HBITMAP,
        };
        println!("{:?} is valid: {}", icon.0, !icon.is_invalid());
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

        let dc: windows::Win32::Graphics::Gdi::HDC = windows::Win32::Graphics::Gdi::GetDC(
            windows::Win32::Foundation::HWND(std::ptr::null_mut()),
        );
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
        let result = windows::Win32::Graphics::Gdi::ReleaseDC(
            windows::Win32::Foundation::HWND(std::ptr::null_mut()),
            dc,
        );
        assert!(result == 1);
        DeleteObject(HGDIOBJ(info.hbmColor.0)).unwrap();

        for chunk in bmp.chunks_exact_mut(4) {
            let [b, _, r, _] = chunk else { unreachable!() };
            mem::swap(b, r);
        }
        RgbaImage::from_vec(width_u32, height_u32, bmp).unwrap()
    }
    /// 将RGBA转换为PNG图像数据
    fn rgba_image_to_png(&self, rgba_image: &RgbaImage) -> Option<Vec<u8>> {
        // 创建一个缓冲区来存储PNG数据
        let mut buffer = Vec::new();

        // 使用 Cursor 作为写入目标
        let mut cursor = Cursor::new(&mut buffer);
        // 尝试将RGBA图像编码为PNG格式
        match rgba_image.write_to(&mut cursor, ImageFormat::Png) {
            Ok(_) => Some(buffer),
            Err(e) => {
                eprintln!("PNG编码失败: {}", e);
                None
            }
        }
    }

    /// 读取图片并将其返回成png图片
    fn load_and_convert_to_png<P: AsRef<Path>>(
        &self,
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
}
