//! 参数解析器 trait 定义

use super::types::{ParameterError, ParameterSnapshot};

/// 参数解析器 trait
///
/// 通过 PluginHandle 暴露给插件和 Executor 使用。
/// 设计说明：
/// - resolve 方法为异步接口，保持与其他 SDK trait 风格统一
/// - count_user_parameters 和 has_system_parameters 为同步方法，因为它们只是纯字符串解析
#[async_trait::async_trait]
pub trait ParameterResolver: Send + Sync {
    /// 解析模板，填充参数
    ///
    /// 参数：
    /// - template: 包含占位符的模板字符串（如 "notepad {clip}"）
    /// - user_args: 用户输入的参数列表
    /// - snapshot: 系统参数快照（不透明句柄，由 capture_parameter_snapshot() 返回）
    ///
    /// 返回：填充后的完整字符串
    async fn resolve(
        &self,
        template: &str,
        user_args: &[String],
        snapshot: &ParameterSnapshot,
    ) -> Result<String, ParameterError>;

    /// 统计模板中需要用户输入的参数数量（同步方法，纯计算）
    fn count_user_parameters(&self, template: &str) -> usize;

    /// 检查模板是否包含系统参数（同步方法，纯计算）
    fn has_system_parameters(&self, template: &str) -> bool;
}
