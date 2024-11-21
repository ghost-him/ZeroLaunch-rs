/// 这个类用于启动应用程序，同时还会维护启动次数
use super::{config::ProgramLauncherConfig, LaunchMethod};
use crate::defer::defer;
use crate::utils::get_u16_vec;
use std::collections::HashMap;
use std::path::Path;
use windows::Win32::Foundation::{GetLastError, ERROR_CANCELLED, ERROR_ELEVATION_REQUIRED};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    ApplicationActivationManager, IApplicationActivationManager, ShellExecuteExW, AO_NONE,
    SHELLEXECUTEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows_core::PCWSTR;
pub struct ProgramLauncher {
    /// 用于存储目标程序的启动方式
    launch_store: HashMap<u64, LaunchMethod>,
    /// 用户记录当前程序的启动次数
    launch_time: HashMap<u64, u64>,
}

impl ProgramLauncher {
    /// 初始化
    pub fn new() -> ProgramLauncher {
        ProgramLauncher {
            launch_store: HashMap::new(),
            launch_time: HashMap::new(),
        }
    }
    /// 重置程序信息
    pub fn clear_program_launch_info(&mut self) {
        self.launch_store.clear();
    }

    ///使用配置文件初始化
    pub fn load_from_config(&self, config: &ProgramLauncherConfig) {}
    /// 将当前的内容保存到配置文件中
    pub fn save_to_config(&self) -> ProgramLauncherConfig {
        ProgramLauncherConfig {}
    }
    /// 注册一个程序
    pub fn register_program(&mut self, program_guid: u64, launch_method: LaunchMethod) {
        println!("register: {} {}", program_guid, launch_method.get_text());
        self.launch_store.insert(program_guid, launch_method);
    }
    /// 通过全局唯一标识符启动程序
    pub fn launch_program(&self, program_guid: u64, is_admin_required: bool) {
        let launch_method = self.launch_store.get(&program_guid).unwrap();
        match launch_method {
            LaunchMethod::Path(path) => {
                self.launch_path_program(path, is_admin_required);
            }
            LaunchMethod::PackageFamilyName(family_name) => {
                self.launch_uwp_program(family_name);
            }
        }
    }
    /// 获取当前程序的动态启动次数
    pub fn program_launch_time(&self, program_guid: u64) -> u64 {
        0_u64
    }

    /// 启动uwp应用
    fn launch_uwp_program(&self, package_family_name: &str) {
        unsafe {
            // Initialize COM
            if CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_err() {
                eprintln!("无法初始化COM库");
            }
            defer(|| {
                CoUninitialize();
            });

            let manager: IApplicationActivationManager =
                CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_ALL).unwrap();

            let app_id_wide: Vec<u16> = get_u16_vec(package_family_name);
            let pid = match manager.ActivateApplication(
                PCWSTR::from_raw(app_id_wide.as_ptr()),
                None,
                AO_NONE,
            ) {
                Ok(pid) => pid,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return;
                }
            };

            println!("activated {} with pid {}", package_family_name, pid);
        }
    }

    /// 启动普通路径的应用
    fn launch_path_program(&self, path: &str, is_admin_required: bool) {
        let program_path = Path::new(&path);
        let working_directory = program_path.parent().unwrap();

        let mut program_path_wide = get_u16_vec(program_path);
        let mut working_directory_wide = get_u16_vec(working_directory);

        if is_admin_required {
            self.launch_path_program_elevation(&mut program_path_wide, &mut working_directory_wide);
        } else {
            let result = self
                .launch_path_program_normal(&mut program_path_wide, &mut working_directory_wide);
            if let Err(error) = result {
                if error == ERROR_ELEVATION_REQUIRED {
                    println!("Normal start failed due to insufficient privileges. Trying with elevation...");
                    self.launch_path_program_elevation(
                        &mut program_path_wide,
                        &mut working_directory_wide,
                    );
                } else {
                    println!("Failed to start process. Error: {}", error.to_hresult());
                }
            }
        }
    }
    /// 使用普通权限启动程序
    fn launch_path_program_normal(
        &self,
        program_path_wide: &mut [u16],
        working_directory_wide: &mut [u16],
    ) -> Result<(), windows::Win32::Foundation::WIN32_ERROR> {
        println!("{:?}", program_path_wide);
        unsafe {
            let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.lpVerb = PCWSTR::from_raw(std::ptr::null());
            sei.lpFile = PCWSTR::from_raw(program_path_wide.as_ptr());
            sei.lpDirectory = PCWSTR::from_raw(working_directory_wide.as_ptr());
            sei.nShow = SW_SHOWNORMAL.0;

            if let Ok(_) = ShellExecuteExW(&mut sei) {
            } else {
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

            if let Ok(_) = ShellExecuteExW(&mut sei) {
            } else {
                let error = unsafe { GetLastError() };
                if error == ERROR_CANCELLED {
                    println!("User declined the elevation request.");
                } else {
                    println!(
                        "Failed to start process with elevation. Error: {}",
                        error.to_hresult()
                    );
                }
            }
        }
    }
}
