use crate::core::platform::window::{
    activate_with_hwnd, get_window_by_process_name, get_window_by_title,
};
use crate::core::storage::utils::get_lnk_target_path;
use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};

/// 窗口激活执行器 - 负责唤醒已存在的程序窗口
/// 无状态结构体，所有方法为纯计算（遍历进程/枚举窗口）
pub struct WindowActivateExecutor;

impl WindowActivateExecutor {
    pub fn new() -> Self {
        Self
    }

    /// 尝试激活已存在的程序窗口
    fn try_activate(&self, target: &ExecutionTarget, name: &str) -> bool {
        match target {
            ExecutionTarget::Path(path) => {
                if path.ends_with(".url") {
                    self.activate_by_title(name)
                } else {
                    let exe_path = if path.ends_with(".exe") {
                        path.clone()
                    } else {
                        get_lnk_target_path(path).unwrap_or_default()
                    };
                    if exe_path.is_empty() {
                        return false;
                    }
                    self.activate_by_exe(&exe_path)
                }
            }
            ExecutionTarget::PackageFamilyName(_) => self.activate_by_title(name),
            _ => false,
        }
    }

    /// 直接使用标题来激活窗口
    fn activate_by_title(&self, program_name: &str) -> bool {
        if let Some(hwnd) = get_window_by_title(program_name) {
            activate_with_hwnd(hwnd);
            return true;
        }
        false
    }

    /// 激活.exe程序的窗口，传入绝对路径
    fn activate_by_exe(&self, str: &str) -> bool {
        let abs_path = std::path::Path::new(str);
        let program_name = match abs_path.file_name() {
            Some(name) => match name.to_str() {
                Some(s) => s.to_string(),
                None => return false,
            },
            None => return false,
        };
        let program_stem = match abs_path.file_stem() {
            Some(stem) => match stem.to_str() {
                Some(s) => s.to_string(),
                None => return false,
            },
            None => return false,
        };

        let hwnd = {
            let mut result = get_window_by_process_name(&program_name);
            if result.is_none() {
                result = get_window_by_title(&program_stem);
            }
            result
        };

        if let Some(hwnd) = hwnd {
            activate_with_hwnd(hwnd);
            true
        } else {
            false
        }
    }
}

impl Configurable for WindowActivateExecutor {
    fn component_id(&self) -> &str {
        "window-activate-executor"
    }

    fn component_name(&self) -> &str {
        "窗口唤醒执行器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::ActionExecutor
    }
}

impl Default for WindowActivateExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionExecutor for WindowActivateExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::Path, TargetType::PackageFamilyName]
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: "activate_window".to_string(),
            label: "唤醒窗口".to_string(),
            icon: String::new(),
            is_default: false,
            shortcut_key: "Shift+Enter".to_string(),
        }]
    }

    fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
        match action_id {
            "activate_window" => {
                if self.try_activate(&ctx.target, &ctx.display_name) {
                    Ok(())
                } else {
                    Err(ExecutionError::ActivationFailed {
                        fallback_action: "execute".to_string(),
                    })
                }
            }
            _ => Err(ExecutionError::UnsupportedAction(
                ctx.target.target_type(),
                action_id.to_string(),
            )),
        }
    }
}
