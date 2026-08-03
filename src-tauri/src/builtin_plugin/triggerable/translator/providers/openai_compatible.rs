use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::Deserialize;
use tracing::error;

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

/// 回复消息（仅取 content 字段）。
///
/// 仅限本文件内使用。
#[derive(Debug, Deserialize)]
struct ChatMessage {
    /// 回复正文（应为 JSON 字符串）。
    content: String,
}

/// 解析 LLM 返回的 JSON 正文（支持 camelCase 字段名）。
#[allow(clippy::type_complexity)]
pub fn parse_llm_content(
    content: &str,
) -> Result<(String, Option<String>, Option<String>, Vec<SenseEntry>), String> {
    let trimmed = content.trim();
    let json_str = strip_markdown_fence(trimmed);
    let payload: LlmTranslationPayload =
        serde_json::from_str(json_str).map_err(|e| format!("JSON 解析失败: {e}"))?;
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

fn strip_markdown_fence(s: &str) -> &str {
    let s = s.trim();
    if !s.starts_with("```") {
        return s;
    }
    // 去掉尾部 ```
    let s = s.strip_suffix("```").unwrap_or(s).trim();
    // 去掉前导 ```
    let after_fence = s.trim_start_matches('`').trim();
    // 去掉可选的 json 标记（大小写不敏感）
    if after_fence.to_lowercase().starts_with("json") {
        after_fence[4..].trim()
    } else {
        after_fence
    }
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
            .map(|c| c.message.content.as_str())
            .unwrap_or("");

        match parse_llm_content(content) {
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
            Err(e) => TranslationResult::err(PROVIDER_ID, "OpenAI 兼容", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rejects_non_json() {
        assert!(parse_llm_content("不是json").is_err());
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
