use async_trait::async_trait;

use super::super::provider::{
    LanguageSupport, SenseEntry, TranslateRequest, TranslationProvider, TranslationResult,
};

pub const PROVIDER_ID: &str = "mock";
pub const PROVIDER_NAME: &str = "模拟示例";

/// 本地联调引擎：真正译文由 registry 从其它引擎复制；此处仅提供无来源时的占位。
pub struct MockProvider;

/// 无其它引擎成功结果可复制时的固定占位。
pub fn placeholder_result() -> TranslationResult {
    TranslationResult::ok(
        PROVIDER_ID,
        PROVIDER_NAME,
        "模拟示例占位译文",
        Some("/ˈmɒk/".into()),
        Some("仅用于多引擎界面联调".into()),
        vec![
            SenseEntry {
                text: "mock translation".into(),
                label: Some("en".into()),
            },
            SenseEntry {
                text: "示例释义".into(),
                label: Some("示".into()),
            },
        ],
        None,
    )
    .normalize_senses()
}

/// 将其它引擎的成功结果改挂为 mock（仅改 id/name）。
pub fn mirror_from(source: &TranslationResult) -> TranslationResult {
    TranslationResult {
        provider_id: PROVIDER_ID.into(),
        provider_name: PROVIDER_NAME.into(),
        text: source.text.clone(),
        phonetic: source.phonetic.clone(),
        computer_sense: source.computer_sense.clone(),
        more_senses: source.more_senses.clone(),
        detected_source: source.detected_source.clone(),
        error: None,
    }
}

#[async_trait]
impl TranslationProvider for MockProvider {
    fn id(&self) -> &str {
        PROVIDER_ID
    }

    fn name(&self) -> &str {
        PROVIDER_NAME
    }

    fn language_support(&self) -> LanguageSupport {
        LanguageSupport::bilingual(&[
            "zh", "zh-TR", "yue", "en", "fr", "pt", "es", "ja", "tr", "ru", "ar", "ko", "th", "it",
            "de", "vi", "ms", "id",
        ])
    }

    async fn translate(&self, _req: &TranslateRequest) -> TranslationResult {
        // 权威结果由 registry 后处理覆盖；此处仅兜底。
        placeholder_result()
    }
}
