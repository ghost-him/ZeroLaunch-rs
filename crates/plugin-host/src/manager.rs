//! PluginHostManager — top-level orchestration for third-party plugins.

use dashmap::DashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_protocol::manifest::Manifest;
use zerolaunch_plugin_protocol::messages::{ComponentDescriptor, ComponentKind};
use zerolaunch_plugin_protocol::ProtocolError;

use crate::adapter::remote_configurable::RemoteConfigurableAdapter;
use crate::adapter::remote_data_source::RemoteDataSourceAdapter;
use crate::adapter::remote_executor::RemoteExecutorAdapter;
use crate::adapter::remote_plugin::RemotePluginAdapter;
use crate::host_dispatch::HostCallHandler;
use crate::process::{PluginProcess, ProcessState};

/// Type alias for the restart callback: receives newly registered adapters
/// and returns a future that re-registers them with ConfigManager / SessionRouter.
pub type RestartCallback =
    Arc<dyn Fn(PluginRegistration) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// 单个第三方插件的完整注册包。
///
/// 一个插件可以在 manifest 中声明提供多个组件（例如同时提供 DataSource 和 ActionExecutor），
/// 每个组件对应一个 `RemoteConfigurableAdapter` 存入 `configurables`；
/// 再根据每个组件的 `ComponentKind` 分别放入 `data_sources` / `executors` / `plugin`。
#[derive(Clone, Debug)]
pub struct PluginRegistration {
    /// 插件唯一标识，对应 manifest.toml 中的 plugin.id
    pub plugin_id: String,
    /// 原始 manifest 全文快照
    pub manifest: Manifest,

    /// 插件类型组件（kind=Plugin）。
    /// None 表示该插件没有声明 Plugin 类型的组件（仅提供 DataSource/ActionExecutor 等）。
    pub plugin: Option<Arc<RemotePluginAdapter>>,

    /// 数据源组件列表（kind=DataSource）。
    /// 一个插件可以声明多个数据源（如 "浏览器书签" + "系统命令"），各自独立。
    pub data_sources: Vec<Arc<RemoteDataSourceAdapter>>,

    /// 执行器组件列表（kind=ActionExecutor）。
    /// 一个插件可以声明多个执行器，各自处理不同的 target_type。
    pub executors: Vec<Arc<RemoteExecutorAdapter>>,

    /// 所有组件的统一可配置接口。
    /// **每个组件**（无论它是 Plugin / DataSource / ActionExecutor）都有一个
    /// `RemoteConfigurableAdapter` 放入此列表，因此长度 = 组件总数。
    /// ConfigManager 通过这个列表管理每个组件的独立配置项、Schema 和启用状态。
    pub configurables: Vec<Arc<RemoteConfigurableAdapter>>,
}

impl PluginRegistration {
    /// 计算该插件的全局排序优先级，取所有组件 `priority()` 的最小值。
    /// 没有任何组件时返回默认值 50。
    pub fn compute_priority(&self) -> u32 {
        self.configurables
            .iter()
            .map(|c| c.priority())
            .min()
            .unwrap_or(50)
    }
}

/// Context needed to restart a crashed plugin.
struct PluginRestartContext {
    manifest: Manifest,
    plugin_dir: PathBuf,
    host_call_handler: Arc<dyn HostCallHandler>,
    /// Clone of the crash notification channel sender, so re-spawned
    /// processes can use the same channel.
    crash_tx: mpsc::Sender<String>,
    /// Called after a successful restart so src-tauri can re-register
    /// the new adapters with ConfigManager and SessionRouter.
    /// Returns a future so the caller can avoid `block_on` and its `!Send` risks.
    on_restart: RestartCallback,
    /// 持久化的重启计数器。每次重新生成前原子递增；
    /// 当达到 manifest.runtime.max_restart 时不再尝试重启。
    restart_count: AtomicU32,
}

impl std::fmt::Debug for PluginRestartContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginRestartContext")
            .field("manifest", &self.manifest)
            .field("plugin_dir", &self.plugin_dir)
            .field("host_call_handler", &self.host_call_handler)
            .field("crash_tx", &self.crash_tx)
            .field("on_restart", &"<RestartCallback>")
            .field("restart_count", &self.restart_count)
            .finish()
    }
}

/// 第三方插件子进程的顶层管理器。
pub struct PluginHostManager {
    /// 已加载插件的子进程映射。
    pub processes: Arc<DashMap<String, Arc<PluginProcess>>>,
    /// 已加载插件的组件注册包（含 DataSource / Executor / Plugin 适配器）。
    pub plugins: Arc<DashMap<String, PluginRegistration>>,
    /// 插件数据的根目录。
    pub data_dir_root: PathBuf,
    /// 插件 stderr 日志的根目录。
    pub log_dir_root: PathBuf,
    /// 插件安装目录。
    plugins_dir: PathBuf,
    /// 每次加载插件时保存重启上下文，崩溃后可重新拉起。
    restart_contexts: Arc<DashMap<String, Arc<PluginRestartContext>>>,
}

/// Error type for plugin loading operations.
#[derive(Debug, thiserror::Error)]
pub enum PluginLoadError {
    #[error("manifest error: {0}")]
    Manifest(String),
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("plugin already loaded: {0}")]
    AlreadyLoaded(String),
    #[error("plugin not found: {0}")]
    NotFound(String),
}

impl PluginHostManager {
    pub fn new(plugins_dir: PathBuf, data_dir_root: PathBuf, log_dir_root: PathBuf) -> Self {
        Self {
            processes: Arc::new(DashMap::new()),
            plugins: Arc::new(DashMap::new()),
            data_dir_root,
            log_dir_root,
            plugins_dir,
            restart_contexts: Arc::new(DashMap::new()),
        }
    }

    /// Load a plugin from a directory containing manifest.toml.
    ///
    /// `on_restart` is stored and called after a successful crash re-spawn
    /// so the caller (src-tauri) can re-register the new adapters.
    pub async fn load(
        &self,
        plugin_dir: &Path,
        host_call_handler: Arc<dyn HostCallHandler>,
        on_restart: RestartCallback,
    ) -> Result<PluginRegistration, PluginLoadError> {
        let manifest_path = plugin_dir.join("manifest.toml");
        let manifest_bytes = std::fs::read_to_string(&manifest_path)
            .map_err(|e| PluginLoadError::Manifest(format!("cannot read manifest.toml: {}", e)))?;

        let manifest: Manifest = toml::from_str(&manifest_bytes)
            .map_err(|e| PluginLoadError::Manifest(format!("invalid manifest: {}", e)))?;

        // Validate manifest
        validate_manifest(&manifest, plugin_dir)?;

        let plugin_id = manifest.plugin.id.clone();

        // Check for duplicate
        if self.processes.contains_key(&plugin_id) {
            return Err(PluginLoadError::AlreadyLoaded(plugin_id));
        }

        let data_dir = self.data_dir_root.join(&plugin_id);
        let log_dir = self.log_dir_root.clone();

        // Ensure data directory exists
        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            warn!(
                "Failed to create plugin data dir {}: {}",
                data_dir.display(),
                e
            );
        }
        if let Err(e) = std::fs::create_dir_all(&log_dir) {
            warn!(
                "Failed to create plugin log dir {}: {}",
                log_dir.display(),
                e
            );
        }

        info!("Loading plugin {} from {}", plugin_id, plugin_dir.display());

        // Create a persistent crash notification channel.
        // The manager owns the receiver; the sender is shared across re-spawns.
        let (crash_tx, crash_rx) = mpsc::channel::<String>(4);

        // Spawn process and run handshake
        let process = PluginProcess::spawn(
            &manifest,
            plugin_dir,
            &data_dir,
            &log_dir,
            host_call_handler.clone(),
            crash_tx.clone(),
            0, // 初次启动，无先前重启记录
        )
        .await?;

        // Extract the client before moving process into the Arc
        let client = process.client.clone();

        // Store restart context for crash recovery
        self.restart_contexts.insert(
            plugin_id.clone(),
            Arc::new(PluginRestartContext {
                manifest: manifest.clone(),
                plugin_dir: plugin_dir.to_path_buf(),
                host_call_handler,
                crash_tx,
                on_restart,
                restart_count: AtomicU32::new(0),
            }),
        );

        // Spawn a listener task that re-loads the plugin on crash notification
        let data_root = self.data_dir_root.clone();
        let log_root = self.log_dir_root.clone();
        let processes = self.processes.clone();
        let plugins = self.plugins.clone();
        let contexts = self.restart_contexts.clone();
        tokio::spawn(async move {
            restart_loop(crash_rx, processes, plugins, contexts, data_root, log_root).await;
        });

        // Insert process into registry BEFORE discovery.
        // This closes the restart window: if the plugin crashes during
        // discover_components(), the watchdog can find the process entry
        // and restart_loop will correctly handle the re-spawn.
        let process = Arc::new(process);
        self.processes.insert(plugin_id.clone(), process.clone());

        // Discover components (through the Arc — discover_components takes &self)
        let init_result = match process.discover_components().await {
            Ok(result) => result,
            Err(e) => {
                // Clean up on discovery failure
                self.processes.remove(&plugin_id);
                self.restart_contexts.remove(&plugin_id);
                return Err(PluginLoadError::Protocol(e));
            }
        };

        // Build adapters from discovered components
        let adapters = build_adapters(&plugin_id, &manifest, client, &init_result);

        // Clone before insert so the return value is ready without a second
        // DashMap lookup + 6 individual field clones.
        let registered = adapters.clone();
        self.plugins.insert(plugin_id.clone(), adapters);
        Ok(registered)
    }

    /// Returns the plugins installation directory (explicitly stored, not derived).
    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }

    /// Unload a plugin: shutdown process and remove from registries.
    pub async fn unload(&self, plugin_id: &str) -> Result<(), PluginLoadError> {
        info!("Unloading plugin {}", plugin_id);

        // shutdown() takes self (ownership), so we must unwrap the Arc.
        // If try_unwrap fails (Arc refcount > 1), log a warning — this
        // indicates the process Arc was cloned elsewhere, which shouldn't
        // happen in normal operation.
        if let Some((_, proc)) = self.processes.remove(plugin_id) {
            match Arc::try_unwrap(proc) {
                Ok(process) => process.shutdown(std::time::Duration::from_secs(5)).await,
                Err(arc) => {
                    warn!(
                        "Plugin {} process Arc has {} strong references; forcing kill. \
                         This may indicate a leaked clone of the process handle.",
                        plugin_id,
                        Arc::strong_count(&arc)
                    );
                    // 先标记 Stopped，让 watchdog 在进程退出后检测到此状态而不触发重启
                    arc.state.write().clone_from(&ProcessState::Stopped);
                    // 通过 PID 强制终止子进程，防止孤儿进程泄漏
                    if let Some(pid) = arc.pid {
                        crate::process::force_kill_process(pid);
                    }
                }
            }
        }
        self.plugins.remove(plugin_id);

        // Remove log file
        let log_file = self.log_dir_root.join(format!("{}.log", plugin_id));
        let _ = std::fs::remove_file(&log_file);
        self.restart_contexts.remove(plugin_id);

        Ok(())
    }

    /// Reload a plugin (unload + load).
    pub async fn reload(
        &self,
        plugin_id: &str,
        plugin_dir: &Path,
        host_call_handler: Arc<dyn HostCallHandler>,
        on_restart: RestartCallback,
    ) -> Result<PluginRegistration, PluginLoadError> {
        self.unload(plugin_id).await?;
        self.load(plugin_dir, host_call_handler, on_restart).await
    }

    /// Build `InstalledPluginInfo` for all loaded adapters.
    ///
    /// `enabled_fn` is called per-adapter to determine the `enabled` field;
    /// callers pass a closure that queries `ConfigManager::is_enabled`.
    pub fn list_plugin_info(
        &self,
        enabled_fn: impl Fn(&PluginRegistration) -> bool,
    ) -> Vec<InstalledPluginInfo> {
        let mut result: Vec<InstalledPluginInfo> = self
            .plugins
            .iter()
            .map(|entry| {
                let plugin_registration = entry.value();
                let process_state = self
                    .processes
                    .get(&plugin_registration.plugin_id)
                    .map(|p| format!("{:?}", *p.state.read()))
                    .unwrap_or_else(|| "unknown".to_string());
                let priority = plugin_registration.compute_priority();
                let enabled = enabled_fn(plugin_registration);
                InstalledPluginInfo {
                    plugin_id: plugin_registration.plugin_id.clone(),
                    name: plugin_registration.manifest.plugin.name.clone(),
                    version: plugin_registration.manifest.plugin.version.clone(),
                    description: plugin_registration.manifest.plugin.description.clone(),
                    author: plugin_registration.manifest.plugin.author.clone(),
                    state: process_state,
                    enabled,
                    priority,
                }
            })
            .collect();
        result.sort_by_key(|p| (p.priority, p.plugin_id.clone()));
        result
    }
}

/// Information about an installed plugin for the management UI / CLI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstalledPluginInfo {
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "version")]
    pub version: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "author")]
    pub author: String,
    #[serde(rename = "state")]
    pub state: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "priority")]
    pub priority: u32,
}

// ─── Helpers ──────────────────────────────────────────────────────

fn validate_manifest(manifest: &Manifest, plugin_dir: &Path) -> Result<(), PluginLoadError> {
    let id = &manifest.plugin.id;

    // Validate plugin ID format (regex compiled once)
    static PLUGIN_ID_RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = PLUGIN_ID_RE.get_or_init(|| {
        regex::Regex::new(zerolaunch_plugin_protocol::manifest::PLUGIN_ID_RE).unwrap()
    });
    if !re.is_match(id) {
        return Err(PluginLoadError::Manifest(format!(
            "invalid plugin id '{}': must match reverse domain",
            id
        )));
    }

    // Validate version
    if semver::Version::parse(&manifest.plugin.version).is_err() {
        return Err(PluginLoadError::Manifest(format!(
            "invalid plugin version '{}'",
            manifest.plugin.version
        )));
    }

    // Validate required provides
    if manifest.components.provides.is_empty() {
        return Err(PluginLoadError::Manifest(
            "components.provides must have at least one entry".into(),
        ));
    }

    for p in &manifest.components.provides {
        if !zerolaunch_plugin_protocol::manifest::REQUIRED_PROVIDES_VALUES.contains(&p.as_str()) {
            return Err(PluginLoadError::Manifest(format!(
                "unknown component type '{}'",
                p
            )));
        }
    }

    // Validate min_host_version
    let host_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|e| PluginLoadError::Manifest(format!("host version parse: {}", e)))?;
    let min_required = semver::Version::parse(&manifest.plugin.min_host_version)
        .map_err(|e| PluginLoadError::Manifest(format!("min_host_version parse: {}", e)))?;
    if host_version < min_required {
        return Err(PluginLoadError::Manifest(format!(
            "plugin requires host >= {}, current is {}",
            min_required, host_version
        )));
    }

    // Validate command path does not escape the plugin directory
    let cmd_path = plugin_dir.join(&manifest.runtime.command);
    let canonical_cmd = cmd_path
        .canonicalize()
        .map_err(|e| PluginLoadError::Manifest(format!("command not found: {}", e)))?;
    let canonical_plugin_dir = plugin_dir
        .canonicalize()
        .map_err(|e| PluginLoadError::Manifest(format!("plugin dir canonicalize: {}", e)))?;
    if !canonical_cmd.starts_with(&canonical_plugin_dir) {
        return Err(PluginLoadError::Manifest(
            "command path escapes plugin directory".into(),
        ));
    }

    Ok(())
}

// ─── 辅助函数：按 component_id 从 Vec<(String, T)> 中查找值 ───

/// 从 `Vec<(String, T)>` 中按 component_id 查找值，找不到返回 default。
fn find_by_id<T: Clone + Default>(map: &[(String, T)], component_id: &str) -> T {
    map.iter()
        .find(|(id, _)| id == component_id)
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// 从 settings_values 中查找，找不到返回 Null（区别于 default）。
fn find_settings_value(
    values: &[(String, serde_json::Value)],
    component_id: &str,
) -> serde_json::Value {
    values
        .iter()
        .find(|(id, _)| id == component_id)
        .map(|(_, v)| v.clone())
        .unwrap_or(serde_json::Value::Null)
}

// ─── build_adapters ─────────────────────────────────────────────────

/// 中间产物：统一提取的公共配置 + 组件描述符引用。
struct ComponentBuildInfo<'a> {
    desc: &'a ComponentDescriptor,
    configurable: Arc<RemoteConfigurableAdapter>,
}

/// 从 InitResult 构建所有 Remote*Adapter。
///
/// 分两步：
/// 1. 统一提取所有组件的 schema/settings/config_actions → RemoteConfigurableAdapter
/// 2. 按 ComponentKind 差异化构造领域 adapter（Plugin / DataSource / ActionExecutor）
fn build_adapters(
    plugin_id: &str,
    manifest: &Manifest,
    client: Arc<crate::client::JsonRpcClient>,
    init_result: &crate::process::InitResult,
) -> PluginRegistration {
    let mut plugin_adapter: Option<Arc<RemotePluginAdapter>> = None;
    let mut data_sources = Vec::new();
    let mut executors = Vec::new();
    let mut configurables = Vec::new();

    // ── 步骤一：统一提取公共配置 ──
    let infos: Vec<ComponentBuildInfo> = init_result
        .components
        .iter()
        .map(|comp| {
            let schema = find_by_id(&init_result.settings_schemas, &comp.component_id);
            let settings = find_settings_value(&init_result.settings_values, &comp.component_id);
            let config_actions = find_by_id(&init_result.config_actions_map, &comp.component_id);

            let configurable = Arc::new(RemoteConfigurableAdapter::new(
                comp.component_id.clone(),
                comp.component_name.clone(),
                comp.component_description.clone(),
                comp.component_type,
                {
                    if comp.priority < 0 {
                        warn!(
                            "component {} has negative priority {}; clamping to 0",
                            comp.component_id, comp.priority
                        );
                    }
                    comp.priority.max(0) as u32
                },
                client.clone(),
                schema,
                settings,
                config_actions,
            ));
            configurables.push(configurable.clone());

            ComponentBuildInfo {
                desc: comp,
                configurable,
            }
        })
        .collect();

    // ── 步骤二：按 kind 差异化构造领域 adapter ──
    for info in &infos {
        match &info.desc.kind {
            ComponentKind::Plugin { .. } => {
                // 以插件自声明的 metadata 为基础，仅覆盖需宿主保证一致性的字段
                let mut metadata = init_result.metadata.clone();
                metadata.id = plugin_id.to_string();
                metadata.version = manifest.plugin.version.clone();
                metadata.author = manifest.plugin.author.clone();
                // name, description, supported_os, trigger_keywords, priority
                // 保留插件通过 plugin/get_metadata 自声明的值

                plugin_adapter = Some(Arc::new(RemotePluginAdapter {
                    metadata,
                    client: client.clone(),
                    configurable: info.configurable.clone(),
                }));
            }
            ComponentKind::DataSource => {
                data_sources.push(Arc::new(RemoteDataSourceAdapter {
                    component_id: info.desc.component_id.clone(),
                    configurable: info.configurable.clone(),
                    client: client.clone(),
                }));
            }
            ComponentKind::ActionExecutor { target_types } => {
                let cached_actions =
                    find_by_id(&init_result.executor_actions_map, &info.desc.component_id);

                executors.push(Arc::new(RemoteExecutorAdapter {
                    component_id: info.desc.component_id.clone(),
                    configurable: info.configurable.clone(),
                    client: client.clone(),
                    cached_target_types: target_types.clone(),
                    cached_actions,
                }));
            }
        }
    }

    PluginRegistration {
        plugin_id: plugin_id.to_string(),
        manifest: manifest.clone(),
        plugin: plugin_adapter,
        data_sources,
        executors,
        configurables,
    }
}

/// Background task that listens for crash notifications on `crash_rx`
/// and triggers a re-spawn of the crashed plugin. This keeps restart
/// logic contained within the manager.
async fn restart_loop(
    mut crash_rx: tokio::sync::mpsc::Receiver<String>,
    processes: Arc<DashMap<String, Arc<PluginProcess>>>,
    plugins: Arc<DashMap<String, PluginRegistration>>,
    contexts: Arc<DashMap<String, Arc<PluginRestartContext>>>,
    data_dir_root: PathBuf,
    log_dir_root: PathBuf,
) {
    // 这个循环处理的是全已所有的崩溃事件。每当有插件崩溃时，watchdog 会通过 crash_tx 发送该插件的 ID 到 crash_rx。restart_loop 监听这个 channel，一旦收到崩溃通知，就会执行以下步骤：
    while let Some(plugin_id) = crash_rx.recv().await {
        warn!("Watchdog triggered restart for plugin: {}", plugin_id);

        // Remove old process and plugins
        processes.remove(&plugin_id);
        plugins.remove(&plugin_id);

        // 取出 owned Arc：DashMap 读守卫仅在闭包内存活，map 返回即释放，
        // 不跨后续 spawn / discover / on_restart 的（可能耗时数秒的）.await。
        let Some(ctx) = contexts.get(&plugin_id).map(|r| Arc::clone(r.value())) else {
            warn!(
                "Crash notification for plugin '{}' but no restart context found — \
                 plugin may have been unloaded concurrently",
                plugin_id
            );
            continue;
        };

        // 原子递增持久化的重启计数器，并检查是否超出 max_restart。
        // 这是**唯一**追踪重启次数的地方——不在看门狗或
        // PluginProcess 中（它们每次重启都会被替换）。
        let new_count = ctx.restart_count.fetch_add(1, Ordering::SeqCst) + 1;
        let max_restart = ctx.manifest.runtime.max_restart;
        if new_count > max_restart {
            error!(
                "Plugin {} exceeded max restarts ({}/{}) — not restarting",
                plugin_id, new_count, max_restart
            );
            contexts.remove(&plugin_id);
            continue;
        }

        let data_dir = data_dir_root.join(&plugin_id);
        let log_dir = log_dir_root.clone();
        let _ = std::fs::create_dir_all(&data_dir);

        match PluginProcess::spawn(
            &ctx.manifest,
            &ctx.plugin_dir,
            &data_dir,
            &log_dir,
            ctx.host_call_handler.clone(),
            ctx.crash_tx.clone(),
            new_count,
        )
        .await
        {
            Ok(new_process) => {
                // Discover components and rebuild adapters
                match new_process.discover_components().await {
                    Ok(init_result) => {
                        let new_adapters = build_adapters(
                            &plugin_id,
                            &ctx.manifest,
                            new_process.client.clone(),
                            &init_result,
                        );
                        processes.insert(plugin_id.clone(), Arc::new(new_process));
                        plugins.insert(plugin_id.clone(), new_adapters.clone());
                        // Notify src-tauri so it can re-register the new adapters
                        (ctx.on_restart)(new_adapters).await;
                        info!("Plugin {} successfully restarted", plugin_id);
                    }
                    Err(e) => {
                        error!(
                            "Failed to discover components after restart of {}: {}",
                            plugin_id, e
                        );
                        contexts.remove(&plugin_id);
                    }
                }
            }
            Err(e) => {
                error!("Failed to restart plugin {}: {}", plugin_id, e);
                contexts.remove(&plugin_id);
            }
        }
    }
}
