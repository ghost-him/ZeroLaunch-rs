use crate::host::CacheLevel;
use serde::{Deserialize, Serialize};

/// 插件 SDK 配置。
/// 各字段可选，不需要配置的服务无需设置，使用默认值。
/// 跨进程经 `host/sdk_config.update` 收发（camelCase 键名）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginSdkConfig {
    /// 图标缓存等级。None 时使用默认值 CacheLevel::Full。
    #[serde(rename = "iconCacheLevel")]
    #[serde(default)]
    pub icon_cache_level: Option<CacheLevel>,
}
