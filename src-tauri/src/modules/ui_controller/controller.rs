// 这里存放所有与界面大小设置相关的函数
use crate::modules::config::{Height, Width};
use crate::ServiceLocator;
// 获得一个选项的大小
pub fn get_item_size() -> (Width, Height) {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();

    let ui_config = runtime_config.get_ui_config();
    let item_width_scale_factor = ui_config.get_item_width_scale_factor();
    let item_height_scale_factor = ui_config.get_item_height_scale_factor();

    let window_state = runtime_config.get_window_state();
    let sys_window_width = window_state.get_sys_window_width();
    let sys_window_height = window_state.get_sys_window_height();

    (
        (sys_window_width as f64 * item_width_scale_factor) as Width,
        (sys_window_height as f64 * item_height_scale_factor) as Height,
    )
}
// 获得窗口的大小
pub fn get_window_size() -> (Width, Height) {
    let item_size = get_item_size();
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();

    let app_config = runtime_config.get_app_config();
    let show_item_count = app_config.get_search_result_count();
    (item_size.0, item_size.1 * (show_item_count as usize + 2))
}

// 获得窗口的原始渲染大小
pub fn get_window_render_origin() -> (Width, Height) {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();
    let window_state = runtime_config.get_window_state();
    let sys_window_width = window_state.get_sys_window_width();
    let sys_window_height = window_state.get_sys_window_height();

    let (window_width, window_height) = get_window_size();
    let window_width_margin = (sys_window_width - window_width) / 2 as Width;
    let window_height_margin = (sys_window_height - window_height) / 2 as Height;
    (window_width_margin, window_height_margin)
}
