//! 关键词触发判定 —— 宿主侧关键词路由与插件侧默认实现的**唯一**语义定义处。
//!
//! 使用范围：宿主插件路由（`SessionDispatcher` 的关键词兜底路径）与内置/第三方插件的
//! `Plugin::match_query` 默认语义。放在 plugin-api 是为了让宿主与 SDK 两侧共用同一份
//! 判定实现，避免"宿主一份、插件一份"的语义漂移。

/// 关键词触发判定（纯函数，无 IO）。
///
/// 语义：输入按首个空格切分，首词小写后必须精确命中 `keywords` 中的某一项（同样按小写比较），
/// 且必须存在剩余内容（单独的触发词不算命中——`"fy"` 不命中，`"fy hello"` 命中）。
///
/// 参数：
/// - `keywords` - 插件声明的触发词列表（大小写不敏感）。
/// - `raw_query` - 用户输入的原始查询串。
///
/// 返回：命中时返回剥离触发词后的剩余输入（`"fy hello"` → `"hello"`）；未命中返回 `None`。
pub fn keyword_trigger_match<'a>(keywords: &[String], raw_query: &'a str) -> Option<&'a str> {
    let mut parts = raw_query.splitn(2, ' ');
    let first = parts.next().unwrap_or("");
    let rest = parts.next()?;
    let first_lower = first.to_lowercase();
    keywords
        .iter()
        .any(|kw| kw.to_lowercase() == first_lower)
        .then_some(rest)
}

#[cfg(test)]
mod tests {
    use super::keyword_trigger_match;

    /// 触发词必须带空格分隔：`"fy hello"` 命中，单独的 `"fy"` 与粘连的 `"fyhello"` 不命中。
    #[test]
    fn requires_space_separated_trigger() {
        let keywords = vec!["fy".to_string()];
        assert_eq!(keyword_trigger_match(&keywords, "fy hello"), Some("hello"));
        assert_eq!(keyword_trigger_match(&keywords, "fy"), None);
        assert_eq!(keyword_trigger_match(&keywords, "fyhello"), None);
        assert_eq!(keyword_trigger_match(&keywords, "xx hello"), None);
    }

    /// 大小写不敏感：声明 `Translate` 时 `translate xxx` 与 `TRANSLATE xxx` 均命中。
    #[test]
    fn case_insensitive() {
        let keywords = vec!["Translate".to_string()];
        assert_eq!(keyword_trigger_match(&keywords, "translate hi"), Some("hi"));
        assert_eq!(keyword_trigger_match(&keywords, "TRANSLATE hi"), Some("hi"));
    }

    /// 无触发词声明时恒不命中（未声明触发词的行内插件不参与关键词路由）。
    #[test]
    fn empty_keywords_never_match() {
        assert_eq!(keyword_trigger_match(&[], "fy hello"), None);
    }
}
