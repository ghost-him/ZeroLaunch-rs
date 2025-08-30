use std::path::Path;
use tracing::{debug, info};
use windows::Win32::UI::Shell::SHGetFolderPathW;
use windows::Win32::UI::Shell::CSIDL_STARTMENU;
use windows::Win32::UI::Shell::KF_FLAG_DEFAULT;
use windows::Win32::UI::Shell::{FOLDERID_RoamingAppData, SHGetKnownFolderPath};
use windows::Win32::UI::Shell::{CSIDL_COMMON_STARTMENU, CSIDL_DESKTOP};
/// 获取
/// 获取当前用户的桌面路径
pub fn get_desktop_path() -> Result<String, String> {
    // 创建缓冲区，足够存储路径
    const MAX_PATH_LEN: usize = 260; // MAX_PATH
    let mut desktop_path_buffer: [u16; MAX_PATH_LEN] = [0; MAX_PATH_LEN];

    unsafe {
        // 获取用户桌面路径
        let hr_desktop = SHGetFolderPathW(
            None,                     // hwndOwner, 通常为 NULL
            CSIDL_DESKTOP as i32,     // nFolder, 指定桌面文件夹
            None,                     // hToken, NULL 表示当前用户
            0,                        // dwFlags, SHGFP_TYPE_CURRENT
            &mut desktop_path_buffer, // pszPath, 输出缓冲区
        );

        if hr_desktop.is_err() {
            return Err(format!(
                "Failed to get CSIDL_DESKTOP. HRESULT: {:?}", // 以十六进制显示 HRESULT
                hr_desktop
            ));
        }

        // 将有效的宽字符缓冲区部分转换为 Rust String
        let desktop_path = widestring::U16CStr::from_ptr_str(&desktop_path_buffer as *const u16)
            .to_string()
            .map_err(|e| format!("Failed to convert common path to string: {:?}", e))?;

        debug!("用户桌面路径： {}", desktop_path);
        Ok(desktop_path)
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

        debug!("菜单路径： {common_path}, {user_path}");
        Ok((common_path, user_path))
    }
}
// 获取数据目录的路径
pub fn get_default_remote_data_dir_path() -> String {
    unsafe {
        // 获取 AppData 目录
        let path = SHGetKnownFolderPath(&FOLDERID_RoamingAppData, KF_FLAG_DEFAULT, None);

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
