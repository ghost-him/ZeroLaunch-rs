//! 默认参数解析器实现

use crate::services::parameter::resolver::ParameterResolver;
use crate::services::parameter::template_parser::{Placeholder, TemplateParser};
use crate::services::parameter_types::{ParameterError, ParameterSnapshot};

/// 默认参数解析器实现
///
/// 使用 TemplateParser 解析模板占位符，从快照和用户参数中填充值。
pub struct DefaultParameterResolver;

impl DefaultParameterResolver {
    /// 创建默认参数解析器
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultParameterResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ParameterResolver for DefaultParameterResolver {
    async fn resolve(
        &self,
        template: &str,
        user_args: &[String],
        snapshot: &ParameterSnapshot,
    ) -> Result<String, ParameterError> {
        let placeholders = TemplateParser::parse(template);

        let mut result = String::new();
        let mut user_arg_index = 0;

        for placeholder in placeholders {
            match placeholder {
                Placeholder::Text(text) => result.push_str(&text),
                Placeholder::UserArg => {
                    if user_arg_index < user_args.len() {
                        result.push_str(&user_args[user_arg_index]);
                        user_arg_index += 1;
                    } else {
                        return Err(ParameterError::InsufficientArguments {
                            required: user_arg_index + 1,
                            actual: user_args.len(),
                        });
                    }
                }
                Placeholder::System(param) => {
                    result.push_str(&snapshot.get(param.as_key()));
                }
            }
        }

        Ok(result)
    }

    fn count_user_parameters(&self, template: &str) -> usize {
        TemplateParser::count_user_args(template)
    }

    fn has_system_parameters(&self, template: &str) -> bool {
        TemplateParser::has_system_params(template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(clipboard: &str, hwnd: &str, selection: &str) -> ParameterSnapshot {
        let mut snapshot = ParameterSnapshot::empty();
        if !clipboard.is_empty() {
            snapshot.insert("clipboard".to_string(), clipboard.to_string());
        }
        if !hwnd.is_empty() {
            snapshot.insert("hwnd".to_string(), hwnd.to_string());
        }
        if !selection.is_empty() {
            snapshot.insert("selection".to_string(), selection.to_string());
        }
        snapshot
    }

    #[tokio::test]
    async fn test_resolve_no_parameters() {
        let resolver = DefaultParameterResolver::new();
        let snapshot = make_snapshot("test", "12345", "");
        let result = resolver.resolve("notepad.exe", &[], &snapshot).await;
        assert_eq!(result.unwrap(), "notepad.exe");
    }

    #[tokio::test]
    async fn test_resolve_positional_parameters() {
        let resolver = DefaultParameterResolver::new();
        let snapshot = ParameterSnapshot::empty();
        let result = resolver
            .resolve("cmd /c echo {}", &["hello".to_string()], &snapshot)
            .await;
        assert_eq!(result.unwrap(), "cmd /c echo hello");
    }

    #[tokio::test]
    async fn test_resolve_clipboard_parameter() {
        let resolver = DefaultParameterResolver::new();
        let snapshot = make_snapshot("clipboard_content", "", "");
        let result = resolver.resolve("notepad {clip}", &[], &snapshot).await;
        assert_eq!(result.unwrap(), "notepad clipboard_content");
    }

    #[tokio::test]
    async fn test_resolve_hwnd_parameter() {
        let resolver = DefaultParameterResolver::new();
        let snapshot = make_snapshot("", "98765", "");
        let result = resolver.resolve("tool --hwnd {hwnd}", &[], &snapshot).await;
        assert_eq!(result.unwrap(), "tool --hwnd 98765");
    }

    #[tokio::test]
    async fn test_resolve_selection_parameter() {
        let resolver = DefaultParameterResolver::new();
        let snapshot = make_snapshot("", "", "selected text");
        let result = resolver
            .resolve("translate {selection}", &[], &snapshot)
            .await;
        assert_eq!(result.unwrap(), "translate selected text");
    }

    #[tokio::test]
    async fn test_resolve_mixed_parameters() {
        let resolver = DefaultParameterResolver::new();
        let snapshot = make_snapshot("clip_data", "54321", "selected");
        let result = resolver
            .resolve(
                "program {} {clip} {} {hwnd} {selection}",
                &["arg1".to_string(), "arg2".to_string()],
                &snapshot,
            )
            .await;
        assert_eq!(
            result.unwrap(),
            "program arg1 clip_data arg2 54321 selected"
        );
    }

    #[tokio::test]
    async fn test_argument_count_mismatch() {
        let resolver = DefaultParameterResolver::new();
        let snapshot = ParameterSnapshot::empty();
        let result = resolver
            .resolve("program {} {}", &["arg1".to_string()], &snapshot)
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_count_user_parameters() {
        let resolver = DefaultParameterResolver::new();
        assert_eq!(resolver.count_user_parameters("program {}"), 1);
        assert_eq!(resolver.count_user_parameters("program {} {}"), 2);
        assert_eq!(resolver.count_user_parameters("program {clip} {hwnd}"), 0);
        assert_eq!(resolver.count_user_parameters("program {} {clip} {}"), 2);
    }

    #[test]
    fn test_has_system_parameters() {
        let resolver = DefaultParameterResolver::new();
        assert!(resolver.has_system_parameters("program {clip}"));
        assert!(resolver.has_system_parameters("program {hwnd}"));
        assert!(resolver.has_system_parameters("program {selection}"));
        assert!(!resolver.has_system_parameters("program {}"));
        assert!(!resolver.has_system_parameters("program"));
    }
}
