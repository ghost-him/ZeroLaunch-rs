//! 请求合并刷新闸门。
//!
//! 用于「同一份缓存被多处触发重建」的场景：并发触发的重建请求合并为一轮执行，
//! 重建期间到达的请求在其结束后补一轮（不丢更新）。闸门不持有业务数据，
//! 只负责排队、票号与完成度，实际重建动作由调用方以闭包传入。

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// 刷新闸门 —— 并发刷新请求合并执行，每个调用方仍能等到「覆盖自己请求」的那一轮完成。
///
/// 使用范围：需要「单飞 + 补跑」语义的缓存重建入口（当前为 `SessionDispatcher` 的
/// 候选项采集）。不承载业务逻辑，不缓存数据，也不感知重建内容。
pub struct CoalescingRefresh {
    /// 执行权：持锁者跑一轮重建，其余调用方排队后判断自己的请求是否已被覆盖。
    executing: tokio::sync::Mutex<()>,
    /// 已登记的请求票号（每次 `run` 取一张，单调递增）。
    requests: AtomicU64,
    /// 已完成重建覆盖到的最大票号（0 = 从未完成过任何一轮）。
    completed: tokio::sync::watch::Sender<u64>,
}

impl Default for CoalescingRefresh {
    fn default() -> Self {
        Self::new()
    }
}

impl CoalescingRefresh {
    /// 创建闸门（尚未完成任何一轮）。
    pub fn new() -> Self {
        Self {
            executing: tokio::sync::Mutex::new(()),
            requests: AtomicU64::new(0),
            completed: tokio::sync::watch::channel(0u64).0,
        }
    }

    /// 是否至少完成过一轮重建。
    pub fn is_ready(&self) -> bool {
        *self.completed.borrow() > 0
    }

    /// 等待至少完成一轮重建。
    /// 参数：timeout - 最长等待时长。
    /// 返回：是否在超时前完成过一轮（超时由调用方按现有数据继续）。
    pub async fn wait_ready(&self, timeout: Duration) -> bool {
        if self.is_ready() {
            return true;
        }
        let mut rx = self.completed.subscribe();
        let outcome = tokio::time::timeout(timeout, rx.wait_for(|done| *done > 0)).await;
        matches!(outcome, Ok(Ok(_)))
    }

    /// 登记一次重建请求并等待完成；排队期间若已被覆盖则不执行 `work`。
    ///
    /// 参数：work - 实际重建动作（最多执行一次，且仅在需要时执行）。
    /// 返回：本轮是否实际执行了 `work`。
    pub async fn run<F, Fut>(&self, work: F) -> bool
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = ()>,
    {
        let ticket = self.requests.fetch_add(1, Ordering::SeqCst) + 1;
        let _guard = self.executing.lock().await;
        // 排队期间本票已被覆盖：所需结果已经产出。
        if *self.completed.borrow() >= ticket {
            return false;
        }
        // 先取覆盖上界再执行：此前登记的请求由本轮覆盖，此后到达的留待下一轮。
        let target = self.requests.load(Ordering::SeqCst);
        work().await;
        self.completed.send_replace(target);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 带计数与耗时的重建动作：每轮 +1 次调用并占用一段时间，
    /// 使并发调用方落入「执行中到达」的窗口。
    /// 参数：gate - 被测闸门；calls - 重建执行次数计数器。
    /// 返回：本轮是否实际执行了重建。
    async fn counted_run(gate: &CoalescingRefresh, calls: &AtomicU64) -> bool {
        gate.run(|| async {
            calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(10)).await;
        })
        .await
    }

    /// 首轮完成前未就绪；完成一轮后立即就绪，`wait_ready` 随之返回。
    #[tokio::test]
    async fn ready_flips_after_first_completed_run() {
        let gate = CoalescingRefresh::new();
        assert!(!gate.is_ready(), "初始应为未就绪");
        assert!(
            !gate.wait_ready(Duration::from_millis(10)).await,
            "未执行时等待应超时返回 false"
        );

        assert!(gate.run(|| async {}).await, "首个请求应执行重建");

        assert!(gate.is_ready(), "完成一轮后应就绪");
        assert!(
            gate.wait_ready(Duration::from_millis(10)).await,
            "就绪后应立即返回 true"
        );
    }

    /// 执行中到达的多个请求合并为一轮补扫：1 轮 + 4 个迟到请求 → 共 2 轮（不是 5 轮）。
    #[tokio::test]
    async fn concurrent_requests_coalesce_into_single_extra_pass() {
        let gate = CoalescingRefresh::new();
        let calls = AtomicU64::new(0);

        // join! 按顺序首次轮询：首个请求先持锁并读取覆盖上界（=1）后进入耗时重建，
        // 其余四个在其执行期间登记票号（2..5）。
        let (first, second, third, fourth, fifth) = tokio::join!(
            counted_run(&gate, &calls),
            counted_run(&gate, &calls),
            counted_run(&gate, &calls),
            counted_run(&gate, &calls),
            counted_run(&gate, &calls),
        );

        assert!(first, "首个请求应执行重建");
        assert!(second, "执行中到达的请求应在结束后触发一轮补扫");
        assert!(!third && !fourth && !fifth, "其余迟到请求应复用补扫结果");
        assert_eq!(calls.load(Ordering::SeqCst), 2, "5 个请求应合并为 2 轮重建");
    }
}
