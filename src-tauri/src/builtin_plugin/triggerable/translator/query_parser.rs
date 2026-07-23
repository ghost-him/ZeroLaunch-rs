use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedQuery {
    pub text: String,
    pub source: LanguageCode,
    pub target: LanguageCode,
    pub raw: String,
}

pub type LanguageCode = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    EmptyText,
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
/// 从而支持 `zh-TR` 等较长码，并避免把普通英文词误判为语言码。
///
/// - 无语言码：自动检测源语，目标为 `default_target`；若与源语相同则回退到另一常用语（zh↔en）
/// - 单语言码：该码为目标语，源语自动检测
/// - 双语言码：源 + 目标
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

    // 尝试双语言码
    if tokens.len() >= 3 && is_lang_token(tokens[0], catalog) && is_lang_token(tokens[1], catalog) {
        let src = resolve_source_code(tokens[0], catalog)?;
        let tgt = resolve_target_code(tokens[1], catalog)?;
        let text = tokens[2..].join(" ");
        if text.is_empty() {
            return Err(ParseError::EmptyText);
        }
        let source = if src == "auto" {
            detect_source(&text)
        } else {
            src
        };
        return Ok(ParsedQuery {
            text,
            source,
            target: tgt,
            raw,
        });
    }

    // 双语言码无正文
    if tokens.len() == 2 && is_lang_token(tokens[0], catalog) && is_lang_token(tokens[1], catalog) {
        let _src = resolve_source_code(tokens[0], catalog)?;
        let _tgt = resolve_target_code(tokens[1], catalog)?;
        return Err(ParseError::EmptyText);
    }

    // 尝试单语言码（目标语）
    if tokens.len() >= 2 && is_lang_token(tokens[0], catalog) {
        let tgt = resolve_target_code(tokens[0], catalog)?;
        let text = tokens[1..].join(" ");
        if text.is_empty() {
            return Err(ParseError::EmptyText);
        }
        let source = detect_source(&text);
        return Ok(ParsedQuery {
            text,
            source,
            target: tgt,
            raw,
        });
    }

    // 形似语言码但不在目录中 → 非法语言码（如 xx）
    if tokens.len() >= 2 && looks_like_lang_token(tokens[0]) && !catalog.contains(tokens[0]) {
        return Err(ParseError::InvalidLanguageCode(
            tokens[0].to_ascii_lowercase(),
        ));
    }

    // 纯文本 / 单 token 语言码无正文
    if tokens.len() == 1 && is_lang_token(tokens[0], catalog) {
        return Err(ParseError::EmptyText);
    }

    let text = trimmed.to_string();
    let source = detect_source(&text);
    let target = resolve_auto_target(&source, default_target, catalog);
    Ok(ParsedQuery {
        text,
        source,
        target,
        raw,
    })
}

fn is_lang_token(token: &str, catalog: &LangCatalog) -> bool {
    eq_ignore_ascii(token, "auto") || catalog.contains(token)
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

/// 语言码形态：2–3 位字母，可选 `-{2,8}` 后缀（如 zh-TR）。
fn looks_like_lang_token(token: &str) -> bool {
    let t = token.to_ascii_lowercase();
    let parts: Vec<&str> = t.split('-').collect();
    match parts.as_slice() {
        [a] => (2..=3).contains(&a.len()) && a.chars().all(|c| c.is_ascii_alphabetic()),
        [a, b] => {
            (2..=3).contains(&a.len())
                && a.chars().all(|c| c.is_ascii_alphabetic())
                && (2..=8).contains(&b.len())
                && b.chars().all(|c| c.is_ascii_alphabetic())
        }
        _ => false,
    }
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
        let p = parse_search_term("en 你好", "zh", &basic_catalog()).unwrap();
        assert_eq!(p.target, "en");
        assert_eq!(p.source, "zh");
        assert_eq!(p.text, "你好");
    }

    #[test]
    fn dual_lang() {
        let p = parse_search_term("zh en hello", "zh", &basic_catalog()).unwrap();
        assert_eq!(p.source, "zh");
        assert_eq!(p.target, "en");
        assert_eq!(p.text, "hello");
    }

    #[test]
    fn zh_tr_canonical() {
        let p = parse_search_term("zh-tr hello", "zh", &basic_catalog()).unwrap();
        assert_eq!(p.target, "zh-TR");
        assert_eq!(p.text, "hello");
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
            parse_search_term("en", "zh", &c),
            Err(ParseError::EmptyText)
        );
        assert_eq!(
            parse_search_term("zh en", "zh", &c),
            Err(ParseError::EmptyText)
        );
    }

    #[test]
    fn invalid_lang_code() {
        match parse_search_term("xx hello", "zh", &basic_catalog()) {
            Err(ParseError::InvalidLanguageCode(code)) => assert_eq!(code, "xx"),
            other => panic!("未预期结果: {:?}", other),
        }
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
        match parse_search_term("ko hello", "zh", &basic_catalog()) {
            Err(ParseError::InvalidLanguageCode(code)) => assert_eq!(code, "ko"),
            other => panic!("未预期结果: {:?}", other),
        }
    }
}
