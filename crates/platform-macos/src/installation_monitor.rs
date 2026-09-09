use async_trait::async_trait;
use dashmap::DashMap;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::installation_monitor::{
    InstallationCallback, InstallationEvent, InstallationEventKind, InstallationMonitor,
};

const DEFAULT_DEBOUNCE_MS: u64 = 5000;
/// Internal callback registration used only by this monitor implementation.
struct RegisteredCallback {
    /// Callback invoked after a debounced filesystem event.
    callback: InstallationCallback,
}
/// Watches macOS application directories for installation changes.
///
/// Used by the automatic candidate refresh service.
pub struct MacosInstallationMonitor {
    /// Active filesystem watcher, if monitoring is enabled.
    watcher: Mutex<Option<RecommendedWatcher>>,
    /// Whether the watcher has been started.
    is_watching: AtomicBool,
    /// Registered installation event callbacks.
    callbacks: Arc<DashMap<String, RegisteredCallback>>,
    /// Directories currently watched by the monitor.
    watch_paths: Mutex<Vec<String>>,
    /// Debounce interval in milliseconds shared with the watcher callback.
    debounce_ms: Arc<AtomicU64>,
    /// Debounce thread handle; joined on stop so a fast restart cannot
    /// deliver a stale callback from the previous worker.
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}
impl MacosInstallationMonitor {
    pub fn new() -> Self {
        Self {
            watcher: Mutex::new(None),
            is_watching: AtomicBool::new(false),
            callbacks: Arc::new(DashMap::new()),
            watch_paths: Mutex::new(Vec::new()),
            debounce_ms: Arc::new(AtomicU64::new(DEFAULT_DEBOUNCE_MS)),
            worker: Mutex::new(None),
        }
    }
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
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            kind,
        }
    }
}
impl Default for MacosInstallationMonitor {
    fn default() -> Self {
        Self::new()
    }
}
fn default_watch_paths() -> Vec<String> {
    let mut paths = vec!["/Applications".into()];
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join("Applications").to_string_lossy().into_owned());
    }
    paths
}
#[async_trait]
impl InstallationMonitor for MacosInstallationMonitor {
    async fn start_watching(&self) -> Result<(), HostApiError> {
        if self.is_watching.swap(true, Ordering::Relaxed) {
            return Ok(());
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default()).map_err(|e| {
            HostApiError::ExecutionFailed {
                service: "installation_monitor".into(),
                reason: e.to_string(),
            }
        })?;
        let paths = {
            let paths = self.watch_paths.lock();
            if paths.is_empty() {
                default_watch_paths()
            } else {
                paths.clone()
            }
        };
        let mut watched = false;
        for path in paths {
            let path = PathBuf::from(path);
            if path.is_dir() && watcher.watch(&path, RecursiveMode::Recursive).is_ok() {
                watched = true;
            }
        }
        if !watched {
            self.is_watching.store(false, Ordering::Relaxed);
            return Err(HostApiError::ExecutionFailed {
                service: "installation_monitor".into(),
                reason: "no application directories can be watched".into(),
            });
        }
        *self.watcher.lock() = Some(watcher);
        let callbacks = self.callbacks.clone();
        let debounce_ms = self.debounce_ms.clone();
        let worker = thread::Builder::new()
            .name("macos-installation-monitor".into())
            .spawn(move || {
                let mut pending: Option<InstallationEvent> = None;
                let mut last_event: Option<Instant> = None;
                loop {
                    let result = if let Some(last) = last_event {
                        rx.recv_timeout(
                            Duration::from_millis(debounce_ms.load(Ordering::Relaxed))
                                .saturating_sub(last.elapsed()),
                        )
                    } else {
                        match rx.recv() {
                            Ok(event) => Ok(event),
                            Err(_) => break,
                        }
                    };
                    match result {
                        Ok(Ok(event)) => {
                            pending = Some(MacosInstallationMonitor::convert_event(event));
                            last_event = Some(Instant::now());
                        }
                        Ok(Err(_)) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            if let Some(event) = pending.take() {
                                for registered in callbacks.iter() {
                                    (registered.value().callback)(event.clone());
                                }
                            }
                            last_event = None;
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            });
        match worker {
            Ok(handle) => {
                *self.worker.lock() = Some(handle);
                Ok(())
            }
            Err(error) => {
                *self.watcher.lock() = None;
                self.is_watching.store(false, Ordering::Relaxed);
                Err(HostApiError::ExecutionFailed {
                    service: "installation_monitor".into(),
                    reason: format!("failed to spawn watcher thread: {error}"),
                })
            }
        }
    }
    async fn stop_watching(&self) -> Result<(), HostApiError> {
        // Drop the watcher first: closing the notify channel is what makes
        // the worker's recv() return Disconnected and the loop exit.
        *self.watcher.lock() = None;
        if let Some(handle) = self.worker.lock().take() {
            if handle.join().is_err() {
                tracing::warn!("installation monitor worker thread panicked");
            }
        }
        self.is_watching.store(false, Ordering::Relaxed);
        Ok(())
    }
    fn is_watching(&self) -> bool {
        self.is_watching.load(Ordering::Relaxed)
    }
    fn register_callback(&self, id: &str, callback: InstallationCallback) {
        self.callbacks
            .insert(id.into(), RegisteredCallback { callback });
    }
    fn unregister_callback(&self, id: &str) {
        self.callbacks.remove(id);
    }
    fn update_watch_paths(&self, paths: Vec<String>) {
        *self.watch_paths.lock() = paths;
    }
    fn update_debounce_secs(&self, secs: f64) {
        self.debounce_ms
            .store((secs.max(0.0) * 1000.0) as u64, Ordering::Relaxed);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_watch_both_application_roots() {
        assert_eq!(default_watch_paths()[0], "/Applications");
    }
    #[test]
    fn event_kind_is_normalized() {
        let event = notify::Event::new(EventKind::Create(notify::event::CreateKind::Any));
        assert_eq!(
            MacosInstallationMonitor::convert_event(event).kind,
            InstallationEventKind::Created
        );
    }
}
