use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};
use crate::utils::defer::defer;
use crate::utils::windows::get_u16_vec;
use tracing::{debug, warn};
use windows::Win32::System::Com::{CoCreateInstance, CoInitialize, CoUninitialize, CLSCTX_ALL};
use windows::Win32::UI::Shell::{
    ApplicationActivationManager, IApplicationActivationManager, AO_NONE,
};
use windows_core::PCWSTR;

/// UWP 执行器 - 负责通过 PackageFamilyName 启动 UWP 应用
/// UWP 应用运行在 AppContainer 沙箱中，不支持管理员权限启动
pub struct UwpExecutor;

impl UwpExecutor {
    pub fn new() -> Self {
        Self
    }

    /// 启动 UWP 应用
    /// 使用 IApplicationActivationManager::ActivateApplication 激活 UWP 应用
    fn execute_uwp(&self, package_family_name: &str) -> Result<(), ExecutionError> {
        unsafe {
            let com_init = CoInitialize(None);
            if com_init.is_err() {
                warn!("初始化COM库失败：{:?}", com_init);
            }
            defer(move || {
                if com_init.is_ok() {
                    CoUninitialize();
                }
            });

            let manager: IApplicationActivationManager =
                CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_ALL).map_err(|e| {
                    ExecutionError::Failed(format!(
                        "Failed to create ApplicationActivationManager: {}",
                        e
                    ))
                })?;

            let app_id_wide: Vec<u16> = get_u16_vec(package_family_name);
            let pid = manager
                .ActivateApplication(PCWSTR::from_raw(app_id_wide.as_ptr()), None, AO_NONE)
                .map_err(|e| ExecutionError::Failed(format!("UWP激活失败: {}", e)))?;

            debug!("activated {} with pid {}", package_family_name, pid);
            Ok(())
        }
    }
}

impl Configurable for UwpExecutor {
    fn component_id(&self) -> &str {
        "uwp-executor"
    }

    fn component_name(&self) -> &str {
        "UWP执行器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Launcher
    }
}

impl Default for UwpExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionExecutor for UwpExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::PackageFamilyName]
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: "execute".to_string(),
            label: "打开".to_string(),
            icon: String::new(),
            is_default: true,
            shortcut_key: String::new(),
        }]
    }

    fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
        let package_family_name = match &ctx.target {
            ExecutionTarget::PackageFamilyName(name) => name,
            _ => {
                return Err(ExecutionError::Failed(
                    "Invalid target type for UwpExecutor".into(),
                ))
            }
        };

        match action_id {
            "execute" => self.execute_uwp(package_family_name),
            _ => Err(ExecutionError::UnsupportedAction(
                TargetType::PackageFamilyName,
                action_id.to_string(),
            )),
        }
    }
}
