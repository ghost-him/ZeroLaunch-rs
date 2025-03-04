/// 这里存放可能会使用到的函数
///
use super::super::utils::service_locator::ServiceLocator;
use crate::modules::version_checker::version_checker::VersionChecker;
use std::path::Path;
/// 背景图片存放的地址
pub fn get_background_picture_path() -> String {
    let state = ServiceLocator::get_state();
    let remote_dir = state.get_remote_config_dir_path();
    Path::new(&remote_dir)
        .join("background.png")
        .to_str()
        .unwrap()
        .to_string()
}

/// 获得当前程序的最新版本
#[tauri::command]
pub async fn command_get_latest_release_version() -> String {
    let result = VersionChecker::get_latest_release_version().await;
    match result {
        Ok(data) => data,
        Err(e) => e.to_string(),
    }
}
