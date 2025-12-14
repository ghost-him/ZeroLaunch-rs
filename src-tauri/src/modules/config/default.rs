use crate::core::storage::windows_utils::get_default_remote_data_dir_path;
use crate::error::{OptionExt, ResultExt};
use crate::RuntimeConfig;
use dashmap::DashMap;
use lazy_static::lazy_static;
use std::path::Path;
// 这里存放的都是在程序初始化以后就不会再改变的变量
lazy_static! {
    static ref DATA_DIR_PATH: String = get_default_remote_data_dir_path();

    pub static ref LOCAL_CONFIG_PATH: String = {
        Path::new(&*DATA_DIR_PATH)
            .join("ZeroLaunch_local_config.json")
            .to_str()
            .expect_programming("Failed to convert path to string")
            .to_string()
    };
    /// 日志文件夹的路径
    pub static ref LOG_DIR: String = {
        Path::new(&*DATA_DIR_PATH)
            .join("logs")
            .to_str()
            .expect_programming("Failed to convert path to string")
            .to_string()
    };
    /// 图标缓存文件夹的路径
    pub static ref ICON_CACHE_DIR: String = {
        Path::new(&*DATA_DIR_PATH)
        .join("icons")
        .to_str()
        .expect_programming("Failed to convert path to string")
        .to_string()
    };
    /// 模型文件的保存路径（与应用程序同级目录下的 models）
    pub static ref MODELS_DIR: String = {
        // 优先使用可执行文件所在目录；失败时回退到当前工作目录
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            });

        exe_dir
            .join("models")
            .to_str()
            .expect_programming("Failed to convert path to string")
            .to_string()
    };
    /// app使用到的图片的路径
    pub static ref APP_PIC_PATH: DashMap<String, String> = DashMap::new();
    /// 默认的配置信息
    pub static ref REMOTE_CONFIG_DEFAULT: String = serde_json::to_string_pretty(&RuntimeConfig::new().to_partial())
        .expect_programming("Failed to serialize default runtime config");
    /// 当前软件的版本号
    pub static ref APP_VERSION: String = env!("CARGO_PKG_VERSION").to_string();

}

pub const REMOTE_CONFIG_NAME: &str = "ZeroLaunch_remote_config.json";

pub const SEMANTIC_DESCRIPTION_FILE_NAME: &str = "ZeroLaunch_program_semantic_description.json";

/// 程序embedding缓存的二进制文件名
pub const SEMANTIC_EMBEDDING_CACHE_FILE_NAME: &str = "ZeroLaunch_program_embeddings.cache";

pub const PINYIN_CONTENT_JS: &str = include_str!("../program_manager/pinyin.json");
