//! 查询形态检测插件（内置）——把"输入看起来像路径/网址"变成一次插件接管。
//!
//! 两个检测器共用同一前端面板类型 `smart-target`（`src-ui/plugins/built-in/smart-target`）：
//! - `path-detect`：盘符绝对路径 / UNC / 相对路径 / `%VAR%` 形态 → 打开目标或所在文件夹；
//! - `url-detect`：带 scheme 或（可配置的）裸域名形态 → 用默认浏览器打开。
//!
//! 匹配入口统一为 `Plugin::match_query`（覆盖框架默认的关键词判定）：
//! 判定逻辑全在插件内，宿主按优先级并发裁决。
//!
//! 面板数据契约（前端 `smart-target` provider 消费）：
//! ```json
//! {
//!   "kind": "path" | "url",
//!   "target": "展开后的执行目标",
//!   "title": "<i18n key 或字面量>",
//!   "subtitle": "展示用原文",
//!   "icon": "<data URL>" | null
//! }
//! ```

pub mod path_detect;
pub mod url_detect;

/// 检测器共用的前端面板类型标识（前端按 `FrontendPlugin.matchType` 匹配渲染组件）。
pub(crate) const PANEL_TYPE: &str = "smart-target";

/// 已提交的检测输入 —— 尾部空格提交信号（或引号形态）归一化后的内容。
///
/// 使用范围：仅 `path_detect` / `url_detect` 的形态判定与面板构造（`match_query` / `query`）。
pub(crate) struct CommittedInput {
    /// 归一化后的内容：去掉尾部空白；引号形态下额外去掉外层引号。
    pub content: String,
    /// 是否来自 `"…"` 引号形态（引号内允许空格；非引号内容按"路径/网址不含空格"处理）。
    pub quoted: bool,
}

/// 归一化输入并判定是否"已提交"。
///
/// 提交语义（与其他行内插件的"触发词 + 空格"约定对齐）：输入以空格结尾表示用户已敲定；
/// 形如 `"…"`（引号成对）的输入视为已提交，允许内含空格。
/// 前置条件与宿主一致：`SessionDispatcher::locate_plugin` 仅在输入含空格时才询问本判定，
/// 故不含空格的输入（含无空格的引号形态）在此同样不提交，避免两侧对同一输入结论相反。
/// 参数：`raw` - 用户原始输入。返回：已提交时返回归一化内容，否则 `None`（不接管）。
pub(crate) fn committed_input(raw: &str) -> Option<CommittedInput> {
    if !raw.contains(' ') {
        return None;
    }
    // 两侧空白均为噪声：首尾任一空白都被视为用户敲过空格的提交信号
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(inner) = strip_quotes(trimmed) {
        return Some(CommittedInput {
            content: inner.to_string(),
            quoted: true,
        });
    }
    if trimmed.len() < raw.len() {
        return Some(CommittedInput {
            content: trimmed.to_string(),
            quoted: false,
        });
    }
    None
}

/// 去掉成对的外层双引号；不成对/不完整时返回 `None`。
fn strip_quotes(text: &str) -> Option<&str> {
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    (!inner.is_empty()).then_some(inner)
}

#[cfg(test)]
mod tests {
    use super::committed_input;

    /// 尾部空格 = 提交信号；无尾空格的裸输入不提交。
    #[test]
    fn trailing_space_commits_input() {
        let committed = committed_input("C:\\Users\\ ").expect("尾空格应视为已提交");
        assert_eq!(committed.content, "C:\\Users\\");
        assert!(!committed.quoted);

        let multi = committed_input("C:\\Users\\   ").expect("多个尾空格同样提交");
        assert_eq!(multi.content, "C:\\Users\\");

        assert!(committed_input("C:\\Users\\").is_none());
        assert!(committed_input("   ").is_none());
    }

    /// 引号形态：无论是否带尾空格都提交，且允许内含空格。
    #[test]
    fn quoted_input_commits_with_spaces() {
        let quoted = committed_input("\"C:\\Program Files\"").expect("引号形态应提交");
        assert_eq!(quoted.content, "C:\\Program Files");
        assert!(quoted.quoted);

        let with_space = committed_input("\"C:\\Program Files\" ").expect("引号 + 尾空格应提交");
        assert_eq!(with_space.content, "C:\\Program Files");
        assert!(with_space.quoted);
    }

    /// 未闭合/空引号不构成提交形态。
    #[test]
    fn unbalanced_quotes_are_not_quoted_form() {
        assert!(committed_input("\"C:\\Program Files").is_none());
        assert!(committed_input("\"\"").is_none());
    }

    /// 无空格的输入（含无空格的引号形态）不提交——与宿主路由前置门一致；
    /// 补一个空格后即恢复提交语义。
    #[test]
    fn input_without_space_never_commits() {
        assert!(committed_input("\"C:\\Users\"").is_none());
        assert!(committed_input("C:\\Users").is_none());
        assert!(committed_input("\"C:\\Users\" ").is_some());
    }
}
