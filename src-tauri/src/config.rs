use crate::impl_singleton;
use crate::interface::{KeyFilterData, SettingWindowPathData};
use crate::program_manager::config::{ProgramLauncherConfig, ProgramManagerConfig};
use crate::singleton::Singleton;
use crate::utils::read_or_create_str;
use dashmap::DashMap;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Once;
pub type Width = usize;
pub type Height = usize;
use crate::utils::get_data_dir_path;
use std::path::Path;

lazy_static! {
    /// 配置文件存在的位置
    pub static ref CONFIG_PATH: String = Path::new(&get_data_dir_path()).join("config.json").to_str().unwrap().to_string();
    /// 配置文件的默认内容
    static ref CONFIG_DEFAULT: String = serde_json::to_string(&Config::default()).unwrap();
    /// 全局app_handle
    pub static ref GLOBAL_APP_HANDLE: Mutex<Option<tauri::AppHandle>> = Mutex::new(None);
    /// 日志文件存在的文件夹
    pub static ref LOG_DIR: String = Path::new(&get_data_dir_path()).join("logs").to_str().unwrap().to_string();
    /// 存储所有图片的路径
    pub static ref PIC_PATH: DashMap<String, String> = DashMap::new();
    /// 背景图片存放的地址
    pub static ref BACKGROUND_PIC_PATH: String = Path::new(&get_data_dir_path()).join("background.png").to_str().unwrap().to_string();
}

/// 与程序设置有关的，比如是不是要开机自动启动等
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// 自定义搜索栏的提示文本
    pub search_bar_placeholder: String,
    /// 自定义搜索无结果时的文本
    pub search_bar_no_result: String,
    /// 是不是要开机自启动
    pub is_auto_start: bool,
    /// 是否静默启动
    pub is_silent_start: bool,
    /// 搜索结果的数量
    pub search_result_count: u32,
    /// 自动刷新数据库的时间
    pub auto_refresh_time: u32,
}
/// 与程序页面设置有关的，比如窗口的大小，显示的界面等
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiConfig {
    /// 显示器的大小与窗口的大小的比例
    /// 窗口的高的比例
    item_width_scale_factor: f64,
    /// 窗口的宽度的比例
    item_height_scale_factor: f64,
    /// 选中项的颜色
    pub selected_item_color: String,
    /// 选项中的字体的颜色
    pub item_font_color: String,
}
/// 综合
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    pub version: u32,
    pub app_config: AppConfig,
    pub ui_config: UiConfig,
    pub program_manager_config: ProgramManagerConfig,
}

impl AppConfig {
    pub fn default() -> Self {
        AppConfig {
            search_bar_placeholder: "Hello, ZeroLaunch!".to_string(),
            search_bar_no_result: "当前搜索无结果".to_string(),
            is_auto_start: false,
            is_silent_start: false,
            search_result_count: 4,
            auto_refresh_time: 30,
        }
    }
}

impl UiConfig {
    pub fn default() -> Self {
        UiConfig {
            item_width_scale_factor: 0.5,
            item_height_scale_factor: 0.0555,
            selected_item_color: "#d55d1d".to_string(),
            item_font_color: "#000000".to_string(),
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
            program_manager_config: ProgramManagerConfig::default(),
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
        let config_content = read_or_create_str(&CONFIG_PATH, Some((*CONFIG_DEFAULT).clone()))
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
        let show_item_count = self.config.app_config.search_result_count;
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

    pub fn get_program_manager_config(&self) -> &ProgramManagerConfig {
        &self.config.program_manager_config
    }

    pub fn get_mut_program_manager_config(&mut self) -> &mut ProgramManagerConfig {
        &mut self.config.program_manager_config
    }

    pub fn get_app_config(&self) -> &AppConfig {
        &self.config.app_config
    }

    pub fn get_ui_config(&self) -> &UiConfig {
        &self.config.ui_config
    }

    pub fn save_app_config(&mut self, app_config: AppConfig) {
        self.config.app_config = app_config.clone();
    }

    pub fn save_path_config(&mut self, path_data: SettingWindowPathData) {
        let path_config = &mut self.config.program_manager_config.loader;
        path_config.forbidden_paths = path_data.forbidden_paths;
        path_config.forbidden_program_key = path_data.forbidden_key;
        path_config.target_paths = path_data.target_paths;
        path_config.is_scan_uwp_programs = path_data.is_scan_uwp_program;
    }

    pub fn save_program_launcher_config(&mut self, launcher_data: &ProgramLauncherConfig) {
        self.config.program_manager_config.launcher = launcher_data.clone();
    }

    pub fn save_key_filter_config(&mut self, key_filter_data: Vec<KeyFilterData>) {
        let path_config = &mut self.config.program_manager_config.loader;
        path_config.forbidden_program_key.clear();
        for item in &key_filter_data {
            path_config
                .program_bias
                .insert(item.key.clone(), (item.bias, item.note.clone()));
        }
    }
    /// 保存当前的程序配置
    /// 1. 更新要保存的东西（动态变化的东西）
    /// 2. 返回已经更新好的配置信息
    pub fn save_config(&self) -> String {
        serde_json::to_string(&self.config).unwrap()
    }

    pub fn save_index_file_info(&mut self, index_file_infos: Vec<String>) {
        let path_config = &mut self.config.program_manager_config.loader;
        path_config.index_file_paths = index_file_infos.clone();
    }

    pub fn save_web_pages_info(&mut self, index_web_pages: Vec<(String, String)>) {
        let path_config = &mut self.config.program_manager_config.loader;
        path_config.index_web_pages = index_web_pages.clone();
    }

    pub fn get_index_files(&self) -> Vec<String> {
        let path_config = &self.config.program_manager_config.loader;
        path_config.index_file_paths.clone()
    }

    pub fn get_web_pages_info(&self) -> Vec<(String, String)> {
        let path_config = &self.config.program_manager_config.loader;
        path_config.index_web_pages.clone()
    }

    pub fn save_selected_item_color(&mut self, color: String) {
        self.config.ui_config.selected_item_color = color.clone();
    }
    pub fn save_item_font_color(&mut self, color: String) {
        self.config.ui_config.item_font_color = color.clone();
    }
}

impl_singleton!(RuntimeConfig);
