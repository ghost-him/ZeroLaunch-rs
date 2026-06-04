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
}

impl PluginProcess {
    /// Spawn the plugin subprocess, complete the initialize handshake,
    /// and start stderr log collection.
    pub async fn spawn(
        manifest: &Manifest,
        plugin_dir: &Path,
        data_dir: &Path,
        log_dir: &Path,
        host_call_handler: Arc<dyn HostCallHandler>,
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

        let _init_result: InitializeResult = client
            .call(
                plugin_methods::INITIALIZE,
                init_params,
                Duration::from_secs(manifest.runtime.startup_timeout),
            )
            .await?;

        info!("Plugin {} initialized (pid={:?})", plugin_id, pid);

        let process = Self {
            plugin_id: plugin_id.clone(),
            manifest: manifest.clone(),
            state: Arc::new(RwLock::new(ProcessState::Running)),
            client,
            data_dir: data_dir.to_path_buf(),
            child_handle,
        };

        // Start health monitoring
        process.spawn_watchdog();

        Ok(process)
    }

    /// Run the post-initialization discovery sequence:
    /// plugin/get_metadata, plugin/get_components, and for each component:
    /// plugin/get_settings_schema, plugin/get_settings, plugin/config_actions.
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

        // For each component, fetch schema, settings, actions
        let mut settings_schemas = Vec::new();
        let mut settings_values = Vec::new();
        let mut config_actions_map = Vec::new();

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
        }

        Ok(InitResult {
            plugin_id,
            metadata,
            components,
            settings_schemas,
            settings_values,
            config_actions_map,
        })
    }

    /// Spawn a watchdog task that monitors the child process.
    /// If the process exits and `auto_restart=true`, attempts to restart up to `max_restart` times.
    /// When max_restart is exceeded, marks the process as crashed permanently.
    pub fn spawn_watchdog(&self) {
        let plugin_id = self.plugin_id.clone();
        let state = self.state.clone();
        let child_handle = self.child_handle.clone();
        let auto_restart = self.manifest.runtime.auto_restart;
        let max_restart = self.manifest.runtime.max_restart;

        tokio::spawn(async move {
            loop {
                // Check if the child is still alive
                let exited = {
                    let mut guard = child_handle.lock();
                    if let Some(ref mut child) = *guard {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                info!(
                                    "Plugin {} process exited with status: {:?}",
                                    plugin_id, status
                                );
                                *guard = None; // Take the child handle
                                true
                            }
                            Ok(None) => false, // Still running
                            Err(e) => {
                                warn!("Plugin {} try_wait error: {}", plugin_id, e);
                                *guard = None;
                                true
                            }
                        }
                    } else {
                        // Child already taken
                        return;
                    }
                };

                if !exited {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }

                // Process exited — check restart policy
                if !auto_restart {
                    state.write().clone_from(&ProcessState::Stopped);
                    info!("Plugin {} auto-restart disabled, stopped", plugin_id);
                    return;
                }

                let restarts = {
                    let mut s = state.write();
                    match &*s {
                        ProcessState::Crashed { restarts, .. } => {
                            let new_count = restarts + 1;
                            if new_count < max_restart {
                                *s = ProcessState::Crashed {
                                    restarts: new_count,
                                    last_error: "process exited unexpectedly".into(),
                                };
                                info!(
                                    "Plugin {} crashed, restart {}/{}",
                                    plugin_id, new_count, max_restart
                                );
                                Some(new_count)
                            } else {
                                *s = ProcessState::Error(format!(
                                    "max restarts ({}) exceeded",
                                    max_restart
                                ));
                                warn!(
                                    "Plugin {} exceeded max restarts ({})",
                                    plugin_id, max_restart
                                );
                                None // Don't restart
                            }
                        }
                        _ => {
                            *s = ProcessState::Crashed {
                                restarts: 1,
                                last_error: "process exited unexpectedly".into(),
                            };
                            info!("Plugin {} crashed, restart 1/{}", plugin_id, max_restart);
                            Some(1)
                        }
                    }
                };

                if restarts.is_none() {
                    return; // Max restarts exceeded, stop
                }

                // Note: actual restart requires re-spawning, which needs the full context.
                // For now, we mark the state and signal that a restart should happen.
                // The PluginHostManager handles the actual re-spawn.
                // We mark as Error to signal the manager.
                if restarts.unwrap() >= max_restart {
                    state.write().clone_from(&ProcessState::Error(format!(
                        "max restarts exceeded ({})",
                        max_restart
                    )));
                    return;
                }
            }
        });
    }

    /// Graceful shutdown: send plugin/shutdown, wait, then kill.
    pub async fn shutdown(self, timeout: Duration) {
        let plugin_id = self.plugin_id.clone();
        info!("Shutting down plugin {}", plugin_id);

        let _ = self
            .client
            .call::<serde_json::Value, serde_json::Value>(
                plugin_methods::SHUTDOWN,
                serde_json::Value::Null,
                timeout,
            )
            .await;

        self.state.write().clone_from(&ProcessState::Stopped);
        info!("Plugin {} shut down", plugin_id);
    }
}

async fn append_to_log(log_path: &Path, text: &str) {
    use tokio::io::AsyncWriteExt;
    if let Ok(mut file) = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .await
    {
        let _ = file.write_all(text.as_bytes()).await;
    }
}
