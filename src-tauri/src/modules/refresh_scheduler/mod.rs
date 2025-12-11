//! 刷新调度器模块
//!
//! 统一管理程序数据库的刷新触发源：
//! - 定时刷新
//! - 安装监控（文件系统变化触发）
//! - 手动触发

pub mod config;
pub mod installation_monitor;

use self::config::{PartialRefreshSchedulerConfig, RefreshSchedulerConfig};
use self::installation_monitor::InstallationMonitor;
use parking_lot::{Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::info;

pub use config::RefreshSchedulerConfigInner;

/// 刷新触发源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshTrigger {
    /// 定时触发
    Timer,
    /// 安装监控触发
    InstallationMonitor,
    /// 手动触发
    Manual,
}

/// 刷新调度器
///
/// 统一管理所有刷新触发源，支持：
/// - 定时刷新（可配置间隔）
/// - 安装监控（监控开始菜单变化）
/// - 手动触发
///
/// 内部实现了 debounce 逻辑，防止频繁刷新。
///
type Callback = Mutex<Option<Arc<dyn Fn(RefreshTrigger) + Send + Sync>>>;

pub struct RefreshScheduler {
    /// 配置
    config: Arc<RefreshSchedulerConfig>,
    /// 安装监控器
    installation_monitor: Mutex<InstallationMonitor>,
    /// 是否正在运行
    is_running: Arc<AtomicBool>,
    /// 停止信号
    stop_flag: Arc<AtomicBool>,
    /// 手动触发信号
    manual_trigger: Arc<AtomicBool>,
    /// 条件变量：用于在需要刷新时唤醒主线程
    trigger_event: Arc<(Mutex<bool>, Condvar)>,
    /// 重置定时器信号（配置更新时设置）
    reset_timer: Arc<AtomicBool>,
    /// 刷新回调函数（用于配置更新时重新启动）
    refresh_callback: Callback,
}

impl std::fmt::Debug for RefreshScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RefreshScheduler")
            .field("config", &self.config)
            .field("is_running", &self.is_running)
            .field("has_callback", &self.refresh_callback.lock().is_some())
            .finish()
    }
}

impl RefreshScheduler {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RefreshSchedulerConfig::new()),
            installation_monitor: Mutex::new(InstallationMonitor::new()),
            is_running: Arc::new(AtomicBool::new(false)),
            stop_flag: Arc::new(AtomicBool::new(false)),
            manual_trigger: Arc::new(AtomicBool::new(false)),
            trigger_event: Arc::new((Mutex::new(false), Condvar::new())),
            reset_timer: Arc::new(AtomicBool::new(false)),
            refresh_callback: Mutex::new(None),
        }
    }

    /// 设置刷新回调函数
    ///
    /// 注意：此函数仅设置回调，不会启动调度器。
    /// 调度器的启动由 `update_config` 触发。
    pub fn set_callback<F>(&self, on_refresh: F)
    where
        F: Fn(RefreshTrigger) + Send + Sync + 'static,
    {
        *self.refresh_callback.lock() = Some(Arc::new(on_refresh));
    }

    /// 停止调度器
    pub fn stop(&self) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }
        self.stop_flag.store(true, Ordering::SeqCst);
        self.installation_monitor.lock().stop();

        // 唤醒主线程，使其能响应停止信号
        let (_, condvar) = self.trigger_event.as_ref();
        condvar.notify_all();
    }

    /// 更新配置
    ///
    /// 配置更新后会保存配置。如果回调函数已设置，会根据新配置启动或重启调度器。
    pub fn update_config(&self, partial: PartialRefreshSchedulerConfig) {
        // 更新配置
        self.config.update(partial);

        // 检查是否有回调函数
        let callback = {
            let lock = self.refresh_callback.lock();
            lock.clone()
        };

        if let Some(on_refresh) = callback {
            // 如果调度器正在运行，先停止
            if self.is_running.load(Ordering::SeqCst) {
                self.stop();

                // 等待当前线程停止
                let start_wait = Instant::now();
                while self.is_running.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(10));
                    if start_wait.elapsed() > Duration::from_secs(5) {
                        tracing::warn!("Timed out waiting for scheduler to stop");
                        break;
                    }
                }
            }

            // 重置状态
            self.stop_flag.store(false, Ordering::SeqCst);
            self.manual_trigger.store(false, Ordering::SeqCst);
            self.reset_timer.store(false, Ordering::SeqCst);

            // 重置触发事件标志
            {
                let (flag_lock, _) = self.trigger_event.as_ref();
                let mut flag = flag_lock.lock();
                *flag = false;
            }

            // 启动调度器
            self.is_running.store(true, Ordering::SeqCst);
            self.start_scheduler_thread(on_refresh);
        } else {
            info!("No callback function found, configuration updated but scheduler not started");
        }
    }

    /// 启动调度线程（内部复用）
    fn start_scheduler_thread(&self, on_refresh: Arc<dyn Fn(RefreshTrigger) + Send + Sync>) {
        let config = Arc::clone(&self.config);
        let is_running = Arc::clone(&self.is_running);
        let stop_flag = Arc::clone(&self.stop_flag);
        let manual_trigger = Arc::clone(&self.manual_trigger);
        let trigger_event = Arc::clone(&self.trigger_event);
        let reset_timer = Arc::clone(&self.reset_timer);

        // 启动安装监控器（如果启用）
        let monitor_enabled = config.get_enable_installation_monitor();
        if monitor_enabled {
            self.installation_monitor
                .lock()
                .start(Arc::clone(&trigger_event));
        }

        // 启动主调度线程
        thread::spawn(move || {
            let mut last_timer_refresh = Instant::now();
            let mut last_monitor_event: Option<Instant> = None;
            let mut pending_monitor_refresh = false;

            loop {
                // 检查停止信号
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                // 检查定时器重置信号
                if reset_timer.swap(false, Ordering::SeqCst) {
                    last_timer_refresh = Instant::now();
                }

                let config_snapshot = config.get_snapshot();
                let timer_interval_mins = config_snapshot.auto_refresh_interval_mins;
                let debounce_secs = config_snapshot.monitor_debounce_secs;
                let monitor_enabled = config_snapshot.enable_installation_monitor;

                // 检查手动触发
                if manual_trigger.swap(false, Ordering::SeqCst) {
                    on_refresh(RefreshTrigger::Manual);
                    last_timer_refresh = Instant::now();
                }

                // 检查定时刷新
                let timer_ready = if timer_interval_mins > 0 {
                    let timer_interval = Duration::from_secs(timer_interval_mins * 60);
                    last_timer_refresh.elapsed() >= timer_interval
                } else {
                    false
                };

                if timer_ready {
                    on_refresh(RefreshTrigger::Timer);
                    last_timer_refresh = Instant::now();
                }

                // 检查监控器事件标志
                if monitor_enabled {
                    let (flag_lock, _) = trigger_event.as_ref();
                    let mut flag = flag_lock.lock();
                    if *flag {
                        pending_monitor_refresh = true;
                        last_monitor_event = Some(Instant::now());
                        *flag = false;
                    }
                }

                // 处理安装监控的 debounce
                if pending_monitor_refresh {
                    if let Some(last_event) = last_monitor_event {
                        let debounce_duration = Duration::from_secs(debounce_secs);
                        if last_event.elapsed() >= debounce_duration {
                            on_refresh(RefreshTrigger::InstallationMonitor);
                            pending_monitor_refresh = false;
                            last_monitor_event = None;
                            last_timer_refresh = Instant::now(); // 重置定时器
                        }
                    }
                }

                // 计算下次需要检查的时间
                let mut wait_duration = Duration::from_secs(60); // 默认等待1分钟

                // 如果定时器启用，计算距离下次定时刷新的时间
                if timer_interval_mins > 0 {
                    let timer_interval = Duration::from_secs(timer_interval_mins * 60);
                    if let Some(remaining) =
                        timer_interval.checked_sub(last_timer_refresh.elapsed())
                    {
                        wait_duration = remaining;
                    }
                }

                // 如果有待处理的监控事件，计算 debounce 等待时间
                if pending_monitor_refresh {
                    if let Some(last_event) = last_monitor_event {
                        let debounce_duration = Duration::from_secs(debounce_secs);
                        if let Some(remaining) = debounce_duration.checked_sub(last_event.elapsed())
                        {
                            wait_duration = wait_duration.min(remaining);
                        }
                    }
                }

                // 使用条件变量等待，支持被事件唤醒或超时返回
                let (flag_lock, condvar) = trigger_event.as_ref();
                let mut flag = flag_lock.lock();
                // 等待条件变量或超时
                let _ = condvar.wait_for(&mut flag, wait_duration);
                drop(flag);
            }

            is_running.store(false, Ordering::SeqCst);
        });
    }

    /// 手动触发一次刷新
    pub fn trigger_refresh(&self) {
        self.manual_trigger.store(true, Ordering::SeqCst);
        // 唤醒主线程，避免等待
        let (_, condvar) = self.trigger_event.as_ref();
        condvar.notify_one();
    }

    /// 检查调度器是否正在运行
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

impl Default for RefreshScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RefreshScheduler {
    fn drop(&mut self) {
        self.stop();
    }
}
