//! 读取内置使用手册页面的工具：数据在编译期内嵌，不访问文件系统。

use super::AgentTool;
use crate::core::agent::wiki;
use async_trait::async_trait;
use serde_json::Value;

/// 读取一页使用手册全文。
pub struct ReadWikiTool;

#[async_trait]
impl AgentTool for ReadWikiTool {
    fn name(&self) -> &'static str {
        "read_wiki"
    }

    fn description(&self) -> &'static str {
        "读取 ZeroLaunch 使用手册的整页内容（只读）。回答功能、界面、操作流程、隐私与数据等问题前先调用；id 取系统消息「使用手册目录」里每页标注的 id。"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "手册页面 id，如 核心搜索体验/搜索窗口交互"
                }
            },
            "required": ["id"]
        })
    }

    async fn execute(&self, arguments: &Value) -> String {
        let Some(id) = arguments.get("id").and_then(Value::as_str) else {
            return "参数错误：缺少 id".to_string();
        };
        match wiki::read(id) {
            Ok(text) => text,
            Err(error) => error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 合法 id 返回页面正文，未知 id 返回提示。
    #[tokio::test]
    async fn execute_reads_known_page_and_reports_unknown() {
        let tool = ReadWikiTool;
        let body = tool.execute(&serde_json::json!({ "id": "Home" })).await;
        assert!(body.starts_with("# "), "应返回页面正文");
        let error = tool
            .execute(&serde_json::json!({ "id": "不存在的页面" }))
            .await;
        assert!(error.contains("未找到手册页面"));
    }
}
