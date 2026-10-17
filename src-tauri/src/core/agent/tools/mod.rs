//! agent 可调用的工具：读取设置当前值（敏感字段脱敏）、把确认后的需求或 bug 报告打开成预填 issue 草稿。
//!
//! 工具不写配置、不碰文件系统；唯一外部副作用来自 issue 工具（打开默认浏览器、必要时写剪贴板），
//! 且一律经 `PluginHandle` 转交宿主能力。工具集由 agent 每轮请求携带。

mod get_settings;
mod open_issue;

pub use get_settings::{GetSettingsTool, MASK};
pub use open_issue::OpenIssueTool;

use async_trait::async_trait;
use serde_json::Value;
use zerolaunch_plugin_api::services::model::ModelTool;

/// 工具契约：名称 / 说明 / 参数 schema / 执行。
#[async_trait]
pub trait AgentTool: Send + Sync {
    /// 工具名（模型按此名调用）。
    fn name(&self) -> &'static str;

    /// 工具用途说明，供模型判断何时调用。
    fn description(&self) -> &'static str;

    /// 参数的 JSON Schema。
    fn parameters(&self) -> Value;

    /// 执行工具；返回给模型的文本结果（失败也以文本返回，由模型自行调整）。
    async fn execute(&self, arguments: &Value) -> String;

    /// 工具的模型侧声明。
    fn spec(&self) -> ModelTool {
        ModelTool {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
        }
    }
}
