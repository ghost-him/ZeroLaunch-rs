use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::default::APP_VERSION;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialAppConfig {
    pub search_bar_placeholder: Option<String>,
    pub tips: Option<String>,
    pub is_auto_start: Option<bool>,
    pub is_silent_start: Option<bool>,
    pub search_result_count: Option<u32>,
    pub auto_refresh_time: Option<u32>,
}

/// 与程序设置有关的，比如是不是要开机自动启动等
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct AppConfigInner {
    /// 自定义搜索栏的提示文本
    #[serde(default = "AppConfigInner::default_search_bar_placeholder")]
    pub search_bar_placeholder: String,
    /// 自定义搜索无结果时的文本
    #[serde(default = "AppConfigInner::default_tips")]
    pub tips: String,
    /// 是不是要开机自启动
    #[serde(default = "AppConfigInner::default_is_auto_start")]
    pub is_auto_start: bool,
    /// 是否静默启动
    #[serde(default = "AppConfigInner::default_is_silent_start")]
    pub is_silent_start: bool,
    /// 搜索结果的数量
    #[serde(default = "AppConfigInner::default_search_result_count")]
    pub search_result_count: u32,
    /// 自动刷新数据库的时间
    #[serde(default = "AppConfigInner::default_auto_refresh_time")]
    pub auto_refresh_time: u32,
    /// 是否是debug模式
    #[serde(default = "AppConfigInner::default_is_debug_mode")]
    pub is_debug_mode: bool,
}

impl Default for AppConfigInner {
    fn default() -> Self {
        Self {
            search_bar_placeholder: Self::default_search_bar_placeholder(),
            tips: Self::default_tips(),
            is_auto_start: Self::default_is_auto_start(),
            is_silent_start: Self::default_is_silent_start(),
            search_result_count: Self::default_search_result_count(),
            auto_refresh_time: Self::default_auto_refresh_time(),
            is_debug_mode: Self::default_is_debug_mode(),
        }
    }
}

impl AppConfigInner {
    pub(crate) fn default_search_bar_placeholder() -> String {
        "Hello, ZeroLaunch!".to_string()
    }

    pub(crate) fn default_tips() -> String {
        format!("ZeroLaunch-rs v{}", APP_VERSION.clone())
    }

    pub(crate) fn default_is_auto_start() -> bool {
        false
    }

    pub(crate) fn default_is_silent_start() -> bool {
        false
    }

    pub(crate) fn default_search_result_count() -> u32 {
        4
    }

    pub(crate) fn default_auto_refresh_time() -> u32 {
        30
    }

    pub(crate) fn default_is_debug_mode() -> bool {
        false
    }
}

impl AppConfigInner {
    pub fn update(&mut self, partial_app_config: PartialAppConfig) {
        if let Some(search_bar_placeholder) = partial_app_config.search_bar_placeholder {
            self.search_bar_placeholder = search_bar_placeholder;
        }
        if let Some(tips) = partial_app_config.tips {
            self.tips = tips;
        }
        if let Some(is_auto_start) = partial_app_config.is_auto_start {
            self.is_auto_start = is_auto_start;
        }
        if let Some(is_silent_start) = partial_app_config.is_silent_start {
            self.is_silent_start = is_silent_start;
        }
        if let Some(search_result_count) = partial_app_config.search_result_count {
            self.search_result_count = search_result_count;
        }
        if let Some(auto_refresh_time) = partial_app_config.auto_refresh_time {
            self.auto_refresh_time = auto_refresh_time;
        }
    }

    // 新增的get方法
    pub fn get_search_bar_placeholder(&self) -> String {
        self.search_bar_placeholder.clone()
    }

    pub fn get_tips(&self) -> String {
        self.tips.clone()
    }

    pub fn get_is_auto_start(&self) -> bool {
        self.is_auto_start
    }

    pub fn get_is_silent_start(&self) -> bool {
        self.is_silent_start
    }

    pub fn get_search_result_count(&self) -> u32 {
        self.search_result_count
    }

    pub fn get_auto_refresh_time(&self) -> u32 {
        self.auto_refresh_time
    }

    pub fn to_partial(&self) -> PartialAppConfig {
        PartialAppConfig {
            search_bar_placeholder: Some(self.search_bar_placeholder.clone()),
            tips: Some(self.tips.clone()),
            is_auto_start: Some(self.is_auto_start),
            is_silent_start: Some(self.is_silent_start),
            search_result_count: Some(self.search_result_count),
            auto_refresh_time: Some(self.auto_refresh_time),
        }
    }
}
#[derive(Debug)]
pub struct AppConfig {
    inner: RwLock<AppConfigInner>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            inner: RwLock::new(AppConfigInner::default()),
        }
    }
}

impl AppConfig {
    pub fn update(&self, partial_app_config: PartialAppConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_app_config);
    }

    // 修正为使用读锁并添加所有get方法
    pub fn get_search_bar_placeholder(&self) -> String {
        let inner = self.inner.read();
        inner.get_search_bar_placeholder()
    }

    pub fn get_tips(&self) -> String {
        let inner = self.inner.read();
        inner.get_tips()
    }

    pub fn get_is_auto_start(&self) -> bool {
        let inner = self.inner.read();
        inner.get_is_auto_start()
    }

    pub fn get_is_silent_start(&self) -> bool {
        let inner = self.inner.read();
        inner.get_is_silent_start()
    }

    pub fn get_search_result_count(&self) -> u32 {
        let inner = self.inner.read();
        inner.get_search_result_count()
    }

    pub fn get_auto_refresh_time(&self) -> u32 {
        let inner = self.inner.read();
        inner.get_auto_refresh_time()
    }

    pub fn to_partial(&self) -> PartialAppConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }
}

// // 手动实现序列化
// impl Serialize for AppConfig {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         // 获取读锁后序列化内部数据
//         let inner = self.inner.read();
//         inner.serialize(serializer)
//     }
// }

// // 手动实现反序列化
// impl<'de> Deserialize<'de> for AppConfig {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         // 先反序列化出内部数据
//         let inner = AppConfigInner::deserialize(deserializer)?;
//         // 用 RwLock 包装后返回
//         Ok(AppConfig {
//             inner: RwLock::new(inner),
//         })
//     }
// }

// // 手动实现 Clone
// impl Clone for AppConfig {
//     fn clone(&self) -> Self {
//         // 获取读锁后克隆内部数据
//         let inner_data = self.inner.read().clone();
//         AppConfig {
//             inner: RwLock::new(inner_data),
//         }
//     }
// }
