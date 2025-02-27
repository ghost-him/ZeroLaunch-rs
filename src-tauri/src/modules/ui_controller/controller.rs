// 这里存放所有与界面大小设置相关的函数
use crate::modules::config::{Height, Width};
use crate::ServiceLocator;

// 获得窗口的大小
pub fn get_window_size() -> (Width, Height) {
    let item_size: u32 = 62; // 一个选项高度
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();

    let app_config = runtime_config.get_app_config();
    let show_item_count = app_config.get_search_result_count();
    let scale_factor = runtime_config
        .get_window_state()
        .get_sys_window_scale_factor();
    // 搜索栏的高度，先写死，后期可调
    let search_bar_height: u32 = 57;
    // 下栏的高度，先写死，后期可调
    let footer_height: u32 = 42;

    let window_width = 1000 as f64 * scale_factor;
    let window_height =
        (item_size * show_item_count + search_bar_height + footer_height) as f64 * scale_factor;

    (window_width as Width, (window_height * 1.1) as Height)
}

// 获得窗口的原始渲染大小
pub fn get_window_render_origin() -> (Width, Height) {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();
    let window_state = runtime_config.get_window_state();
    let sys_window_width = window_state.get_sys_window_width();
    let sys_window_height = window_state.get_sys_window_height();

    let (window_width, window_height) = get_window_size();

    // 水平方向保持居中
    let window_width_margin = ((sys_window_width - window_width) / 2) as Width;

    // 垂直方向向上偏移，使用比例因子0.4（可以根据需要调整）
    // 这个比例表示窗口顶部到屏幕顶部的距离占总可用空间的比例
    let vertical_position_ratio = 0.4; // 比例因子，小于0.5会使窗口偏向上方
    let window_height_margin =
        ((sys_window_height - window_height) as f64 * vertical_position_ratio) as Height;

    (window_width_margin, window_height_margin)
}
