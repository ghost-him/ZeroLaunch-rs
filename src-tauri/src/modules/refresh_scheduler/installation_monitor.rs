//! 安装监控器
//! 监控 Windows 开始菜单的变化，检测程序的安装和卸载

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::{Condvar, Mutex};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tracing::{error, info};

/// 安装监控器
///
/// 负责监控 Windows 开始菜单目录的变化，当检测到文件系统变化时，
/// 通过条件变量触发刷新事件。
#[derive(Debug)]
pub struct InstallationMonitor {
    /// 文件系统监控器
    watcher: Option<RecommendedWatcher>,
}

impl InstallationMonitor {
    pub fn new() -> Self {
        Self { watcher: None }
    }

    /// 获取需要监控的路径列表
    fn get_watch_paths() -> Vec<PathBuf> {
        let mut paths = vec![PathBuf::from(
            "C:\\ProgramData\\Microsoft\\Windows\\Start Menu",
        )];

        if let Ok(appdata) = std::env::var("APPDATA") {
            let user_start_menu = PathBuf::from(appdata).join("Microsoft\\Windows\\Start Menu");
            paths.push(user_start_menu);
        }
        paths
    }

    /// 启动监控器，返回条件变量的引用
    ///
    /// 当监控到文件系统变化时，会通过条件变量的 notify_one() 发送信号。
    /// 调用方应该通过条件变量等待事件。
    pub fn start(&mut self, notify_condvar: Arc<(Mutex<bool>, Condvar)>) -> bool {
        if self.watcher.is_some() {
            info!("Installation monitor is already running");
            return false;
        }

        let (notify_tx, notify_rx) = std::sync::mpsc::channel();

        let mut watcher = match RecommendedWatcher::new(notify_tx, Config::default()) {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to create watcher: {:?}", e);
                return false;
            }
        };

        let paths = Self::get_watch_paths();

        for path in &paths {
            if path.exists() {
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    error!("Failed to watch path {:?}: {:?}", path, e);
                }
            }
        }

        self.watcher = Some(watcher);

        // 启动事件处理线程
        thread::spawn(move || {
            loop {
                match notify_rx.recv() {
                    Ok(res) => {
                        match res {
                            Ok(_) => {
                                // 通过条件变量通知主线程
                                let (lock, condvar) = notify_condvar.as_ref();
                                let mut flag = lock.lock();
                                *flag = true;
                                drop(flag);
                                condvar.notify_one();
                            }
                            Err(e) => error!("Watch error: {:?}", e),
                        }
                    }
                    Err(_) => {
                        // Channel 已关闭，退出线程
                        info!("Installation monitor channel closed, stopping...");
                        break;
                    }
                }
            }
        });

        true
    }

    /// 停止监控器
    pub fn stop(&mut self) {
        if self.watcher.is_some() {
            info!("Stopping installation monitor...");
            self.watcher = None;
        }
    }

    /// 检查监控器是否正在运行
    pub fn is_running(&self) -> bool {
        self.watcher.is_some()
    }
}

impl Default for InstallationMonitor {
    fn default() -> Self {
        Self::new()
    }
}
