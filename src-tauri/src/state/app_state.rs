use crate::error::AppError;
use crate::modules::{
    config::config_manager::RuntimeConfig, program_manager::ProgramManager,
    storage::windows_utils::get_data_dir_path,
};
use parking_lot::RwLock;
use std::sync::Arc;
use tauri::AppHandle;
use timer::{Guard, Timer};

pub struct AppState {
    /// 远程配置目录路径
    remote_config_dir_path: RwLock<String>,
    /// 运行时配置
    runtime_config: RwLock<Option<Arc<RuntimeConfig>>>,
    /// 程序管理器
    program_manager: RwLock<Option<Arc<ProgramManager>>>,
    /// 主窗口句柄
    main_handle: RwLock<Option<Arc<AppHandle>>>,
    /// 定时器守卫
    timer_guard: RwLock<Option<Guard>>,
    /// 定时器
    timer: Arc<Timer>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            remote_config_dir_path: RwLock::new(String::new()),
            runtime_config: RwLock::new(None),
            program_manager: RwLock::new(None),
            main_handle: RwLock::new(None),
            timer_guard: RwLock::new(None),
            timer: Arc::new(Timer::new()),
        }
    }

    // region: Remote Config Directory Path 访问方法
    /// 获取远程配置目录路径
    pub fn get_remote_config_dir_path(&self) -> String {
        self.remote_config_dir_path.read().clone()
    }

    /// 设置远程配置目录路径
    pub fn set_remote_config_dir_path(&self, path: String) {
        *self.remote_config_dir_path.write() = path;
    }
    // endregion

    // region: Runtime Config 访问方法
    /// 获取运行时配置的克隆
    pub fn get_runtime_config(&self) -> Result<Arc<RuntimeConfig>, AppError> {
        self.runtime_config
            .read()
            .as_ref()
            .cloned()
            .ok_or(AppError::NotInitialized {
                resource: "runtime_config".to_string(),
                context: None,
            })
    }

    /// 更新运行时配置
    pub fn set_runtime_config(&self, config: Arc<RuntimeConfig>) {
        *self.runtime_config.write() = Some(config);
    }
    // endregion

    // region: Program Manager 访问方法
    /// 获取程序管理器的克隆
    pub fn get_program_manager(&self) -> Result<Arc<ProgramManager>, AppError> {
        self.program_manager
            .read()
            .as_ref()
            .cloned()
            .ok_or(AppError::NotInitialized {
                resource: "program_manager".to_string(),
                context: None,
            })
    }

    /// 更新程序管理器
    pub fn set_program_manager(&self, manager: Arc<ProgramManager>) {
        *self.program_manager.write() = Some(manager);
    }
    // endregion

    // region: Main Window Handle 访问方法
    /// 获取主窗口句柄的克隆
    pub fn get_main_handle(&self) -> Result<Arc<AppHandle>, AppError> {
        self.main_handle
            .read()
            .as_ref()
            .cloned()
            .ok_or(AppError::NotInitialized {
                resource: "main_handle".to_string(),
                context: None,
            })
    }

    /// 更新主窗口句柄
    pub fn set_main_handle(&self, handle: Arc<AppHandle>) {
        *self.main_handle.write() = Some(handle);
    }
    // endregion

    pub fn get_timer(&self) -> Arc<Timer> {
        self.timer.clone()
    }

    pub fn set_timer_guard(&self, guard: Guard) {
        *self.timer_guard.write() = Some(guard);
    }

    pub fn take_timer_guard(&self) -> Option<Guard> {
        self.timer_guard.write().take()
    }
}

// Custom Debug implementation for AppState
impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("remote_config_dir_path", &self.remote_config_dir_path)
            .field("runtime_config", &self.runtime_config)
            .field("program_manager", &self.program_manager)
            .field("main_handle", &self.main_handle)
            .field("timer_guard", &"<Timer Guard>")
            .field("timer", &"<Timer>")
            .finish()
    }
}
