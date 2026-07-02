use crate::host::CacheLevel;

/// 插件 SDK 配置。
/// 各字段可选，不需要配置的服务无需设置，使用默认值。
#[derive(Debug, Clone, Default)]
pub struct PluginSdkConfig {
    /// 图标缓存等级。None 时使用默认值 CacheLevel::Full。
    pub icon_cache_level: Option<CacheLevel>,
}
