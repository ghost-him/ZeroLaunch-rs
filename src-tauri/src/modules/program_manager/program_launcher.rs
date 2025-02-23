use crate::program_manager::config::program_launcher_config::PartialProgramLauncherConfig;
use crate::program_manager::config::program_launcher_config::ProgramLauncherConfig;
use crate::program_manager::LaunchMethod;
use crate::utils::dashmap_to_hashmap;
use crate::utils::defer::defer;
use crate::utils::generate_current_date;
use crate::utils::hashmap_to_dashmap;
use crate::utils::is_date_current;
use crate::utils::windows::get_u16_vec;
use dashmap::DashMap;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::RwLock;
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
#[derive(Debug)]
struct ProgramLauncherInner {
    launch_store: DashMap<u64, LaunchMethod>,
    launch_time: VecDeque<DashMap<String, u64>>,
    history_launch_time: DashMap<String, u64>,
    last_update_data: String,
}

impl ProgramLauncherInner {
    fn new() -> Self {
        let mut deque = VecDeque::new();
        deque.push_front(DashMap::new());
        ProgramLauncherInner {
            launch_store: DashMap::new(),
            launch_time: deque,
            history_launch_time: DashMap::new(),
            last_update_data: generate_current_date(),
        }
    }

    fn clear_program_launch_info(&mut self) {
        self.launch_store.clear();
    }

    fn load_from_config(&mut self, config: &ProgramLauncherConfig) {
        self.launch_time.clear();
        let launch_info = config.get_launch_info();
        launch_info.iter().for_each(|k| {
            let dash_map = hashmap_to_dashmap(k);
            self.launch_time.push_back(dash_map);
        });

        self.last_update_data = config.get_last_update_data();
        self.history_launch_time = hashmap_to_dashmap(&config.get_history_launch_time());
        self.update_launch_info();
    }

    fn to_partial(&mut self) -> PartialProgramLauncherConfig {
        self.update_launch_info();

        let mut launch_info_data: VecDeque<HashMap<String, u64>> = VecDeque::new();
        for item in &self.launch_time {
            launch_info_data.push_back(dashmap_to_hashmap(&item));
        }

        PartialProgramLauncherConfig {
            launch_info: Some(launch_info_data),
            history_launch_time: Some(dashmap_to_hashmap(&self.history_launch_time)),
            last_update_data: Some(generate_current_date()),
        }
    }

    fn register_program(&mut self, program_guid: u64, launch_method: LaunchMethod) {
        debug!("register: {} {}", program_guid, launch_method.get_text());
        self.launch_store.insert(program_guid, launch_method);
    }

    fn launch_program(&mut self, program_guid: u64, is_admin_required: bool) {
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

    fn launch_file(&self, file_name: &str) {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let result = std::process::Command::new("cmd")
            .args(&["/C", "start", "", file_name])
            .creation_flags(CREATE_NO_WINDOW) // 隐藏命令窗口
            .spawn();

        if result.is_err() {
            warn!("启动失败：{:?}", result);
        }
    }

    fn program_dynamic_value_based_launch_time(&self, program_guid: u64) -> f64 {
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

    fn program_history_launch_time(&mut self, program_guid: u64) -> u64 {
        let program_string = self.launch_store.get(&program_guid).unwrap();
        let count = self
            .history_launch_time
            .entry(program_string.get_text())
            .or_insert(0);
        count.clone()
    }

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

    fn update_launch_info(&mut self) {
        if !is_date_current(&self.last_update_data) {
            self.launch_time.push_front(DashMap::new());
            if self.launch_time.len() > 7 {
                self.launch_time.pop_back();
            }
            self.last_update_data = generate_current_date();
        }
    }
}
#[derive(Debug)]
pub struct ProgramLauncher {
    inner: RwLock<ProgramLauncherInner>,
}

impl ProgramLauncher {
    pub fn new() -> Self {
        ProgramLauncher {
            inner: RwLock::new(ProgramLauncherInner::new()),
        }
    }

    pub fn clear_program_launch_info(&self) {
        self.inner.write().unwrap().clear_program_launch_info();
    }

    pub fn load_from_config(&self, config: &ProgramLauncherConfig) {
        self.inner.write().unwrap().load_from_config(config);
    }

    pub fn to_partial(&self) -> PartialProgramLauncherConfig {
        self.inner.write().unwrap().to_partial()
    }

    pub fn register_program(&self, program_guid: u64, launch_method: LaunchMethod) {
        self.inner
            .write()
            .unwrap()
            .register_program(program_guid, launch_method);
    }

    pub fn launch_program(&self, program_guid: u64, is_admin_required: bool) {
        self.inner
            .write()
            .unwrap()
            .launch_program(program_guid, is_admin_required);
    }

    pub fn program_dynamic_value_based_launch_time(&self, program_guid: u64) -> f64 {
        self.inner
            .read()
            .unwrap()
            .program_dynamic_value_based_launch_time(program_guid)
    }

    pub fn program_history_launch_time(&self, program_guid: u64) -> u64 {
        self.inner
            .write()
            .unwrap()
            .program_history_launch_time(program_guid)
    }
}
