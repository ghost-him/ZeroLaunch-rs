use crate::error::{OptionExt, ResultExt};
use crate::program_manager::LaunchMethod;
use crate::utils::defer::defer;
use crate::utils::windows::{get_u16_vec, shell_execute_open};
use parking_lot::RwLock;
use std::os::windows::process::CommandExt;
use std::path::Path;
use tracing::{debug, warn};
use windows::Win32::Foundation::{GetLastError, ERROR_CANCELLED, ERROR_ELEVATION_REQUIRED};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::UI::Shell::{
    ApplicationActivationManager, IApplicationActivationManager, ShellExecuteExW, AO_NONE,
    SHELLEXECUTEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows_core::PCWSTR;

/// 程序启动器内部实现
#[derive(Debug)]
struct ProgramLauncherInner;

impl ProgramLauncherInner {
    fn new() -> Self {
        ProgramLauncherInner
    }

    /// 启动程序
    fn launch_program(
        &self,
        launch_method: &LaunchMethod,
        is_admin_required: bool,
    ) {
        match launch_method {
            LaunchMethod::Path(path) => {
                self.launch_path_program(path, is_admin_required);
            }
            LaunchMethod::PackageFamilyName(family_name) => {
                self.launch_uwp_program(family_name);
            }
            LaunchMethod::File(file_name) => {
                self.launch_file(file_name);
            }
            LaunchMethod::Command(command) => {
                self.launch_command(command);
            }
        }
    }

    fn launch_command(&self, command: &str) {
        // 分割命令和参数
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        let program = parts[0];
        let args = &parts[1..];

        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;

        if let Err(error) = std::process::Command::new(program)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
            .spawn()
        {
            warn!("启动命令失败: {:?}", error);
        }
    }

    fn launch_file(&self, file_name: &str) {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let result = std::process::Command::new("cmd")
            .args(["/C", "start", "", file_name])
            .creation_flags(CREATE_NO_WINDOW) // 隐藏命令窗口
            .spawn();

        if result.is_err() {
            warn!("启动失败：{:?}", result);
        }
    }

    

    fn launch_uwp_program(&self, package_family_name: &str) {
        unsafe {
            // Initialize COM
            let com_init = windows::Win32::System::Com::CoInitialize(None);
            if com_init.is_err() {
                warn!("初始化com库失败：{:?}", com_init);
            }

            defer(move || {
                if com_init.is_ok() {
                    windows::Win32::System::Com::CoUninitialize();
                }
            });

            let manager: IApplicationActivationManager =
                CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_ALL)
                    .expect_programming("Failed to create ApplicationActivationManager");

            let app_id_wide: Vec<u16> = get_u16_vec(package_family_name);
            let pid = match manager.ActivateApplication(
                PCWSTR::from_raw(app_id_wide.as_ptr()),
                None,
                AO_NONE,
            ) {
                Ok(pid) => pid,
                Err(e) => {
                    warn!("error: {}", e);
                    return;
                }
            };

            debug!("activated {} with pid {}", package_family_name, pid);
        }
    }

    fn launch_path_program(&self, path: &str, is_admin_required: bool) {
        let program_path = Path::new(&path);
        let working_directory = program_path
            .parent()
            .expect_programming("Program path should have a parent directory");

        let mut program_path_wide = get_u16_vec(program_path);
        let mut working_directory_wide = get_u16_vec(working_directory);

        if is_admin_required {
            self.launch_path_program_elevation(&mut program_path_wide, &mut working_directory_wide);
        } else {
            let result = self
                .launch_path_program_normal(&mut program_path_wide, &mut working_directory_wide);
            if let Err(error) = result {
                if error == ERROR_ELEVATION_REQUIRED {
                    debug!("Normal start failed due to insufficient privileges. Trying with elevation...");
                    self.launch_path_program_elevation(
                        &mut program_path_wide,
                        &mut working_directory_wide,
                    );
                } else {
                    warn!("Failed to start process. Error: {}", error.to_hresult());
                }
            }
        }
    }

    fn launch_path_program_normal(
        &self,
        program_path_wide: &mut [u16],
        working_directory_wide: &mut [u16],
    ) -> Result<(), windows::Win32::Foundation::WIN32_ERROR> {
        debug!("{:?}", program_path_wide);
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

    fn launch_path_program_elevation(
        &self,
        program_path_wide: &mut [u16],
        working_directory_wide: &mut [u16],
    ) {
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

    #[allow(clippy::zombie_processes)]
    pub fn open_target_folder(&self, launch_method: &LaunchMethod) -> bool {
        // 只支持命令和uwp应用以外的程序
        match launch_method {
            LaunchMethod::Command(_) => {
                return false;
            }
            LaunchMethod::PackageFamilyName(_) => {
                return false;
            }
            _ => {}
        }
        let target_path = launch_method.get_text();
        let target_path = Path::new(&target_path);

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
            warn!(
                "Target folder does not exist and cannot be opened: {}",
                folder_to_open.display()
            );
            return false;
        }

        if let Err(error) = shell_execute_open(folder_to_open) {
            warn!(
                "Failed to open folder with default file manager. Error code: {}",
                error.to_hresult()
            );
            return false;
        }

        true
    }
}

/// 程序启动器 - 负责启动程序
#[derive(Debug)]
pub struct ProgramLauncher {
    inner: RwLock<ProgramLauncherInner>,
}

impl Default for ProgramLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgramLauncher {
    pub fn new() -> Self {
        ProgramLauncher {
            inner: RwLock::new(ProgramLauncherInner::new()),
        }
    }

    pub fn launch_program(
        &self,
        launch_method: &LaunchMethod,
        is_admin_required: bool,
    ) {
        self.inner
            .read()
            .launch_program(launch_method, is_admin_required);
    }

    pub fn open_target_folder(&self, launch_method: &LaunchMethod) -> bool {
        self.inner.read().open_target_folder(launch_method)
    }
}
