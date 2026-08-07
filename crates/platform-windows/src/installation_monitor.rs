use async_trait::async_trait;
use dashmap::DashMap;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};
use windows::Win32::UI::Shell::{
    FOLDERID_CommonStartMenu, FOLDERID_StartMenu, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
};
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::installation_monitor::{
    InstallationCallback, InstallationEvent, InstallationEventKind, InstallationMonitor,
};

/// 默认去抖时长（毫秒），与配置组件 `monitor_debounce_secs` 的默认值 5.0 对应。
const DEFAULT_DEBOUNCE_MS: u64 = 5000;

/// 回调注册信息（内部使用）。
struct CallbackRegistration {
    /// 回调函数
    pub callback: InstallationCallback,
}

/// Windows 平台安装监控器实现。
/// 使用 `notify` crate 监控指定目录的文件系统变化，
/// 通过 DashMap 管理多个回调，事件发生时依次调用。
///
/// 仅在 platform-windows crate 内使用，由 HostApi 装配时注入；
/// 事件在平台层做滑动窗口去抖（静默满去抖时长才分发合并事件），
/// 调用方（配置组件/宿主）通过 HostApi 的 update_* 方法下发参数。
pub struct WindowsInstallationMonitor {
    /// 文件系统监控器
    watcher: Mutex<Option<RecommendedWatcher>>,
    /// 是否正在监控
    is_watching: AtomicBool,
    /// 回调注册表
    callbacks: Arc<DashMap<String, CallbackRegistration>>,
    /// 当前监控路径列表（空列表表示使用平台默认开始菜单路径）
    watch_paths: Mutex<Vec<String>>,
    /// 事件去抖时长（毫秒）：事件静默满该时长后才分发回调。
    /// 使用 Arc<AtomicU64> 供事件线程无锁读取，仅在 update_debounce_secs 时写入。
    debounce_ms: Arc<AtomicU64>,
}

impl WindowsInstallationMonitor {
    /// 创建 WindowsInstallationMonitor 实例。
    pub fn new() -> Self {
        Self {
            watcher: Mutex::new(None),
            is_watching: AtomicBool::new(false),
            callbacks: Arc::new(DashMap::new()),
            watch_paths: Mutex::new(Vec::new()),
            debounce_ms: Arc::new(AtomicU64::new(DEFAULT_DEBOUNCE_MS)),
        }
    }

    /// 将 notify::Event 转换为 InstallationEvent。
    fn convert_event(event: notify::Event) -> InstallationEvent {
        let kind = match event.kind {
            EventKind::Create(_) => InstallationEventKind::Created,
            EventKind::Modify(_) => InstallationEventKind::Modified,
            EventKind::Remove(_) => InstallationEventKind::Removed,
            _ => InstallationEventKind::Other,
        };
        InstallationEvent {
            changed_paths: event
                .paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            kind,
        }
    }
}

/// 平台默认监控路径：公共开始菜单 + 当前用户开始菜单（Windows 开始菜单）。
///
/// 优先使用 SHGetKnownFolderPath 解析（与 WindowsPathResolver 同一实现），
/// 全部失败时回退到环境变量拼接（老版做法），保证默认配置下监控可用。
fn default_watch_paths() -> Vec<String> {
    let mut paths = Vec::new();
    unsafe {
        for folder_id in [&FOLDERID_CommonStartMenu, &FOLDERID_StartMenu] {
            if let Ok(pwstr) = SHGetKnownFolderPath(folder_id, KF_FLAG_DEFAULT, None) {
                if let Ok(s) = pwstr.to_string() {
                    if !s.is_empty() {
                        paths.push(s);
                    }
                }
            }
        }
    }
    if paths.is_empty() {
        // 兜底：环境变量拼接
        if let Ok(program_data) = std::env::var("ProgramData") {
            paths.push(format!(r"{}\Microsoft\Windows\Start Menu", program_data));
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            paths.push(format!(r"{}\Microsoft\Windows\Start Menu", appdata));
        }
    }
    paths
}

impl Default for WindowsInstallationMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InstallationMonitor for WindowsInstallationMonitor {
    async fn start_watching(&self) -> Result<(), HostApiError> {
        if self.is_watching.load(Ordering::Relaxed) {
            info!("Installation monitor is already watching");
            return Ok(());
        }

        let (notify_tx, notify_rx) = std::sync::mpsc::channel();

        let mut watcher =
            RecommendedWatcher::new(notify_tx, notify::Config::default()).map_err(|e| {
                HostApiError::ExecutionFailed {
                    service: "installation_monitor".to_string(),
                    reason: format!("创建文件监控器失败: {:?}", e),
                }
            })?;

        // 获取监控路径：优先使用用户配置的路径，为空则回退到平台默认路径（开始菜单）
        let paths = {
            let configured = self.watch_paths.lock();
            if configured.is_empty() {
                info!("未配置监控路径，使用平台默认路径（开始菜单）");
                default_watch_paths()
            } else {
                configured.clone()
            }
        };

        for path in &paths {
            if PathBuf::from(path).exists() {
                if let Err(e) = watcher.watch(&PathBuf::from(path), RecursiveMode::Recursive) {
                    warn!("Failed to watch path {:?}: {:?}", path, e);
                } else {
                    info!("Started watching path: {}", path);
                }
            } else {
                warn!("Watch path does not exist, skipping: {}", path);
            }
        }

        *self.watcher.lock() = Some(watcher);
        self.is_watching.store(true, Ordering::Relaxed);

        // 启动事件处理线程：滑动窗口去抖。
        // 事件到达后进入去抖窗口，窗口内新事件重置计时并合并（保留最新事件）；
        // 静默满 debounce_ms 后仅分发一次合并事件，避免安装/卸载批处理触发多次刷新。
        let callbacks = self.callbacks.clone();
        let debounce_ms = self.debounce_ms.clone();
        thread::spawn(move || {
            let mut pending: Option<InstallationEvent> = None;
            let mut last_event_at: Option<Instant> = None;
            loop {
                let debounce = Duration::from_millis(debounce_ms.load(Ordering::Relaxed));
                if pending.is_some() {
                    // 去抖窗口内：等待剩余静默时间或新事件（新事件重置计时）
                    let wait = match last_event_at {
                        Some(t) => debounce.saturating_sub(t.elapsed()),
                        None => debounce,
                    };
                    match notify_rx.recv_timeout(wait) {
                        Ok(Ok(event)) => {
                            pending = Some(WindowsInstallationMonitor::convert_event(event));
                            last_event_at = Some(Instant::now());
                        }
                        Ok(Err(e)) => {
                            error!("Watch error: {:?}", e);
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            // 静默满去抖时长：分发合并事件
                            if let Some(event) = pending.take() {
                                for entry in callbacks.iter() {
                                    (entry.value().callback)(event.clone());
                                }
                                last_event_at = None;
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                            // Channel 已关闭，退出线程
                            info!("Installation monitor channel closed, stopping...");
                            break;
                        }
                    }
                } else {
                    match notify_rx.recv() {
                        Ok(Ok(event)) => {
                            // 收到首个事件：进入去抖窗口
                            pending = Some(WindowsInstallationMonitor::convert_event(event));
                            last_event_at = Some(Instant::now());
                        }
                        Ok(Err(e)) => {
                            error!("Watch error: {:?}", e);
                        }
                        Err(_) => {
                            // Channel 已关闭，退出线程
                            info!("Installation monitor channel closed, stopping...");
                            break;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    async fn stop_watching(&self) -> Result<(), HostApiError> {
        if !self.is_watching.load(Ordering::Relaxed) {
            return Ok(());
        }

        info!("Stopping installation monitor...");
        // 丢弃 watcher 会关闭 channel，事件处理线程将退出
        *self.watcher.lock() = None;
        self.is_watching.store(false, Ordering::Relaxed);

        Ok(())
    }

    fn is_watching(&self) -> bool {
        self.is_watching.load(Ordering::Relaxed)
    }

    fn register_callback(&self, id: &str, callback: InstallationCallback) {
        self.callbacks
            .insert(id.to_string(), CallbackRegistration { callback });
    }

    fn unregister_callback(&self, id: &str) {
        self.callbacks.remove(id);
    }

    fn update_watch_paths(&self, paths: Vec<String>) {
        *self.watch_paths.lock() = paths;
    }

    fn update_debounce_secs(&self, secs: f64) {
        // <=0 视为立即分发（不进入去抖窗口）；正常范围由配置组件校验（1-60 秒）
        let ms = if secs > 0.0 {
            (secs * 1000.0) as u64
        } else {
            0
        };
        self.debounce_ms.store(ms, Ordering::Relaxed);
    }
}
