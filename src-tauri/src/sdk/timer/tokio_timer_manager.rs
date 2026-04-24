use crate::sdk::host_api::HostApiError;
use crate::sdk::timer::timer_manager::TimerManager;
use crate::sdk::timer::types::{TimerCallback, TimerId, TimerMode};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::MissedTickBehavior;

/// 定时器条目，用于通过条件变量机制取消定时器。
struct TimerEntry {
    /// 条件变量通知器，用于唤醒等待中的定时器任务以提前取消。
    notify: Arc<Notify>,
}

/// 基于 tokio 的定时器管理器实现。
/// 内部使用 tokio::sync::Notify 作为条件变量机制，支持高效的取消操作。
/// 所有定时器在 tokio 运行时中异步执行，不阻塞任何线程。
pub struct TokioTimerManager {
    /// 自增 ID 生成器
    next_id: AtomicU64,
    /// 活跃定时器集合（使用 Arc 包装以便在异步任务中共享）
    timers: Arc<DashMap<TimerId, TimerEntry>>,
}

impl TokioTimerManager {
    /// 创建 TokioTimerManager 实例。
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            timers: Arc::new(DashMap::new()),
        }
    }
}

impl Default for TokioTimerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TimerManager for TokioTimerManager {
    async fn set_timer(
        &self,
        delay: Duration,
        mode: TimerMode,
        callback: TimerCallback,
    ) -> Result<TimerId, HostApiError> {
        let id = TimerId::new(self.next_id.fetch_add(1, Ordering::SeqCst));

        let notify = Arc::new(Notify::new());

        self.timers.insert(
            id,
            TimerEntry {
                notify: notify.clone(),
            },
        );

        let timers = self.timers.clone();

        tokio::spawn(async move {
            match mode {
                TimerMode::OneShot => {
                    tokio::select! {
                        _ = tokio::time::sleep(delay) => {
                            callback(id);
                            timers.remove(&id);
                        }
                        _ = notify.notified() => {
                        }
                    }
                }
                TimerMode::Interval => {
                    let mut interval = tokio::time::interval(delay);
                    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
                    interval.tick().await;

                    loop {
                        tokio::select! {
                            _ = interval.tick() => {
                                callback(id);
                            }
                            _ = notify.notified() => {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(id)
    }

    async fn cancel_timer(&self, id: TimerId) -> Result<(), HostApiError> {
        if let Some((_, entry)) = self.timers.remove(&id) {
            entry.notify.notify_one();
        }
        Ok(())
    }

    async fn cancel_all(&self) -> Result<(), HostApiError> {
        let entries: Vec<TimerEntry> = self
            .timers
            .iter()
            .map(|e| TimerEntry {
                notify: e.notify.clone(),
            })
            .collect();
        self.timers.clear();
        for entry in entries {
            entry.notify.notify_one();
        }
        Ok(())
    }
}
