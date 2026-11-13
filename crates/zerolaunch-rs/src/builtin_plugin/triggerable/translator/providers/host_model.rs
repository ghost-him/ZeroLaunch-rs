use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::Deserialize;
use tracing::warn;
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::model::{
    ModelChatMessage, ModelChatRequest, ModelChatRole, ModelError,
};

use super::super::provider::{
    LanguageSupport, SenseEntry, TranslateRequest, TranslationProvider, TranslationResult,
};

pub const PROVIDER_ID: &str = "host-model";

/// 宿主模型 LLM 翻译引擎 Provider。
///
/// 经 PluginHandle 调用宿主统一模型服务（ModelManager）的 chat 能力，
/// 将模型输出解析为统一翻译结果。语言能力为静态双语列表（与旧自研引擎一致）。
pub struct HostModelProvider {
    /// 宿主插件句柄（init 时由 TranslatorPlugin 注入），提供 model_chat 能力。
    handle: RwLock<Option<Arc<PluginHandle>>>,
    /// 当前选中的宿主模型全局 id（如 `openai/gpt-4o-mini`），随设置同步。
    model_id: RwLock<String>,
}

impl HostModelProvider {
    pub fn new() -> Self {
        Self {
            handle: RwLock::new(None),
            model_id: RwLock::new(String::new()),
        }
    }

    /// 注入插件句柄（init 时调用一次）。
    pub fn set_handle(&self, handle: Arc<PluginHandle>) {
        *self.handle.write() = Some(handle);
    }

    /// 同步当前选中的模型 id（apply_settings 时调用）。
    pub fn set_model_id(&self, model_id: &str) {
        *self.model_id.write() = model_id.to_string();
    }
}

impl Default for HostModelProvider {
    fn default() -> Self {
        Self::new()
    }
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

/// LLM 返回 JSON 的主负载（camelCase 字段与提示词约定一致）。
///
/// 仅限本文件内使用；用于解析宿主模型 chat 输出的 content。
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

/// 解析成功时的负载：(text, phonetic, computer_sense, more_senses)。
type ParsedLlmFields = (String, Option<String>, Option<String>, Vec<SenseEntry>);

/// 解析模型返回的 JSON 正文（支持 camelCase 字段名）。
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

/// 将 `ModelError` 映射为面向用户的中文错误文案（复用旧引擎的错误语义）。
fn model_error_message(e: &ModelError) -> String {
    match e {
        ModelError::NotSupported => "模型提供方不支持该操作".into(),
        ModelError::ModelNotFound(id) => format!("模型不存在: {id}"),
        ModelError::ProviderUnavailable(detail) => format!("模型提供方不可用: {detail}"),
        ModelError::InvalidRequest(detail) => format!("模型请求参数非法: {detail}"),
        ModelError::Transport(detail) => format!("模型传输失败: {detail}"),
        ModelError::Internal(detail) => format!("模型内部错误: {detail}"),
    }
}

#[async_trait]
impl TranslationProvider for HostModelProvider {
    fn id(&self) -> &str {
        PROVIDER_ID
    }

    fn name(&self) -> &str {
        "宿主模型"
    }

    fn language_support(&self) -> LanguageSupport {
        LanguageSupport::bilingual(SUPPORTED_LANGUAGES)
    }

    async fn translate(&self, req: &TranslateRequest) -> TranslationResult {
        let model_id = self.model_id.read().clone();
        if model_id.trim().is_empty() {
            return TranslationResult::err(PROVIDER_ID, "宿主模型", "请在设置中选择翻译模型");
        }

        let handle = match self.handle.read().clone() {
            Some(h) => h,
            None => {
                return TranslationResult::err(
                    PROVIDER_ID,
                    "宿主模型",
                    "插件服务句柄不可用，请重启应用后重试",
                );
            }
        };

        let chat_req = ModelChatRequest {
            model_id,
            messages: vec![
                ModelChatMessage::text(ModelChatRole::System, DEFAULT_TRANSLATION_SYSTEM_PROMPT),
                ModelChatMessage::text(ModelChatRole::User, build_user_message(req)),
            ],
            temperature: None,
            max_tokens: None,
            reasoning_effort: None,
            tools: Vec::new(),
        };

        let resp = match handle.model_chat(chat_req).await {
            Ok(r) => r,
            Err(e) => {
                warn!("宿主模型翻译失败: {}; model={}", e, req.text.len());
                return TranslationResult::err(
                    PROVIDER_ID,
                    "宿主模型",
                    format!("模型调用失败: {}", model_error_message(&e)),
                );
            }
        };

        match parse_llm_content(&resp.content) {
            Ok((text, phonetic, computer_sense, more_senses)) => TranslationResult::ok(
                PROVIDER_ID,
                "宿主模型",
                text,
                phonetic,
                computer_sense,
                more_senses,
                // 仅显式源语言回传；auto（自动检测）时模型不回报检测结果，置 None。
                (!req.source.eq_ignore_ascii_case("auto")).then(|| req.source.clone()),
            )
            .normalize_senses(),
            Err(e) => {
                // 解析失败属预期高频场景（轻量模型不按格式输出）：只记可定位信息，
                // 不记录模型输出内容（用户译文）
                warn!("模型 JSON 解析失败: {}; model={}", e, req.text.len());
                TranslationResult::err(PROVIDER_ID, "宿主模型", e)
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

    #[tokio::test]
    async fn missing_model_returns_chinese_error() {
        let p = HostModelProvider::new();
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

    #[tokio::test]
    async fn missing_handle_returns_chinese_error() {
        let p = HostModelProvider::new();
        p.set_model_id("openai/gpt-4o-mini");
        let r = p
            .translate(&TranslateRequest {
                text: "hi".into(),
                source: "en".into(),
                target: "zh".into(),
            })
            .await;
        assert!(!r.is_success());
        assert!(r.error.as_deref().unwrap_or("").contains("句柄"));
    }
}
