//! 进程级 Tokio runtime。
//!
//! 内核、CLI 与 GPUI 宿主共用同一个 runtime：
//! - 其他模块只经本模块的 `spawn` / `spawn_blocking` / `block_on` 创建任务，
//!   不直接调用 tokio 的协程创建函数，也不依赖调用线程的 runtime 上下文。

use std::future::Future;
use std::sync::LazyLock;
use tokio::runtime::{Builder, Handle, Runtime};
use tokio::task::JoinHandle;

/// 进程内唯一的 Tokio runtime；首次访问时创建多线程 runtime。
static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .enable_all()
        .thread_name("zerolaunch")
        .build()
        .expect("初始化全局 Tokio runtime 失败")
});

/// 返回全局 runtime 的 handle（可克隆，供需要显式指定 runtime 的调用方使用）。
pub fn handle() -> Handle {
    RUNTIME.handle().clone()
}

/// 在全局 runtime 上创建异步任务。
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    RUNTIME.spawn(future)
}

/// 在全局 runtime 的阻塞线程池上执行同步任务。
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    RUNTIME.spawn_blocking(f)
}

/// 在全局 runtime 上阻塞执行一个 future。
///
/// 仅用于启动、测试与命令同步边界；不得在 runtime 工作线程内调用。
pub fn block_on<F: Future>(future: F) -> F::Output {
    RUNTIME.block_on(future)
}
