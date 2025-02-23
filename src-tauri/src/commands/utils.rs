/// 这里存放可能会使用到的函数
///
use super::super::utils::service_locator::ServiceLocator;
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
