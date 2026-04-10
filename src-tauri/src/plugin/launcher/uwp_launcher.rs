use crate::plugin_system::types::{
    LaunchError, LaunchMethod, LaunchMethodType, Launcher, ResultAction,
};
use crate::utils::defer::defer;
use crate::utils::windows::get_u16_vec;
use tracing::{debug, warn};
use windows::Win32::System::Com::{CoCreateInstance, CoInitialize, CoUninitialize, CLSCTX_ALL};
use windows::Win32::UI::Shell::{
    ApplicationActivationManager, IApplicationActivationManager, AO_NONE,
};
use windows_core::PCWSTR;

/// UWP 启动器 - 负责通过 PackageFamilyName 启动 UWP 应用
/// UWP 应用运行在 AppContainer 沙箱中，不支持管理员权限启动
pub struct UwpLauncher;

impl UwpLauncher {
    pub fn new() -> Self {
        Self
    }

    /// 启动 UWP 应用
    /// 使用 IApplicationActivationManager::ActivateApplication 激活 UWP 应用
    fn launch_uwp(&self, package_family_name: &str) -> Result<(), LaunchError> {
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
                    LaunchError::Failed(format!(
                        "Failed to create ApplicationActivationManager: {}",
                        e
                    ))
                })?;

            let app_id_wide: Vec<u16> = get_u16_vec(package_family_name);
            let pid = manager
                .ActivateApplication(PCWSTR::from_raw(app_id_wide.as_ptr()), None, AO_NONE)
                .map_err(|e| LaunchError::Failed(format!("UWP激活失败: {}", e)))?;

            debug!("activated {} with pid {}", package_family_name, pid);
            Ok(())
        }
    }
}

impl Default for UwpLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl Launcher for UwpLauncher {
    fn supported_method(&self) -> LaunchMethodType {
        LaunchMethodType::PackageFamilyName
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: "launch".to_string(),
            label: "打开".to_string(),
            icon: String::new(),
            is_default: true,
        }]
    }

    fn execute(&self, method: &LaunchMethod, action_id: &str) -> Result<(), LaunchError> {
        let package_family_name = match method {
            LaunchMethod::PackageFamilyName(name) => name,
            _ => {
                return Err(LaunchError::Failed(
                    "Invalid launch method for UwpLauncher".into(),
                ))
            }
        };

        match action_id {
            "launch" => self.launch_uwp(package_family_name),
            _ => Err(LaunchError::UnsupportedAction(action_id.to_string())),
        }
    }
}
