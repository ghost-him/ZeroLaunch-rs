//! 全部 plugin-api 服务 trait 的桩实现。
//! 每个桩返回合理默认值（空 / Ok / false）。

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
use crate::services::clipboard::ClipboardManager;
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

// ===== 主题提供方 =====

/// 固定返回浅色主题的主题提供器桩，供测试构造 PluginHandle。
pub struct StubThemeProvider;

impl crate::services::theme::ThemeProvider for StubThemeProvider {
    /// 返回浅色主题，不触达真实系统主题。
    fn current_system_theme(&self) -> Result<crate::services::Theme, HostApiError> {
        Ok(crate::services::Theme::Light)
    }
}

// ===== 图标提取器 =====

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

// ===== Shell 执行器 =====

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

// ===== 模型服务 =====

pub struct StubModelService;

#[async_trait::async_trait]
impl crate::services::model::ModelService for StubModelService {
    fn list_models(&self) -> Vec<crate::services::model::ModelInfo> {
        Vec::new()
    }

    fn model_info(&self, _model_id: &str) -> Option<crate::services::model::ModelInfo> {
        None
    }

    async fn chat(
        &self,
        _req: crate::services::model::ModelChatRequest,
    ) -> Result<crate::services::model::ModelChatResponse, crate::services::model::ModelError> {
        Err(crate::services::model::ModelError::NotSupported)
    }

    async fn stream_chat(
        &self,
        _req: crate::services::model::ModelChatRequest,
        _tx: tokio::sync::mpsc::Sender<crate::services::model::ModelStreamChunk>,
    ) -> Result<(), crate::services::model::ModelError> {
        Err(crate::services::model::ModelError::NotSupported)
    }

    async fn embedding(
        &self,
        _req: crate::services::model::ModelEmbeddingRequest,
    ) -> Result<crate::services::model::ModelEmbeddingResponse, crate::services::model::ModelError>
    {
        Err(crate::services::model::ModelError::NotSupported)
    }

    async fn similarity(
        &self,
        _req: crate::services::model::ModelSimilarityRequest,
    ) -> Result<crate::services::model::ModelSimilarityResponse, crate::services::model::ModelError>
    {
        Err(crate::services::model::ModelError::NotSupported)
    }
}

// ===== 剪贴板管理 =====

/// 剪贴板管理器桩：记录写入文本，供上层断言「谁写了什么」。
///
/// 仅在 `mock` feature 下供测试构造平台面使用（如 issue 工具的剪贴板兜底路径）。
#[derive(Default)]
pub struct StubClipboardManager {
    /// 依次写入的文本（按调用顺序，与实际写入时序一致）。
    pub writes: Mutex<Vec<String>>,
}

impl ClipboardManager for StubClipboardManager {
    fn set_text(&self, text: &str) -> Result<(), HostApiError> {
        self.writes.lock().push(text.to_string());
        Ok(())
    }
}

// ===== 窗口管理 =====

pub struct StubWindowManager;

#[async_trait]
impl WindowManager for StubWindowManager {
    async fn activate_window_by_process(&self, _process_name: &str) -> Result<bool, HostApiError> {
        Ok(false)
    }
    async fn activate_window_by_title(&self, _title: &str) -> Result<bool, HostApiError> {
        Ok(false)
    }
    async fn activate_window_by_pid(&self, _pid: u32) -> Result<bool, HostApiError> {
        Ok(false)
    }
}

// ===== 窗口定位器 =====

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

// ===== 路径解析 =====

pub struct StubPathResolver;

impl PathResolver for StubPathResolver {
    fn resolve_path(&self, _path: KnownPath) -> Result<String, HostApiError> {
        Ok(String::new())
    }
}

// ===== 应用枚举器 =====

pub struct StubAppEnumerator;

#[async_trait]
impl AppEnumerator for StubAppEnumerator {
    async fn enumerate_apps(&self) -> Vec<AppInfo> {
        vec![]
    }
}

// ===== 应用启动器 =====

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

// ===== 快捷方式解析 =====

pub struct StubLnkResolver;

impl LnkResolver for StubLnkResolver {
    fn resolve_lnk_target(&self, _lnk_path: &str) -> Option<String> {
        None
    }
}

// ===== 资源加载器 =====

pub struct StubResourceLoader;

impl ResourceLoader for StubResourceLoader {
    fn parse_localized_names_from_dir(&self, _dir_path: &Path) -> HashMap<String, String> {
        HashMap::new()
    }
}

// ===== 参数解析器 =====

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

// ===== 系统参数提供方 =====

pub struct StubSystemParameterProvider;

#[async_trait]
impl SystemParameterProvider for StubSystemParameterProvider {
    async fn get_value(&self) -> Result<String, ProviderError> {
        Ok(String::new())
    }
}

// ===== 开机自启动管理 =====

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

// ===== 热键管理 =====

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

// ===== 安装监控 =====

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
    fn update_debounce_secs(&self, _secs: f64) {}
}

// ===== 焦点监控 =====

pub struct StubFocusMonitor;

impl FocusMonitor for StubFocusMonitor {
    fn register_callback(&self, _id: &str, _callback: FocusCallback) {}
    fn unregister_callback(&self, _id: &str) {}
}

// ===== 存储服务 =====

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
