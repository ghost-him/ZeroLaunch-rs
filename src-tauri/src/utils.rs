use chrono::{Local, NaiveDate};
use dashmap::DashMap;
use image::codecs::png::PngEncoder;
use image::DynamicImage;
use image::ImageFormat;
use image::ImageReader;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::hash::Hash;
use std::io;
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use tracing::warn;
use tracing::{debug, info};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::SHGetFolderPathW;
use windows::Win32::UI::Shell::CSIDL_COMMON_STARTMENU;
use windows::Win32::UI::Shell::CSIDL_STARTMENU;
use windows::Win32::UI::Shell::KF_FLAG_DEFAULT;
use windows::Win32::UI::Shell::{FOLDERID_RoamingAppData, SHGetKnownFolderPath};

use crate::config::get_background_picture_path;
pub fn read_or_create_str(path: &str, content: Option<String>) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(data) => Ok(data),
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                if let Some(parent) = Path::new(path).parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        return Err(format!("无法创建文件夹: {}", e));
                    }
                }
                let initial_content = content.unwrap_or("".to_string());
                match fs::write(path, initial_content.clone()) {
                    Ok(_) => Ok(initial_content),
                    Err(write_err) => Err(format!("无法写入文件: {}", write_err)),
                }
            } else {
                Err(format!("无法读取： {}", e))
            }
        }
    }
}

pub fn read_or_create_bytes(path: &str, content: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
    match fs::read(path) {
        Ok(data) => Ok(data),
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                if let Some(parent) = Path::new(path).parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        return Err(format!("无法创建文件夹: {}", e));
                    }
                }
                let initial_content = content.unwrap_or_else(Vec::new);
                match fs::write(path, &initial_content) {
                    Ok(_) => Ok(initial_content),
                    Err(write_err) => Err(format!("无法写入文件: {}", write_err)),
                }
            } else {
                Err(format!("无法读取文件: {}", e))
            }
        }
    }
}

/// 获取公共和用户的开始菜单路径
pub fn get_start_menu_paths() -> Result<(String, String), String> {
    // 创建缓冲区，足够存储路径
    const MAX_PATH_LEN: usize = 260;
    let mut common_path_buffer: [u16; MAX_PATH_LEN] = [0; MAX_PATH_LEN];
    let mut user_path_buffer: [u16; MAX_PATH_LEN] = [0; MAX_PATH_LEN];

    unsafe {
        // 获取公共开始菜单路径
        let hr_common = SHGetFolderPathW(
            None,
            CSIDL_COMMON_STARTMENU as i32,
            None,
            0,
            &mut common_path_buffer,
        );

        if hr_common.is_err() {
            return Err(format!(
                "Failed to get CSIDL_COMMON_STARTMENU: {:?}",
                hr_common
            ));
        }

        // 获取用户开始菜单路径
        let hr_user =
            SHGetFolderPathW(None, CSIDL_STARTMENU as i32, None, 0, &mut user_path_buffer);

        if hr_user.is_err() {
            return Err(format!("Failed to get CSIDL_STARTMENU: {:?}", hr_user));
        }

        // 将宽字符缓冲区转换为 Rust String
        let common_path = widestring::U16CStr::from_ptr_str(&common_path_buffer as *const u16)
            .to_string()
            .map_err(|e| format!("Failed to convert common path to string: {:?}", e))?;

        let user_path = widestring::U16CStr::from_ptr_str(&user_path_buffer as *const u16)
            .to_string()
            .map_err(|e| format!("Failed to convert user path to string: {:?}", e))?;

        debug!("自动生成路径： {common_path}, {user_path}");
        Ok((common_path, user_path))
    }
}

pub fn get_data_dir_path() -> String {
    unsafe {
        // 获取 AppData 目录
        let path = SHGetKnownFolderPath(&FOLDERID_RoamingAppData, KF_FLAG_DEFAULT.into(), None);

        // 将 PWSTR 转换为 Rust 字符串
        let path_str = path.unwrap().to_string().unwrap();
        let app_data_str = Path::new(&path_str)
            .join("ZeroLaunch-rs")
            .to_str()
            .unwrap()
            .to_string();
        info!("AppData Directory: {}", app_data_str);
        app_data_str
    }
}

/// 将一个字符串转成windows的宽字符
pub fn get_u16_vec<P: AsRef<Path>>(path: P) -> Vec<u16> {
    path.as_ref()
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// 生成当前日期的函数
pub fn generate_current_date() -> String {
    let current_date = Local::now().date_naive();
    current_date.format("%Y-%m-%d").to_string()
}

/// 比较日期字符串与当前日期的函数
pub fn is_date_current(date_str: &str) -> bool {
    // 解析输入的日期字符串
    let input_date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => return false, // 如果解析失败,返回false
    };

    // 获取当前日期
    let current_date = Local::now().date_naive();

    // 比较两个日期
    input_date == current_date
}

// 将 DashMap 转换为 HashMap
pub fn dashmap_to_hashmap<K, V>(dash_map: &DashMap<K, V>) -> HashMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    dash_map
        .iter()
        .map(|r| (r.key().clone(), r.value().clone()))
        .collect()
}

// 将 HashMap 转换为 DashMap
pub fn hashmap_to_dashmap<K, V>(hash_map: &HashMap<K, V>) -> DashMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    let dash_map = DashMap::with_capacity(hash_map.len());
    for (key, value) in hash_map {
        dash_map.insert(key.clone(), value.clone());
    }
    dash_map
}

// 将一个图片复制到指定的位置
pub fn copy_background_picture(file: String) -> Result<(), String> {
    let mut content: Vec<u8> = Vec::new();

    if !file.is_empty() {
        // 尝试打开图像文件
        let img_reader = match ImageReader::open(&file) {
            Ok(reader) => reader,
            Err(e) => {
                warn!("{}", e.to_string());
                return Err(e.to_string());
            }
        };

        // 尝试读取图像格式
        let format = match img_reader.format() {
            Some(fmt) => fmt,
            None => {
                warn!("读取图片格式失败");
                return Err("加载图片失败".to_string());
            }
        };

        // 尝试解码图像
        let mut img = match img_reader.decode() {
            Ok(image) => image,
            Err(e) => {
                warn!("{}", e.to_string());
                return Err(e.to_string());
            }
        };

        // 如果不是 PNG 格式，则转换为 RGBA8
        if format != ImageFormat::Png {
            img = DynamicImage::ImageRgba8(img.to_rgba8());
        }

        // 尝试使用 PNG 编码器将图像写入 Vec<u8>
        let encoder = PngEncoder::new(&mut content);
        if img.write_with_encoder(encoder).is_err() {
            // 编码失败，保持 content 为空
            content.clear();
        }
    }

    let target_path = get_background_picture_path();

    if let Ok(mut file) = File::create(target_path) {
        // 将所有字节写入文件
        let _ = file.write_all(&content);
    }

    Ok(())
}

pub fn get_background_picture() -> Result<Vec<u8>, String> {
    let target_path = get_background_picture_path();
    read_or_create_bytes(&target_path, None)
}

pub fn is_writable_directory(path: &str) -> bool {
    let path = Path::new(path);

    // 检查路径是否存在且是一个目录
    if !path.is_dir() {
        return false;
    }

    // 尝试在目录中创建一个临时文件
    let temp_file_path = path.join("temp_test_file.txt");
    match fs::write(&temp_file_path, "test content") {
        Ok(_) => {
            // 如果成功创建文件，尝试修改它
            match fs::write(&temp_file_path, "modified content") {
                Ok(_) => {
                    // 清理：删除临时文件
                    if let Err(e) = fs::remove_file(&temp_file_path) {
                        eprintln!("警告：无法删除临时文件: {}", e);
                    }
                    true
                }
                Err(_) => {
                    // 清理：尝试删除临时文件
                    let _ = fs::remove_file(&temp_file_path);
                    false
                }
            }
        }
        Err(_) => false,
    }
}
