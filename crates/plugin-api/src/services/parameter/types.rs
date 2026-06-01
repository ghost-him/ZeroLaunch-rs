//! 参数模块公开与内部类型定义

/// 系统参数类型（crate 内部可见，不对外公开）
///
/// 定义 SDK 支持的系统参数枚举，外部无法知道具体有哪些参数类型，保证封装性。
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum SystemParameter {
    /// 剪贴板内容 {clip}
    Clipboard,
    /// 当前前台窗口句柄 {hwnd}
    WindowHandle,
    /// 前台窗口选中文本 {selection}
    Selection,
}

impl SystemParameter {
    /// 将枚举转换为字符串键，用于访问 ParameterSnapshot
    pub fn as_key(&self) -> &'static str {
        match self {
            SystemParameter::Clipboard => "clipboard",
            SystemParameter::WindowHandle => "hwnd",
            SystemParameter::Selection => "selection",
        }
    }

    /// 从占位符名称解析系统参数类型
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "clip" => Some(SystemParameter::Clipboard),
            "hwnd" => Some(SystemParameter::WindowHandle),
            "selection" => Some(SystemParameter::Selection),
            _ => None,
        }
    }
}
