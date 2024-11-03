use crate::impl_singleton;
use crate::singleton::Singleton;
use crate::utils::read_or_create;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::from_str;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Once;
pub type Width = usize;
pub type Height = usize;

lazy_static! {
    /// 配置文件存在的位置
    static ref CONFIG_PATH: String = "./config.js".to_string();
    /// 配置文件的默认内容
    static ref CONFIG_DEFAULT: String = serde_json::to_string(&Config::default()).unwrap();
}

/// 与程序设置有关的，比如是不是要开机自动启动等
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppConfig {
    show_item_count: u32,
}
/// 与程序页面设置有关的，比如窗口的大小，显示的界面等
#[derive(Serialize, Deserialize, Debug, Clone)]
struct UiConfig {
    /// 显示器的大小与窗口的大小的比例
    /// 窗口的高的比例
    item_width_scale_factor: f64,
    /// 窗口的宽度的比例
    item_height_scale_factor: f64,
}
/// 综合
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    pub version: u32,
    pub app_config: AppConfig,
    pub ui_config: UiConfig,
}

impl AppConfig {
    pub fn default() -> Self {
        AppConfig { show_item_count: 4 }
    }
}

impl UiConfig {
    pub fn default() -> Self {
        UiConfig {
            item_width_scale_factor: 0.5,
            item_height_scale_factor: 0.0555,
        }
    }

    pub fn get_item_size(
        &self,
        sys_window_width: Width,
        sys_window_height: Height,
    ) -> (Width, Height) {
        (
            (sys_window_width as f64 * self.item_width_scale_factor) as Width,
            (sys_window_height as f64 * self.item_height_scale_factor) as Height,
        )
    }
}

impl Config {
    pub fn default() -> Config {
        Config {
            version: 1,
            app_config: AppConfig::default(),
            ui_config: UiConfig::default(),
        }
    }
}

/// 运行时确定的
pub struct RuntimeConfig {
    /// 当前屏幕的缩放比例
    sys_window_scale_factor: f64,
    /// 显示器的宽
    sys_window_width: Width,
    /// 显示器的长
    sys_window_height: Height,
    /// 配置文件
    config: Config,
}

impl RuntimeConfig {
    fn new() -> RuntimeConfig {
        // 读取配置文件
        let config_content = read_or_create(&*CONFIG_PATH, Some((*CONFIG_DEFAULT).clone()))
            .expect("无法读取配置文件");
        let final_config: Config;
        match serde_json::from_str::<Config>(&config_content) {
            Ok(config) => {
                // 如果已经正常的读到文件了，则判断文件是不是正常读取了
                if config.version == Config::default().version {
                    final_config = config;
                } else {
                    final_config = Config::default();
                }
            }
            Err(_e) => {
                final_config = Config::default();
            }
        }

        RuntimeConfig {
            sys_window_scale_factor: 1.0,
            sys_window_width: 0,
            sys_window_height: 0,
            config: final_config,
        }
    }

    pub fn set_sys_window_size(&mut self, sys_window_width: Width, sys_window_height: Height) {
        self.sys_window_height = sys_window_height;
        self.sys_window_width = sys_window_width;
    }

    pub fn get_item_size(&self) -> (Width, Height) {
        self.config
            .ui_config
            .get_item_size(self.sys_window_width, self.sys_window_height)
    }

    pub fn get_window_size(&self) -> (Width, Height) {
        let item_size = self
            .config
            .ui_config
            .get_item_size(self.sys_window_width, self.sys_window_height);
        let show_item_count = self.config.app_config.show_item_count;
        (item_size.0, item_size.1 * (show_item_count as usize + 2))
    }

    pub fn get_window_render_origin(&self) -> (Width, Height) {
        let (window_width, window_height) = self.get_window_size();
        let window_width_margin = (self.sys_window_width - window_width) / 2 as Width;
        let window_height_margin = (self.sys_window_height - window_height) / 2 as Height;
        (window_width_margin, window_height_margin)
    }
    pub fn set_window_scale_factor(&mut self, factor: f64) {
        self.sys_window_scale_factor = factor;
    }
    pub fn get_window_scale_factor(&self) -> f64 {
        self.sys_window_scale_factor
    }
}

impl_singleton!(RuntimeConfig);
