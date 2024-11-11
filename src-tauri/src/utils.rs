use crate::config::RuntimeConfig;
use crate::singleton::Singleton;

use crate::program_manager::PROGRAM_MANAGER;
use serde::Serialize;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{Manager, Runtime};
use windows::core::PWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::SHGetFolderPathW;
use windows::Win32::UI::Shell::CSIDL_COMMON_STARTMENU;
use windows::Win32::UI::Shell::CSIDL_STARTMENU;

pub fn read_or_create(path: &str, content: Option<String>) -> Result<String, String> {
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

/// 获取公共和用户的开始菜单路径
pub fn get_start_menu_paths() -> Result<(String, String), String> {
    // 创建缓冲区，足够存储路径
    const MAX_PATH_LEN: usize = 260;
    let mut common_path_buffer: [u16; MAX_PATH_LEN] = [0; MAX_PATH_LEN];
    let mut user_path_buffer: [u16; MAX_PATH_LEN] = [0; MAX_PATH_LEN];

    unsafe {
        // 获取公共开始菜单路径
        let hr_common = SHGetFolderPathW(
            HWND(std::ptr::null_mut()),
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
        let hr_user = SHGetFolderPathW(
            HWND(std::ptr::null_mut()),
            CSIDL_STARTMENU as i32,
            None,
            0,
            &mut user_path_buffer,
        );

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

        Ok((common_path, user_path))
    }
}

#[tauri::command]
pub fn get_item_size() -> Vec<usize> {
    let (item_width, item_height) = RuntimeConfig::instance().lock().unwrap().get_item_size();
    println!("item {} {}", item_width, item_height);
    vec![item_width, item_height]
}

#[tauri::command]
pub fn get_window_size() -> Vec<usize> {
    let (window_width, window_height) = RuntimeConfig::instance().lock().unwrap().get_window_size();
    println!("window {} {}", window_width, window_height);
    vec![window_width, window_height]
}

#[tauri::command]
pub fn get_window_scale_factor() -> f64 {
    let result = RuntimeConfig::instance()
        .lock()
        .unwrap()
        .get_window_scale_factor();
    println!("scale factor: {}", result);
    result
}

#[derive(Serialize, Debug)]
pub struct SearchResult(u64, String);

/// 处理前端发来的消息
#[tauri::command]
pub fn handle_search_text(search_text: String) -> Vec<SearchResult> {
    // 处理消息
    let manager = PROGRAM_MANAGER.lock().unwrap();
    let results = manager.update(&search_text, 4);
    // 解锁
    drop(manager);
    let mut ret = Vec::new();
    for item in results {
        ret.push(SearchResult(item.0, item.1));
    }
    println!("{:?}", ret);
    ret
}

/// 隐藏窗口
#[tauri::command]
pub fn hide_window<R: Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    let main_window = Arc::new(app.get_webview_window("main").unwrap());
    main_window.hide().unwrap();
    Ok(())
}
