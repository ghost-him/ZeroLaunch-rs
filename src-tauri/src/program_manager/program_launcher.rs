use super::super::utils::{dashmap_to_hashmap, hashmap_to_dashmap};
use super::super::utils::{generate_current_date, is_date_current};
/// 这个类用于启动应用程序，同时还会维护启动次数
use super::{config::ProgramLauncherConfig, LaunchMethod};
use crate::defer::defer;
use crate::utils::get_u16_vec;
use dashmap::DashMap;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use tracing::{debug, warn};
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
    launch_store: DashMap<u64, LaunchMethod>,
    /// 用户记录当前程序的启动次数
    launch_time: VecDeque<DashMap<String, u64>>,
    /// 记录历史的启动次数
    history_launch_time: DashMap<String, u64>,
    /// 上一次更新的时间
    last_update_data: String,
}

impl ProgramLauncher {
    /// 初始化
    pub fn new() -> ProgramLauncher {
        let mut deque = VecDeque::new();
        deque.push_front(DashMap::new());
        ProgramLauncher {
            launch_store: DashMap::new(),
            launch_time: deque,
            history_launch_time: DashMap::new(),
            last_update_data: generate_current_date(),
        }
    }
    /// 重置程序信息
    pub fn clear_program_launch_info(&mut self) {
        self.launch_store.clear();
    }

    ///使用配置文件初始化
    pub fn load_from_config(&mut self, config: &ProgramLauncherConfig) {
        self.launch_time.clear();
        config.launch_info.iter().for_each(|k| {
            let dash_map = hashmap_to_dashmap(k);
            self.launch_time.push_back(dash_map);
        });

        self.last_update_data = config.last_update_data.clone();
        self.history_launch_time = hashmap_to_dashmap(&config.history_launch_time);
        self.update_launch_info();
    }
    /// 将当前的内容保存到配置文件中
    pub fn save_to_config(&mut self) -> ProgramLauncherConfig {
        self.update_launch_info();

        let mut launch_info_data: VecDeque<HashMap<String, u64>> = VecDeque::new();
        for item in &self.launch_time {
            launch_info_data.push_back(dashmap_to_hashmap(&item));
        }

        ProgramLauncherConfig {
            launch_info: launch_info_data,
            history_launch_time: dashmap_to_hashmap(&self.history_launch_time),
            last_update_data: generate_current_date(),
        }
    }
    /// 注册一个程序
    pub fn register_program(&mut self, program_guid: u64, launch_method: LaunchMethod) {
        debug!("register: {} {}", program_guid, launch_method.get_text());
        self.launch_store.insert(program_guid, launch_method);
    }
    /// 通过全局唯一标识符启动程序
    pub fn launch_program(&mut self, program_guid: u64, is_admin_required: bool) {
        let launch_method = self.launch_store.get(&program_guid).unwrap();

        self.launch_time[0]
            .entry(launch_method.get_text())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        self.history_launch_time
            .entry(launch_method.get_text())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        match &*launch_method {
            LaunchMethod::Path(path) => {
                self.launch_path_program(path, is_admin_required);
            }
            LaunchMethod::PackageFamilyName(family_name) => {
                self.launch_uwp_program(family_name);
            }
            LaunchMethod::File(file_name) => {
                self.launch_file(file_name);
            }
        }
    }
    /// 使用默认程序启动文件
    pub fn launch_file(&self, file_name: &str) {
        let result = std::process::Command::new("cmd")
            .args(&["/C", "start", "", file_name])
            .spawn();
        if result.is_err() {
            warn!("启动失败：{:?}", result);
        }
    }
    /// 获取当前程序的动态启动次数
    pub fn program_dynamic_value_based_launch_time(&self, program_guid: u64) -> f64 {
        let program_string = self.launch_store.get(&program_guid).unwrap();
        let mut result: f64 = 0.0;
        let mut k: f64 = 1.0;
        self.launch_time.iter().for_each(|day| {
            if let Some(time) = day.get(&program_string.get_text()) {
                result += (*time as f64) * k;
            }
            k /= 1.5
        });
        result
    }
    /// 获取当前程序的历史启动次数
    pub fn program_history_launch_time(&mut self, program_guid: u64) -> u64 {
        let program_string = self.launch_store.get(&program_guid).unwrap();
        let count = self
            .history_launch_time
            .entry(program_string.get_text())
            .or_insert(0);
        count.clone()
    }

    /// 启动uwp应用
    fn launch_uwp_program(&self, package_family_name: &str) {
        unsafe {
            // Initialize COM
            if CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_err() {
                warn!("无法初始化COM库");
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
                    warn!("error: {}", e);
                    return;
                }
            };

            debug!("activated {} with pid {}", package_family_name, pid);
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
                    warn!("Normal start failed due to insufficient privileges. Trying with elevation...");
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
    /// 使用普通权限启动程序
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
    /// 更新一下启动次数统计
    /// 1.在读取配置文件的时候会处理一下
    /// 2.在记录到配置文件的时候会处理一下
    fn update_launch_info(&mut self) {
        if !is_date_current(&self.last_update_data) {
            // 如果不是,则更新
            self.launch_time.push_front(DashMap::new());
            if self.launch_time.len() > 7 {
                self.launch_time.pop_back();
            }
        }
    }
}
