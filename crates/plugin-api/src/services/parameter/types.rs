//! 参数模块公开与内部类型定义

use std::collections::HashMap;

/// 参数解析错误类型
#[derive(Debug, thiserror::Error)]
pub enum ParameterError {
    /// 模板解析失败
    #[error("模板解析失败: {0}")]
    TemplateParseFailed(String),

    /// 参数数量不足
    #[error("参数数量不足: 需要 {required} 个，实际提供 {actual} 个")]
    InsufficientArguments { required: usize, actual: usize },

    /// 无效的占位符
    #[error("无效的占位符: {0}")]
    InvalidPlaceholder(String),
}

/// 系统参数快照（不透明句柄）
///
/// 外部只能持有此句柄，无法访问内部数据。
/// 由 HostApi::capture_parameter_snapshot() 创建，由 PluginHandle::resolve_parameters() 消费。
#[derive(Debug, Clone)]
pub struct ParameterSnapshot {
    /// 私有字段，外部不可访问
    /// 使用 String 作为键，避免暴露 SystemParameter 类型
    inner: HashMap<String, String>,
}

impl ParameterSnapshot {
    /// 创建空快照（用于无参数场景或降级处理）
    pub fn empty() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// 获取参数值
    ///
    /// 参数：key - 参数键名
    /// 返回：参数值，缺失时返回空字符串并记录警告日志
    pub fn get(&self, key: &str) -> String {
        self.inner.get(key).cloned().unwrap_or_else(|| {
            tracing::warn!("系统参数 {} 未捕获，使用空字符串", key);
            String::new()
        })
    }

    /// 插入参数值
    ///
    /// 参数：key - 参数键名；value - 参数值
    pub fn insert(&mut self, key: String, value: String) {
        self.inner.insert(key, value);
    }
}

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
