use serde::{Deserialize, Serialize};

/// 缓存等级枚举，控制图标服务的缓存策略。
/// 插件通过 PluginSdkConfig 注册时指定，HostApi 根据等级决定缓存行为。
/// 跨进程经 `host/sdk_config.update` 以 camelCase 名称收发。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CacheLevel {
    /// 双层缓存（L1 → L2 → 提取 → 更新 L1+L2）。
    /// 适用于图标被频繁提取的场景。
    #[default]
    #[serde(rename = "full")]
    Full,
    /// 跳过内存缓存（L2 → 提取 → 更新 L2）。
    /// 适用于图标只在每次启动时提取的场景。
    #[serde(rename = "skipMemory")]
    SkipMemory,
    /// 跳过所有缓存（直接提取）。
    /// 适用于图标在几天的时间内可能只被提取一次的场景。
    #[serde(rename = "skipAll")]
    SkipAll,
    /// 只读缓存（L1 → L2，L2 命中回填 L1），不触发提取/网络。
    /// 未命中时 `get_icon` 返回错误，`get_icon_or_default` 回退默认图标。
    /// 宿主（查询响应等零网络开销路径）与插件（`PluginSdkConfig` 自注册）都可用。
    #[serde(rename = "cacheOnly")]
    CacheOnly,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 跨进程键名契约：插件经 `host/sdk_config.update` 按这些字面量收发缓存等级。
    #[test]
    fn cache_level_wire_names_roundtrip() {
        let cases = [
            (CacheLevel::Full, "\"full\""),
            (CacheLevel::SkipMemory, "\"skipMemory\""),
            (CacheLevel::SkipAll, "\"skipAll\""),
            (CacheLevel::CacheOnly, "\"cacheOnly\""),
        ];
        for (level, wire) in cases {
            assert_eq!(serde_json::to_string(&level).unwrap(), wire);
            assert_eq!(serde_json::from_str::<CacheLevel>(wire).unwrap(), level);
        }
    }
}
