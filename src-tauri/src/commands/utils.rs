use std::collections::HashSet;

use crate::core::storage::utils::read_str;
use crate::core::storage::windows_utils::get_default_remote_data_dir_path;
use crate::error::ResultExt;
use crate::modules::version_checker::VersionChecker;
use crate::utils::font_database::get_fonts;

/// 获得当前程序的最新版本
#[tauri::command]
pub async fn command_get_latest_release_version() -> String {
    let result = VersionChecker::get_latest_release_version().await;
    match result {
        Ok(data) => data,
        Err(e) => e.to_string(),
    }
}

/// 获得默认的远程配置文件保存地址
#[tauri::command]
pub fn command_get_default_remote_data_dir_path() -> String {
    get_default_remote_data_dir_path()
}

/// 获取当前的字体
/// 虽然后端向前端发送的是HashSet，但是 tauri 会在传输的过程中将这个变量一个普通的数组，所以前端可以使用string[]来接收
#[tauri::command]
pub fn command_get_system_fonts() -> HashSet<String> {
    get_fonts()
}

#[tauri::command]
pub async fn command_read_file(path: String) -> String {
    let result = tauri::async_runtime::spawn_blocking(move || {
        // 在这里可以安全地执行阻塞代码
        read_str(&path).expect_programming(&format!("读取当前文件失败：{}", &path))
    })
    .await;
    result.expect_programming("读取文件失败")
}
