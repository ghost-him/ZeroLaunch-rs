//! 应用级命令枚举与全局命令通道。
//!
//! ## 设计决策：为什么这里使用全局状态（OnceLock）
//!
//! `AppCommand` 是**应用基础设施**级别的概念——它与 `HostApi`、`ConfigManager` 的
//! 生命周期相同，整个进程中只有一个应用事件循环、一个命令消费者 task。
//!
//! 如果采用传统的依赖注入（层层传递 `mpsc::Sender`），会造成"霰弹枪式修改"
//! (Shotgun Surgery)：每新增一个需要发送命令的组件，就必须修改 `PluginManager`、
//! `InventoryContext` 等中间结构体的签名和字段。这些中间人并不消费通道——它们只是
//! "路过" (Tramp Data)，导致构造函数参数膨胀和跨层耦合。
//!
//! 放在 `AppState` 中也非最优解：`AppState` 是一个已有的上帝对象，不应继续膨胀；
//! 且让 `BuiltinCommandExecutor` 依赖 `Arc<AppState>` 比依赖一个只写一次的通信原语
//! 引入了更重的耦合。
//!
//! `OnceLock<Sender>` 是 Rust 标准库（1.70+）的原语，语义精确：设置一次，永不修改。
//! 这与 `tracing::subscriber`、`log::logger` 等 Rust 生态中公认的全局基础设施模式
//! 同属一类——不是可变的全局业务状态，而是写一次的通信基础设施。
//!
//! 若未来有单元测试需求，可以添加 `#[cfg(test)]` 下的 `reset()` 函数（替换为新的
//! channel），或直接使用 `tokio::sync::mpsc::channel` 构造独立的测试通道绕过全局。

use std::sync::OnceLock;
use tokio::sync::mpsc;
use tracing::warn;

/// 应用级命令枚举，供内置命令执行器和托盘菜单使用。
///
/// 通过全局 mpsc channel 从 BuiltinCommandExecutor / TrayManager
/// 发送到 bootstrap.rs 中 spawn 的消费者 task。
#[derive(Debug, Clone)]
pub enum AppCommand {
    /// 打开设置窗口
    ShowSettings,
    /// 刷新所有候选项缓存
    RefreshCandidates,
    /// 重新注册全局快捷键
    ReregisterHotkeys,
    /// 切换游戏模式
    ToggleGameMode,
    /// 退出程序
    ExitProgram,
}

/// AppCommand 的 tokio mpsc 发送端类型别名。
pub type CommandSender = mpsc::Sender<AppCommand>;

/// 全局命令发送端。由 `bootstrap.rs` 在启动时通过 `init_command_channel` 设置一次。
static SENDER: OnceLock<CommandSender> = OnceLock::new();

/// 初始化全局命令通道。
///
/// **必须**在应用启动早期（创建任何可能调用 `send` 的组件之前）调用。
/// 重复调用时静默忽略后续调用（`OnceLock::set` 保证只写入一次）。
///
/// # Panics
///
/// 仅在 `OnceLock` 已被设置时再次调用会静默失败（不会 panic），
/// 但生产代码不应依赖此行为——应在唯一的位置调用一次。
pub fn init_command_channel(sender: CommandSender) {
    if SENDER.set(sender).is_err() {
        warn!("AppCommand 全局通道已被初始化，忽略重复调用");
    }
}

/// 发送应用级命令到消费者 task。
///
/// 使用 `try_send`（非阻塞）：生产者（TrayManager 的同步回调、BuiltinCommandExecutor
/// 的 async execute）都不会因通道满而被阻塞。消费者 task 处理速度远快于人工操作频率，
/// buffer 容量 32 在实践中足够。若通道满，记 warn 日志并丢弃（不 panic）。
///
/// # 通道未初始化时
///
/// 若 `init_command_channel` 未被调用（通常是启动顺序错误），记 error 日志并静默丢弃。
pub fn send(cmd: AppCommand) {
    match SENDER.get() {
        Some(sender) => {
            if let Err(e) = sender.try_send(cmd) {
                warn!("发送 AppCommand 失败: {}", e);
            }
        }
        None => {
            tracing::error!(
                "AppCommand 通道未初始化，命令 {:?} 被丢弃。请确保 bootstrap.rs 中调用了 init_command_channel",
                cmd
            );
        }
    }
}
