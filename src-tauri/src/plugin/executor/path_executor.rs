use crate::core::types::{ComponentType, Configurable};
use crate::plugin_system::types::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ResultAction, TargetType,
};
use crate::utils::windows::{get_u16_vec, shell_execute_open};
use std::os::windows::process::CommandExt;
use std::path::Path;
use tracing::{debug, warn};
use windows::Win32::Foundation::{GetLastError, ERROR_CANCELLED};
use windows::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows_core::PCWSTR;

/// 路径执行器 - 负责通过文件路径启动程序
/// 支持普通启动、管理员启动和打开所在文件夹
pub struct PathExecutor;

impl PathExecutor {
    pub fn new() -> Self {
        Self
    }

    /// 普通启动程序
    /// 优先使用 explorer.exe 代理启动以实现进程分离，失败时回退到 ShellExecuteExW
    fn execute_normal(&self, path: &str) {
        let program_path = Path::new(path);
        let working_directory = program_path.parent().unwrap_or_else(|| Path::new("."));
        let path_str = program_path.to_string_lossy();

        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;

        let result = std::process::Command::new("explorer")
            .arg(&*path_str)
            .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
            .spawn();

        match result {
            Ok(_) => {
                debug!("已请求 Explorer 启动: {}", path);
            }
            Err(e) => {
                warn!("Explorer 启动失败: {:?}, 尝试回退到 ShellExecute...", e);
                let mut program_path_wide = get_u16_vec(program_path);
                let mut working_directory_wide = get_u16_vec(working_directory);
                let _ =
                    self.launch_with_shellexec(&mut program_path_wide, &mut working_directory_wide);
            }
        }
    }

    /// 以管理员权限启动程序
    /// 使用 ShellExecuteExW 的 runas verb 触发 UAC 提升对话框
    fn execute_elevation(&self, path: &str) {
        let program_path = Path::new(path);
        let working_directory = program_path.parent().unwrap_or_else(|| Path::new("."));
        let program_path_wide = get_u16_vec(program_path);
        let working_directory_wide = get_u16_vec(working_directory);

        unsafe {
            let lp_verb = get_u16_vec("runas");
            let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.lpVerb = PCWSTR::from_raw(lp_verb.as_ptr());
            sei.lpFile = PCWSTR::from_raw(program_path_wide.as_ptr());
            sei.lpDirectory = PCWSTR::from_raw(working_directory_wide.as_ptr());
            sei.nShow = SW_SHOWNORMAL.0;

            if ShellExecuteExW(&mut sei).is_err() {
                let error = GetLastError();
                if error == ERROR_CANCELLED {
                    warn!("User declined the elevation request.");
                } else {
                    warn!(
                        "Failed to start process with elevation. Error: {}",
                        error.to_hresult()
                    );
                }
            }
        }
    }

    /// 打开目标文件所在的文件夹
    /// 返回成功或失败
    fn open_folder(&self, path: &str) -> Result<(), ExecutionError> {
        let target_path = Path::new(path);

        let folder_to_open = if target_path.is_dir() {
            target_path
        } else {
            target_path.parent().unwrap_or_else(|| {
                warn!(
                    "Target path has no parent, fallback to original path: {}",
                    target_path.display()
                );
                target_path
            })
        };

        if !folder_to_open.exists() {
            let msg = format!(
                "Target folder does not exist and cannot be opened: {}",
                folder_to_open.display()
            );
            warn!("{}", msg);
            return Err(ExecutionError::Failed(msg));
        }

        if let Err(error) = shell_execute_open(folder_to_open) {
            let msg = format!(
                "Failed to open folder with default file manager. Error code: {}",
                error.to_hresult()
            );
            warn!("{}", msg);
            return Err(ExecutionError::Failed(msg));
        }

        Ok(())
    }

    /// 使用 ShellExecuteExW 启动程序（作为 explorer 启动失败时的回退方案）
    fn launch_with_shellexec(
        &self,
        program_path_wide: &mut [u16],
        working_directory_wide: &mut [u16],
    ) -> Result<(), windows::Win32::Foundation::WIN32_ERROR> {
        unsafe {
            let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.lpVerb = PCWSTR::from_raw(std::ptr::null());
            sei.lpFile = PCWSTR::from_raw(program_path_wide.as_ptr());
            sei.lpDirectory = PCWSTR::from_raw(working_directory_wide.as_ptr());
            sei.nShow = SW_SHOWNORMAL.0;

            if ShellExecuteExW(&mut sei).is_err() {
                return Err(GetLastError());
            }
            Ok(())
        }
    }
}

impl Configurable for PathExecutor {
    fn component_id(&self) -> &str {
        "path-executor"
    }

    fn component_name(&self) -> &str {
        "路径执行器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Launcher
    }
}

impl Default for PathExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionExecutor for PathExecutor {
    fn supported_target_types(&self) -> Vec<TargetType> {
        vec![TargetType::Path]
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![
            ResultAction {
                id: "execute".to_string(),
                label: "打开".to_string(),
                icon: String::new(),
                is_default: true,
                shortcut_key: String::new(),
            },
            ResultAction {
                id: "execute_admin".to_string(),
                label: "以管理员身份运行".to_string(),
                icon: String::new(),
                is_default: false,
                shortcut_key: "Ctrl+Enter".to_string(),
            },
            ResultAction {
                id: "open_folder".to_string(),
                label: "打开所在文件夹".to_string(),
                icon: String::new(),
                is_default: false,
                shortcut_key: String::new(),
            },
        ]
    }

    fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
        let path = match &ctx.target {
            ExecutionTarget::Path(p) => p,
            _ => {
                return Err(ExecutionError::Failed(
                    "Invalid target type for PathExecutor".into(),
                ))
            }
        };

        match action_id {
            "execute" => {
                self.execute_normal(path);
                Ok(())
            }
            "execute_admin" => {
                self.execute_elevation(path);
                Ok(())
            }
            "open_folder" => self.open_folder(path),
            _ => Err(ExecutionError::UnsupportedAction(
                TargetType::Path,
                action_id.to_string(),
            )),
        }
    }
}
