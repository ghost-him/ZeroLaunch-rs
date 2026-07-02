//! Stub implementations of all plugin-api service traits.
//! Each stub returns a sensible default (empty / Ok / false).

use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::host::error::HostApiError;
use crate::host::open_target::OpenTarget;
use crate::services::app::app_enumerator::AppEnumerator;
use crate::services::app::app_launcher::AppLauncher;
use crate::services::app::AppInfo;
use crate::services::autostart::AutoStartManager;
use crate::services::focus_monitor::{FocusCallback, FocusMonitor};
use crate::services::hotkey::types::{Hotkey, HotkeyCallback, HotkeyEventFilter};
use crate::services::hotkey::HotkeyManager;
use crate::services::icon::icon_extractor::IconExtractor;
use crate::services::installation_monitor::{InstallationCallback, InstallationMonitor};
use crate::services::parameter::provider::{ProviderError, SystemParameterProvider};
use crate::services::parameter::resolver::ParameterResolver;
use crate::services::parameter::types::{ParameterError, ParameterSnapshot};
use crate::services::path::{KnownPath, PathResolver};
use crate::services::shell::lnk_resolver::LnkResolver;
use crate::services::shell::resource_loader::ResourceLoader;
use crate::services::shell::ShellExecutor;
use crate::services::storage::storage_error::StorageError;
use crate::services::storage::storage_service::StorageService;
use crate::services::window::window_positioner::{
    PositionRequest, WindowPosition, WindowPositioner,
};
use crate::services::window::WindowManager;

// ===== Icon Extractor =====

pub struct StubIconExtractor;

#[async_trait]
impl IconExtractor for StubIconExtractor {
    async fn extract_from_path(&self, _path: &str) -> Result<Vec<u8>, HostApiError> {
        Ok(vec![])
    }
    async fn extract_from_url(&self, _url: &str) -> Result<Vec<u8>, HostApiError> {
        Ok(vec![])
    }
    async fn extract_from_extension(&self, _ext: &str) -> Result<Vec<u8>, HostApiError> {
        Ok(vec![])
    }
    fn default_app_icon_path(&self) -> &str {
        ""
    }
    fn default_web_icon_path(&self) -> &str {
        ""
    }
    fn is_network_available(&self) -> bool {
        false
    }
}

// ===== Shell Executor =====

#[derive(Default)]
pub struct StubShellExecutor {
    pub opens: Mutex<Vec<OpenTarget>>,
}

#[async_trait]
impl ShellExecutor for StubShellExecutor {
    async fn shell_open(&self, target: &OpenTarget) -> Result<(), HostApiError> {
        self.opens.lock().push(target.clone());
        Ok(())
    }
    async fn shell_open_folder(&self, _path: &str) -> Result<(), HostApiError> {
        Ok(())
    }
    async fn shell_execute_elevation(&self, _path: &str) -> Result<(), HostApiError> {
        Ok(())
    }
    async fn shell_execute_command(&self, _cmd: &str) -> Result<(), HostApiError> {
        Ok(())
    }
}

// ===== Window Manager =====

pub struct StubWindowManager;

#[async_trait]
impl WindowManager for StubWindowManager {
    async fn activate_window_by_process(&self, _process_name: &str) -> Result<bool, HostApiError> {
        Ok(false)
    }
    async fn activate_window_by_title(&self, _title: &str) -> Result<bool, HostApiError> {
        Ok(false)
    }
}

// ===== Window Positioner =====

pub struct StubWindowPositioner;

#[async_trait]
impl WindowPositioner for StubWindowPositioner {
    async fn compute_position(
        &self,
        _request: PositionRequest,
    ) -> Result<WindowPosition, HostApiError> {
        Ok(WindowPosition { x: 0, y: 0 })
    }
}

// ===== Path Resolver =====

pub struct StubPathResolver;

impl PathResolver for StubPathResolver {
    fn resolve_path(&self, _path: KnownPath) -> Result<String, HostApiError> {
        Ok(String::new())
    }
}

// ===== App Enumerator =====

pub struct StubAppEnumerator;

#[async_trait]
impl AppEnumerator for StubAppEnumerator {
    async fn enumerate_apps(&self) -> Vec<AppInfo> {
        vec![]
    }
}

// ===== App Launcher =====

pub struct StubAppLauncher;

#[async_trait]
impl AppLauncher for StubAppLauncher {
    async fn launch_app(
        &self,
        _app_id: &str,
        _args: Option<&[String]>,
    ) -> Result<u32, HostApiError> {
        Ok(0)
    }
}

// ===== Lnk Resolver =====

pub struct StubLnkResolver;

impl LnkResolver for StubLnkResolver {
    fn resolve_lnk_target(&self, _lnk_path: &str) -> Option<String> {
        None
    }
}

// ===== Resource Loader =====

pub struct StubResourceLoader;

impl ResourceLoader for StubResourceLoader {
    fn parse_localized_names_from_dir(&self, _dir_path: &Path) -> HashMap<String, String> {
        HashMap::new()
    }
}

// ===== Parameter Resolver =====

pub struct StubParameterResolver;

#[async_trait]
impl ParameterResolver for StubParameterResolver {
    async fn resolve(
        &self,
        _template: &str,
        _user_args: &[String],
        _snapshot: &ParameterSnapshot,
    ) -> Result<String, ParameterError> {
        Ok(String::new())
    }
    fn count_user_parameters(&self, _template: &str) -> usize {
        0
    }
    fn has_system_parameters(&self, _template: &str) -> bool {
        false
    }
}

// ===== System Parameter Provider =====

pub struct StubSystemParameterProvider;

#[async_trait]
impl SystemParameterProvider for StubSystemParameterProvider {
    async fn get_value(&self) -> Result<String, ProviderError> {
        Ok(String::new())
    }
}

// ===== AutoStart Manager =====

pub struct StubAutoStartManager;

#[async_trait]
impl AutoStartManager for StubAutoStartManager {
    async fn enable(&self, _task_name: &str, _exe_path: &str) -> Result<(), HostApiError> {
        Ok(())
    }
    async fn disable(&self, _task_name: &str) -> Result<(), HostApiError> {
        Ok(())
    }
    async fn is_enabled(&self, _task_name: &str) -> Result<bool, HostApiError> {
        Ok(false)
    }
    fn default_task_name(&self) -> String {
        "ZeroLaunch".to_string()
    }
}

// ===== Hotkey Manager =====

pub struct StubHotkeyManager;

#[async_trait]
impl HotkeyManager for StubHotkeyManager {
    async fn register_hotkey(&self, _hotkey: &Hotkey) -> Result<(), HostApiError> {
        Ok(())
    }
    async fn unregister_hotkey(&self, _hotkey: &Hotkey) -> Result<(), HostApiError> {
        Ok(())
    }
    async fn unregister_all(&self) -> Result<(), HostApiError> {
        Ok(())
    }
    async fn set_double_ctrl_enabled(&self, _enabled: bool) -> Result<(), HostApiError> {
        Ok(())
    }
    async fn start_listening(&self) -> Result<(), HostApiError> {
        Ok(())
    }
    async fn stop_listening(&self) -> Result<(), HostApiError> {
        Ok(())
    }
    fn is_listening(&self) -> bool {
        false
    }
    fn register_callback(&self, _id: &str, _filter: HotkeyEventFilter, _callback: HotkeyCallback) {}
    fn unregister_callback(&self, _id: &str) {}
}

// ===== Installation Monitor =====

pub struct StubInstallationMonitor;

#[async_trait]
impl InstallationMonitor for StubInstallationMonitor {
    async fn start_watching(&self) -> Result<(), HostApiError> {
        Ok(())
    }
    async fn stop_watching(&self) -> Result<(), HostApiError> {
        Ok(())
    }
    fn is_watching(&self) -> bool {
        false
    }
    fn register_callback(&self, _id: &str, _callback: InstallationCallback) {}
    fn unregister_callback(&self, _id: &str) {}
    fn update_watch_paths(&self, _paths: Vec<String>) {}
}

// ===== Focus Monitor =====

pub struct StubFocusMonitor;

impl FocusMonitor for StubFocusMonitor {
    fn register_callback(&self, _id: &str, _callback: FocusCallback) {}
    fn unregister_callback(&self, _id: &str) {}
}

// ===== Storage Service =====

pub struct StubStorageService;

#[async_trait]
impl StorageService for StubStorageService {
    async fn upload(&self, _file_name: &str, _data: &[u8]) -> Result<(), StorageError> {
        Ok(())
    }
    async fn download(&self, _file_name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(None)
    }
    fn target_dir_path(&self) -> String {
        String::new()
    }
    async fn validate(&self) -> bool {
        true
    }
    async fn delete(&self, _file_name: &str) -> Result<(), StorageError> {
        Ok(())
    }
    async fn list(&self, _prefix: &str) -> Result<Vec<String>, StorageError> {
        Ok(vec![])
    }
}
