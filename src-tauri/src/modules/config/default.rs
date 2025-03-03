use crate::modules::storage::windows_utils::get_data_dir_path;
use crate::RuntimeConfig;
use dashmap::DashMap;
use lazy_static::lazy_static;
use std::path::Path;
// 这里存放的都是在程序初始化以后就不会再改变的变量
lazy_static! {
    static ref DATA_DIR_PATH: String = get_data_dir_path();
    pub static ref LOCAL_CONFIG_PATH: String = {
        Path::new(&*DATA_DIR_PATH)
            .join("ZeroLaunch_local_config.json")
            .to_str()
            .expect("Failed to convert path to string")
            .to_string()
    };
    pub static ref LOG_DIR: String = {
        Path::new(&*DATA_DIR_PATH)
            .join("logs")
            .to_str()
            .expect("Failed to convert path to string")
            .to_string()
    };
    // app使用到的图片的路径
    pub static ref APP_PIC_PATH: DashMap<String, String> = DashMap::new();
    // 默认的图片
    pub static ref CONFIG_DEFAULT: String = serde_json::to_string(&RuntimeConfig::new("./".to_string()).to_partial()).unwrap();
    // 当前软件的版本号
    pub static ref APP_VERSION: String = env!("CARGO_PKG_VERSION").to_string();

}

pub const REMOTE_CONFIG_NAME: &str = "ZeroLaunch_remote_config.json";

pub const PINYIN_CONTENT_JS: &str = include_str!("../program_manager/pinyin.json");
