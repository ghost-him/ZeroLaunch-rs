//! 日志系统模块
//!
//! 提供统一的日志管理功能，包括：
//! - 日志初始化和配置
//! - 系统信息记录
//! - Panic处理
//! - 日志文件清理

use crate::error::{OptionExt, ResultExt};
use crate::modules::config::default::LOG_DIR;
use backtrace::Backtrace;
use chrono::{DateTime, Local};
use std::fs::File;
use std::io::Write;
use std::panic;
use std::path::Path;
use tracing::{debug, error, info, warn, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber;

/// 日志系统配置
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// 日志级别
    pub level: Level,
    /// 日志文件保留天数
    pub retention_days: i64,
    /// 是否启用控制台输出
    pub enable_console: bool,
    /// 日志文件名前缀
    pub log_file_prefix: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: Level::DEBUG,
            retention_days: 5,
            enable_console: true,
            log_file_prefix: "info".to_string(),
        }
    }
}

/// 初始化日志系统
///
/// # Arguments
///
/// * `config` - 日志配置，如果为None则使用默认配置
///
/// # Returns
///
/// 返回日志守护者，需要保持其生命周期直到程序结束
pub fn init_logging(config: Option<LoggingConfig>) -> tracing_appender::non_blocking::WorkerGuard {
    let config = config.unwrap_or_default();

    // 确保日志目录存在
    if let Err(e) = std::fs::create_dir_all(&*LOG_DIR) {
        eprintln!("创建日志目录失败: {}", e);
    }

    // 打印系统信息
    print_system_info();

    // 创建按日期滚动的日志文件
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        LOG_DIR.clone(),
        format!("{}.log", config.log_file_prefix),
    );

    // 创建非阻塞的日志写入器
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 配置订阅者
    if config.enable_console {
        // 同时输出到文件和控制台
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);

        let console_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(true)
            .with_target(true)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false);

        tracing_subscriber::registry()
            .with(file_layer)
            .with(console_layer)
            .with(tracing_subscriber::filter::LevelFilter::from_level(
                config.level,
            ))
            .init();
    } else {
        // 仅输出到文件
        let subscriber = tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_max_level(config.level)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect_programming("设置全局默认订阅者失败");
    }

    // 设置panic hook
    setup_panic_hook();

    // 清理旧日志文件
    cleanup_old_logs(&LOG_DIR, config.retention_days);

    info!("日志系统初始化完成");
    info!("日志级别: {:?}", config.level);
    info!("日志保留天数: {}", config.retention_days);
    info!("日志目录: {}", *LOG_DIR);

    guard
}

/// 打印系统信息
fn print_system_info() {
    // 由于tracing还未初始化，这里使用println!输出到控制台
    println!("=== ZeroLaunch-rs 系统信息 ===");
    println!("应用版本: {}", env!("CARGO_PKG_VERSION"));
    println!(
        "构建时间: {}",
        std::env::var("VERGEN_BUILD_TIMESTAMP").unwrap_or_else(|_| "未知".to_string())
    );
    println!("操作系统: {}", std::env::consts::OS);
    println!("系统架构: {}", std::env::consts::ARCH);
    println!(
        "Rust版本: {}",
        std::env::var("VERGEN_RUSTC_SEMVER").unwrap_or_else(|_| "未知".to_string())
    );

    // 获取系统时间
    let now = Local::now();
    println!("启动时间: {}", now.format("%Y-%m-%d %H:%M:%S"));

    // 获取工作目录
    if let Ok(current_dir) = std::env::current_dir() {
        println!("工作目录: {:?}", current_dir);
    }

    // 获取可执行文件路径
    if let Ok(exe_path) = std::env::current_exe() {
        println!("可执行文件: {:?}", exe_path);
    }

    // 获取环境变量信息
    if let Ok(user) = std::env::var("USERNAME") {
        println!("当前用户: {}", user);
    }

    println!("日志目录: {}", *LOG_DIR);
    println!("==============================");
}

/// 设置panic处理钩子
fn setup_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .expect_programming("无法获取panic位置信息");
        let message = match panic_info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match panic_info.payload().downcast_ref::<String>() {
                Some(s) => s.as_str(),
                None => "未知panic消息",
            },
        };

        let log_dir = LOG_DIR.clone();
        let panic_file_path = Path::new(&log_dir)
            .join("panic.log")
            .to_str()
            .expect_programming("无法转换panic日志路径为字符串")
            .to_string();

        // 写入panic日志文件
        if let Ok(mut file) = File::create(&panic_file_path) {
            let now = Local::now();
            let _ = writeln!(
                file,
                "[{}] Panic发生在文件 '{}' 第{}行: {}",
                now.format("%Y-%m-%d %H:%M:%S"),
                location.file(),
                location.line(),
                message
            );

            // 获取并写入堆栈跟踪
            let backtrace = Backtrace::new();
            let _ = writeln!(file, "\n堆栈跟踪:");
            let _ = writeln!(file, "{:?}", backtrace);

            // 写入系统信息
            let _ = writeln!(file, "\n系统信息:");
            let _ = writeln!(file, "应用版本: {}", env!("CARGO_PKG_VERSION"));
            let _ = writeln!(file, "操作系统: {}", std::env::consts::OS);
            let _ = writeln!(file, "系统架构: {}", std::env::consts::ARCH);

            if let Ok(current_dir) = std::env::current_dir() {
                let _ = writeln!(file, "工作目录: {:?}", current_dir);
            }
        }

        // 同时记录到tracing日志
        error!(
            "Panic发生: {} (位置: {}:{})",
            message,
            location.file(),
            location.line()
        );

        // 输出到stderr
        eprintln!(
            "[PANIC] {} ({}:{})",
            message,
            location.file(),
            location.line()
        );
    }));
}

/// 清理旧的日志文件
///
/// # Arguments
///
/// * `log_dir` - 日志目录路径
/// * `retention_days` - 保留天数
pub fn cleanup_old_logs(log_dir: &str, retention_days: i64) {
    debug!("开始清理旧日志文件，保留天数: {}", retention_days);

    let now: DateTime<Local> = Local::now();
    let mut cleaned_count = 0;
    let mut total_size_cleaned = 0u64;

    // 读取日志目录中的所有文件
    let entries = match std::fs::read_dir(log_dir) {
        Ok(entries) => entries,
        Err(e) => {
            error!("无法读取日志目录 '{}': {}", log_dir, e);
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            // 获取文件的元数据
            if let Ok(metadata) = std::fs::metadata(&path) {
                // 获取文件大小
                let file_size = metadata.len();

                // 获取文件的修改时间
                if let Ok(modified) = metadata.modified() {
                    // 将 SystemTime 转换为 DateTime
                    let modified_datetime: DateTime<Local> = modified.into();
                    // 计算文件的年龄
                    let age = now.signed_duration_since(modified_datetime);

                    if age.num_days() > retention_days {
                        // 删除文件
                        match std::fs::remove_file(&path) {
                            Ok(_) => {
                                cleaned_count += 1;
                                total_size_cleaned += file_size;
                                info!(
                                    "已删除旧日志文件: {:?} (大小: {} bytes, 年龄: {} 天)",
                                    path,
                                    file_size,
                                    age.num_days()
                                );
                            }
                            Err(e) => {
                                error!("无法删除旧日志文件 '{:?}': {}", path, e);
                            }
                        }
                    }
                }
            }
        }
    }

    if cleaned_count > 0 {
        info!(
            "日志清理完成: 删除了 {} 个文件，释放了 {} bytes 空间",
            cleaned_count, total_size_cleaned
        );
    } else {
        debug!("没有需要清理的旧日志文件");
    }
}

/// 记录应用启动信息
pub fn log_application_start() {
    info!("=== ZeroLaunch-rs 应用启动 ===");
    info!("应用版本: {}", env!("CARGO_PKG_VERSION"));
    info!("操作系统: {}", std::env::consts::OS);
    info!("系统架构: {}", std::env::consts::ARCH);

    let now = Local::now();
    info!("启动时间: {}", now.format("%Y-%m-%d %H:%M:%S"));

    if let Ok(current_dir) = std::env::current_dir() {
        info!("工作目录: {:?}", current_dir);
    }

    if let Ok(exe_path) = std::env::current_exe() {
        info!("可执行文件: {:?}", exe_path);
    }

    if let Ok(user) = std::env::var("USERNAME") {
        info!("当前用户: {}", user);
    }

    info!("日志目录: {}", *LOG_DIR);
    info!("==============================");
}

/// 记录应用关闭信息
pub fn log_application_shutdown() {
    let now = Local::now();
    info!("=== ZeroLaunch-rs 应用关闭 ===");
    info!("关闭时间: {}", now.format("%Y-%m-%d %H:%M:%S"));
    info!("==============================");
}

/// 记录性能指标
pub fn log_performance_metrics(operation: &str, duration_ms: u64) {
    if duration_ms > 1000 {
        warn!("性能警告: {} 耗时 {}ms", operation, duration_ms);
    } else {
        debug!("性能指标: {} 耗时 {}ms", operation, duration_ms);
    }
}

/// 记录错误信息
pub fn log_error_with_context(error: &dyn std::error::Error, context: &str) {
    error!("错误发生在 {}: {}", context, error);

    // 记录错误链
    let mut source = error.source();
    let mut level = 1;
    while let Some(err) = source {
        error!("  原因 {}: {}", level, err);
        source = err.source();
        level += 1;
    }
}
