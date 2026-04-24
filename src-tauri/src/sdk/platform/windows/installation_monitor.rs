use crate::sdk::host_api::HostApiError;
use crate::sdk::installation_monitor::types::{
    CallbackRegistration, InstallationCallback, InstallationEvent, InstallationEventKind,
};
use crate::sdk::installation_monitor::InstallationMonitor;
use async_trait::async_trait;
use dashmap::DashMap;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tracing::{error, info, warn};

/// Windows 平台安装监控器实现。
/// 使用 `notify` crate 监控指定目录的文件系统变化，
/// 通过 DashMap 管理多个回调，事件发生时依次调用。
pub struct WindowsInstallationMonitor {
    /// 文件系统监控器
    watcher: Mutex<Option<RecommendedWatcher>>,
    /// 是否正在监控
    is_watching: AtomicBool,
    /// 回调注册表
    callbacks: Arc<DashMap<String, CallbackRegistration>>,
    /// 当前监控路径列表
    watch_paths: Mutex<Vec<String>>,
}

impl WindowsInstallationMonitor {
    /// 创建 WindowsInstallationMonitor 实例。
    pub fn new() -> Self {
        Self {
            watcher: Mutex::new(None),
            is_watching: AtomicBool::new(false),
            callbacks: Arc::new(DashMap::new()),
            watch_paths: Mutex::new(Vec::new()),
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

        // 获取监控路径：优先使用用户配置的路径，为空则使用平台默认路径
        let paths = {
            let configured = self.watch_paths.lock();
            configured.clone()
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

        // 启动事件处理线程
        let callbacks = self.callbacks.clone();
        thread::spawn(move || {
            loop {
                match notify_rx.recv() {
                    Ok(Ok(event)) => {
                        let install_event = WindowsInstallationMonitor::convert_event(event);
                        for entry in callbacks.iter() {
                            (entry.value().callback)(install_event.clone());
                        }
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
        self.callbacks.insert(
            id.to_string(),
            CallbackRegistration {
                id: id.to_string(),
                callback,
            },
        );
    }

    fn unregister_callback(&self, id: &str) {
        self.callbacks.remove(id);
    }

    fn update_watch_paths(&self, paths: Vec<String>) {
        *self.watch_paths.lock() = paths;
    }
}
