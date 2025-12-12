//! 模板解析器 - 解析模板字符串中的参数占位符

use super::parameter_types::{ParameterType, SystemParameter};

/// 参数在模板中的信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    /// 参数类型
    pub param_type: ParameterType,
    /// 在模板中的起始位置
    pub start_pos: usize,
    /// 在模板中的结束位置(不包含)
    pub end_pos: usize,
    /// 原始占位符文本(包括大括号)
    pub placeholder: String,
}

/// 模板解析器
pub struct TemplateParser;

impl TemplateParser {
    /// 解析模板,返回所有参数的位置和类型
    ///
    /// 支持的占位符格式:
    /// - `{}` - 位置参数
    /// - `{clip}` - 剪贴板内容
    /// - `{hwnd}` - 窗口句柄
    pub fn parse(template: &str) -> Vec<Parameter> {
        let mut parameters = Vec::new();
        let mut positional_index = 0;
        let chars: Vec<char> = template.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '{' {
                // 找到左大括号,开始查找右大括号
                if let Some(end) = Self::find_closing_brace(&chars, i) {
                    let start_pos = i;
                    let end_pos = end + 1;
                    let placeholder = chars[start_pos..end_pos].iter().collect::<String>();
                    let content = chars[start_pos + 1..end].iter().collect::<String>();

                    // 根据内容判断参数类型
                    let param_type = if content.is_empty() {
                        // {} - 位置参数
                        let param = ParameterType::Positional(positional_index);
                        positional_index += 1;
                        param
                    } else if let Some(sys_param) = SystemParameter::from_name(&content) {
                        // {clip} 或 {hwnd} - 系统参数
                        ParameterType::System(sys_param)
                    } else {
                        // 未知参数名,当作位置参数处理
                        let param = ParameterType::Positional(positional_index);
                        positional_index += 1;
                        param
                    };

                    parameters.push(Parameter {
                        param_type,
                        start_pos,
                        end_pos,
                        placeholder,
                    });

                    i = end_pos;
                    continue;
                }
            }
            i += 1;
        }

        parameters
    }

    /// 查找匹配的右大括号
    fn find_closing_brace(chars: &[char], start: usize) -> Option<usize> {
        ((start + 1)..chars.len()).find(|&i| chars[i] == '}')
    }

    /// 统计模板中需要用户输入的参数数量(只统计位置参数)
    pub fn count_user_parameters(template: &str) -> usize {
        Self::parse(template)
            .iter()
            .filter(|p| matches!(p.param_type, ParameterType::Positional(_)))
            .count()
    }

    /// 检查模板是否包含系统参数
    pub fn has_system_parameters(template: &str) -> bool {
        Self::parse(template)
            .iter()
            .any(|p| matches!(p.param_type, ParameterType::System(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_template() {
        let params = TemplateParser::parse("");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_parse_no_parameters() {
        let params = TemplateParser::parse("notepad.exe test.txt");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_parse_positional_parameters() {
        let params = TemplateParser::parse("cmd /c echo {}");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].param_type, ParameterType::Positional(0));
        assert_eq!(params[0].placeholder, "{}");
        assert_eq!(params[0].start_pos, 12);
        assert_eq!(params[0].end_pos, 14);
    }

    #[test]
    fn test_parse_multiple_positional() {
        let params = TemplateParser::parse("program {} {} {}");
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].param_type, ParameterType::Positional(0));
        assert_eq!(params[1].param_type, ParameterType::Positional(1));
        assert_eq!(params[2].param_type, ParameterType::Positional(2));
    }

    #[test]
    fn test_parse_clipboard_parameter() {
        let params = TemplateParser::parse("notepad {clip}");
        assert_eq!(params.len(), 1);
        assert_eq!(
            params[0].param_type,
            ParameterType::System(SystemParameter::Clipboard)
        );
        assert_eq!(params[0].placeholder, "{clip}");
    }

    #[test]
    fn test_parse_hwnd_parameter() {
        let params = TemplateParser::parse("tool --hwnd {hwnd}");
        assert_eq!(params.len(), 1);
        assert_eq!(
            params[0].param_type,
            ParameterType::System(SystemParameter::WindowHandle)
        );
        assert_eq!(params[0].placeholder, "{hwnd}");
    }

    #[test]
    fn test_parse_mixed_parameters() {
        let params = TemplateParser::parse("program {} {clip} {} {hwnd}");
        assert_eq!(params.len(), 4);
        assert_eq!(params[0].param_type, ParameterType::Positional(0));
        assert_eq!(
            params[1].param_type,
            ParameterType::System(SystemParameter::Clipboard)
        );
        assert_eq!(params[2].param_type, ParameterType::Positional(1));
        assert_eq!(
            params[3].param_type,
            ParameterType::System(SystemParameter::WindowHandle)
        );
    }

    #[test]
    fn test_parse_selection_parameter() {
        let params = TemplateParser::parse("translate {selection}");
        assert_eq!(params.len(), 1);
        assert_eq!(
            params[0].param_type,
            ParameterType::System(SystemParameter::Selection)
        );
        assert_eq!(params[0].placeholder, "{selection}");
    }

    #[test]
    fn test_parse_all_system_parameters() {
        let params = TemplateParser::parse("program {clip} {hwnd} {selection}");
        assert_eq!(params.len(), 3);
        assert_eq!(
            params[0].param_type,
            ParameterType::System(SystemParameter::Clipboard)
        );
        assert_eq!(
            params[1].param_type,
            ParameterType::System(SystemParameter::WindowHandle)
        );
        assert_eq!(
            params[2].param_type,
            ParameterType::System(SystemParameter::Selection)
        );
    }

    #[test]
    fn test_count_user_parameters() {
        assert_eq!(
            TemplateParser::count_user_parameters("program {} {clip} {} {hwnd}"),
            2
        );
        assert_eq!(
            TemplateParser::count_user_parameters("program {clip} {hwnd}"),
            0
        );
        assert_eq!(TemplateParser::count_user_parameters("program {} {}"), 2);
        assert_eq!(
            TemplateParser::count_user_parameters("program {selection}"),
            0
        );
        assert_eq!(
            TemplateParser::count_user_parameters("program {} {selection} {}"),
            2
        );
    }

    #[test]
    fn test_has_system_parameters() {
        assert!(TemplateParser::has_system_parameters("program {clip}"));
        assert!(TemplateParser::has_system_parameters("program {hwnd}"));
        assert!(TemplateParser::has_system_parameters("program {selection}"));
        assert!(TemplateParser::has_system_parameters("program {} {clip}"));
        assert!(!TemplateParser::has_system_parameters("program {}"));
        assert!(!TemplateParser::has_system_parameters("program"));
    }
}
