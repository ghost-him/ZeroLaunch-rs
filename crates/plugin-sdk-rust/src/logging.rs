//! 插件日志系统。
//!
//! 提供双写能力：
//! - 主干：`tracing` 事件 → stderr → 宿主收集到 `<log-dir>/<plugin-id>.log`（所有级别，崩溃安全）
//! - 旁路：WARN/ERROR 事件 → 非阻塞通道 → 后台任务 → `host/log` RPC（尽力而为转发到宿主）
//!
//! 插件开发者只需使用标准 `tracing::info!()` / `tracing::warn!()` / `tracing::error!()` 宏，
//! 无需调用 `host().log(...).await`。

use std::io;
use tokio::sync::mpsc;

/// 一条待转发到宿主的日志条目。
pub struct LogEntry {
    pub level: String,
    pub message: String,
}

/// 将 `tracing` 格式化输出写入 log 通道的 writer。
///
/// 每个实例绑定一个固定的日志级别（"warn" 或 "error"），
/// 通过不同的 `tracing_subscriber::fmt::Layer` 实例分级别过滤。
#[derive(Clone)]
struct HostLogWriter {
    tx: mpsc::UnboundedSender<LogEntry>,
    level: &'static str,
}

impl HostLogWriter {
    fn new(tx: mpsc::UnboundedSender<LogEntry>, level: &'static str) -> Self {
        Self { tx, level }
    }
}

impl io::Write for HostLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let message = String::from_utf8_lossy(buf).trim_end().to_string();
        if !message.is_empty() {
            // 通道满了就丢弃，不阻塞 tracing 调用者
            let _ = self.tx.send(LogEntry {
                level: self.level.to_string(),
                message,
            });
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for HostLogWriter {
    type Writer = HostLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// 初始化插件日志系统。
///
/// # 返回值
///
/// 返回 `mpsc::UnboundedReceiver<LogEntry>`，调用方需要 spawn 一个后台任务消费该通道，
/// 将 WARN/ERROR 日志转发到宿主。
///
/// # Panics
///
/// 如果全局 subscriber 已被设置（例如在测试中），`try_init()` 会静默忽略。
pub fn init_logging() -> mpsc::UnboundedReceiver<LogEntry> {
    use tracing_subscriber::filter::{filter_fn, LevelFilter};
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::Registry;

    let (log_tx, log_rx) = mpsc::unbounded_channel::<LogEntry>();

    // 如果已设置（如嵌套测试），不重复初始化
    let _ = Registry::default()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(io::stderr)
                .with_filter(LevelFilter::INFO),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(HostLogWriter::new(log_tx.clone(), "warn"))
                .with_filter(filter_fn(|metadata| {
                    *metadata.level() == tracing::Level::WARN
                })),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(HostLogWriter::new(log_tx.clone(), "error"))
                .with_filter(filter_fn(|metadata| {
                    *metadata.level() == tracing::Level::ERROR
                })),
        )
        .try_init();

    log_rx
}
