use std::fs;
use std::io;
use std::path::{Path, PathBuf};
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
