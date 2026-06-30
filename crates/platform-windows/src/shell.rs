use crate::utils::get_u16_vec;
use async_trait::async_trait;
use std::os::windows::process::CommandExt;
use std::path::Path;
use tracing::{debug, warn};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{GetLastError, ERROR_CANCELLED};
use windows::Win32::UI::Shell::{ShellExecuteExW, ShellExecuteW, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use zerolaunch_plugin_api::host::{HostApiError, OpenTarget};
use zerolaunch_plugin_api::services::shell::ShellExecutor;

/// 使用 ShellExecuteW 以系统默认方式打开指定路径
fn shell_execute_open(path: &str) -> Result<(), HostApiError> {
    let wide_path = get_u16_vec(path);

    unsafe {
        let result = ShellExecuteW(
            None,
            PCWSTR::from_raw(std::ptr::null()),
            PCWSTR::from_raw(wide_path.as_ptr()),
            PCWSTR::from_raw(std::ptr::null()),
            PCWSTR::from_raw(std::ptr::null()),
            SW_SHOWNORMAL,
        );

        if result.0 as isize <= 32 {
            let error = GetLastError();
            Err(HostApiError::ShellOperationFailed {
                target: path.to_string(),
                reason: format!("ShellExecuteW 失败，错误码: {}", error.0),
            })
        } else {
            Ok(())
        }
    }
}

/// Windows 平台 Shell 执行器实现
pub struct WindowsShellExecutor;

impl Default for WindowsShellExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsShellExecutor {
    pub fn new() -> Self {
        Self
    }

    /// 通过 explorer.exe 代理启动文件，模拟用户在资源管理器中双击的效果。
    /// 优势：进程分离，被启动程序不会成为当前进程的子进程；
    ///       同时 explorer 会正确处理各种文件类型（.exe, .lnk, .url, steam:// 协议等）。
    /// 失败时回退到 ShellExecuteW。
    /// 参数：path - 要打开的文件路径。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn shell_open_file(&self, path: &str) -> Result<(), HostApiError> {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;

        let result = std::process::Command::new("explorer")
            .arg(path)
            .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
            .spawn();

        match result {
            Ok(_) => {
                debug!("已请求 Explorer 启动: {}", path);
                Ok(())
            }
            Err(e) => {
                warn!("Explorer 启动失败: {:?}, 回退到 ShellExecuteW...", e);
                shell_execute_open(path)
            }
        }
    }
}

#[async_trait]
impl ShellExecutor for WindowsShellExecutor {
    /// 使用系统默认方式打开目标。
    /// 对 File 类型：优先通过 explorer.exe 代理启动（模拟双击，进程分离），失败时回退到 ShellExecuteW。
    /// 对 Url/Folder 类型：直接使用 ShellExecuteW。
    async fn shell_open(&self, target: &OpenTarget) -> Result<(), HostApiError> {
        match target {
            OpenTarget::File(path) => self.shell_open_file(path).await,
            OpenTarget::Url(url) => shell_execute_open(url),
            OpenTarget::Folder(path) => shell_execute_open(path),
        }
    }

    /// 打开目标文件所在的文件夹。
    /// 如果目标是目录则直接打开，否则打开其父目录。
    async fn shell_open_folder(&self, path: &str) -> Result<(), HostApiError> {
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
            return Err(HostApiError::ShellOperationFailed {
                target: path.to_string(),
                reason: format!("目标文件夹不存在: {}", folder_to_open.display()),
            });
        }

        let folder_str = folder_to_open.to_string_lossy();
        shell_execute_open(&folder_str)
    }

    /// 以管理员权限启动程序。
    /// 使用 ShellExecuteExW 的 "runas" verb 触发 UAC 提升对话框。
    async fn shell_execute_elevation(&self, path: &str) -> Result<(), HostApiError> {
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
                    warn!("用户取消了 UAC 提升请求");
                    return Err(HostApiError::ShellOperationFailed {
                        target: path.to_string(),
                        reason: "用户取消了 UAC 提升请求".to_string(),
                    });
                } else {
                    return Err(HostApiError::ShellOperationFailed {
                        target: path.to_string(),
                        reason: format!("管理员启动失败，错误码: {}", error.to_hresult()),
                    });
                }
            }
        }

        Ok(())
    }

    /// 执行命令字符串（后台运行，无窗口）。
    /// 使用 cmd /D /S /C 执行命令，CREATE_NO_WINDOW | DETACHED_PROCESS 防止弹出控制台窗口。
    /// 输入验证：空命令或纯空白命令返回错误。
    async fn shell_execute_command(&self, command: &str) -> Result<(), HostApiError> {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;

        let command = command.trim();
        if command.is_empty() {
            return Err(HostApiError::ShellOperationFailed {
                target: String::new(),
                reason: "命令为空".to_string(),
            });
        }

        let result = std::process::Command::new("cmd")
            .args(["/D", "/S", "/C"])
            .raw_arg(command)
            .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
            .spawn();

        match result {
            Ok(_) => {
                debug!("命令启动成功: {}", command);
                Ok(())
            }
            Err(e) => {
                let msg = format!("命令启动失败: {:?}", e);
                warn!("{}", msg);
                Err(HostApiError::ShellOperationFailed {
                    target: command.to_string(),
                    reason: msg,
                })
            }
        }
    }
}
