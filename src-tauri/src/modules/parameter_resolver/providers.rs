//! 参数提供者 - 用于获取动态参数值
//!
//! 设计原则:
//! - Provider 不持有状态,不依赖外部服务
//! - 只负责从系统获取**当前时刻**的值
//! - 窗口句柄等需要特殊时机捕获的值,应在调用方提前获取并通过 Snapshot 传递

use thiserror::Error;
use tracing::warn;

thread_local! {
    static UI_AUTOMATION: once_cell::unsync::OnceCell<uiautomation::UIAutomation> =
        const { once_cell::unsync::OnceCell::new() };
}

/// 参数提供者错误
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("无法获取剪贴板内容: {0}")]
    ClipboardError(String),

    #[error("无法获取窗口句柄: {0}")]
    WindowHandleError(String),

    #[error("无法获取选中文本: {0}")]
    SelectionError(String),
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

/// 选中文本参数提供者
///
/// 使用 Windows UI Automation API 获取指定窗口中焦点元素的选中文本
///
/// 设计原则:
/// - 不污染剪贴板
/// - 优雅降级：如果无法获取，返回空字符串
#[derive(Debug)]
pub struct SelectionProvider;

impl SelectionProvider {
    fn is_com_changed_mode(err: &uiautomation::Error) -> bool {
        // RPC_E_CHANGED_MODE: 0x80010106
        const RPC_E_CHANGED_MODE: i32 = -2147417850;
        err.code() == RPC_E_CHANGED_MODE
    }

    fn create_ui_automation() -> Result<uiautomation::UIAutomation, uiautomation::Error> {
        match uiautomation::UIAutomation::new() {
            Ok(automation) => Ok(automation),
            Err(e) if Self::is_com_changed_mode(&e) => {
                // 当前线程已用 STA 初始化，uiautomation::UIAutomation::new() 会尝试 MTA 初始化从而失败。
                // 这里改用 new_direct()，避免重复初始化 COM。
                uiautomation::UIAutomation::new_direct()
            }
            Err(e) => Err(e),
        }
    }

    fn get_ui_automation() -> Result<uiautomation::UIAutomation, uiautomation::Error> {
        UI_AUTOMATION.with(|cell| {
            if let Some(automation) = cell.get() {
                return Ok(automation.clone());
            }

            let automation = Self::create_ui_automation()?;
            let _ = cell.set(automation.clone());
            Ok(automation)
        })
    }

    /// 获取指定窗口句柄中的选中文本
    ///
    /// **注意**: 此函数需要在特定时机调用（唤醒搜索栏前），
    /// 并传入之前保存的窗口句柄。
    ///
    /// # 参数
    /// - `hwnd`: 目标窗口的句柄值
    ///
    /// # 返回
    /// - 成功时返回选中的文本内容
    /// - 如果没有选中文本或无法访问，返回空字符串
    pub fn get_value_from_hwnd(hwnd: isize) -> Result<String, ProviderError> {
        use uiautomation::patterns::UITextPattern;

        // 如果句柄为 0 或无效，直接返回空字符串
        if hwnd == 0 {
            tracing::debug!("窗口句柄为 0，跳过选中文本获取");
            return Ok(String::new());
        }

        // 创建 UI Automation 实例
        let automation = Self::get_ui_automation()
            .map_err(|e| ProviderError::SelectionError(format!("创建 UIAutomation 失败: {}", e)))?;

        // 尝试获取当前焦点元素的选中文本
        // 注意：我们使用 get_focused_element 而不是从特定窗口获取，
        // 因为在调用此函数时，目标窗口应该还是前台窗口
        let focused_element = match automation.get_focused_element() {
            Ok(element) => element,
            Err(e) => {
                tracing::debug!("无法获取焦点元素: {}, 返回空字符串", e);
                return Ok(String::new());
            }
        };

        // 尝试获取 TextPattern
        let text_pattern: UITextPattern = match focused_element.get_pattern() {
            Ok(pattern) => pattern,
            Err(e) => {
                tracing::debug!("焦点元素不支持 TextPattern: {}, 尝试其他方式", e);
                // 尝试使用 ValuePattern 获取值（某些控件可能使用这种方式）
                return Self::try_get_value_from_value_pattern(&focused_element);
            }
        };

        // 获取选中的文本范围
        let selections = match text_pattern.get_selection() {
            Ok(selections) => selections,
            Err(e) => {
                tracing::debug!("无法获取选中范围: {}, 返回空字符串", e);
                return Ok(String::new());
            }
        };

        // 获取第一个选中范围的文本
        if let Some(first_selection) = selections.first() {
            match first_selection.get_text(-1) {
                Ok(text) => {
                    tracing::debug!("成功获取选中文本: {} 字符", text.len());
                    return Ok(text);
                }
                Err(e) => {
                    tracing::debug!("无法获取选中文本内容: {}", e);
                }
            }
        }

        Ok(String::new())
    }

    /// 尝试从 ValuePattern 获取选中文本（作为备选方案）
    fn try_get_value_from_value_pattern(
        _element: &uiautomation::UIElement,
    ) -> Result<String, ProviderError> {
        // 某些控件（如文本框）可能支持 ValuePattern
        // 但这不是真正的"选中文本"，所以我们这里不使用它
        // 仅作为扩展点保留
        tracing::debug!("ValuePattern 不适用于获取选中文本，返回空字符串");
        Ok(String::new())
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

    #[test]
    fn test_selection_provider_invalid_hwnd() {
        // 测试无效句柄时返回空字符串
        let result = SelectionProvider::get_value_from_hwnd(0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_selection_provider_get_value() {
        // 测试不会panic
        let result = SelectionProvider::get_value_from_hwnd(12345);
        assert!(result.is_ok());
    }
}
