use crate::sdk::host_api::HostApiError;
use crate::sdk::timer::types::{TimerCallback, TimerId, TimerMode};
use async_trait::async_trait;
use std::time::Duration;

/// 定时器管理器 trait，定义平台无关的定时调度能力契约。
/// 支持一次性定时和重复定时的创建与取消。
/// 内部基于 tokio::sync::Notify（条件变量机制）实现触发。
#[async_trait]
pub trait TimerManager: Send + Sync {
    /// 创建一个定时器。
    ///
    /// 参数：
    /// - delay: 延迟时长（OneShot 为触发延迟，Interval 为重复间隔）
    /// - mode: 定时模式（一次性或重复）
    /// - callback: 触发时调用的回调函数，接收 TimerId 作为参数
    ///
    /// 返回：TimerId，可用于后续取消定时器。
    async fn set_timer(
        &self,
        delay: Duration,
        mode: TimerMode,
        callback: TimerCallback,
    ) -> Result<TimerId, HostApiError>;

    /// 取消指定 ID 的定时器。
    ///
    /// 参数：id - 要取消的定时器 ID。
    /// 返回：成功返回 Ok(())，定时器不存在时也返回 Ok(())。
    async fn cancel_timer(&self, id: TimerId) -> Result<(), HostApiError>;

    /// 取消所有定时器。
    async fn cancel_all(&self) -> Result<(), HostApiError>;
}
