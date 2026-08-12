use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::Deserialize;
use tracing::{error, warn};

use super::super::provider::{
    LanguageSupport, SenseEntry, TranslateRequest, TranslationProvider, TranslationResult,
};

pub const PROVIDER_ID: &str = "openai-compatible";

/// OpenAI 兼容 LLM 引擎的连接配置（Base URL / API Key / Model）。
///
/// 仅 TranslatorPlugin 内部使用，不跨 IPC；
/// 由 apply_settings 从 TranslatorSettings 的 llm_* 字段同步而来。
#[derive(Debug, Clone, Default)]
pub struct LlmConfig {
    /// API Base URL（如 `https://api.deepseek.com`）。
    pub base_url: String,
    /// API Key。
    pub api_key: String,
    /// 模型名（如 `deepseek-chat`）。
    pub model: String,
}

pub const DEFAULT_TRANSLATION_SYSTEM_PROMPT: &str = r#"你是能够敏锐感知语言语体的翻译专家。根据用户给出的源语言、目标语言与原文，输出且仅输出一个 JSON 对象，不要 markdown 代码块，不要额外说明。

核心原则：译文的语气随原文语气自然变化。
- 原文正式/学术/商务 → 译文庄重、精确、用词考究
- 原文口语/聊天/轻松 → 译文自然、简洁、接地气
- 原文技术文档/代码 → 译文术语准确、句式干净
- 原文文学/创意 → 保留原文的情绪张力与节奏
- 总之：让译文读起来像该语言中本就该有的表达，避免翻译腔

JSON 字段（camelCase）：
- text（string，必填）：主译文
- phonetic（string，可选）：音标或读音
- computerSense（string，可选）：计算机/IT 领域释义（仅原文为计算机术语时提供）
- moreSenses（array，可选，最多 4 条）：更多释义，每项含 label（可选，如词性/领域）与 text（string）

示例 1（技术→中文，含音标/计算机释义）：{"text":"缓存失效策略使用 LRU 淘汰","phonetic":"/kæʃ/","computerSense":"高速缓冲存储器","moreSenses":[{"label":"v.","text":"存入缓存"}]}
示例 2（口语→中文，语气轻松）：{"text":"老哥这应用太牛了","moreSenses":[{"label":"adj.","text":"很酷的"}]}"#;

const SUPPORTED_LANGUAGES: &[&str] = &[
    "zh", "zh-TR", "yue", "en", "fr", "pt", "es", "ja", "tr", "ru", "ar", "ko", "th", "it", "de",
    "vi", "ms", "id",
];

/// OpenAI 兼容 LLM 翻译引擎 Provider。
///
/// 通过 `{base_url}/chat/completions` 调用兼容 API，将模型输出解析为统一翻译结果。
/// 仅在 TranslatorPlugin 内部使用，通过 ProviderRegistry 管理。
pub struct OpenAiCompatibleProvider {
    /// LLM 连接配置（Base URL / API Key / Model），通过 Arc<RwLock> 由 TranslatorPlugin 注入。
    config: Arc<RwLock<LlmConfig>>,
    /// 复用的 HTTP 客户端，在构造时创建一次，避免每次翻译请求新建连接池。
    client: reqwest::Client,
}

impl OpenAiCompatibleProvider {
    /// 创建新的 Provider 实例。
    pub fn new(config: Arc<RwLock<LlmConfig>>) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

/// LLM 返回 JSON 的主负载（camelCase 字段与提示词约定一致）。
///
/// 仅限本文件内使用；用于解析 chat/completions 的 model 输出。
#[derive(Debug, Deserialize)]
struct LlmTranslationPayload {
    /// 主译文。
    text: String,
    /// 音标/读音（可选）。
    #[serde(default)]
    phonetic: Option<String>,
    /// 计算机/IT 领域释义（可选）。
    #[serde(default, rename = "computerSense")]
    computer_sense: Option<String>,
    /// 更多释义条目（可选，最多 4 条）。
    #[serde(default, rename = "moreSenses")]
    more_senses: Vec<LlmSenseEntry>,
}

/// LLM 返回 JSON 中的单条更多释义。
///
/// 仅限本文件内使用。
#[derive(Debug, Deserialize)]
struct LlmSenseEntry {
    /// 释义文本。
    text: String,
    /// 领域/词性标签（可选）。
    #[serde(default)]
    label: Option<String>,
}

/// chat/completions 响应（仅取 choices 字段）。
///
/// 仅限本文件内使用。
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    /// 候选回复列表。
    choices: Vec<ChatChoice>,
}

/// 单个候选回复。
///
/// 仅限本文件内使用。
#[derive(Debug, Deserialize)]
struct ChatChoice {
    /// 回复消息。
    message: ChatMessage,
}

/// 回复消息 content：兼容 string / 多段数组 / 已解析的 JSON 对象。
///
/// 仅限本文件内使用；用于反序列化 chat/completions 响应的 content 字段。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MessageContent {
    /// 纯文本正文（绝大多数 OpenAI 兼容 API 的返回形式，可能是 JSON 字符串）。
    Text(String),
    /// 多模态分段正文（部分网关返回 parts 数组），仅拼接各段的 text。
    Array(Vec<ContentPart>),
    /// 已被网关解析为对象的正文，经 `to_string()` 重新序列化后交给解析层。
    Object(serde_json::Value),
}

/// 多模态 content 数组中的单段。
///
/// 仅限本文件内使用；非文本段（如图片）不含 text，拼接时被忽略。
#[derive(Debug, Deserialize)]
struct ContentPart {
    /// 本段的文本内容；None 表示该段无可拼接文本。
    #[serde(default)]
    text: Option<String>,
}

/// 回复消息（仅取 content 字段）。
///
/// 仅限本文件内使用。
#[derive(Debug, Deserialize)]
struct ChatMessage {
    /// 回复正文（字符串 JSON / 多模态数组 / 已是对象）。
    #[serde(default)]
    content: Option<MessageContent>,
}

/// 将 `Option<MessageContent>` 归一化为字符串供 `parse_llm_content` 解析。
///
/// None 与畸形 JSON 值（数字/布尔/数组）返回空串，由解析层报错；
/// Object 仅接受 JSON 对象，避免把 `42`/`true` 序列化后当作译文。
fn message_content_to_string(content: &Option<MessageContent>) -> String {
    match content {
        None => String::new(),
        Some(MessageContent::Text(s)) => s.clone(),
        Some(MessageContent::Object(v)) => {
            if v.is_object() {
                v.to_string()
            } else {
                String::new()
            }
        }
        Some(MessageContent::Array(parts)) => parts
            .iter()
            .filter_map(|p| p.text.as_deref())
            .collect::<Vec<_>>()
            .join(""),
    }
}

/// 解析成功时的负载：(text, phonetic, computer_sense, more_senses)。
type ParsedLlmFields = (String, Option<String>, Option<String>, Vec<SenseEntry>);

/// 解析 LLM 返回的 JSON 正文（支持 camelCase 字段名）。
///
/// 容忍常见脏输出：markdown 代码块、`<think>` 前缀、说明文字后夹带 JSON、
/// 字符串值内未转义的控制字符；若确认无可用 JSON 对象则回退为纯文本译文。
///
/// 提取判定（防止把正文中的花括号片段误当 JSON）：
/// - 只采纳"提取出的 JSON 对象之后无其他内容"的结果，避免静默截断译文；
/// - 提取成功但解析失败时继续向后扫描下一个对象（防 `<think>` 内的 JSON 毒化）；
/// - 全部失败时：以 `{` 开头（模型承诺 JSON）或含残缺花括号（半截 JSON）→ 报错，
///   否则回退纯文本（正文花括号/普通文本），回退时剥离 `<think>` 块。
pub fn parse_llm_content(content: &str) -> Result<ParsedLlmFields, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("LLM 返回空内容".into());
    }
    let after_fence = strip_markdown_fence(trimmed);
    if after_fence.trim().is_empty() {
        // fence 剥离后为空（如输出恰为 ``` 标记）
        return Err("LLM 返回空内容".into());
    }

    // 至多尝试 3 个候选对象：think/前言里的 JSON 片段解析失败后继续向后找真正的 payload
    let mut scan_from = after_fence;
    let mut saw_complete_object = false;
    for _ in 0..3 {
        let Some((start, end)) = extract_first_json_object(scan_from) else {
            break;
        };
        saw_complete_object = true;
        let json_raw = &scan_from[start..=end];
        let rest = &scan_from[end + 1..];
        let sanitized = escape_control_chars_in_json_strings(json_raw);
        match serde_json::from_str::<LlmTranslationPayload>(&sanitized) {
            Ok(payload) => {
                // 仅当对象之后没有其他内容时才采纳，否则视为正文中的 JSON 片段
                if rest.trim().is_empty() {
                    return payload_to_result(payload);
                }
                break;
            }
            Err(_) => scan_from = rest,
        }
    }

    // 无可用 JSON 对象：判定是回退纯文本还是报错
    if after_fence.starts_with('{') {
        // 模型承诺输出 JSON 却失败 → 不回退，避免把乱码当译文
        return Err("JSON 解析失败: 未找到完整 JSON 对象".into());
    }
    if !saw_complete_object && after_fence.contains('{') {
        // 含 `{` 但从未配对出完整对象 → 半截 JSON，同样报错而非当译文
        return Err("JSON 解析失败: 未找到完整 JSON 对象".into());
    }
    // 正文花括号或纯文本：部分模型忽略格式约定，直接返回纯译文（剥离 think 块）
    Ok((strip_think_blocks(after_fence), None, None, Vec::new()))
}

/// 校验解析出的负载并组装为 `ParsedLlmFields`；text 为空时返回中文错误。
fn payload_to_result(payload: LlmTranslationPayload) -> Result<ParsedLlmFields, String> {
    if payload.text.trim().is_empty() {
        return Err("JSON 缺少有效 text 字段".into());
    }
    let more_senses = payload
        .more_senses
        .into_iter()
        .filter(|s| !s.text.trim().is_empty())
        .map(|s| SenseEntry {
            text: s.text,
            label: s.label,
        })
        .collect();
    Ok((
        payload.text,
        payload.phonetic,
        payload.computer_sense,
        more_senses,
    ))
}

/// 剥离 markdown 代码块围栏，支持两种形态：
/// 整段为围栏块（` ```json\n{...}\n``` `）或说明文字后夹带围栏块
/// （`译文如下：\n```json\n{...}\n``` `，返回围栏内内容）。
/// 无闭合围栏或仅正文提及 ``` 时原样返回。
fn strip_markdown_fence(s: &str) -> &str {
    let s = s.trim();
    let Some(fence_start) = s.find("```") else {
        return s;
    };
    // 围栏起点后的内容：去掉可选语言标记（到行尾）
    let after_open = &s[fence_start + 3..];
    let after_open = match after_open.find('\n') {
        Some(nl) => &after_open[nl + 1..],
        None => {
            // ``` 后无换行：整段为裸围栏标记 → 视为空；否则是正文提及，原样返回
            return if fence_start == 0 { "" } else { s };
        }
    };
    // 去掉尾部 ```（若存在）
    match after_open.rfind("```") {
        Some(pos) => after_open[..pos].trim(),
        None => s,
    }
}

/// 从混杂文本中抽出第一个完整 JSON 对象（花括号配对，忽略字符串内括号），
/// 返回 `(起始字节索引, 结束字节索引)`，供调用方切片并继续向后扫描。
fn extract_first_json_object(s: &str) -> Option<(usize, usize)> {
    let start = s.find('{')?;
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_string {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((start, i));
                }
            }
            _ => {}
        }
    }
    None
}

/// 剥离 `<think>...</think>` 块（可多个、可跨行）；未闭合的块保留剩余原文。
fn strip_think_blocks(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        rest = &rest[start + "<think>".len()..];
        match rest.find("</think>") {
            Some(end) => rest = &rest[end + "</think>".len()..],
            None => {
                // 未闭合：保留标签与剩余内容，避免丢失模型输出
                out.push_str("<think>");
                out.push_str(rest);
                return out;
            }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

/// 将 JSON 字符串值内的裸控制字符转义为 `\n` / `\r` / `\t` / `\uXXXX`。
/// 部分模型会在 `"text": "..."` 里直接换行，导致严格 JSON 解析失败。
fn escape_control_chars_in_json_strings(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_string = false;
    let mut escape = false;
    for c in s.chars() {
        if !in_string {
            if c == '"' {
                in_string = true;
            }
            out.push(c);
            continue;
        }
        if escape {
            out.push(c);
            escape = false;
            continue;
        }
        match c {
            '\\' => {
                out.push(c);
                escape = true;
            }
            '"' => {
                out.push(c);
                in_string = false;
            }
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn missing_config_error() -> String {
    "请先在设置中填写 LLM 服务的 Base URL、API Key 和 Model".into()
}

fn config_ready(config: &LlmConfig) -> bool {
    !config.base_url.trim().is_empty()
        && !config.api_key.trim().is_empty()
        && !config.model.trim().is_empty()
}

fn format_language(code: &str) -> &str {
    if code.eq_ignore_ascii_case("auto") {
        "自动检测"
    } else {
        code
    }
}

fn build_user_message(req: &TranslateRequest) -> String {
    format!(
        "源语言：{}\n目标语言：{}\n原文：{}",
        format_language(&req.source),
        format_language(&req.target),
        req.text
    )
}

#[async_trait]
impl TranslationProvider for OpenAiCompatibleProvider {
    fn id(&self) -> &str {
        PROVIDER_ID
    }

    fn name(&self) -> &str {
        "OpenAI 兼容"
    }

    fn language_support(&self) -> LanguageSupport {
        LanguageSupport::bilingual(SUPPORTED_LANGUAGES)
    }

    async fn translate(&self, req: &TranslateRequest) -> TranslationResult {
        let config = self.config.read().clone();
        if !config_ready(&config) {
            return TranslationResult::err(PROVIDER_ID, "OpenAI 兼容", missing_config_error());
        }

        let base_url = config.base_url.trim().trim_end_matches('/');
        // P0-2: 校验 scheme，防止 file:///etc/passwd 等危险输入
        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            return TranslationResult::err(
                PROVIDER_ID,
                "OpenAI 兼容",
                "LLM Base URL 必须以 http:// 或 https:// 开头",
            );
        }

        let url = format!("{base_url}/chat/completions");
        let body = serde_json::json!({
            "model": config.model,
            "stream": false,
            "messages": [
                {"role": "system", "content": DEFAULT_TRANSLATION_SYSTEM_PROMPT},
                {"role": "user", "content": build_user_message(req)},
            ],
        });

        let response = match self
            .client
            .post(&url)
            .bearer_auth(config.api_key.trim())
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return TranslationResult::err(
                    PROVIDER_ID,
                    "OpenAI 兼容",
                    format!("请求 LLM 服务失败: {e}"),
                );
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            // P0-3: 服务端错误详情只记日志，不暴露给前端
            let detail = response.text().await.unwrap_or_default();
            error!("LLM 服务返回错误 ({}): {}", status, detail);
            return TranslationResult::err(
                PROVIDER_ID,
                "OpenAI 兼容",
                format!("LLM 服务返回错误 ({status})"),
            );
        }

        let completion: ChatCompletionResponse = match response.json().await {
            Ok(c) => c,
            Err(e) => {
                return TranslationResult::err(
                    PROVIDER_ID,
                    "OpenAI 兼容",
                    format!("解析 LLM 响应失败: {e}"),
                );
            }
        };

        let content = completion
            .choices
            .first()
            .map(|c| message_content_to_string(&c.message.content))
            .unwrap_or_default();

        match parse_llm_content(&content) {
            Ok((text, phonetic, computer_sense, more_senses)) => TranslationResult::ok(
                PROVIDER_ID,
                "OpenAI 兼容",
                text,
                phonetic,
                computer_sense,
                more_senses,
                Some(req.source.clone()),
            )
            .normalize_senses(),
            Err(e) => {
                // 解析失败属预期高频场景（轻量模型不按格式输出）：只记可定位信息，
                // 不记录模型输出内容（用户译文）与 base_url（可能内嵌凭据）
                warn!("LLM JSON 解析失败: {}; model={}", e, config.model);
                TranslationResult::err(PROVIDER_ID, "OpenAI 兼容", e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rejects_broken_json_object() {
        let err = parse_llm_content(r#"{"text":"#).unwrap_err();
        assert!(err.contains("JSON") || err.contains("解析"), "err={err}");
    }

    #[test]
    fn parse_accepts_full_payload() {
        let raw = r#"{"text":"缓存","phonetic":"/kæʃ/","computerSense":"高速缓冲","moreSenses":[{"label":"v.","text":"存入缓存"}]}"#;
        let (text, ph, cs, more) = parse_llm_content(raw).unwrap();
        assert_eq!(text, "缓存");
        assert_eq!(ph.as_deref(), Some("/kæʃ/"));
        assert_eq!(cs.as_deref(), Some("高速缓冲"));
        assert_eq!(more.len(), 1);
    }

    #[test]
    fn parse_accepts_json_with_unescaped_newline_in_string() {
        // 部分模型会在 JSON 字符串值里直接换行，严格解析会报 control character
        let raw = "{\n  \"text\": \"hello\nworld\",\n  \"phonetic\": \"\"\n}";
        let (text, _, _, _) = parse_llm_content(raw).unwrap();
        assert_eq!(text, "hello\nworld");
    }

    #[test]
    fn parse_plain_text_fallback_when_no_json_object() {
        let (text, ph, cs, more) = parse_llm_content("Hello, world").unwrap();
        assert_eq!(text, "Hello, world");
        assert!(ph.is_none());
        assert!(cs.is_none());
        assert!(more.is_empty());
    }

    #[test]
    fn parse_extracts_json_after_think_tags() {
        let raw = r#"<think>先分析语气</think>
{"text":"你好","moreSenses":[{"label":"int.","text":"打招呼"}]}"#;
        let (text, _, _, more) = parse_llm_content(raw).unwrap();
        assert_eq!(text, "你好");
        assert_eq!(more.len(), 1);
    }

    #[test]
    fn parse_extracts_json_from_preamble_and_fence() {
        let raw = r#"译文如下：
```json
{"text":"缓存失效"}
```
"#;
        let (text, _, _, _) = parse_llm_content(raw).unwrap();
        assert_eq!(text, "缓存失效");
    }

    #[test]
    fn parse_rejects_empty_content() {
        let err = parse_llm_content("   ").unwrap_err();
        assert!(err.contains("空"), "err={err}");
    }

    #[test]
    fn parse_preserves_braces_in_plain_text() {
        // 纯文本译文含代码花括号：不得因提取到 `{ return 1; }` 而硬报错，应整体回退
        let (text, _, _, _) = parse_llm_content("fn f() { return 1; }").unwrap();
        assert_eq!(text, "fn f() { return 1; }");
    }

    #[test]
    fn parse_does_not_extract_nested_json_example() {
        // 说明文字里夹带的 JSON 示例不是译文：不得静默截断为示例中的 text
        let raw = r#"JSON 格式如 {"text":"abc"} 所示"#;
        let (text, _, _, _) = parse_llm_content(raw).unwrap();
        assert_eq!(text, raw);
    }

    #[test]
    fn parse_skips_json_fragment_inside_think() {
        // think 块内的 JSON 片段（缺 text）不得毒化解析，应继续扫描到真正的 payload
        let raw = r#"<think>{"unfinished": true}</think>
{"text":"你好"}"#;
        let (text, _, _, _) = parse_llm_content(raw).unwrap();
        assert_eq!(text, "你好");
    }

    #[test]
    fn parse_rejects_truncated_json_in_text() {
        // 文本中夹带残缺 JSON（花括号未闭合）：不得把半截 JSON 当译文返回
        let err = parse_llm_content(r#"译文：{"text":"broken"#).unwrap_err();
        assert!(err.contains("JSON") || err.contains("解析"), "err={err}");
    }

    #[test]
    fn parse_strips_think_tags_in_fallback() {
        // 纯文本回退路径同样剥离 think 块，思考内容不得混入译文
        let (text, _, _, _) = parse_llm_content("<think>先分析语气</think>你好").unwrap();
        assert_eq!(text, "你好");
    }

    #[test]
    fn parse_rejects_bare_fence_marker() {
        // 输出恰为 ``` 标记：fence 剥离后为空，按空内容报错而非返回空译文
        let err = parse_llm_content("```").unwrap_err();
        assert!(err.contains("空"), "err={err}");
    }

    #[test]
    fn message_content_to_string_handles_all_variants() {
        assert_eq!(message_content_to_string(&None), "");
        assert_eq!(
            message_content_to_string(&Some(MessageContent::Text("hi".into()))),
            "hi"
        );
        assert_eq!(
            message_content_to_string(&Some(MessageContent::Object(
                serde_json::json!({"text": "hi"})
            ))),
            r#"{"text":"hi"}"#
        );
        // 畸形值（数字/布尔/数组）不得被序列化后当作译文：按空串处理交给解析层报错
        assert_eq!(
            message_content_to_string(&Some(MessageContent::Object(serde_json::json!(42)))),
            ""
        );
        let parts = vec![
            ContentPart {
                text: Some("a".into()),
            },
            ContentPart { text: None },
            ContentPart {
                text: Some("b".into()),
            },
        ];
        assert_eq!(
            message_content_to_string(&Some(MessageContent::Array(parts))),
            "ab"
        );
    }

    #[tokio::test]
    async fn missing_config_returns_chinese_error() {
        let p = OpenAiCompatibleProvider::new(Arc::new(RwLock::new(LlmConfig::default())));
        let r = p
            .translate(&TranslateRequest {
                text: "hi".into(),
                source: "en".into(),
                target: "zh".into(),
            })
            .await;
        assert!(!r.is_success());
        assert!(r.error.as_deref().unwrap_or("").contains("设置"));
    }
}
