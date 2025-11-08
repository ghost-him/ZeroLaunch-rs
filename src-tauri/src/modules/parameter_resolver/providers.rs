//! 参数提供者 - 用于获取动态参数值
//!
//! 设计原则:
//! - Provider 不持有状态,不依赖外部服务
//! - 只负责从系统获取**当前时刻**的值
//! - 窗口句柄等需要特殊时机捕获的值,应在调用方提前获取并通过 Snapshot 传递

use thiserror::Error;
use tracing::warn;

/// 参数提供者错误
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("无法获取剪贴板内容: {0}")]
    ClipboardError(String),

    #[error("无法获取窗口句柄: {0}")]
    WindowHandleError(String),
}

/// 剪贴板参数提供者
///
/// 从系统剪贴板获取当前的文本内容
#[derive(Debug)]
pub struct ClipboardProvider;

impl ClipboardProvider {
    /// 获取当前剪贴板内容
    pub fn get_value() -> Result<String, ProviderError> {
        // 使用 arboard crate 获取剪贴板内容
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                match clipboard.get_text() {
                    Ok(text) => Ok(text),
                    Err(e) => {
                        // 如果剪贴板为空或无法访问，返回空字符串
                        warn!("无法获取剪贴板文本: {:?}, 使用空字符串", e);
                        Ok(String::new())
                    }
                }
            }
            Err(e) => {
                warn!("无法初始化剪贴板: {:?}, 使用空字符串", e);
                Ok(String::new())
            }
        }
    }
}

/// 窗口句柄参数提供者
///
/// 从 Windows API 获取当前的前台窗口句柄
#[derive(Debug)]
pub struct WindowHandleProvider;

impl WindowHandleProvider {
    /// 获取当前前台窗口句柄
    ///
    /// **注意**: 此函数获取的是**调用时**的前台窗口。
    /// 如果需要捕获特定时刻的窗口句柄(如唤醒前的窗口),
    /// 应该在那个时刻调用此函数并保存结果。
    pub fn get_value() -> Result<String, ProviderError> {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                warn!("无法获取前台窗口句柄，返回 0");
                return Ok("0".to_string());
            }

            // 返回十进制格式的句柄值
            Ok((hwnd.0 as isize).to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_provider_get_value() {
        // 测试不会panic,即使剪贴板为空
        let result = ClipboardProvider::get_value();
        assert!(result.is_ok());
    }

    #[test]
    fn test_window_handle_provider_get_value() {
        // 测试不会panic
        let result = WindowHandleProvider::get_value();
        assert!(result.is_ok());
    }
}
