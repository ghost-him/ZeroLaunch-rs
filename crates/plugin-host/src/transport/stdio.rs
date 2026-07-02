use std::path::Path;
use std::process::Stdio;
use tokio::io::{BufReader, BufWriter};
use tokio::process::Child;
use zerolaunch_plugin_protocol::ProtocolError;

/// 子进程传输层：启动插件进程并接好 stdin/stdout/stderr 三根管道。
///
/// 本结构体只负责"启动进程 + 暴露管道句柄"，不关心 JSON-RPC 协议。
/// 调用方（PluginProcess::spawn）拿到后会立即拆解它，把三根管道分发给不同的消费者：
///   - stdin  → JsonRpcClient 的 write_loop（向插件发请求）
///   - stdout → JsonRpcClient 的 read_loop（接收插件的响应）
///   - stderr → 后台日志收集任务（写入日志文件）
///   - child  → Watchdog 任务（检测进程是否存活）
pub struct StdioTransport {
    /// 子进程句柄，用于查询 pid 和检测退出状态。
    pub child: Child,
    /// 子进程的标准输入：宿主往这里写，子进程从 stdin 读到。
    pub stdin: BufWriter<tokio::process::ChildStdin>,
    /// 子进程的标准输出：子进程往这里写，宿主从 stdout 读到。
    pub stdout: BufReader<tokio::process::ChildStdout>,
    /// 子进程的标准错误：子进程往这里写，宿主写入日志文件。
    pub stderr: tokio::process::ChildStderr,
}

impl StdioTransport {
    /// 启动插件进程，建立三根管道。
    ///
    /// # 参数
    /// - `command`: 插件可执行文件的完整路径（manifest 中 runtime.command 相对于 plugin_dir 解析）
    /// - `args`: 启动参数（来自 manifest.runtime.args）
    /// - `cwd`: 工作目录（设为 plugin_dir）
    /// - `env`: 自定义环境变量（ZEROLAUNCH_PLUGIN_ID / ZEROLAUNCH_DATA_DIR / ZEROLAUNCH_LOG_DIR）
    ///
    /// # 返回
    /// 组装好的 StdioTransport，调用方拆解后分发三个管道。
    pub async fn spawn(
        command: &Path,
        args: &[String],
        cwd: &Path,
        env: &[(String, String)],
    ) -> Result<Self, ProtocolError> {
        // --- 1. 准备命令 ---
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args)
            .current_dir(cwd)
            // 将子进程的三条标准流全部设为"管道"模式，这样宿主才能读写它们。
            // 如果设为 Inherit，子进程会继承宿主的终端，宿主就拿不到管道了。
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // 当宿主进程退出时，自动杀掉子进程（防止孤儿进程泄露）。
            .kill_on_drop(true);

        // --- 2. 注入自定义环境变量 ---
        for (k, v) in env {
            cmd.env(k, v);
        }

        // --- 3. 启动子进程 ---
        // 此时子进程已经开始运行。它可能已经在尝试从 stdin 读取数据，
        // 或者向 stdout/stderr 写入数据了。
        let mut child = cmd.spawn().map_err(|e| {
            ProtocolError::InvalidFrame(format!("failed to spawn '{}': {}", command.display(), e))
        })?;

        // --- 4. 从子进程中取出三个管道的宿主侧句柄 ---
        //
        // tokio::process::Command::spawn() 返回的 Child 结构体内部持有三个 Option<> 管道。
        // .take() 取出 Some(...) 值并将其替换为 None（所有权转移）。
        // 这些管道的一端在子进程里，另一端在宿主手里，宿主通过它们与子进程通信。
        //
        // 注意：如果 .take() 返回 None，说明管道已经被取走了（不应该发生），直接报错。

        // stdin 的宿主侧是"写入端"：宿主 → 子进程。
        // 包上 BufWriter 减少系统调用次数（攒一批数据再 flush）。
        let stdin = BufWriter::new(
            child
                .stdin
                .take()
                .ok_or_else(|| ProtocolError::InvalidFrame("stdin not available".into()))?,
        );

        // stdout 的宿主侧是"读取端"：子进程 → 宿主。
        // 包上 BufReader 提高读取效率（按行/按帧读取）。
        let stdout = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| ProtocolError::InvalidFrame("stdout not available".into()))?,
        );

        // stderr 的宿主侧也是"读取端"。
        // 注意 stderr 没有包 BufReader——调用方（PluginProcess::spawn）会
        // 使用原始的 AsyncReadExt::read 逐块读取原始字节并写入日志文件。
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ProtocolError::InvalidFrame("stderr not available".into()))?;

        Ok(Self {
            child,
            stdin,
            stdout,
            stderr,
        })
    }

    /// 获取子进程的 PID（供日志和健康监控使用）。
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
}
