use std::collections::HashSet;

/// 解析后的翻译查询：正文 + 源/目标语言码。
///
/// 仅 TranslatorPlugin 内部使用，不跨 IPC；
/// 面板 JSON 由 plugin.rs 的 query_to_json 另行构造，键名为 camelCase。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedQuery {
    /// 待翻译的正文（触发词与语言码前缀已剥离）。
    pub text: String,
    /// 源语言码（`auto` 表示自动检测）。
    pub source: LanguageCode,
    /// 目标语言码。
    pub target: LanguageCode,
    /// 解析前的原始输入（用于面板回显）。
    pub raw: String,
}

pub type LanguageCode = String;

/// 翻译查询解析失败原因。
///
/// 仅 TranslatorPlugin 内部使用，不跨 IPC。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// 输入为空。
    EmptyText,
    /// 语言码不在当前启用引擎的能力目录中（携带非法码）。
    InvalidLanguageCode(String),
}

/// 由当前启用引擎的语言能力汇总而成的解析目录。
#[derive(Debug, Clone)]
pub struct LangCatalog {
    /// 小写码 → 规范写法
    map: std::collections::HashMap<String, String>,
}

impl LangCatalog {
    pub fn from_codes(codes: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let mut map = std::collections::HashMap::new();
        for c in codes {
            let raw = c.as_ref().to_string();
            map.entry(raw.to_ascii_lowercase()).or_insert(raw);
        }
        Self { map }
    }

    pub fn from_lowercase_set(keys: &HashSet<String>) -> Self {
        let mut map = std::collections::HashMap::new();
        for k in keys {
            map.entry(k.clone()).or_insert_with(|| k.clone());
        }
        Self { map }
    }

    pub fn contains(&self, code: &str) -> bool {
        self.map.contains_key(&code.to_ascii_lowercase())
    }

    pub fn canonicalize(&self, code: &str) -> Option<String> {
        self.map.get(&code.to_ascii_lowercase()).cloned()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// 解析插件模式下的 search_term（触发词已剥离）。
///
/// `catalog` 来自当前启用引擎的语言并集；语言码识别以目录成员为准，
/// 从而支持 `zh-TR` 等较长码。语言码一律以 `@` 前缀显式标记，
/// 裸首词始终按正文处理，避免 `it`、`go` 等英文词被误判为语言码。
///
/// - 无语言码：自动检测源语，目标为 `default_target`；若与源语相同则回退到另一常用语（zh↔en）
/// - 单语言码（`@目标`）：该码为目标语，源语自动检测
/// - 双语言码（`@源 @目标`）：源 + 目标；`@auto` 表示源语自动检测
pub fn parse_search_term(
    search_term: &str,
    default_target: &str,
    catalog: &LangCatalog,
) -> Result<ParsedQuery, ParseError> {
    let raw = search_term.to_string();
    let trimmed = search_term.trim();
    if trimmed.is_empty() {
        return Err(ParseError::EmptyText);
    }

    let tokens: Vec<&str> = trimmed.split_whitespace().collect();

    // 收集开头的 @ 语言码（最多两个，其余归入正文）
    let mut codes: Vec<&str> = Vec::new();
    let mut text_start = 0usize;
    for tok in tokens.iter() {
        match tok.strip_prefix('@') {
            Some(code) if codes.len() < 2 => {
                codes.push(code);
                text_start += 1;
            }
            _ => break,
        }
    }

    let text = tokens[text_start..].join(" ");
    if text.is_empty() {
        return Err(ParseError::EmptyText);
    }

    let (source, target) = match codes.as_slice() {
        [] => {
            let source = detect_source(&text);
            let target = resolve_auto_target(&source, default_target, catalog);
            (source, target)
        }
        [tgt] => (detect_source(&text), resolve_target_code(tgt, catalog)?),
        [src, tgt] => {
            let src = resolve_source_code(src, catalog)?;
            let tgt = resolve_target_code(tgt, catalog)?;
            let source = if src == "auto" {
                detect_source(&text)
            } else {
                src
            };
            (source, tgt)
        }
        _ => unreachable!("@ 语言码最多收集两个"),
    };

    Ok(ParsedQuery {
        text,
        source,
        target,
        raw,
    })
}

fn resolve_source_code(token: &str, catalog: &LangCatalog) -> Result<String, ParseError> {
    if eq_ignore_ascii(token, "auto") {
        return Ok("auto".into());
    }
    catalog
        .canonicalize(token)
        .ok_or_else(|| ParseError::InvalidLanguageCode(token.to_ascii_lowercase()))
}

fn resolve_target_code(token: &str, catalog: &LangCatalog) -> Result<String, ParseError> {
    if eq_ignore_ascii(token, "auto") {
        return Err(ParseError::InvalidLanguageCode("auto".into()));
    }
    catalog
        .canonicalize(token)
        .ok_or_else(|| ParseError::InvalidLanguageCode(token.to_ascii_lowercase()))
}

fn detect_source(text: &str) -> LanguageCode {
    if text.chars().any(|c| {
        ('\u{4e00}'..='\u{9fff}').contains(&c)
            || ('\u{3400}'..='\u{4dbf}').contains(&c)
            || ('\u{f900}'..='\u{faff}').contains(&c)
    }) {
        "zh".into()
    } else {
        "en".into()
    }
}

/// 无显式语言码时：优先使用设置中的默认目标语；若与源语相同则回退，避免同语种空转。
fn resolve_auto_target(source: &str, default_target: &str, catalog: &LangCatalog) -> LanguageCode {
    let preferred = catalog
        .canonicalize(default_target)
        .unwrap_or_else(|| default_target.to_string());

    if !source.eq_ignore_ascii_case(&preferred) {
        return preferred;
    }

    let fallback = match source {
        "zh" | "zh-TR" | "yue" => "en",
        "en" => "zh",
        _ => "en",
    };
    catalog
        .canonicalize(fallback)
        .unwrap_or_else(|| fallback.to_string())
}

fn eq_ignore_ascii(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn basic_catalog() -> LangCatalog {
        LangCatalog::from_codes(["zh", "en", "zh-TR", "ja"])
    }

    #[test]
    fn auto_detect_english() {
        let p = parse_search_term("hello", "zh", &basic_catalog()).unwrap();
        assert_eq!(p.text, "hello");
        assert_eq!(p.source, "en");
        assert_eq!(p.target, "zh");
        assert_eq!(p.raw, "hello");
    }

    #[test]
    fn auto_detect_chinese() {
        let p = parse_search_term("你好世界", "zh", &basic_catalog()).unwrap();
        assert_eq!(p.source, "zh");
        // 源语与默认目标相同 → 回退到 en
        assert_eq!(p.target, "en");
    }

    #[test]
    fn default_target_ja_for_english() {
        let c = LangCatalog::from_codes(["zh", "en", "ja"]);
        let p = parse_search_term("hello", "ja", &c).unwrap();
        assert_eq!(p.source, "en");
        assert_eq!(p.target, "ja");
    }

    #[test]
    fn default_target_ja_for_chinese() {
        let c = LangCatalog::from_codes(["zh", "en", "ja"]);
        let p = parse_search_term("你好", "ja", &c).unwrap();
        assert_eq!(p.source, "zh");
        assert_eq!(p.target, "ja");
    }

    #[test]
    fn single_lang_is_target() {
        let p = parse_search_term("@en 你好", "zh", &basic_catalog()).unwrap();
        assert_eq!(p.target, "en");
        assert_eq!(p.source, "zh");
        assert_eq!(p.text, "你好");
    }

    #[test]
    fn dual_lang() {
        let p = parse_search_term("@zh @en hello", "zh", &basic_catalog()).unwrap();
        assert_eq!(p.source, "zh");
        assert_eq!(p.target, "en");
        assert_eq!(p.text, "hello");
    }

    #[test]
    fn zh_tr_canonical() {
        let p = parse_search_term("@zh-tr hello", "zh", &basic_catalog()).unwrap();
        assert_eq!(p.target, "zh-TR");
        assert_eq!(p.text, "hello");
    }

    #[test]
    fn auto_source_explicit() {
        let p = parse_search_term("@auto @en 你好", "zh", &basic_catalog()).unwrap();
        assert_eq!(p.source, "zh");
        assert_eq!(p.target, "en");
        assert_eq!(p.text, "你好");
    }

    #[test]
    fn auto_as_target_rejected() {
        match parse_search_term("@auto hello", "zh", &basic_catalog()) {
            Err(ParseError::InvalidLanguageCode(code)) => assert_eq!(code, "auto"),
            other => panic!("未预期结果: {:?}", other),
        }
    }

    #[test]
    fn at_most_two_lang_codes() {
        // 第三个 @ token 归入正文
        let p = parse_search_term("@zh @en @fr hello", "zh", &basic_catalog()).unwrap();
        assert_eq!(p.source, "zh");
        assert_eq!(p.target, "en");
        assert_eq!(p.text, "@fr hello");
    }

    #[test]
    fn empty_and_lang_only_are_empty() {
        let c = basic_catalog();
        assert_eq!(parse_search_term("", "zh", &c), Err(ParseError::EmptyText));
        assert_eq!(
            parse_search_term("   ", "zh", &c),
            Err(ParseError::EmptyText)
        );
        assert_eq!(
            parse_search_term("@en", "zh", &c),
            Err(ParseError::EmptyText)
        );
        assert_eq!(
            parse_search_term("@zh @en", "zh", &c),
            Err(ParseError::EmptyText)
        );
        assert_eq!(
            parse_search_term("@auto @en", "zh", &c),
            Err(ParseError::EmptyText)
        );
    }

    #[test]
    fn invalid_lang_code() {
        match parse_search_term("@xx hello", "zh", &basic_catalog()) {
            Err(ParseError::InvalidLanguageCode(code)) => assert_eq!(code, "xx"),
            other => panic!("未预期结果: {:?}", other),
        }
    }

    /// 回归：含 it/id 等语言码的完整引擎目录下，英文句首词必须按正文处理。
    fn realistic_catalog() -> LangCatalog {
        LangCatalog::from_codes([
            "zh", "zh-TR", "yue", "en", "fr", "pt", "es", "ja", "tr", "ru", "ar", "ko", "th", "it",
            "de", "vi", "ms", "id",
        ])
    }

    #[test]
    fn english_first_word_it_not_lang_code() {
        let p = parse_search_term("it works", "zh", &realistic_catalog()).unwrap();
        assert_eq!(p.text, "it works");
        assert_eq!(p.source, "en");
        assert_eq!(p.target, "zh");
    }

    #[test]
    fn english_first_word_go_home_not_error() {
        let p = parse_search_term("go home", "zh", &realistic_catalog()).unwrap();
        assert_eq!(p.text, "go home");
        assert_eq!(p.target, "zh");
    }

    #[test]
    fn at_prefix_target_still_works() {
        let p = parse_search_term("@it works", "zh", &realistic_catalog()).unwrap();
        assert_eq!(p.target, "it");
        assert_eq!(p.text, "works");
        assert_eq!(p.source, "en");
    }

    #[test]
    fn multi_word_plain_text_two_tokens() {
        let p = parse_search_term("hello world", "zh", &basic_catalog()).unwrap();
        assert_eq!(p.text, "hello world");
        assert_eq!(p.source, "en");
        assert_eq!(p.target, "zh");
    }

    #[test]
    fn multi_word_plain_text_three_tokens() {
        let p = parse_search_term("hello world foo", "zh", &basic_catalog()).unwrap();
        assert_eq!(p.text, "hello world foo");
        assert_eq!(p.source, "en");
        assert_eq!(p.target, "zh");
    }

    #[test]
    fn unsupported_lang_not_in_catalog() {
        match parse_search_term("@ko hello", "zh", &basic_catalog()) {
            Err(ParseError::InvalidLanguageCode(code)) => assert_eq!(code, "ko"),
            other => panic!("未预期结果: {:?}", other),
        }
    }
}
