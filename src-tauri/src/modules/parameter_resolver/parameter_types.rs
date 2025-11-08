//! 参数类型定义

use serde::{Deserialize, Serialize};

/// 参数占位符类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterType {
    /// 按顺序填充的位置参数 {}
    Positional(usize),

    /// 系统提供的动态参数 {clip} {hwnd}
    System(SystemParameter),
}

/// 系统提供的动态参数类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemParameter {
    /// 剪贴板内容 {clip}
    Clipboard,

    /// 当前活动窗口句柄 {hwnd}
    WindowHandle,
}

impl SystemParameter {
    /// 从参数名称字符串解析系统参数类型
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "clip" => Some(SystemParameter::Clipboard),
            "hwnd" => Some(SystemParameter::WindowHandle),
            _ => None,
        }
    }

    /// 获取参数名称
    pub fn name(&self) -> &'static str {
        match self {
            SystemParameter::Clipboard => "clip",
            SystemParameter::WindowHandle => "hwnd",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_parameter_from_name() {
        assert_eq!(
            SystemParameter::from_name("clip"),
            Some(SystemParameter::Clipboard)
        );
        assert_eq!(
            SystemParameter::from_name("hwnd"),
            Some(SystemParameter::WindowHandle)
        );
        assert_eq!(SystemParameter::from_name("unknown"), None);
    }

    #[test]
    fn test_system_parameter_name() {
        assert_eq!(SystemParameter::Clipboard.name(), "clip");
        assert_eq!(SystemParameter::WindowHandle.name(), "hwnd");
    }
}
