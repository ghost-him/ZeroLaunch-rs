//! 日志系统模块
//!
//! 提供统一的日志管理功能，包括：
//! - 日志初始化和配置
//! - 系统信息记录
//! - Panic处理
//! - 日志文件清理

use chrono::{DateTime, Local};
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::format::{DefaultFields, DefaultVisitor, FmtSpan};

use tracing_subscriber::{self, reload, EnvFilter};

// ============================================================================
// 多 Layer 类型隔离包装
// ============================================================================

/// 包装 `DefaultFields` 以产生不同的泛型类型参数。
///
/// 当 file_layer 和 console_layer 都使用默认的 `DefaultFields` 时，
/// 两个 layer 共享同一个 `FormattedFields<DefaultFields>` span extension。
/// `on_record` 被两个 layer 各调用一次，导致 trace_id 等运行时记录的字段被重复追加。
///
/// 此包装类型委托所有行为到 `DefaultFields`，但作为不同的泛型参数 `N`，
/// 使 console_layer 的 `FormattedFields<ConsoleFormatFields>` 与 file_layer 的
/// `FormattedFields<DefaultFields>` 存储在独立的 extension key 下，互不干扰。
#[derive(Debug, Default)]
struct ConsoleFormatFields(DefaultFields);

impl<'a> tracing_subscriber::field::MakeVisitor<tracing_subscriber::fmt::format::Writer<'a>>
    for ConsoleFormatFields
{
    type Visitor = DefaultVisitor<'a>;

    fn make_visitor(&self, target: tracing_subscriber::fmt::format::Writer<'a>) -> Self::Visitor {
        self.0.make_visitor(target)
    }
}

// ============================================================================
// 日志系统常量
// ============================================================================

/// 默认日志文件名前缀
pub const DEFAULT_LOG_FILE_PREFIX: &str = "info";

/// 默认日志保留天数
pub const DEFAULT_LOG_RETENTION_DAYS: i64 = 7;

/// 日志目录总大小上限；超出后从最旧的文件开始删除（panic.log 最后删）。
const MAX_TOTAL_LOG_BYTES: u64 = 64 * 1024 * 1024;

/// panic.log 单文件上限；超出后轮转为 panic.log.1，避免多次崩溃互相覆盖。
const MAX_PANIC_LOG_BYTES: u64 = 1024 * 1024;

/// 全局日志目录（在初始化时设置）
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 本次进程的会话标识（8 位十六进制），用于把用户反馈与具体一次运行对应起来。
static SESSION_ID: LazyLock<String> = LazyLock::new(|| {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
        .hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    format!("{:08x}", hasher.finish() as u32)
});

/// 全局日志级别重载句柄
static LOG_RELOAD_HANDLE: OnceLock<reload::Handle<EnvFilter, tracing_subscriber::Registry>> =
    OnceLock::new();

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
            retention_days: DEFAULT_LOG_RETENTION_DAYS,
            enable_console: true,
            log_file_prefix: DEFAULT_LOG_FILE_PREFIX.to_string(),
        }
    }
}

/// 初始化日志系统
///
/// # Arguments
///
/// * `log_dir` - 日志目录路径
/// * `config` - 日志配置，如果为None则使用默认配置
///
/// # Returns
///
/// 返回日志守护者，需要保持其生命周期直到程序结束
pub fn init_logging(
    log_dir: impl Into<PathBuf>,
    config: Option<LoggingConfig>,
) -> tracing_appender::non_blocking::WorkerGuard {
    let log_dir = log_dir.into();
    let config = config.unwrap_or_default();

    // 设置全局日志目录
    let _ = LOG_DIR.set(log_dir.clone());

    // 确保日志目录存在
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        // tracing 尚未初始化，此处只能用 eprintln
        eprintln!("创建日志目录失败: {}", e);
    }

    // 创建按日期滚动的日志文件（prefix=前缀, suffix=后缀 → {prefix}.{date}.{suffix}）
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(config.log_file_prefix.as_str())
        .filename_suffix("log")
        .build(log_dir.clone())
        .expect("初始化日志文件 appender 失败");

    // 创建非阻塞的日志写入器
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 创建可重载的过滤器
    let level_str = config.level.as_str().to_lowercase();
    let env_filter = EnvFilter::new("info")
        .add_directive(
            format!("{}={}", "zerolaunch_rs", level_str)
                .parse()
                .unwrap(),
        )
        .add_directive(
            format!("{}={}", "zerolaunch_rs_lib", level_str)
                .parse()
                .unwrap(),
        );

    let (filter, reload_handle) = reload::Layer::new(env_filter);

    // 存储重载句柄
    let _ = LOG_RELOAD_HANDLE.set(reload_handle);

    // 配置订阅者
    if config.enable_console && cfg!(debug_assertions) {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::CLOSE);

        let console_layer = tracing_subscriber::fmt::layer()
            .fmt_fields(ConsoleFormatFields::default())
            .with_writer(std::io::stdout)
            .with_ansi(true)
            .with_target(true)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .with_span_events(FmtSpan::CLOSE);

        tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .with(console_layer)
            .init();
    } else {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::CLOSE);

        tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .init();
    }

    // tracing 已初始化，后续日志使用 info!/error! 等宏
    print_system_info(&log_dir);

    setup_panic_hook();

    // 清理旧日志文件
    cleanup_old_logs(&log_dir, config.retention_days);

    info!("日志系统初始化完成");
    info!("日志级别: {:?}", config.level);
    info!("日志保留天数: {}", config.retention_days);
    info!("日志目录: {}", log_dir.display());
    info!("启用特性: {}", format_enabled_features());

    guard
}

/// 获取日志目录
pub fn get_log_dir() -> Option<&'static PathBuf> {
    LOG_DIR.get()
}

/// 本次进程的会话标识；日志、诊断包与 panic.log 共用同一值。
pub fn session_id() -> &'static str {
    SESSION_ID.as_str()
}

/// 构建时注入的 git 提交（`<短 sha>[-dirty]`）；未注入时为 `unknown`。
fn git_sha() -> &'static str {
    option_env!("ZEROLAUNCH_GIT_SHA").unwrap_or("unknown")
}

/// 构建时间：构建时注入 unix 秒，运行期转本地时间展示；未注入时为 `未知`。
fn build_time_display() -> String {
    option_env!("ZEROLAUNCH_BUILD_UNIX")
        .and_then(|secs| secs.parse::<i64>().ok())
        .and_then(|secs| DateTime::from_timestamp(secs, 0))
        .map(|dt| {
            dt.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| "未知".to_string())
}

/// 构建时注入的编译器版本；未注入时为 `unknown`。
fn rustc_version() -> &'static str {
    option_env!("ZEROLAUNCH_RUSTC").unwrap_or("unknown")
}

/// 读取已安装的 WebView2 Runtime 版本。
///
/// 先查机器级安装（WOW6432Node），再查用户级安装；都读不到时返回 `未检测到`。
/// WebView2 是 Windows 上 UI 无法启动的首要外部依赖，版本必须进日志与诊断包。
fn get_webview2_version() -> String {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;

        const CLIENT: &str = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
        let machine = format!(r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{CLIENT}");
        let user = format!(r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{CLIENT}");

        let read = |root: winreg::HKEY, path: &str| -> Option<String> {
            let key = RegKey::predef(root).open_subkey(path).ok()?;
            key.get_value::<String, _>("pv")
                .ok()
                .filter(|version| !version.trim().is_empty())
        };

        read(HKEY_LOCAL_MACHINE, &machine)
            .or_else(|| read(HKEY_CURRENT_USER, &user))
            .unwrap_or_else(|| "未检测到".to_string())
    }
    #[cfg(not(windows))]
    {
        "不支持".to_string()
    }
}

/// 运行环境快照 —— `diagnostics.json` 中环境事实的唯一类型。
///
/// 由 `environment_snapshot()` 单一构造；不是域实体投影，也不参与 IPC 契约，
/// 仅随日志导出写入诊断包。只包含诊断必需的环境事实，不含配置内容、查询词或用户文档。
#[derive(Debug, Clone, Serialize)]
pub struct EnvironmentSnapshot {
    /// 快照生成时间（本地时间，`YYYY-MM-DD HH:MM:SS`）。
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    /// 应用版本（`CARGO_PKG_VERSION`）。
    #[serde(rename = "appVersion")]
    pub app_version: String,
    /// 构建时注入的 git 提交（`<短 sha>[-dirty]`）；未注入时为 `unknown`。
    #[serde(rename = "gitSha")]
    pub git_sha: String,
    /// 构建时间（本地时间）；未注入时为 `未知`。
    #[serde(rename = "buildTime")]
    pub build_time: String,
    /// 构建时注入的编译器版本；未注入时为 `unknown`。
    #[serde(rename = "rustc")]
    pub rustc: String,
    /// 当前进程 ID。
    #[serde(rename = "pid")]
    pub pid: u32,
    /// 本次进程的会话标识（8 位十六进制）。
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Windows 版本描述（产品名 + DisplayVersion + Build）。
    #[serde(rename = "os")]
    pub os: String,
    /// 目标架构（`x86_64` / `aarch64`）。
    #[serde(rename = "arch")]
    pub arch: String,
    /// 启用的 Cargo 特性列表；无特性时为 `无`。
    #[serde(rename = "features")]
    pub features: String,
    /// 是否为便携模式构建。
    #[serde(rename = "portable")]
    pub portable: bool,
    /// WebView2 Runtime 版本；未检测到时为 `未检测到`。
    #[serde(rename = "webview2")]
    pub webview2: String,
    /// 可执行文件路径；读取失败时为 `None`。
    #[serde(rename = "exe")]
    pub exe: Option<String>,
    /// 当前工作目录；读取失败时为 `None`。
    #[serde(rename = "cwd")]
    pub cwd: Option<String>,
    /// 日志目录；日志系统未初始化时为 `None`。
    #[serde(rename = "logDir")]
    pub log_dir: Option<String>,
}

/// 生成运行环境快照（随日志导出为 `diagnostics.json`）。
///
/// 返回：环境事实快照；字段全部来自当前进程与构建期注入，不做二次加工。
pub fn environment_snapshot() -> EnvironmentSnapshot {
    EnvironmentSnapshot {
        generated_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        git_sha: git_sha().to_string(),
        build_time: build_time_display(),
        rustc: rustc_version().to_string(),
        pid: std::process::id(),
        session_id: session_id().to_string(),
        os: get_windows_version(),
        arch: std::env::consts::ARCH.to_string(),
        features: format_enabled_features(),
        portable: cfg!(feature = "portable"),
        webview2: get_webview2_version(),
        exe: std::env::current_exe()
            .ok()
            .map(|p| p.display().to_string()),
        cwd: std::env::current_dir()
            .ok()
            .map(|p| p.display().to_string()),
        log_dir: LOG_DIR.get().map(|p| p.display().to_string()),
    }
}

// ============================================================================
// 日志导出
// ============================================================================

/// 日志归档错误 —— 把日志目录打包为 zip 时可能失败的内部原因。
///
/// 仅在 `logging` 模块内产生，经 `commands/log_export.rs` 边界转换为 `BridgeError`；
/// 其他模块不应依赖此类型。
#[derive(Debug, thiserror::Error)]
pub enum LogArchiveError {
    /// 日志系统尚未初始化（启动顺序被破坏），没有可打包的日志目录。
    #[error("日志系统尚未初始化，无法导出日志")]
    NotInitialized,
    /// 遍历日志目录、读取日志文件或创建目标文件失败。
    #[error("访问日志文件失败: {0}")]
    Io(#[from] std::io::Error),
    /// 写入 zip 结构（条目头、压缩数据、中央目录）失败。
    #[error("写入日志压缩包失败: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// 日志目录内没有任何文件，导出的压缩包没有内容。
    #[error("日志目录中没有可导出的文件")]
    Empty,
    /// 后台打包任务未能正常结束（线程 panic 或运行时关闭），由命令层填入错误文本。
    #[error("日志打包任务异常: {0}")]
    Task(String),
}

/// 把当前日志目录打包成 zip 写入 `dest`。
///
/// 参数：`dest` —— 目标 zip 路径；由前端保存对话框选定，父目录须已存在。
///       `diagnostics` —— 诊断快照 JSON（写入 `diagnostics.json`）；`None` 时不写该条目。
/// 返回：成功时 `Ok(())`；日志系统未初始化、目录为空或读写失败时返回 `LogArchiveError`。
/// 行为：目录内日志文件（含按日滚动的历史文件与 panic.log）以 `logs/<文件名>` 为条目名；
///       同级 `plugin-logs/` 以 `plugin-logs/<文件名>` 一并打包；
///       目标文件已存在时直接覆盖（覆盖确认由保存对话框负责）。
pub fn export_log_archive(
    dest: &Path,
    diagnostics: Option<Vec<u8>>,
) -> Result<(), LogArchiveError> {
    let log_dir = get_log_dir().ok_or(LogArchiveError::NotInitialized)?;
    write_log_archive(log_dir, dest, diagnostics)
}

/// 把 `log_dir` 下的直接子文件、同级的 `plugin-logs/` 以及可选的诊断快照打包写入 `dest`。
///
/// 参数：`log_dir` —— 日志目录；`dest` —— 目标 zip 路径；
///       `diagnostics` —— `diagnostics.json` 内容（调用方已序列化）。
/// 返回：成功时 `Ok(())`；日志目录为空且没有诊断快照时返回 `Empty`，读写失败返回对应错误变体。
/// 条目按文件名排序，保证同一目录多次导出的 zip 结构稳定。
fn write_log_archive(
    log_dir: &Path,
    dest: &Path,
    diagnostics: Option<Vec<u8>>,
) -> Result<(), LogArchiveError> {
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            entries.push((format!("logs/{name}"), path));
        }
    }

    // 插件子进程 stderr 与宿主日志目录同级；插件相关问题的证据只在这里。
    if let Some(parent) = log_dir.parent() {
        let plugin_log_dir = parent.join("plugin-logs");
        if let Ok(plugin_entries) = std::fs::read_dir(&plugin_log_dir) {
            for entry in plugin_entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    entries.push((format!("plugin-logs/{name}"), path));
                }
            }
        }
    }

    if entries.is_empty() && diagnostics.is_none() {
        return Err(LogArchiveError::Empty);
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let file_count = entries.len();
    let mut writer = zip::ZipWriter::new(std::io::BufWriter::new(File::create(dest)?));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    if let Some(bytes) = diagnostics {
        writer.start_file("diagnostics.json", options)?;
        writer.write_all(&bytes)?;
    }
    for (entry_name, path) in entries {
        writer.start_file(entry_name, options)?;
        writer.write_all(&std::fs::read(&path)?)?;
    }
    // finish 写出中央目录后，BufWriter 仍需显式刷新才能落盘。
    let mut inner = writer.finish()?;
    inner.flush()?;

    info!("日志已导出: {}（{} 个文件）", dest.display(), file_count);
    Ok(())
}

/// 获取当前启用的 Cargo 特性列表
fn collect_enabled_features() -> Vec<&'static str> {
    let mut features = Vec::new();

    if cfg!(feature = "custom-protocol") {
        features.push("custom-protocol");
    }

    if cfg!(feature = "portable") {
        features.push("portable");
    }

    features
}

/// 将启用的特性列表格式化为字符串
fn format_enabled_features() -> String {
    let features = collect_enabled_features();
    if features.is_empty() {
        "无".to_string()
    } else {
        features.join(", ")
    }
}

/// 获取 Windows 详细版本信息
fn get_windows_version() -> String {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;

        let hk_common = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hk_common.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion") {
            let product_name: String = key
                .get_value("ProductName")
                .unwrap_or_else(|_| "Windows".to_string());
            let current_build: String = key
                .get_value("CurrentBuild")
                .unwrap_or_else(|_| "".to_string());
            let display_version: String = key
                .get_value("DisplayVersion")
                .unwrap_or_else(|_| "".to_string());

            let build_num: u32 = current_build.parse().unwrap_or(0);
            let mut os_name = product_name;

            if build_num >= 22000 {
                if os_name.contains("Windows 10") {
                    os_name = os_name.replace("Windows 10", "Windows 11");
                } else if !os_name.contains("Windows 11") {
                    os_name = format!("Windows 11 ({})", os_name);
                }
            }

            if !display_version.is_empty() {
                format!("{} {} (Build {})", os_name, display_version, current_build)
            } else {
                format!("{} (Build {})", os_name, current_build)
            }
        } else {
            "Windows (Unknown)".to_string()
        }
    }
    #[cfg(not(windows))]
    {
        std::env::consts::OS.to_string()
    }
}

/// 打印系统信息
fn print_system_info(log_dir: &Path) {
    info!("=== ZeroLaunch-rs 系统信息 ===");
    info!("应用版本: {}", env!("CARGO_PKG_VERSION"));
    info!("Git 提交: {}", git_sha());
    info!("构建时间: {}", build_time_display());
    info!("Rust版本: {}", rustc_version());
    info!("操作系统: {}", get_windows_version());
    info!("系统架构: {}", std::env::consts::ARCH);
    info!("进程 ID: {} 会话: {}", std::process::id(), session_id());
    info!("WebView2: {}", get_webview2_version());
    info!("便携模式: {}", cfg!(feature = "portable"));

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

    info!("日志目录: {}", log_dir.display());
    info!("启用特性: {}", format_enabled_features());
    info!("==============================");
}

/// 设置panic处理钩子
///
/// 崩溃现场是线上最难复现的证据，这里遵守三条约束：
/// 1. 先写关键行（时间/线程/位置/消息）并 flush，再采堆栈——符号化可能很慢甚至卡住，
///    不能让它挡住必得信息；
/// 2. 追加写 + 超限轮转，多次崩溃互不覆盖；
/// 3. 全程不使用会 panic 的调用（`expect`/`unwrap`），避免钩子自身二次 panic 丢失证据。
fn setup_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "未知位置".to_string());
        let message = match panic_info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match panic_info.payload().downcast_ref::<String>() {
                Some(s) => s.as_str(),
                None => "未知panic消息",
            },
        };

        // 尝试获取日志目录
        let log_dir = LOG_DIR.get().cloned().unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .map(|d| d.join("logs"))
                .unwrap_or_else(|| PathBuf::from("."))
        });

        let panic_file_path = log_dir.join("panic.log");
        rotate_panic_log(&panic_file_path);

        // 写入panic日志文件
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&panic_file_path)
        {
            let now = Local::now();
            let thread = std::thread::current();
            let _ = writeln!(
                file,
                "\n[{}] PID {} 会话 {} 线程 {:?}",
                now.format("%Y-%m-%d %H:%M:%S"),
                std::process::id(),
                session_id(),
                thread.name().unwrap_or("<unnamed>")
            );
            let _ = writeln!(file, "Panic: {} @ {}", message, location);
            let _ = writeln!(
                file,
                "版本: {} git {} 构建 {}",
                env!("CARGO_PKG_VERSION"),
                git_sha(),
                build_time_display()
            );
            let _ = writeln!(
                file,
                "系统: {} / {} / WebView2 {}",
                get_windows_version(),
                std::env::consts::ARCH,
                get_webview2_version()
            );
            if let Ok(current_dir) = std::env::current_dir() {
                let _ = writeln!(file, "工作目录: {:?}", current_dir);
            }
            let _ = file.flush();

            // 堆栈放在必得信息之后：即使符号化卡死/失败，上面的现场也已落盘。
            let _ = writeln!(file, "\n堆栈跟踪:");
            let _ = writeln!(file, "{}", std::backtrace::Backtrace::force_capture());
            let _ = file.flush();
        }

        error!("[PANIC] {} ({})", message, location);
    }));
}

/// panic.log 超过 `MAX_PANIC_LOG_BYTES` 时轮转为 panic.log.1（只保留上一份）。
fn rotate_panic_log(path: &Path) {
    let oversized = std::fs::metadata(path)
        .map(|meta| meta.len() > MAX_PANIC_LOG_BYTES)
        .unwrap_or(false);
    if !oversized {
        return;
    }
    let rotated = path.with_extension("log.1");
    let _ = std::fs::remove_file(&rotated);
    let _ = std::fs::rename(path, &rotated);
}

/// 清理旧的日志文件
pub fn cleanup_old_logs(log_dir: impl AsRef<Path>, retention_days: i64) {
    let log_dir = log_dir.as_ref();
    debug!("开始清理旧日志文件，保留天数: {}", retention_days);

    let now: DateTime<Local> = Local::now();
    let mut cleaned_count: u32 = 0;
    let mut total_size_cleaned = 0u64;

    let entries = match std::fs::read_dir(log_dir) {
        Ok(entries) => entries,
        Err(e) => {
            error!("无法读取日志目录 '{:?}': {}", log_dir, e);
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Ok(metadata) = std::fs::metadata(&path) {
                let file_size = metadata.len();

                if let Ok(modified) = metadata.modified() {
                    let modified_datetime: DateTime<Local> = modified.into();
                    let age = now.signed_duration_since(modified_datetime);

                    if age.num_days() > retention_days {
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

    // 体积上限：按修改时间从旧到新删除，panic.log 最后删。
    let (cap_cleaned, cap_freed) = enforce_total_size_cap(log_dir, MAX_TOTAL_LOG_BYTES);
    if cap_cleaned > 0 {
        info!(
            "日志体积超过上限（{} bytes），已删除 {} 个最旧文件，释放 {} bytes",
            MAX_TOTAL_LOG_BYTES, cap_cleaned, cap_freed
        );
    }
    cleaned_count += cap_cleaned;
    total_size_cleaned += cap_freed;

    if cleaned_count > 0 {
        info!(
            "日志清理完成: 删除了 {} 个文件，释放了 {} bytes 空间",
            cleaned_count, total_size_cleaned
        );
    } else {
        debug!("没有需要清理的旧日志文件");
    }
}

/// 日志目录总量超过 `limit` 时，按修改时间从旧到新删除，
/// 直到回落到上限内；`panic.log` 排在最后删除（崩溃证据优先保留）。
///
/// 返回 `(删除的文件数, 释放的字节数)`；目录不可读时返回 `(0, 0)` 并记录错误。
fn enforce_total_size_cap(log_dir: &Path, limit: u64) -> (u32, u64) {
    let mut files: Vec<(PathBuf, u64, SystemTime)> = match std::fs::read_dir(log_dir) {
        Ok(entries) => entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if !path.is_file() {
                    return None;
                }
                let metadata = std::fs::metadata(&path).ok()?;
                let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
                Some((path, metadata.len(), modified))
            })
            .collect(),
        Err(e) => {
            error!("无法读取日志目录 '{:?}': {}", log_dir, e);
            return (0, 0);
        }
    };

    let mut total: u64 = files.iter().map(|(_, size, _)| *size).sum();
    if total <= limit {
        return (0, 0);
    }

    files.sort_by_key(|(_, _, modified)| *modified);
    // 稳定排序：保持「旧→新」次序，仅把 panic.log 移到末位最后删除。
    files.sort_by_key(|(path, _, _)| {
        path.file_name()
            .map(|name| name == "panic.log")
            .unwrap_or(false)
    });

    let mut cleaned = 0u32;
    let mut freed = 0u64;
    for (path, size, _) in files {
        if total <= limit {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            cleaned += 1;
            freed += size;
            total = total.saturating_sub(size);
        }
    }
    (cleaned, freed)
}

/// 记录应用启动信息
pub fn log_application_start() {
    info!("=== ZeroLaunch-rs 应用启动 ===");
    info!("应用版本: {}", env!("CARGO_PKG_VERSION"));
    info!("Git 提交: {}", git_sha());
    info!("进程 ID: {} 会话: {}", std::process::id(), session_id());
    info!("WebView2: {}", get_webview2_version());
    info!("操作系统: {}", get_windows_version());
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

    if let Some(log_dir) = LOG_DIR.get() {
        info!("日志目录: {}", log_dir.display());
    }
    info!("启用特性: {}", format_enabled_features());
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

    let mut source = error.source();
    let mut level = 1;
    while let Some(err) = source {
        error!("  原因 {}: {}", level, err);
        source = err.source();
        level += 1;
    }
}

/// 动态更新日志级别
pub fn update_log_level(new_level: Level) -> Result<(), String> {
    if let Some(handle) = LOG_RELOAD_HANDLE.get() {
        let level_str = new_level.as_str().to_lowercase();
        let new_filter = EnvFilter::new("info")
            .add_directive(
                format!("{}={}", "zerolaunch_rs", level_str)
                    .parse()
                    .unwrap(),
            )
            .add_directive(
                format!("{}={}", "zerolaunch_rs_lib", level_str)
                    .parse()
                    .unwrap(),
            );
        handle
            .reload(new_filter)
            .map_err(|e| format!("更新日志级别失败: {}", e))?;
        info!("日志级别已更新为: {:?}", new_level);
        Ok(())
    } else {
        Err("日志系统尚未初始化或不支持动态更新".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{enforce_total_size_cap, write_log_archive, LogArchiveError};
    use std::io::Read;

    /// 导出应把日志目录内全部文件以 `logs/` 前缀写入 zip，同级插件日志进 `plugin-logs/`，
    /// 诊断快照进 `diagnostics.json`；空目录且无诊断时返回 `Empty`。
    #[test]
    fn write_log_archive_packs_all_files() {
        let root = tempfile::tempdir().unwrap();
        let log_dir = root.path().join("app-logs");
        std::fs::create_dir(&log_dir).unwrap();
        std::fs::write(log_dir.join("info.2026-11-06.log"), "hello").unwrap();
        std::fs::write(log_dir.join("panic.log"), "boom").unwrap();
        let plugin_log_dir = root.path().join("plugin-logs");
        std::fs::create_dir(&plugin_log_dir).unwrap();
        std::fs::write(plugin_log_dir.join("demo.plugin.log"), "plugin stderr").unwrap();
        let dest = root.path().join("zerolaunch-log.zip");

        write_log_archive(
            &log_dir,
            &dest,
            Some(br#"{"sessionId":"abcd1234"}"#.to_vec()),
        )
        .unwrap();

        let mut archive = zip::ZipArchive::new(std::fs::File::open(&dest).unwrap()).unwrap();
        let mut names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "diagnostics.json",
                "logs/info.2026-11-06.log",
                "logs/panic.log",
                "plugin-logs/demo.plugin.log",
            ]
        );
        let mut content = String::new();
        archive
            .by_name("logs/panic.log")
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "boom");

        let empty_root = tempfile::tempdir().unwrap();
        let empty_dir = empty_root.path().join("empty");
        std::fs::create_dir(&empty_dir).unwrap();
        let err =
            write_log_archive(&empty_dir, &empty_root.path().join("empty.zip"), None).unwrap_err();
        assert!(matches!(err, LogArchiveError::Empty));
    }

    /// 体积超限时按最旧优先删除，panic.log 保留到最后。
    #[test]
    fn size_cap_deletes_oldest_but_keeps_panic_log_last() {
        let root = tempfile::tempdir().unwrap();
        let log_dir = root.path();
        std::fs::write(log_dir.join("info.2026-11-01.log"), [0u8; 100]).unwrap();
        std::fs::write(log_dir.join("panic.log"), [0u8; 100]).unwrap();

        let (cleaned, freed) = enforce_total_size_cap(log_dir, 150);

        assert_eq!(cleaned, 1);
        assert_eq!(freed, 100);
        assert!(!log_dir.join("info.2026-11-01.log").exists());
        assert!(log_dir.join("panic.log").exists());
    }
}
