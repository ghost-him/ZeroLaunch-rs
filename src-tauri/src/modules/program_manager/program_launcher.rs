use crate::error::{OptionExt, ResultExt};
use crate::program_manager::config::program_launcher_config::PartialProgramLauncherConfig;
use crate::program_manager::config::program_launcher_config::ProgramLauncherConfig;
use crate::program_manager::LaunchMethod;
use crate::utils::dashmap_to_hashmap;
use crate::utils::defer::defer;
use crate::utils::hashmap_to_dashmap;
use crate::utils::is_date_current;
use crate::utils::windows::get_u16_vec;
use crate::utils::{generate_current_date, get_current_time};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::BTreeSet;
use std::collections::{HashMap, VecDeque};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use tracing::{debug, warn};
use windows::Win32::Foundation::{GetLastError, ERROR_CANCELLED, ERROR_ELEVATION_REQUIRED};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
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
    latest_launch_time: DashMap<String, i64>,
    // 运行过程中的数据结构，（上一次启动的时间，目标程序的guid）
    runtime_latest_launch_time: BTreeSet<(i64, u64)>,
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
            latest_launch_time: DashMap::new(),
            runtime_latest_launch_time: BTreeSet::new(),
        }
    }

    fn load_from_config(&mut self, config: &ProgramLauncherConfig) {
        self.launch_time.clear();
        self.launch_store.clear();
        let launch_info = config.get_launch_info();
        launch_info.iter().for_each(|k| {
            let dash_map = hashmap_to_dashmap(k);
            self.launch_time.push_back(dash_map);
        });

        self.last_update_data = config.get_last_update_data();
        self.history_launch_time = hashmap_to_dashmap(&config.get_history_launch_time());
        self.update_launch_info();
        // 维护最近启动次数
        self.latest_launch_time.clear();
        self.latest_launch_time = hashmap_to_dashmap(&config.get_latest_launch_time());

        self.runtime_latest_launch_time.clear();
    }

    fn get_runtime_data(&mut self) -> PartialProgramLauncherConfig {
        self.update_launch_info();

        let mut launch_info_data: VecDeque<HashMap<String, u64>> = VecDeque::new();
        for item in &self.launch_time {
            launch_info_data.push_back(dashmap_to_hashmap(item));
        }

        PartialProgramLauncherConfig {
            launch_info: Some(launch_info_data),
            history_launch_time: Some(dashmap_to_hashmap(&self.history_launch_time)),
            last_update_data: Some(generate_current_date()),
            latest_launch_time: Some(dashmap_to_hashmap(&self.latest_launch_time)),
        }
    }

    fn register_program(&mut self, program_guid: u64, launch_method: LaunchMethod) {
        debug!("register: {} {}", program_guid, launch_method.get_text());
        let key = launch_method.get_text();
        self.launch_store.insert(program_guid, launch_method);

        self.latest_launch_time.entry(key.clone()).or_insert(0);

        self.latest_launch_time
            .entry(key)
            .and_modify(|latest_launch_time| {
                self.runtime_latest_launch_time
                    .insert((*latest_launch_time, program_guid));
            });
    }

    fn launch_program(&mut self, program_guid: u64, is_admin_required: bool) {
        let launch_method = self
            .launch_store
            .get(&program_guid)
            .expect_programming("Program GUID should exist in launch store");
        self.launch_time[0]
            .entry(launch_method.get_text())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        self.history_launch_time
            .entry(launch_method.get_text())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        // 更新启动的时间
        self.latest_launch_time
            .entry(launch_method.get_text())
            .and_modify(|last_launch_time| {
                // 去除之前老的数据
                assert!(self
                    .runtime_latest_launch_time
                    .remove(&(*last_launch_time, program_guid)));
                let current_time = get_current_time();
                *last_launch_time = current_time;
                self.runtime_latest_launch_time
                    .insert((current_time, program_guid));
            });

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
            LaunchMethod::Command(command) => {
                self.launch_command(command);
            }
        }
    }

    /// 获得启动器维护的数据
    pub fn get_latest_launch_program(&self, program_count: u32) -> Vec<u64> {
        let mut result = Vec::new();
        for (_, program_guid) in self
            .runtime_latest_launch_time
            .iter()
            .rev()
            .take(program_count as usize)
        {
            result.push(*program_guid);
        }
        result
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

    fn program_dynamic_value_based_launch_time(&self, program_guid: u64) -> f64 {
        let program_string = self
            .launch_store
            .get(&program_guid)
            .expect_programming("Program GUID should exist in launch store");
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
        let program_string = self
            .launch_store
            .get(&program_guid)
            .expect_programming("Program GUID should exist in launch store");
        let count = self
            .history_launch_time
            .entry(program_string.get_text())
            .or_insert(0);
        *count
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

    fn update_launch_info(&mut self) {
        if !is_date_current(&self.last_update_data) {
            self.launch_time.push_front(DashMap::new());
            if self.launch_time.len() > 7 {
                self.launch_time.pop_back();
            }
            self.last_update_data = generate_current_date();
        }
    }

    #[allow(clippy::zombie_processes)]
    pub fn open_target_folder(&self, program_guid: u64) -> bool {
        let program_method = self
            .launch_store
            .get(&program_guid)
            .expect_programming("Program GUID should exist in launch store");
        let target_method = program_method.clone();
        // 只支持命令和uwp应用以外的程序
        match &target_method {
            LaunchMethod::Command(_) => {
                return false;
            }
            LaunchMethod::PackageFamilyName(_) => {
                return false;
            }
            _ => {}
        }
        let target_path = target_method.get_text();

        // 不需要获取父目录，直接使用/select参数
        Command::new("explorer")
            .args(["/select,", &target_path]) // 使用/select参数并指定完整文件路径
            .spawn()
            .expect_programming("Failed to spawn explorer process");
        true
    }
}
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

    pub fn load_from_config(&self, config: &ProgramLauncherConfig) {
        self.inner.write().load_from_config(config);
    }

    pub fn get_runtime_data(&self) -> PartialProgramLauncherConfig {
        self.inner.write().get_runtime_data()
    }

    pub fn register_program(&self, program_guid: u64, launch_method: LaunchMethod) {
        self.inner
            .write()
            .register_program(program_guid, launch_method);
    }

    pub fn launch_program(&self, program_guid: u64, is_admin_required: bool) {
        self.inner
            .write()
            .launch_program(program_guid, is_admin_required);
    }

    pub fn program_dynamic_value_based_launch_time(&self, program_guid: u64) -> f64 {
        self.inner
            .read()
            .program_dynamic_value_based_launch_time(program_guid)
    }

    pub fn program_history_launch_time(&self, program_guid: u64) -> u64 {
        self.inner.write().program_history_launch_time(program_guid)
    }

    pub fn open_target_folder(&self, program_guid: u64) -> bool {
        self.inner.read().open_target_folder(program_guid)
    }

    pub fn get_latest_launch_program(&self, program_count: u32) -> Vec<u64> {
        self.inner.read().get_latest_launch_program(program_count)
    }

    pub fn load_and_register_programs(&self, config: &ProgramLauncherConfig, programs: &[(u64, LaunchMethod)]) {
        let mut inner = self.inner.write(); // 获取一次写锁
        inner.load_from_config(config);     // 加载配置
        for (program_guid, launch_method) in programs {
            inner.register_program(*program_guid, launch_method.clone()); // 注册程序
        }
    }
}
