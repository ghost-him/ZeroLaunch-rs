//! 把确认后的需求打开成预填 issue 草稿：渲染固定模板正文，拼 GitHub 新建 issue 链接并打开默认浏览器。
//!
//! 唯一外部副作用出口是 `PluginHandle`（打开浏览器、必要时写剪贴板）；
//! 打开的是草稿页，是否提交由用户自己在浏览器里决定。

use super::AgentTool;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use zerolaunch_plugin_api::host::{OpenTarget, PluginHandle};

/// 新建 issue 的页面路径（拼在仓库地址之后）。
const NEW_ISSUE_PATH: &str = "/issues/new";

/// 预填链接的长度上限。
///
/// 实测（2026-10 直连 github.com）：长度 ≈4.6k 时 302 到登录页且 `return_to` 保留完整 query，
/// ≈6.9k 起 `return_to` 被丢弃，≥≈8.2k 直接 414 URI Too Long。中文按百分号编码膨胀 9 倍
/// （3 字节 → 3 个 `%XX`），故上限取 6000 留余量；超限时退化为「只填标题 + 正文进剪贴板」。
const MAX_URL_LEN: usize = 6000;

/// issue 标题的字符上限：标题不进正文，过长只会挤占链接空间，超出即截断。
const MAX_TITLE_CHARS: usize = 200;

/// GitHub 新建 issue 工具：把确认后的需求渲染成固定模板正文，打开默认浏览器的预填页面。
///
/// 只在 `core::agent` 的工具循环内构造与调用（每次提问新建一个实例）；
/// 打开的是草稿页，是否提交由用户自己在浏览器里决定。
pub struct OpenIssueTool {
    /// 宿主句柄：打开浏览器与写剪贴板都经它转交宿主能力。
    handle: Arc<PluginHandle>,
    /// 助手模型 id，写入正文页脚，供维护者判断内容来源。
    model_id: String,
    /// 本次提问是否已打开过页面：模型重复调用时不再连开标签页。
    opened: AtomicBool,
}

impl OpenIssueTool {
    /// 创建工具。
    /// 参数：handle - 宿主句柄；model_id - 助手模型全局 id（写入正文页脚）。
    pub fn new(handle: Arc<PluginHandle>, model_id: String) -> Self {
        Self {
            handle,
            model_id,
            opened: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl AgentTool for OpenIssueTool {
    fn name(&self) -> &'static str {
        "open_issue_draft"
    }

    fn description(&self) -> &'static str {
        "把已经和用户确认过的需求整理成 issue 正文，并在默认浏览器打开 GitHub 新建 issue 页面\
（标题与正文预填、不会被自动提交）。只在想要的能力当前不支持、需求已问清、且用户同意现在提 issue 时调用；\
一次提问最多调用一次。"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "issue 标题，一句话说清想要什么"
                },
                "summary": {
                    "type": "string",
                    "description": "需求描述：想要的用法与使用场景（2-4 句）"
                },
                "expected": {
                    "type": "string",
                    "description": "期望行为：希望程序如何表现、何时触发、给出什么结果"
                },
                "current_workaround": {
                    "type": "string",
                    "description": "现有做法与不足：目前怎么替代、为什么不够；没有就不传"
                }
            },
            "required": ["title", "summary", "expected"]
        })
    }

    async fn execute(&self, arguments: &Value) -> String {
        if self.opened.load(Ordering::SeqCst) {
            return "本次提问已经打开过新建 issue 页面，不要重复调用；直接告知用户去页面确认并提交即可。"
                .to_string();
        }
        let (Some(title), Some(summary), Some(expected)) = (
            text_arg(arguments, "title"),
            text_arg(arguments, "summary"),
            text_arg(arguments, "expected"),
        ) else {
            return "参数错误：title / summary / expected 均为必填且不能为空".to_string();
        };
        let workaround = text_arg(arguments, "current_workaround");
        let body = render_issue_body(&summary, &expected, workaround.as_deref(), &self.model_id);
        let repo = repository_url();
        let title = truncate_chars(&title, MAX_TITLE_CHARS);
        // 正文放得下就整段预填；放不下退化为「只预填标题 + 正文进剪贴板」，避免链接被 GitHub 拒收。
        let full_url = new_issue_url(&repo, title, Some(&body));
        let (url, copied_to_clipboard) = if full_url.len() <= MAX_URL_LEN {
            (full_url, false)
        } else {
            match self.handle.set_clipboard_text(&body) {
                Ok(()) => (new_issue_url(&repo, title, None), true),
                Err(error) => {
                    return format!(
                        "打开失败：正文过长（链接上限 {MAX_URL_LEN} 字符）且写入剪贴板也失败（{error}）。\
请把下面的正文交给用户，让其在 GitHub 上手动提交：\n\n{body}"
                    )
                }
            }
        };
        match self.handle.shell_open(OpenTarget::Url(url.clone())).await {
            Ok(()) => {
                self.opened.store(true, Ordering::SeqCst);
                if copied_to_clipboard {
                    format!(
                        "已在默认浏览器打开新建 issue 页面并预填标题；正文较长放不进链接，已整段复制到剪贴板。\
请告知用户在页面里粘贴正文（Ctrl+V）后确认提交。链接：{url}"
                    )
                } else {
                    format!(
                        "已在默认浏览器打开新建 issue 页面并预填标题与正文（草稿，尚未提交）。\
请告知用户去页面确认内容并提交。链接：{url}"
                    )
                }
            }
            Err(error) => format!(
                "打开浏览器失败：{error}。请把下面的正文交给用户，让其在 GitHub 上手动提交：\n\n{body}"
            ),
        }
    }
}

/// 读取非空字符串参数；缺失或去空白后为空时返回 None。
fn text_arg(arguments: &Value, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// 仓库地址：唯一权威是 workspace `Cargo.toml` 的 `repository`（由 Cargo 编译期注入），
/// 去掉可能的 `.git` 后缀与结尾 `/`，保证可直接拼路径。
fn repository_url() -> String {
    let raw = env!("CARGO_PKG_REPOSITORY").trim_end_matches('/');
    let raw = raw.strip_suffix(".git").unwrap_or(raw);
    debug_assert!(
        raw.starts_with("https://"),
        "仓库地址应来自 Cargo.toml 的 repository 字段"
    );
    raw.to_string()
}

/// 拼新建 issue 链接：`<repo>/issues/new?title=…&body=…`（body 为 None 时只带标题）。
///
/// 编码按表单规则处理 query 值，再把空格编码 `+` 换成 `%20`：表单序列化会把字面 `+`
/// 编成 `%2B`，故该替换只影响空格，且 `%20` 是 query 中最通用的空格表示。
fn new_issue_url(repo: &str, title: &str, body: Option<&str>) -> String {
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    query.append_pair("title", title);
    if let Some(body) = body {
        query.append_pair("body", body);
    }
    format!(
        "{repo}{NEW_ISSUE_PATH}?{}",
        query.finish().replace('+', "%20")
    )
}

/// 按字符数截断，不切断 UTF-8 边界。
fn truncate_chars(text: &str, max_chars: usize) -> &str {
    match text.char_indices().nth(max_chars) {
        Some((index, _)) => &text[..index],
        None => text,
    }
}

/// 渲染 issue 正文：固定小节 + 出处页脚；`workaround` 为 None 时省略那一节。
fn render_issue_body(
    summary: &str,
    expected: &str,
    workaround: Option<&str>,
    model_id: &str,
) -> String {
    let mut body = format!("### 需求描述\n{summary}\n\n### 期望行为\n{expected}\n");
    if let Some(workaround) = workaround {
        body.push_str(&format!("\n### 现有做法与不足\n{workaround}\n"));
    }
    body.push_str(&format!(
        "\n---\n> 本 issue 由 ZeroLaunch 设置助手（v{}，模型 {model_id}）根据对话整理，提交前请确认内容。\n",
        env!("CARGO_PKG_VERSION")
    ));
    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use zerolaunch_plugin_api::mock::helpers::mock_plugin_handle_recording;

    /// 解析链接里的 query 参数（key → 值），失败返回空表。
    fn query_params(url: &str) -> HashMap<String, String> {
        url.split_once('?')
            .map(|(_, query)| {
                url::form_urlencoded::parse(query.as_bytes())
                    .map(|(key, value)| (key.into_owned(), value.into_owned()))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn new_issue_url_round_trips_title_and_body() {
        let title = "支持按拼音搜索";
        let body = "### 需求\n换行 & 与 # 与 emoji 🚀\n";
        let url = new_issue_url(&repository_url(), title, Some(body));
        assert!(!url.contains('+'), "空格应编码为 %20：{url}");
        let params = query_params(&url);
        assert_eq!(params["title"], title);
        assert_eq!(params["body"], body);
    }

    #[test]
    fn new_issue_url_omits_body_when_absent() {
        let url = new_issue_url(&repository_url(), "只有标题", None);
        let params = query_params(&url);
        assert_eq!(params["title"], "只有标题");
        assert!(!params.contains_key("body"));
    }

    #[tokio::test]
    async fn open_issue_tool_opens_prefilled_url() {
        let (handle, shell, clipboard) = mock_plugin_handle_recording();
        let tool = OpenIssueTool::new(handle, "ollama/qwen3:8b".to_string());
        let report = tool
            .execute(&serde_json::json!({
                "title": "支持按拼音搜索",
                "summary": "希望能用拼音首字母搜到中文应用",
                "expected": "输入拼音前缀即出现匹配应用",
                "current_workaround": "现在只能输入完整中文名"
            }))
            .await;

        let opens = shell.opens.lock().clone();
        assert_eq!(opens.len(), 1, "应只打开一次：{report}");
        let OpenTarget::Url(url) = &opens[0] else {
            panic!("应以 URL 方式打开");
        };
        assert!(report.contains(url.as_str()), "工具结果应带上链接");
        let params = query_params(url);
        assert_eq!(params["title"], "支持按拼音搜索");
        assert!(params["body"].contains("希望能用拼音首字母搜到中文应用"));
        assert!(params["body"].contains("现在只能输入完整中文名"));
        assert!(params["body"].contains("### 期望行为"));
        assert!(clipboard.writes.lock().is_empty(), "预填成功时不应动剪贴板");
    }

    #[tokio::test]
    async fn open_issue_tool_falls_back_to_clipboard_when_url_too_long() {
        let (handle, shell, clipboard) = mock_plugin_handle_recording();
        let tool = OpenIssueTool::new(handle, "ollama/qwen3:8b".to_string());
        // 900 个汉字按百分号编码约 8100 字符，必超 6000 上限
        let summary = "需".repeat(900);
        let report = tool
            .execute(&serde_json::json!({
                "title": "正文很长的需求",
                "summary": summary.clone(),
                "expected": "照常打开页面"
            }))
            .await;

        let opens = shell.opens.lock().clone();
        let OpenTarget::Url(url) = &opens[0] else {
            panic!("应以 URL 方式打开");
        };
        let params = query_params(url);
        assert_eq!(params["title"], "正文很长的需求");
        assert!(!params.contains_key("body"), "超长时正文不进链接");
        let writes = clipboard.writes.lock().clone();
        assert_eq!(writes.len(), 1, "超长时正文应整段进剪贴板：{report}");
        assert!(writes[0].contains(&summary));
    }

    #[tokio::test]
    async fn open_issue_tool_rejects_repeat_call() {
        let (handle, shell, _clipboard) = mock_plugin_handle_recording();
        let tool = OpenIssueTool::new(handle, "ollama/qwen3:8b".to_string());
        let arguments = serde_json::json!({
            "title": "重复调用",
            "summary": "第一次就够",
            "expected": "只开一个标签页"
        });
        tool.execute(&arguments).await;
        let second = tool.execute(&arguments).await;
        assert!(second.contains("不要重复调用"), "{second}");
        assert_eq!(shell.opens.lock().len(), 1);
    }

    #[tokio::test]
    async fn open_issue_tool_reports_missing_argument() {
        let (handle, shell, clipboard) = mock_plugin_handle_recording();
        let tool = OpenIssueTool::new(handle, "ollama/qwen3:8b".to_string());
        let report = tool
            .execute(&serde_json::json!({ "title": "缺字段", "summary": "只有两段" }))
            .await;
        assert!(report.contains("expected"), "{report}");
        assert!(shell.opens.lock().is_empty());
        assert!(clipboard.writes.lock().is_empty());
    }
}
