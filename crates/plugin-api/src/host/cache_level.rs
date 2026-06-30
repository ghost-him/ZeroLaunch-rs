/// 缓存等级枚举，控制图标服务的缓存策略。
/// 插件通过 PluginSdkConfig 注册时指定，HostApi 根据等级决定缓存行为。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheLevel {
    /// 双层缓存（L1 → L2 → 提取 → 更新 L1+L2）。
    /// 适用于图标被频繁提取的场景。
    #[default]
    Full,
    /// 跳过内存缓存（L2 → 提取 → 更新 L2）。
    /// 适用于图标只在每次启动时提取的场景。
    SkipMemory,
    /// 跳过所有缓存（直接提取）。
    /// 适用于图标在几天的时间内可能只被提取一次的场景。
    SkipAll,
}
