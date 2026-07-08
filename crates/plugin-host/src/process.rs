//! PluginProcess — subprocess lifecycle management.

use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use zerolaunch_plugin_protocol::manifest::Manifest;
use zerolaunch_plugin_protocol::messages::*;
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;
use zerolaunch_plugin_protocol::ProtocolError;

use crate::client::{IncomingRequest, JsonRpcClient};
use crate::host_dispatch::HostCallHandler;
use crate::transport::stdio::StdioTransport;

/// Tracks the lifecycle state of a plugin subprocess.
#[derive(Debug, Clone)]
pub enum ProcessState {
    Starting,
    Running,
    Crashed { restarts: u32, last_error: String },
    Stopped,
    Error(String),
}

/// Result of the initialization handshake with a plugin subprocess.
pub struct InitResult {
    pub plugin_id: String,
    pub metadata: zerolaunch_plugin_api::PluginMetadata,
    pub components: Vec<ComponentDescriptor>,
    pub settings_schemas: Vec<(
        String,
        Vec<zerolaunch_plugin_api::config::SettingDefinition>,
    )>,
    pub settings_values: Vec<(String, serde_json::Value)>,
    pub config_actions_map: Vec<(String, Vec<zerolaunch_plugin_api::config::ConfigActionDef>)>,
    /// 每个 ActionExecutor 组件的 supported_actions 结果，按 component_id 索引。
    /// 在 discover_components 期间通过 plugin/supported_actions 获取，
    /// 供 build_adapters 直接使用，避免 ConfigActionDef → ResultAction 语义错配。
    pub executor_actions_map: Vec<(String, Vec<zerolaunch_plugin_api::ResultAction>)>,
}

/// Manages a single plugin subprocess instance.
pub struct PluginProcess {
    pub plugin_id: String,
    pub manifest: Manifest,
    pub state: Arc<RwLock<ProcessState>>,
    pub client: Arc<JsonRpcClient>,
    pub data_dir: PathBuf,
    /// Handle to the child process for health monitoring.
    child_handle: Arc<parking_lot::Mutex<Option<tokio::process::Child>>>,
    /// Channel to notify PluginHostManager when the process crashes and needs restart.
    crash_tx: mpsc::Sender<String>,
    /// 该插件已重启的次数（0 = 初次启动）。
    /// 仅用于观测；权威的重启计数器在 PluginRestartContext 中。
    pub restart_count: u32,
    /// 子进程的 PID，用于强制终止（当优雅关闭超时时兜底）。
    pub pid: Option<u32>,
}

impl PluginProcess {
    /// Spawn the plugin subprocess, complete the initialize handshake,
    /// and start stderr log collection.
    ///
    /// `crash_tx` is a channel sender owned by the `PluginHostManager`.
    /// When the watchdog detects a crash, it sends the `plugin_id` on this
    /// channel so the manager can trigger a re-spawn.
    pub async fn spawn(
        manifest: &Manifest,
        plugin_dir: &Path,
        data_dir: &Path,
        log_dir: &Path,
        host_call_handler: Arc<dyn HostCallHandler>,
        crash_tx: mpsc::Sender<String>,
        // 该插件已重启的次数（0 = 初次加载）。
        restart_count: u32,
    ) -> Result<Self, ProtocolError> {
        let plugin_id = manifest.plugin.id.clone();

        // Resolve the command path
        let cmd_path = plugin_dir.join(&manifest.runtime.command);
        if !cmd_path.exists() {
            return Err(ProtocolError::InvalidFrame(format!(
                "plugin command not found: {}",
                cmd_path.display()
            )));
        }

        // Build environment: pass data_dir and log_dir
        let env = vec![
            ("ZEROLAUNCH_PLUGIN_ID".to_string(), plugin_id.clone()),
            (
                "ZEROLAUNCH_DATA_DIR".to_string(),
                data_dir.to_string_lossy().to_string(),
            ),
            (
                "ZEROLAUNCH_LOG_DIR".to_string(),
                log_dir.to_string_lossy().to_string(),
            ),
        ];

        info!("Spawning plugin {}: {:?}", plugin_id, cmd_path);

        let transport =
            StdioTransport::spawn(&cmd_path, &manifest.runtime.args, plugin_dir, &env).await?;

        let pid = transport.pid();
        // Split transport so we can keep the child handle for health monitoring
        let StdioTransport {
            child,
            stdin: child_stdin,
            stdout: child_stdout,
            stderr: child_stderr,
        } = transport;
        let child_handle = Arc::new(parking_lot::Mutex::new(Some(child)));

        // Start stderr logger task
        let log_file = log_dir.join(format!("{}.log", plugin_id));
        let stderr_pid = plugin_id.clone();
        let mut stderr_reader = child_stderr;
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match tokio::io::AsyncReadExt::read(&mut stderr_reader, &mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buf[..n]);
                        let _ = append_to_log(&log_file, &text).await;
                    }
                    Err(e) => {
                        debug!("stderr read error for {}: {}", stderr_pid, e);
                        break;
                    }
                }
            }
        });

        // Set up incoming request/notification channels
        let (incoming_request_tx, mut incoming_request_rx) = mpsc::channel::<IncomingRequest>(64);
        let (incoming_notification_tx, _incoming_notification_rx) =
            mpsc::channel::<(String, serde_json::Value)>(64);

        let client = JsonRpcClient::new(
            child_stdout,
            child_stdin,
            incoming_request_tx,
            incoming_notification_tx,
        );

        // Spawn task to handle incoming requests from the plugin (host/* calls)
        let hc = host_call_handler.clone();
        let cl = client.clone();
        tokio::spawn(async move {
            while let Some(incoming) = incoming_request_rx.recv().await {
                let result = hc.handle_host_call(&incoming.method, incoming.params).await;
                match result {
                    Ok(val) => {
                        if let Err(e) = cl.respond_ok(incoming.id, val).await {
                            error!("Failed to send response for {}: {}", incoming.method, e);
                        }
                    }
                    Err(err) => {
                        if let Err(e) = cl.respond_err(incoming.id, err).await {
                            error!(
                                "Failed to send error response for {}: {}",
                                incoming.method, e
                            );
                        }
                    }
                }
            }
        });

        let host_version = env!("CARGO_PKG_VERSION").to_string();
        let protocol_version = zerolaunch_plugin_protocol::PROTOCOL_VERSION.to_string();

        // Step 1: plugin/initialize
        let init_params = InitializeParams {
            host_version: host_version.clone(),
            protocol_version: protocol_version.clone(),
            data_dir: data_dir.to_string_lossy().to_string(),
            log_dir: log_dir.to_string_lossy().to_string(),
            plugin_id: plugin_id.clone(),
            locale: "zh-CN".to_string(),
        };

        let init_result: InitializeResult = client
            .call(
                plugin_methods::INITIALIZE,
                init_params,
                Duration::from_secs(manifest.runtime.startup_timeout),
            )
            .await?;

        info!(
            "Plugin {} initialized (pid={:?}), result: {:?}",
            plugin_id, pid, init_result
        );

        let process = Self {
            plugin_id: plugin_id.clone(),
            manifest: manifest.clone(),
            state: Arc::new(RwLock::new(ProcessState::Running)),
            client,
            data_dir: data_dir.to_path_buf(),
            child_handle,
            crash_tx,
            restart_count,
            pid,
        };

        // Start health monitoring
        process.spawn_watchdog();

        Ok(process)
    }

    /// Run the post-initialization discovery sequence:
    /// plugin/get_metadata, plugin/get_components, and for each component:
    /// plugin/get_settings_schema, plugin/get_settings, plugin/config_actions.
    /// 对 ActionExecutor 组件还会额外调用 plugin/supported_target_types
    /// 和 plugin/supported_actions，收集真实的 ResultAction 列表。
    pub async fn discover_components(&self) -> Result<InitResult, ProtocolError> {
        let plugin_id = self.plugin_id.clone();

        // plugin/get_metadata
        let metadata: zerolaunch_plugin_api::PluginMetadata = self
            .client
            .call(
                plugin_methods::GET_METADATA,
                serde_json::Value::Null,
                Duration::from_secs(5),
            )
            .await?;

        // plugin/get_components
        let components: Vec<ComponentDescriptor> = self
            .client
            .call(
                plugin_methods::GET_COMPONENTS,
                serde_json::Value::Null,
                Duration::from_secs(5),
            )
            .await?;

        // 为每个组件拉取 schema、settings、config_actions。
        // 对 ActionExecutor 组件额外拉取 supported_actions。
        let mut settings_schemas = Vec::new();
        let mut settings_values = Vec::new();
        let mut config_actions_map = Vec::new();
        let mut executor_actions_map: Vec<(String, Vec<zerolaunch_plugin_api::ResultAction>)> =
            Vec::new();

        for comp in &components {
            let schema: Vec<zerolaunch_plugin_api::config::SettingDefinition> = self
                .client
                .call(
                    plugin_methods::GET_SETTINGS_SCHEMA,
                    GetSettingsSchemaParams {
                        component_id: comp.component_id.clone(),
                    },
                    Duration::from_secs(5),
                )
                .await?;
            settings_schemas.push((comp.component_id.clone(), schema));

            let settings: serde_json::Value = self
                .client
                .call(
                    plugin_methods::GET_SETTINGS,
                    GetSettingsParams {
                        component_id: comp.component_id.clone(),
                    },
                    Duration::from_secs(5),
                )
                .await?;
            settings_values.push((comp.component_id.clone(), settings));

            let actions: Vec<zerolaunch_plugin_api::config::ConfigActionDef> = self
                .client
                .call(
                    plugin_methods::CONFIG_ACTIONS,
                    ConfigActionsParams {
                        component_id: comp.component_id.clone(),
                    },
                    Duration::from_secs(5),
                )
                .await?;
            config_actions_map.push((comp.component_id.clone(), actions));

            // 对 ActionExecutor 组件，遍历每种支持的 target_type 拉取
            // supported_actions，按 (id, label) 去重后合并。
            if matches!(comp.kind, ComponentKind::ActionExecutor { .. }) {
                let target_types: Vec<zerolaunch_plugin_api::TargetType> = self
                    .client
                    .call(
                        plugin_methods::SUPPORTED_TARGET_TYPES,
                        SupportedTargetTypesParams {
                            component_id: comp.component_id.clone(),
                        },
                        Duration::from_secs(5),
                    )
                    .await?;

                let mut all_actions: Vec<zerolaunch_plugin_api::ResultAction> = Vec::new();
                let mut seen = std::collections::HashSet::new();
                for tt in &target_types {
                    let actions: Vec<zerolaunch_plugin_api::ResultAction> = self
                        .client
                        .call(
                            plugin_methods::SUPPORTED_ACTIONS,
                            SupportedActionsParams {
                                component_id: comp.component_id.clone(),
                                target_type: *tt,
                            },
                            Duration::from_secs(5),
                        )
                        .await?;
                    for a in actions {
                        // 按 (id, label) 去重 — 同一动作可能被多个 target_type 返回
                        if seen.insert((a.id.clone(), a.label.clone())) {
                            all_actions.push(a);
                        }
                    }
                }
                executor_actions_map.push((comp.component_id.clone(), all_actions));
            }
        }

        Ok(InitResult {
            plugin_id,
            metadata,
            components,
            settings_schemas,
            settings_values,
            config_actions_map,
            executor_actions_map,
        })
    }

    /// 生成一个看门狗任务，监控子进程的生命周期。
    /// 使用事件驱动的等待（`child.wait().await`）——**没有轮询循环**。
    ///
    /// 当进程退出时：
    /// - 如果状态已经是 `Stopped`（优雅关闭），静默返回。
    /// - 如果 `auto_restart=false`，将状态设为 `Stopped` 并返回。
    /// - 否则，将 `plugin_id` 发到 `crash_tx` 上，通知 `PluginHostManager`
    ///   重新拉起。管理者（`restart_loop`）负责追踪重启次数和强制
    ///   `max_restart` 上限。
    pub fn spawn_watchdog(&self) {
        let plugin_id = self.plugin_id.clone();
        let state = self.state.clone();
        let child_handle = self.child_handle.clone();
        let auto_restart = self.manifest.runtime.auto_restart;
        let restart_count = self.restart_count;
        let crash_tx = self.crash_tx.clone();

        tokio::spawn(async move {
            // 从 Mutex 中取出子进程句柄，然后立即释放锁。
            // 关键：parking_lot::Mutex 绝对不能在 .await 期间持有。
            let mut child = {
                let mut guard = child_handle.lock();
                match guard.take() {
                    Some(c) => c,
                    None => return, // 已被取走（例如被先前的 shutdown 取走）
                }
            };

            // 事件驱动等待：tokio 挂起此任务，直到进程退出。
            // 零轮询——运行时只在 OS 通知进程终止时唤醒我们。
            let status = match child.wait().await {
                Ok(s) => s,
                Err(e) => {
                    warn!("Plugin {} wait error: {}", plugin_id, e);
                    return;
                }
            };

            info!(
                "Plugin {} process exited with status: {:?}",
                plugin_id, status
            );

            // 如果是优雅关闭（PluginProcess::shutdown 已将状态设为 Stopped），不重启。
            if matches!(*state.read(), ProcessState::Stopped) {
                debug!(
                    "Plugin {} was gracefully stopped, not restarting",
                    plugin_id
                );
                return;
            }

            if !auto_restart {
                *state.write() = ProcessState::Stopped;
                info!("Plugin {} auto-restart disabled, stopped", plugin_id);
                return;
            }

            // 记录崩溃（仅用于观测）并通知管理者。
            // restart_loop 通过 PluginRestartContext 追踪持久化计数
            // 并强制 max_restart 上限——看门狗只负责检测。
            *state.write() = ProcessState::Crashed {
                restarts: restart_count + 1,
                last_error: "process exited unexpectedly".into(),
            };
            info!(
                "Plugin {} crashed (restart #{} so far), notifying manager",
                plugin_id,
                restart_count + 1
            );

            if crash_tx.send(plugin_id.clone()).await.is_err() {
                debug!("Plugin {} crash notification channel closed", plugin_id);
            }
        });
    }

    /// Graceful shutdown: send plugin/shutdown, wait, then force-kill if needed.
    pub async fn shutdown(self, timeout: Duration) {
        let plugin_id = self.plugin_id.clone();
        let pid = self.pid;
        info!("Shutting down plugin {}", plugin_id);

        // Step 1: send graceful shutdown RPC
        let _ = self
            .client
            .call::<serde_json::Value, serde_json::Value>(
                plugin_methods::SHUTDOWN,
                serde_json::Value::Null,
                timeout,
            )
            .await;

        // Step 2: force-kill (in case the RPC was ignored).
        // The watchdog is blocked on child.wait(). It will unblock once
        // the OS process terminates (killed by PID here), then detect
        // Stopped state and exit without restarting.
        if let Some(pid) = pid {
            force_kill_process(pid);
        }

        // Step 3: mark as Stopped so watchdog won't restart
        self.state.write().clone_from(&ProcessState::Stopped);
        info!("Plugin {} shut down", plugin_id);
    }
}

/// 强制终止指定 PID 的进程。
/// 使用平台原生的 kill 命令（Windows: taskkill, Unix: kill -9）。
pub(crate) fn force_kill_process(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
    }
}

/// Maximum size of a plugin stderr log file (10 MB).
const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;

async fn append_to_log(log_path: &Path, text: &str) {
    use tokio::io::AsyncWriteExt;

    // Rotate log if it exceeds the size cap
    if let Ok(meta) = tokio::fs::metadata(log_path).await {
        if meta.len() > MAX_LOG_SIZE {
            // Simple rotation: keep only one old file
            let rotated = log_path.with_extension("log.1");
            let _ = tokio::fs::remove_file(&rotated).await;
            let _ = tokio::fs::rename(log_path, &rotated).await;
        }
    }

    if let Ok(mut file) = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .await
    {
        let _ = file.write_all(text.as_bytes()).await;
    }
}
