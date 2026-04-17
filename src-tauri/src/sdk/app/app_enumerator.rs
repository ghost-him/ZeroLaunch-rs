use crate::sdk::app::AppInfo;
use async_trait::async_trait;

/// 应用枚举器 trait，定义平台原语。
/// 各平台实现通过系统 API 发现已安装应用，插件通过 PluginHandle 委托调用。
#[async_trait]
pub trait AppEnumerator: Send + Sync {
    /// 枚举所有已安装应用。
    /// 参数：无。
    /// 返回：应用信息列表。
    async fn enumerate_apps(&self) -> Vec<AppInfo>;
}
