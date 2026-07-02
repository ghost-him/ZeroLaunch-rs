/// Lnk 快捷方式解析器 trait，定义平台原语。
/// 各平台实现通过系统 API 解析 .lnk 快捷方式文件的目标路径，
/// 插件通过 PluginHandle 委托调用。
pub trait LnkResolver: Send + Sync {
    /// 解析 .lnk 快捷方式文件的目标路径。
    /// 参数：lnk_path - .lnk 文件的路径。
    /// 返回：解析成功返回目标路径，失败返回 None。
    fn resolve_lnk_target(&self, lnk_path: &str) -> Option<String>;
}
