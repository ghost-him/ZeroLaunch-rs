use std::fmt::Debug;
use std::sync::Arc;

/// Timer 唯一标识符，用于取消定时器。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(u64);

impl TimerId {
    /// 由定时器 ID 数值构造（宿主侧把跨进程收到的 u64 还原为 TimerId）。
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// 返回内部 ID 数值（跨进程传输定时器标识时使用）。
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// 定时器模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerMode {
    /// 一次性定时，延迟指定时长后触发一次。
    OneShot,
    /// 重复定时，每隔指定时长触发一次，直到被取消。
    Interval,
}

/// 定时器回调函数类型。
/// 当定时器触发时，携带对应 TimerId 调用此回调。
pub type TimerCallback = Arc<dyn Fn(TimerId) + Send + Sync>;
