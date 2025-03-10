/// 这里存放可能会使用到的函数
///
use super::super::utils::service_locator::ServiceLocator;
use crate::core::storage::storage_manager;
use crate::core::storage::windows_utils::get_default_remote_data_dir_path;
use crate::modules::version_checker::version_checker::VersionChecker;
use std::path::Path;

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
