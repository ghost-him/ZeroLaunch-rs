use crate::get_window_render_origin;
use crate::get_window_size;
use crate::PartialUiConfig;
use crate::PhysicalSize;
/// 用于调整主窗口的位置的
use crate::ServiceLocator;
use crate::{
    recommend_footer_height, recommend_result_item_height, recommend_search_bar_height,
    recommend_window_width,
};
use device_query::DeviceQuery;
use device_query::DeviceState;
use tauri::Manager;
use tauri::PhysicalPosition;
use tracing::debug;
/// 更新当前窗口的大小与位置
pub fn update_window_size_and_position() {
    let state = ServiceLocator::get_state();
    let main_window = state
        .get_main_handle()
        .unwrap()
        .get_webview_window("main")
        .unwrap();
    let config = state.get_runtime_config().unwrap();
    let ui_config = config.get_ui_config();
    let app_config = config.get_app_config();

    // 判断一下窗口的大小是不是默认的大小，如果是，并且还是第一次启动程序，则将其变成比例式的大小
    if ui_config.is_default_window_size() && app_config.get_is_initial() {
        // 如果什么都没变，说明用户是第一次启动这个软件，则可以使用自适应窗口大小来优化显示
        let mut update_config = PartialUiConfig::default();
        update_config.search_bar_height = Some(recommend_search_bar_height() as u32);
        update_config.result_item_height = Some(recommend_result_item_height() as u32);
        update_config.footer_height = Some(recommend_footer_height() as u32);
        update_config.window_width = Some(recommend_window_width() as u32);
        ui_config.update(update_config);
    }

    let window_size = get_window_size();
    main_window
        .set_size(PhysicalSize::new(
            window_size.0 as u32,
            window_size.1 as u32,
        ))
        .unwrap();

    if app_config.get_is_enable_drag_window() {
        let position = app_config.get_window_position();
        // 如果是读取之前的存储位置，则需要先判断一下目标的位置是不是在窗口内
        let windows = main_window.available_monitors().unwrap();
        if !windows.iter().any(|window| {
            // 对每个窗口作判断
            let window_position = window.position();
            let size = window.size();

            // 检查鼠标坐标是否在显示器边界内
            return position.0 >= window_position.x
                && position.0 < (window_position.x + size.width as i32)
                && position.1 >= window_position.y
                && position.1 < (window_position.y + size.height as i32);
        }) {
            debug!("当前没有一个窗口符合目前条件");
            return;
        }
        // 如果存在一个窗口符合条件，则设置位置
        main_window
            .set_position(PhysicalPosition::new(position.0, position.1))
            .unwrap();
        return;
    }

    // 要么没有设置成窗口自定义位置，要么窗口的位置不合条件，则进入这个选项
    let vertical_position_ratio = ui_config.get_vertical_position_ratio();
    let mut show_position = get_window_render_origin(vertical_position_ratio);
    debug!(
        "修正前的唤醒的位置: {} {}",
        show_position.0, show_position.1
    );
    // 如果设置了窗口跟随鼠标，则要重新计算新的显示的位置
    if app_config.get_show_pos_follow_mouse() {
        debug!("进入判断");
        let device_state = DeviceState::new();
        let mouse_state = device_state.get_mouse();
        let mouse_position = mouse_state.coords;
        debug!("当前鼠标的位置：{}, {}", mouse_position.0, mouse_position.1);
        let windows = main_window.available_monitors().unwrap();
        let mut target_window_pos = (0, 0);
        windows.iter().any(|window| {
            let window_position = window.position();
            let size = window.size();
            debug!("窗口的位置：{} {}", window_position.x, window_position.y);
            if mouse_position.0 >= window_position.x
                && mouse_position.0 < (window_position.x + size.width as i32)
                && mouse_position.1 >= window_position.y
                && mouse_position.1 < (window_position.y + size.height as i32)
            {
                // 如果鼠标在这个窗口中
                target_window_pos = (window_position.x, window_position.y);
                debug!("找到了鼠标所在的窗口");
                return true;
            }
            return false;
        });
        show_position.0 += target_window_pos.0 as usize;
        show_position.1 += target_window_pos.1 as usize;
        debug!(
            "经过修正后的唤醒的位置：{} {}",
            show_position.0, show_position.1
        );
    }

    main_window
        .set_position(PhysicalPosition::new(
            show_position.0 as u32,
            show_position.1 as u32,
        ))
        .unwrap();
}
