use crate::sdk::app::app_launcher::AppLauncher;
use crate::sdk::common::ComGuard;
use crate::sdk::host_api::HostApiError;
use crate::utils::windows::get_u16_vec;
use async_trait::async_trait;
use tracing::debug;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::UI::Shell::{
    ApplicationActivationManager, IApplicationActivationManager, AO_NONE,
};
use windows_core::PCWSTR;

/// Windows 应用启动器实现。
/// 通过 IApplicationActivationManager 激活 UWP 应用。
pub struct WindowsAppLauncher;

impl Default for WindowsAppLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsAppLauncher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AppLauncher for WindowsAppLauncher {
    /// 启动应用。
    /// 通过 IApplicationActivationManager::ActivateApplication 激活 UWP 应用。
    /// 参数：app_id - AppUserModelID；args - 启动参数（可选，暂未使用）。
    /// 返回：成功返回 Ok(pid)，失败返回 HostApiError。
    async fn launch_app(
        &self,
        app_id: &str,
        _args: Option<&[String]>,
    ) -> Result<u32, HostApiError> {
        unsafe {
            let _com_guard = ComGuard::init();

            let manager: IApplicationActivationManager =
                CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_ALL).map_err(|e| {
                    HostApiError::AppLaunchFailed {
                        app_id: app_id.to_string(),
                        reason: format!("Failed to create ApplicationActivationManager: {}", e),
                    }
                })?;

            let app_id_wide: Vec<u16> = get_u16_vec(app_id);
            let pid = manager
                .ActivateApplication(PCWSTR::from_raw(app_id_wide.as_ptr()), None, AO_NONE)
                .map_err(|e| HostApiError::AppLaunchFailed {
                    app_id: app_id.to_string(),
                    reason: format!("UWP激活失败: {}", e),
                })?;

            debug!("activated {} with pid {}", app_id, pid);
            Ok(pid)
        }
    }
}
