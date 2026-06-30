use std::collections::HashMap;
use std::path::Path;

/// 平台资源加载器 trait，定义平台原语。
/// 各平台实现通过系统 API 加载本地化字符串资源，
/// 插件通过 PluginHandle 委托调用。
pub trait ResourceLoader: Send + Sync {
    /// 解析指定目录下的 desktop.ini 文件，提取 [LocalizedFileNames] 部分。
    /// 参数：dir_path - 要解析的目录路径。
    /// 返回：从原始文件名到本地化名称的映射。
    fn parse_localized_names_from_dir(&self, dir_path: &Path) -> HashMap<String, String>;
}
