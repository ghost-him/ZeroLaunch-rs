//! 模板解析器 - 解析模板字符串中的参数占位符

use crate::sdk::parameter::types::SystemParameter;

/// 模板中的占位符类型
///
/// 解析模板后产生的占位符序列，按出现顺序排列。
/// 新设计使用枚举替代旧版的带位置信息的 struct，从左到右顺序构建结果字符串，
/// 避免旧方案中从后往前替换时的位置偏移问题。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Placeholder {
    /// 纯文本片段
    Text(String),
    /// 用户提供的位置参数 {}
    UserArg,
    /// 系统提供的参数 {clip} {hwnd} {selection}
    System(SystemParameter),
}

/// 模板解析器
///
/// 负责将模板字符串解析为占位符序列，供 ParameterResolver 使用。
pub(crate) struct TemplateParser;

impl TemplateParser {
    /// 解析模板，返回占位符序列
    ///
    /// 支持的占位符格式：
    /// - `{}` - 位置参数
    /// - `{clip}` - 剪贴板内容
    /// - `{hwnd}` - 窗口句柄
    /// - `{selection}` - 选中文本
    pub fn parse(template: &str) -> Vec<Placeholder> {
        let mut placeholders = Vec::new();
        let chars: Vec<char> = template.chars().collect();
        let mut i = 0;
        let mut text_start = 0;

        while i < chars.len() {
            if chars[i] == '{' {
                // 找到左大括号，先保存之前的文本
                if i > text_start {
                    let text: String = chars[text_start..i].iter().collect();
                    placeholders.push(Placeholder::Text(text));
                }

                // 查找右大括号
                if let Some(end) = Self::find_closing_brace(&chars, i) {
                    let content: String = chars[i + 1..end].iter().collect();

                    let placeholder = if content.is_empty() {
                        // {} - 位置参数
                        Placeholder::UserArg
                    } else if let Some(sys_param) = SystemParameter::from_name(&content) {
                        // {clip} / {hwnd} / {selection} - 系统参数
                        Placeholder::System(sys_param)
                    } else {
                        // 未知参数名，当作位置参数处理
                        Placeholder::UserArg
                    };

                    placeholders.push(placeholder);
                    i = end + 1;
                    text_start = i;
                    continue;
                }
            }
            i += 1;
        }

        // 保存末尾的文本
        if text_start < chars.len() {
            let text: String = chars[text_start..].iter().collect();
            placeholders.push(Placeholder::Text(text));
        }

        placeholders
    }

    /// 查找匹配的右大括号
    fn find_closing_brace(chars: &[char], start: usize) -> Option<usize> {
        ((start + 1)..chars.len()).find(|&i| chars[i] == '}')
    }

    /// 统计模板中需要用户输入的参数数量（只统计位置参数）
    pub fn count_user_args(template: &str) -> usize {
        Self::parse(template)
            .iter()
            .filter(|p| matches!(p, Placeholder::UserArg))
            .count()
    }

    /// 检查模板是否包含系统参数
    pub fn has_system_params(template: &str) -> bool {
        Self::parse(template)
            .iter()
            .any(|p| matches!(p, Placeholder::System(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_template() {
        let placeholders = TemplateParser::parse("");
        assert_eq!(placeholders.len(), 0);
    }

    #[test]
    fn test_parse_no_parameters() {
        let placeholders = TemplateParser::parse("notepad.exe test.txt");
        assert_eq!(placeholders.len(), 1);
        assert_eq!(
            placeholders[0],
            Placeholder::Text("notepad.exe test.txt".to_string())
        );
    }

    #[test]
    fn test_parse_positional_parameter() {
        let placeholders = TemplateParser::parse("cmd /c echo {}");
        assert_eq!(placeholders.len(), 2);
        assert_eq!(
            placeholders[0],
            Placeholder::Text("cmd /c echo ".to_string())
        );
        assert_eq!(placeholders[1], Placeholder::UserArg);
    }

    #[test]
    fn test_parse_multiple_positional() {
        let placeholders = TemplateParser::parse("program {} {} {}");
        assert_eq!(placeholders.len(), 6);
        let user_args: Vec<_> = placeholders
            .iter()
            .filter(|p| matches!(p, Placeholder::UserArg))
            .collect();
        assert_eq!(user_args.len(), 3);
    }

    #[test]
    fn test_parse_clipboard_parameter() {
        let placeholders = TemplateParser::parse("notepad {clip}");
        assert_eq!(placeholders.len(), 2);
        assert_eq!(placeholders[0], Placeholder::Text("notepad ".to_string()));
        assert_eq!(
            placeholders[1],
            Placeholder::System(SystemParameter::Clipboard)
        );
    }

    #[test]
    fn test_parse_hwnd_parameter() {
        let placeholders = TemplateParser::parse("tool --hwnd {hwnd}");
        assert_eq!(placeholders.len(), 2);
        assert_eq!(
            placeholders[1],
            Placeholder::System(SystemParameter::WindowHandle)
        );
    }

    #[test]
    fn test_parse_selection_parameter() {
        let placeholders = TemplateParser::parse("translate {selection}");
        assert_eq!(placeholders.len(), 2);
        assert_eq!(
            placeholders[1],
            Placeholder::System(SystemParameter::Selection)
        );
    }

    #[test]
    fn test_parse_mixed_parameters() {
        let placeholders = TemplateParser::parse("program {} {clip} {} {hwnd}");
        assert_eq!(placeholders.len(), 8);
    }

    #[test]
    fn test_count_user_args() {
        assert_eq!(
            TemplateParser::count_user_args("program {} {clip} {} {hwnd}"),
            2
        );
        assert_eq!(TemplateParser::count_user_args("program {clip} {hwnd}"), 0);
        assert_eq!(TemplateParser::count_user_args("program {} {}"), 2);
        assert_eq!(TemplateParser::count_user_args("program {selection}"), 0);
        assert_eq!(
            TemplateParser::count_user_args("program {} {selection} {}"),
            2
        );
    }

    #[test]
    fn test_has_system_params() {
        assert!(TemplateParser::has_system_params("program {clip}"));
        assert!(TemplateParser::has_system_params("program {hwnd}"));
        assert!(TemplateParser::has_system_params("program {selection}"));
        assert!(TemplateParser::has_system_params("program {} {clip}"));
        assert!(!TemplateParser::has_system_params("program {}"));
        assert!(!TemplateParser::has_system_params("program"));
    }

    #[test]
    fn test_parse_trailing_text() {
        let placeholders = TemplateParser::parse("cmd /c echo {} --flag");
        assert_eq!(placeholders.len(), 3);
        assert_eq!(placeholders[2], Placeholder::Text(" --flag".to_string()));
    }
}
