use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::query_parser::LanguageCode;

/// 一次翻译请求。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslateRequest {
    pub text: String,
    pub source: LanguageCode,
    pub target: LanguageCode,
}

/// 更多释义条目（含可选领域标签）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SenseEntry {
    pub text: String,
    pub label: Option<String>,
}

/// 更多释义最多保留条数。
pub const MAX_MORE_SENSES: usize = 4;

/// 单个引擎的翻译结果（成功或失败均用同一结构表示）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslationResult {
    pub provider_id: String,
    pub provider_name: String,
    pub text: String,
    pub phonetic: Option<String>,
    pub computer_sense: Option<String>,
    pub more_senses: Vec<SenseEntry>,
    pub detected_source: Option<LanguageCode>,
    pub error: Option<String>,
}

impl TranslationResult {
    pub fn ok(
        provider_id: impl Into<String>,
        provider_name: impl Into<String>,
        text: impl Into<String>,
        phonetic: Option<String>,
        computer_sense: Option<String>,
        more_senses: Vec<SenseEntry>,
        detected_source: Option<LanguageCode>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            provider_name: provider_name.into(),
            text: text.into(),
            phonetic,
            computer_sense,
            more_senses,
            detected_source,
            error: None,
        }
    }

    pub fn err(
        provider_id: impl Into<String>,
        provider_name: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            provider_name: provider_name.into(),
            text: String::new(),
            phonetic: None,
            computer_sense: None,
            more_senses: Vec::new(),
            detected_source: None,
            error: Some(error.into()),
        }
    }

    /// 规范化释义字段：截断更多释义、空白音标/计算机释义置为 None。
    pub fn normalize_senses(mut self) -> Self {
        if self.more_senses.len() > MAX_MORE_SENSES {
            self.more_senses.truncate(MAX_MORE_SENSES);
        }
        self.phonetic = non_empty_opt(self.phonetic.take());
        self.computer_sense = non_empty_opt(self.computer_sense.take());
        self
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none() && !self.text.is_empty()
    }
}

fn non_empty_opt(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// 某引擎在**当前配置**下支持的语言能力（随模型/参数变化）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LanguageSupport {
    /// 源语言规范码（如 `zh`、`zh-TR`）
    pub sources: Vec<String>,
    /// 目标语言规范码
    pub targets: Vec<String>,
}

impl LanguageSupport {
    /// 源/目标使用同一份语言列表（多数翻译引擎如此）。
    pub fn bilingual(codes: &[&str]) -> Self {
        let list: Vec<String> = codes.iter().map(|c| (*c).to_string()).collect();
        Self {
            sources: list.clone(),
            targets: list,
        }
    }

    pub fn supports_source(&self, code: &str) -> bool {
        eq_ignore_ascii(code, "auto")
            || self
                .sources
                .iter()
                .any(|c| eq_ignore_ascii(c, code))
    }

    pub fn supports_target(&self, code: &str) -> bool {
        self.targets.iter().any(|c| eq_ignore_ascii(c, code))
    }

    pub fn supports_pair(&self, source: &str, target: &str) -> bool {
        self.supports_source(source) && self.supports_target(target)
    }

    /// 将用户输入规范为引擎使用的写法；未知则返回 None。
    pub fn canonicalize(&self, code: &str) -> Option<String> {
        if eq_ignore_ascii(code, "auto") {
            return Some("auto".into());
        }
        self.sources
            .iter()
            .chain(self.targets.iter())
            .find(|c| eq_ignore_ascii(c, code))
            .cloned()
    }

    /// 合并多个引擎的语言能力（取并集，保留首次出现的规范写法）。
    pub fn union(items: impl IntoIterator<Item = LanguageSupport>) -> Self {
        let mut source_map: HashMap<String, String> = HashMap::new();
        let mut target_map: HashMap<String, String> = HashMap::new();
        for item in items {
            for s in item.sources {
                source_map
                    .entry(s.to_ascii_lowercase())
                    .or_insert(s);
            }
            for t in item.targets {
                target_map
                    .entry(t.to_ascii_lowercase())
                    .or_insert(t);
            }
        }
        let mut sources: Vec<_> = source_map.into_values().collect();
        let mut targets: Vec<_> = target_map.into_values().collect();
        sources.sort();
        targets.sort();
        Self { sources, targets }
    }

    /// 供查询解析使用的小写语言码集合（不含 auto）。
    pub fn catalog_keys(&self) -> HashSet<String> {
        self.sources
            .iter()
            .chain(self.targets.iter())
            .map(|c| c.to_ascii_lowercase())
            .collect()
    }
}

fn eq_ignore_ascii(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// 可插拔翻译引擎契约；后续新增引擎只需实现本 trait 并注册进 Registry。
///
/// `language_support` **必须**反映当前运行时配置（如 LLM 模型或 API 区域），
/// 以便解析校验与面板选项随参数变更。
#[async_trait]
pub trait TranslationProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;

    /// 当前配置下支持的源/目标语言。
    fn language_support(&self) -> LanguageSupport;

    async fn translate(&self, req: &TranslateRequest) -> TranslationResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_truncates_more_senses_to_four() {
        let senses: Vec<_> = (0..6)
            .map(|i| SenseEntry {
                text: format!("s{i}"),
                label: None,
            })
            .collect();
        let r = TranslationResult::ok("p", "P", "主译", None, None, senses, None)
            .normalize_senses();
        assert_eq!(r.more_senses.len(), 4);
    }

    #[test]
    fn normalize_empty_phonetic_to_none() {
        let r = TranslationResult::ok(
            "p",
            "P",
            "主译",
            Some("  ".into()),
            Some("".into()),
            vec![],
            None,
        )
        .normalize_senses();
        assert!(r.phonetic.is_none());
        assert!(r.computer_sense.is_none());
    }
}
