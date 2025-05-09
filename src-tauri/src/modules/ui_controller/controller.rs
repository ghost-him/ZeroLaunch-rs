use tokio::runtime;

// 这里存放所有与界面大小设置相关的函数
use crate::modules::config::{Height, Width};
use crate::ServiceLocator;

//    3840px / 1.5 = 2560 逻辑像素宽度
//    2160px / 1.5 = 1440 逻辑像素高度
const REF_LOGICAL_SCREEN_WIDTH: f64 = 2560.0;
const REF_LOGICAL_SCREEN_HEIGHT: f64 = 1440.0;

// 在参考逻辑屏幕尺寸下，各元素的“目标”逻辑尺寸
const TARGET_WINDOW_LOGICAL_WIDTH_AT_REF: f64 = 800.0;
const TARGET_SEARCH_BAR_LOGICAL_HEIGHT_AT_REF: f64 = 65.0;
const TARGET_RESULT_ITEM_LOGICAL_HEIGHT_AT_REF: f64 = 62.0;
const TARGET_FOOTER_LOGICAL_HEIGHT_AT_REF: f64 = 42.0;

// 各元素的最小和最大逻辑尺寸约束
const MIN_WINDOW_LOGICAL_WIDTH: f64 = 500.0;
const MAX_WINDOW_LOGICAL_WIDTH: f64 = 1200.0;

const MIN_SEARCH_BAR_LOGICAL_HEIGHT: f64 = 48.0;
const MAX_SEARCH_BAR_LOGICAL_HEIGHT: f64 = 80.0;

const MIN_RESULT_ITEM_LOGICAL_HEIGHT: f64 = 44.0;
const MAX_RESULT_ITEM_LOGICAL_HEIGHT: f64 = 75.0;

const MIN_FOOTER_LOGICAL_HEIGHT: f64 = 30.0;
const MAX_FOOTER_LOGICAL_HEIGHT: f64 = 55.0;

// 辅助函数：计算推荐的逻辑尺寸，应用比例缩放并进行钳位(clamp)
fn calculate_recommended_dim(
    current_screen_logical_dim: f64, // 当前屏幕的逻辑宽度或高度
    ref_screen_logical_dim: f64,     // 参考屏幕的逻辑宽度或高度
    target_dim_at_ref: f64,          // 元素在参考屏幕上的目标逻辑尺寸
    min_dim: f64,                    // 元素的最小逻辑尺寸
    max_dim: f64,                    // 元素的最大逻辑尺寸
) -> f64 {
    if ref_screen_logical_dim <= 0.0 {
        // 防止除以零
        return target_dim_at_ref.clamp(min_dim, max_dim);
    }

    // 按比例计算目标尺寸
    let proportion = current_screen_logical_dim / ref_screen_logical_dim;
    let calculated_dim = target_dim_at_ref * proportion;

    // 将计算结果限制在最小和最大尺寸之间
    calculated_dim.clamp(min_dim, max_dim)
}

pub fn recommend_window_width() -> Width {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();
    let window_state = runtime_config.get_window_state();

    let logical_width =
        window_state.get_sys_window_width() as f64 / window_state.get_sys_window_scale_factor();
    let width = calculate_recommended_dim(
        logical_width,
        REF_LOGICAL_SCREEN_WIDTH,
        TARGET_WINDOW_LOGICAL_WIDTH_AT_REF,
        MIN_WINDOW_LOGICAL_WIDTH,
        MAX_WINDOW_LOGICAL_WIDTH,
    );
    width.round() as Width
}

pub fn recommend_search_bar_height() -> Height {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();
    let window_state = runtime_config.get_window_state();

    let logical_height =
        window_state.get_sys_window_height() as f64 / window_state.get_sys_window_scale_factor();
    let height = calculate_recommended_dim(
        logical_height,
        REF_LOGICAL_SCREEN_HEIGHT,
        TARGET_SEARCH_BAR_LOGICAL_HEIGHT_AT_REF,
        MIN_SEARCH_BAR_LOGICAL_HEIGHT,
        MAX_SEARCH_BAR_LOGICAL_HEIGHT,
    );
    height.round() as Height
}

pub fn recommend_result_item_height() -> Height {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();
    let window_state = runtime_config.get_window_state();
    let logical_height =
        window_state.get_sys_window_height() as f64 / window_state.get_sys_window_scale_factor();
    let height = calculate_recommended_dim(
        logical_height,
        REF_LOGICAL_SCREEN_HEIGHT,
        TARGET_RESULT_ITEM_LOGICAL_HEIGHT_AT_REF,
        MIN_RESULT_ITEM_LOGICAL_HEIGHT,
        MAX_RESULT_ITEM_LOGICAL_HEIGHT,
    );
    height.round() as Height
}

pub fn recommend_footer_height() -> Height {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();
    let window_state = runtime_config.get_window_state();
    let logical_height =
        window_state.get_sys_window_height() as f64 / window_state.get_sys_window_scale_factor();
    let height = calculate_recommended_dim(
        logical_height,
        REF_LOGICAL_SCREEN_HEIGHT,
        TARGET_FOOTER_LOGICAL_HEIGHT_AT_REF,
        MIN_FOOTER_LOGICAL_HEIGHT,
        MAX_FOOTER_LOGICAL_HEIGHT,
    );
    height.round() as Height
}

// 获得窗口的大小
pub fn get_window_size() -> (Width, Height) {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();

    let app_config = runtime_config.get_app_config();
    let ui_config = runtime_config.get_ui_config();
    let show_item_count = app_config.get_search_result_count();
    let scale_factor = runtime_config
        .get_window_state()
        .get_sys_window_scale_factor();
    // 结果栏一项的高度
    let item_size = ui_config.get_result_item_height();
    // 搜索栏的高度
    let search_bar_height = ui_config.get_search_bar_height();
    // 下栏的高度
    let footer_height: u32 = ui_config.get_footer_height();
    // 窗口的宽度
    let window_width: f64 = ui_config.get_window_width() as f64 * scale_factor;
    let window_height =
        (item_size * show_item_count + search_bar_height + footer_height) as f64 * scale_factor;

    (window_width as Width, (window_height) as Height)
}

// 获得窗口的原始渲染大小
// 参数：vertical_position_ratio
// 垂直方向向上偏移，使用比例因子0.4（可以根据需要调整）
// 这个比例表示窗口顶部到屏幕顶部的距离占总可用空间的比例
// 比例因子，小于0.5会使窗口偏向上方
pub fn get_window_render_origin(vertical_position_ratio: f64) -> (Width, Height) {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();
    let window_state = runtime_config.get_window_state();
    let sys_window_width = window_state.get_sys_window_width();
    let sys_window_height = window_state.get_sys_window_height();

    let (window_width, window_height) = get_window_size();

    // 水平方向保持居中
    let window_width_margin = ((sys_window_width - window_width) / 2) as Width;

    let window_height_margin =
        ((sys_window_height - window_height) as f64 * vertical_position_ratio) as Height;

    (window_width_margin, window_height_margin)
}
