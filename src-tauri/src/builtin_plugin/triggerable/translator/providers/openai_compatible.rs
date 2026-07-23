use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::Deserialize;

use super::super::provider::{
    LanguageSupport, SenseEntry, TranslateRequest, TranslationProvider, TranslationResult,
};

pub const PROVIDER_ID: &str = "openai-compatible";

#[derive(Debug, Clone, Default)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

pub const DEFAULT_TRANSLATION_SYSTEM_PROMPT: &str = r#"你是专业翻译助手。根据用户给出的源语言、目标语言与原文，输出且仅输出一个 JSON 对象，不要 markdown 代码块，不要额外说明。

JSON 字段（camelCase）：
- text（string，必填）：主译文
- phonetic（string，可选）：音标或读音
- computerSense（string，可选）：计算机/IT 领域释义
- moreSenses（array，可选，最多 4 条）：更多释义，每项含 label（可选，如词性/领域）与 text（string）

示例：{"text":"缓存","phonetic":"/kæʃ/","computerSense":"高速缓冲","moreSenses":[{"label":"v.","text":"存入缓存"}]}"#;

const SUPPORTED_LANGUAGES: &[&str] = &[
    "zh", "zh-TR", "yue", "en", "fr", "pt", "es", "ja", "tr", "ru", "ar", "ko", "th", "it", "de",
    "vi", "ms", "id",
];

pub struct OpenAiCompatibleProvider {
    config: Arc<RwLock<LlmConfig>>,
}

impl OpenAiCompatibleProvider {
    pub fn new(config: Arc<RwLock<LlmConfig>>) -> Self {
        Self { config }
    }
}

#[derive(Debug, Deserialize)]
struct LlmTranslationPayload {
    text: String,
    #[serde(default)]
    phonetic: Option<String>,
    #[serde(default, rename = "computerSense")]
    computer_sense: Option<String>,
    #[serde(default, rename = "moreSenses")]
    more_senses: Vec<LlmSenseEntry>,
}

#[derive(Debug, Deserialize)]
struct LlmSenseEntry {
    text: String,
    #[serde(default)]
    label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: String,
}

/// 解析 LLM 返回的 JSON 正文（支持 camelCase 字段名）。
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
    let inner = s.trim_start_matches('`').trim_start_matches("json").trim();
    inner
        .strip_suffix("```")
        .map(str::trim)
        .unwrap_or(inner)
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

        let url = format!(
            "{}/chat/completions",
            config.base_url.trim().trim_end_matches('/')
        );
        let body = serde_json::json!({
            "model": config.model,
            "stream": false,
            "messages": [
                {"role": "system", "content": DEFAULT_TRANSLATION_SYSTEM_PROMPT},
                {"role": "user", "content": build_user_message(req)},
            ],
        });

        let client = match reqwest::Client::builder().build() {
            Ok(c) => c,
            Err(e) => {
                return TranslationResult::err(
                    PROVIDER_ID,
                    "OpenAI 兼容",
                    format!("创建 HTTP 客户端失败: {e}"),
                );
            }
        };

        let response = match client
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
            let detail = response.text().await.unwrap_or_default();
            return TranslationResult::err(
                PROVIDER_ID,
                "OpenAI 兼容",
                format!("LLM 服务返回错误 ({status}): {detail}"),
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
