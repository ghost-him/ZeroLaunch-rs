use serde::{Deserialize, Serialize};
use tauri::async_runtime::TokioRuntime;

use utils::read_file;
use windows::Win32::System::WindowsProgramming::COPY_FILE_REQUEST_SECURITY_PRIVILEGES;

use crate::utils::read_file;

type Weight = usize;
type Height = usize;

/// 配置文件存在的位置
const CONFIG_PATH: String = "./config.js".to_string();

/// 与程序设置有关的，比如是不是要开机自动启动等
#[derive(Serialize, Deserialize, Debug)]
struct AppConfig {}
/// 与程序页面设置有关的，比如窗口的大小，显示的界面等
#[derive(Serialize, Deserialize, Debug)]
struct UiConfig {
    /// 显示器的大小与窗口的大小的比例
    /// 窗口的高的比例
    window_height_scale_factor: f64,
    /// 窗口的宽度的比例
    window_weight_scale_factor: f64,
}

/// 综合
#[derive(Serialize, Deserialize, Debug)]
struct Config {
    app_config: AppConfig,
    ui_config: UiConfig,
}

impl UiConfig {
    fn new() -> Self {
        UiConfig {
            window_height_scale_factor: 0.5,
            window_weight_scale_factor: 0.33,
        }
    }

    fn get_window_size(
        &self,
        sys_window_weight: Weight,
        sys_window_height: Height,
    ) -> (weight, height) {
        (
            sys_window_weight * self.window_weight_scale_factor,
            sys_window_height * self.window_height_scale_factor,
        )
    }
    fn get_window_render_origin(
        &self,
        sys_window_weight: Weight,
        sys_window_height: Height,
    ) -> (weight, height) {
        let (window_weight, window_height) =
            self.get_window_size(sys_window_weight, sys_window_height);
        let window_weight_margin = (sys_window_weight - window_weight) / 2;
        let window_height_margin = (sys_window_height - window_height) / 2;
        (window_weight_margin, window_height_margin)
    }
}

impl config {
    fn new() {}
}

/// 运行时确定的
struct RuntimeConfig {
    /// 显示器的宽
    sys_window_weight: Weight,
    /// 显示器的长
    sys_window_height: Height,
    /// 配置文件
    config: Config,
}

impl RuntimeConfig {
    fn new() -> RuntimeConfig {
        // 读取配置文件
        let config_content = read_file(CONFIG_PATH);
        config_content = read_file(CONFIG_PATH);
        RuntimeConfig {
            sys_window_weight: 0,
            sys_window_height: 0,
            config: serde::from_str(&config_content).unwrap(),
        }
    }
}

#[macro_use(impl_singleton)]
impl_singleton!(RuntimeConfig);
