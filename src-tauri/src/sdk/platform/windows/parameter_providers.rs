//! Windows 平台系统参数提供者实现

use crate::sdk::parameter::provider::{ProviderError, SystemParameterProvider};
use tracing::warn;

thread_local! {
    static UI_AUTOMATION: once_cell::unsync::OnceCell<uiautomation::UIAutomation> =
        const { once_cell::unsync::OnceCell::new() };
}

/// 剪贴板参数提供者（Windows 实现）
///
/// 从系统剪贴板获取当前的文本内容
pub struct WindowsClipboardProvider;

#[async_trait::async_trait]
impl SystemParameterProvider for WindowsClipboardProvider {
    /// 获取当前剪贴板内容
    ///
    /// 返回：剪贴板文本内容，失败时返回空字符串（不返回错误，保证降级可用）
    async fn get_value(&self) -> Result<String, ProviderError> {
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.get_text() {
                Ok(text) => Ok(text),
                Err(e) => {
                    warn!("无法获取剪贴板文本: {:?}, 使用空字符串", e);
                    Ok(String::new())
                }
            },
            Err(e) => {
                warn!("无法初始化剪贴板: {:?}, 使用空字符串", e);
                Ok(String::new())
            }
        }
    }
}

/// 窗口句柄参数提供者（Windows 实现）
///
/// 从 Windows API 获取当前的前台窗口句柄
pub struct WindowsWindowHandleProvider;

#[async_trait::async_trait]
impl SystemParameterProvider for WindowsWindowHandleProvider {
    /// 获取当前前台窗口句柄
    ///
    /// 返回：十进制格式的窗口句柄字符串，失败时返回 "0"
    async fn get_value(&self) -> Result<String, ProviderError> {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                warn!("无法获取前台窗口句柄，返回 0");
                return Ok("0".to_string());
            }

            Ok((hwnd.0 as isize).to_string())
        }
    }
}

/// 选中文本参数提供者（Windows 实现）
///
/// 使用 Windows UI Automation API 获取焦点元素的选中文本
/// 设计原则：
/// - 不污染剪贴板
/// - 优雅降级：如果无法获取，返回空字符串
pub struct WindowsSelectionProvider;

impl WindowsSelectionProvider {
    /// 检查是否为 COM 模式变更错误
    fn is_com_changed_mode(err: &uiautomation::Error) -> bool {
        // RPC_E_CHANGED_MODE: 0x80010106
        const RPC_E_CHANGED_MODE: i32 = -2147417850;
        err.code() == RPC_E_CHANGED_MODE
    }

    /// 创建 UIAutomation 实例，处理 COM 模式冲突
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

    /// 获取缓存的 UIAutomation 实例
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
}

#[async_trait::async_trait]
impl SystemParameterProvider for WindowsSelectionProvider {
    /// 获取当前焦点元素的选中文本
    ///
    /// 返回：选中的文本内容，失败或无选中时返回空字符串（不返回错误，保证降级可用）
    async fn get_value(&self) -> Result<String, ProviderError> {
        use uiautomation::patterns::UITextPattern;

        let automation = match Self::get_ui_automation() {
            Ok(a) => a,
            Err(e) => {
                tracing::debug!("创建 UIAutomation 失败: {}, 返回空字符串", e);
                return Ok(String::new());
            }
        };

        let focused_element = match automation.get_focused_element() {
            Ok(element) => element,
            Err(e) => {
                tracing::debug!("无法获取焦点元素: {}, 返回空字符串", e);
                return Ok(String::new());
            }
        };

        let text_pattern: UITextPattern = match focused_element.get_pattern() {
            Ok(pattern) => pattern,
            Err(e) => {
                tracing::debug!("焦点元素不支持 TextPattern: {}, 返回空字符串", e);
                return Ok(String::new());
            }
        };

        let selections = match text_pattern.get_selection() {
            Ok(selections) => selections,
            Err(e) => {
                tracing::debug!("无法获取选中范围: {}, 返回空字符串", e);
                return Ok(String::new());
            }
        };

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
}
