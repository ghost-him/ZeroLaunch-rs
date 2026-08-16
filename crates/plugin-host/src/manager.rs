//! PluginHostManager — top-level orchestration for third-party plugins.

use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use base64::Engine;

use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_api::plugin::{PluginKind, PluginMetadata, PluginMode};
use zerolaunch_plugin_protocol::manifest::Manifest;
use zerolaunch_plugin_protocol::messages::ComponentKind;
use zerolaunch_plugin_protocol::ProtocolError;

use crate::adapter::remote_component::{RemoteComponent, RemoteComponentKind};
use crate::host_dispatch::HostCallHandler;
use crate::process::force_kill_process;
use crate::process::{PluginProcess, ProcessState};

/// 重启回调类型别名：接收重新注册的适配器，
/// 返回一个 future 用于把它们重新注册进 ConfigManager / SessionRouter。
pub type RestartCallback =
    Arc<dyn Fn(PluginRegistration) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// 崩溃即解注册回调：接收崩溃插件的旧注册包，通知 src-tauri 立即解注册
/// CM/SR 中的组件并清理 HostApi 句柄。在崩溃处理的第一步调用，
/// 与后续重启成败无关——放弃重启时不需要任何额外清理。
pub type CrashCallback = Arc<dyn Fn(PluginRegistration) + Send + Sync>;

/// 单个第三方插件的完整注册包。
///
/// 一个插件可以在 manifest 中声明提供多个组件（例如同时提供 DataSource 和 ActionExecutor），
/// 所有组件统一存放在 `components` 中，由消费者按 `RemoteComponent::kind` 过滤后注册到
/// 对应的子系统（Configurable / DataSource / ActionExecutor / Plugin）。
#[derive(Clone, Debug)]
pub struct PluginRegistration {
    /// 插件唯一标识，对应 manifest.toml 中的 plugin.id
    pub plugin_id: String,
    /// 原始 manifest 全文快照
    pub manifest: Manifest,
    /// 插件级元数据（插件自声明为基础，宿主覆盖 id/version/author/kind）。
    /// **唯一源**：与各 `RemoteComponentKind::Plugin` 共享同一 `Arc`，
    /// 由 build_components 一次性构造后不可变；任何字段覆盖只能发生在该函数内。
    /// 供 build_plugin_info 直接取插件级 priority，避免组件最小优先级的双源。
    pub metadata: Arc<PluginMetadata>,

    /// 该插件的所有远程组件。
    /// 每个组件都是一个 `RemoteComponent`，同时实现多个 trait；
    /// 消费者通过 `as_data_source()` / `as_action_executor()` / `as_plugin()` 按需转换。
    pub components: Vec<Arc<RemoteComponent>>,
}

/// 插件运行状态（跨 IPC 序列化的运行状态契约）。
///
/// 由 plugin-host 从 `ProcessState` 映射产出，经 `InstalledPluginInfo.state` 下发，
/// 消费方为前端插件管理页与 CLI 的插件列表展示；序列化为 snake_case 小写字符串
/// （`starting` / `running` / `stopped` / `crashed` / `error` / `unknown`），
/// 前端/CLI 按枚举值精确匹配。无负载变体：`ProcessState` 的 restarts/last_error
/// 属诊断信息，经日志可观测，不进入 IPC 契约（Debug 字符串格式曾直接透传，已废弃）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntimeState {
    /// 子进程启动中 —— 握手/组件发现阶段产生，前端非运行态展示。
    #[serde(rename = "starting")]
    Starting,
    /// 运行中 —— 握手完成、watchdog 存活期间；内置插件恒为该态。
    #[serde(rename = "running")]
    Running,
    /// 已停止 —— 优雅关闭或 auto_restart 关闭后退出。
    #[serde(rename = "stopped")]
    Stopped,
    /// 已崩溃 —— 进程异常退出且重启失败（restarts 细节见日志）。
    #[serde(rename = "crashed")]
    Crashed,
    /// 启动错误 —— 握手/协议失败（具体错误见日志）。
    #[serde(rename = "error")]
    Error,
    /// 状态未知 —— 进程条目不存在（如崩溃重启间隙），默认值。
    #[serde(rename = "unknown")]
    #[default]
    Unknown,
}

impl From<&ProcessState> for PluginRuntimeState {
    fn from(state: &ProcessState) -> Self {
        match state {
            ProcessState::Starting => Self::Starting,
            ProcessState::Running => Self::Running,
            ProcessState::Stopped => Self::Stopped,
            ProcessState::Crashed { .. } => Self::Crashed,
            ProcessState::Error(_) => Self::Error,
        }
    }
}

/// Context needed to restart a crashed plugin.
struct PluginRestartContext {
    manifest: Manifest,
    plugin_dir: PathBuf,
    host_call_handler: Arc<dyn HostCallHandler>,
    /// 崩溃通知通道发送端副本：重新拉起的进程沿用同一通道。
    crash_tx: mpsc::Sender<String>,
    /// 重启成功后调用：供 src-tauri 用新适配器重新注册
    /// ConfigManager 与 SessionRouter。
    /// 返回 future 使调用方可避免 `block_on` 及其 `!Send` 风险。
    on_restart: RestartCallback,
    /// 崩溃即解注册回调：崩溃处理第一步调用（旧注册包交还 src-tauri 清理 CM/SR）。
    on_crash: CrashCallback,
    /// 持久化的重启计数器。每次重新生成前原子递增；
    /// 当达到 manifest.runtime.max_restart 时不再尝试重启。
    restart_count: AtomicU32,
    /// 加载时的宿主界面语言，重启沿用；崩溃时宿主经 update_locale 更新为实时语言。
    locale: RwLock<String>,
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
    /// 内置组件 id 集合（冲突预检数据源之一）。
    ///
    /// 由 src-tauri 在启动时注入一次（内置组件注册完毕、第三方加载之前）；
    /// 前提是内置组件集合启动后稳定。RwLock 仅用于装配期一次性写入，
    /// 预检读取为短临界区同步查询，不跨 .await。
    builtin_component_ids: RwLock<HashSet<String>>,
    /// 自引用 Arc：供内部崩溃处理任务（crash_loop/handle_crash）复用 load() 等 &self 方法。
    ///
    /// new() 构造时一次性写入，之后只读；不使用 RwLock 的原子自引模式
    /// （AsyncDrop 竞态），OnceLock 在构造完成前无并发访问。
    self_arc: OnceLock<Arc<Self>>,
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
    /// 组件 id 与已注册组件（内置或其他插件）冲突，插件加载被拒。
    #[error(
        "component id collision for plugin {plugin_id}: '{component_id}' is already registered"
    )]
    ComponentIdCollision {
        /// 被拒绝加载的插件 id。
        plugin_id: String,
        /// 与已注册组件冲突的组件 id。
        component_id: String,
    },
}

impl PluginHostManager {
    pub fn new(plugins_dir: PathBuf, data_dir_root: PathBuf, log_dir_root: PathBuf) -> Arc<Self> {
        let mgr = Arc::new(Self {
            processes: Arc::new(DashMap::new()),
            plugins: Arc::new(DashMap::new()),
            data_dir_root,
            log_dir_root,
            plugins_dir,
            restart_contexts: Arc::new(DashMap::new()),
            builtin_component_ids: RwLock::new(HashSet::new()),
            self_arc: OnceLock::new(),
        });
        if mgr.self_arc.set(Arc::clone(&mgr)).is_err() {
            panic!("PluginHostManager self arc already set");
        }
        mgr
    }

    /// 注入内置组件 id 集合（冲突预检数据源）。
    ///
    /// 由 src-tauri 在内置组件注册完毕后调用一次；此后内置组件集合
    /// 启动期稳定，不再更新。预检据此识别「与内置组件撞 id」。
    pub fn set_builtin_component_ids(&self, ids: HashSet<String>) {
        *self.builtin_component_ids.write() = ids;
    }

    /// 启动崩溃处理任务。
    ///
    /// 同步包装：async 闭包在独立函数中构造并立即 move 进 tokio::spawn，
    /// 避免 load 的 future 状态机保守保留该闭包（其内部 await crash_loop →
    /// handle_crash → load 形成间接递归），导致 Send 推断失败。
    fn spawn_crash_loop(&self, crash_rx: mpsc::Receiver<String>) {
        let mgr = Arc::clone(self.self_arc.get().expect("self arc set in load"));
        tokio::spawn(async move {
            crash_loop(mgr, crash_rx).await;
        });
    }

    /// 从包含 manifest.toml 的目录加载一个插件。
    ///
    /// `on_restart` 存入重启上下文，崩溃重启成功后被调用，
    /// 供调用方（src-tauri）重新注册新适配器。
    /// `on_crash` 在崩溃检测到的那一刻（任何重启尝试之前）以旧注册包调用，
    /// 供调用方解注册 CM/SR 中的过期组件。
    /// `restart_count` 为已发生的重启次数（初次加载传 0），用于延续崩溃恢复计数。
    /// `locale` 为宿主当前界面语言，随 initialize 握手下发插件进程。
    pub async fn load(
        &self,
        plugin_dir: &Path,
        host_call_handler: Arc<dyn HostCallHandler>,
        on_restart: RestartCallback,
        on_crash: CrashCallback,
        restart_count: u32,
        locale: &str,
    ) -> Result<PluginRegistration, PluginLoadError> {
        let manifest_path = plugin_dir.join("manifest.toml");
        let manifest_bytes = std::fs::read_to_string(&manifest_path)
            .map_err(|e| PluginLoadError::Manifest(format!("cannot read manifest.toml: {}", e)))?;

        let manifest: Manifest = toml::from_str(&manifest_bytes)
            .map_err(|e| PluginLoadError::Manifest(format!("invalid manifest: {}", e)))?;

        // Validate manifest
        validate_manifest(&manifest, plugin_dir)?;

        let plugin_id = manifest.plugin.id.clone();

        // 查重：已加载则拒绝
        if self.processes.contains_key(&plugin_id) {
            return Err(PluginLoadError::AlreadyLoaded(plugin_id));
        }

        let data_dir = self.data_dir_root.join(&plugin_id);
        let log_dir = self.log_dir_root.clone();

        // 确保数据目录存在
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

        // 创建持久崩溃通知通道：管理器持有接收端，发送端跨多次重启共享。
        let (crash_tx, crash_rx) = mpsc::channel::<String>(4);

        // 启动子进程并完成握手
        let process = PluginProcess::spawn(
            &manifest,
            plugin_dir,
            &data_dir,
            &log_dir,
            host_call_handler.clone(),
            crash_tx.clone(),
            restart_count,
            locale,
        )
        .await?;

        // 在把进程移入 Arc 之前取出 client
        let client = process.client.clone();

        // 保存重启上下文，供崩溃恢复使用
        self.restart_contexts.insert(
            plugin_id.clone(),
            Arc::new(PluginRestartContext {
                manifest: manifest.clone(),
                plugin_dir: plugin_dir.to_path_buf(),
                host_call_handler,
                crash_tx,
                on_restart,
                on_crash,
                restart_count: AtomicU32::new(restart_count),
                locale: RwLock::new(locale.to_string()),
            }),
        );

        // 启动崩溃监听任务：崩溃时重新加载该插件
        self.spawn_crash_loop(crash_rx);

        // 在发现组件之前先登记进程，闭合重启窗口：
        // 若插件在 discover_components() 期间崩溃，watchdog 能找到进程条目，
        // 且 crash_loop 会正确处理重新拉起。
        let process = Arc::new(process);
        self.processes.insert(plugin_id.clone(), process.clone());

        // 发现组件（经 Arc 调用——discover_components 取 &self）
        let init_result = match process.discover_components().await {
            Ok(result) => result,
            Err(e) => {
                // 发现失败时清理登记
                self.processes.remove(&plugin_id);
                self.restart_contexts.remove(&plugin_id);
                return Err(PluginLoadError::Protocol(e));
            }
        };

        // 从发现的组件构建适配器
        let adapters = build_components(&plugin_id, plugin_dir, &manifest, client, &init_result);

        // 冲突预检：组件 id 清单来自插件运行时自报（get_components RPC），
        // 只能在 spawn 之后获得。这里在登记 hm.plugins 之前校验，
        // 任一组件 id 与已加载插件或内置组件冲突则整包拒绝：
        // 关闭子进程并清理全部登记，避免「进程残留 + 半提交注册」。
        // 自身组件尚未登记进 plugins map，自碰撞在结构上不可能。
        let component_ids = adapters
            .components
            .iter()
            .map(|c| c.component_id().to_string())
            .collect::<Vec<_>>();
        if let Some(collision) =
            find_component_id_collision(&component_ids, |id| self.component_id_is_taken(id))
        {
            error!(
                "拒绝加载插件 {}：组件 id '{}' 已被其他已注册组件占用，清理进程并放弃加载",
                plugin_id, collision
            );
            self.teardown(&plugin_id).await;
            return Err(PluginLoadError::ComponentIdCollision {
                plugin_id,
                component_id: collision,
            });
        }

        // 先 clone 再登记，返回值无需二次 DashMap 查找 + 6 次字段克隆。
        let registered = adapters.clone();
        self.plugins.insert(plugin_id.clone(), adapters);
        Ok(registered)
    }

    /// 返回插件安装目录（显式存储，非派生）。
    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }

    /// 组件 id 占用查询：已加载第三方插件的组件 或 内置组件集合命中即视为占用。
    ///
    /// 仅用于 load 冲突预检（同步短临界区，不跨 .await）；
    /// 查询时自身组件尚未登记进 plugins map，故不会误判自碰撞。
    fn component_id_is_taken(&self, id: &str) -> bool {
        let taken_by_plugin = self
            .plugins
            .iter()
            .any(|e| e.value().components.iter().any(|c| c.component_id() == id));
        taken_by_plugin || self.builtin_component_ids.read().contains(id)
    }

    /// Unload a plugin: shutdown process and remove from registries.
    pub async fn unload(&self, plugin_id: &str) -> Result<(), PluginLoadError> {
        info!("Unloading plugin {}", plugin_id);
        self.teardown(plugin_id).await;
        Ok(())
    }

    /// 关闭插件子进程并从全部注册表移除（卸载与加载失败清理共用）。
    ///
    /// 若进程 Arc 无法独占（有泄漏的 clone），先标记 Stopped 让 watchdog
    /// 不触发重启，再通过 PID 强制终止子进程，防止孤儿进程泄漏。
    async fn teardown(&self, plugin_id: &str) {
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
                        force_kill_process(pid);
                    }
                }
            }
        }
        self.plugins.remove(plugin_id);

        // Remove log file
        let log_file = self.log_dir_root.join(format!("{}.log", plugin_id));
        let _ = std::fs::remove_file(&log_file);
        self.restart_contexts.remove(plugin_id);
    }

    /// 更新插件重启上下文中的语言快照。
    ///
    /// 宿主在崩溃回调中调用（崩溃时刻取最新语言），保证崩溃重启的
    /// initialize 握手携带最新 locale，而非首次加载时的快照。
    /// 插件不存在（正常卸载等）时静默。
    pub fn update_locale(&self, plugin_id: &str, locale: &str) {
        if let Some(ctx) = self.restart_contexts.get_mut(plugin_id) {
            *ctx.locale.write() = locale.to_string();
        }
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
                let registration = entry.value();
                let process_state = self
                    .processes
                    .get(&registration.plugin_id)
                    .map(|p| PluginRuntimeState::from(&*p.state.read()))
                    .unwrap_or_default();
                build_plugin_info(registration, process_state, enabled_fn(registration))
            })
            .collect();
        result.sort_by_key(|p| (p.priority, p.plugin_id.clone()));
        result
    }

    /// 单个插件的运行时信息（按 id 直查 hm.plugins，不遍历全量列表）。
    ///
    /// 与 list_plugin_info 共用同一构造逻辑（build_plugin_info）。
    pub fn get_plugin_info(
        &self,
        plugin_id: &str,
        enabled_fn: impl Fn(&PluginRegistration) -> bool,
    ) -> Option<InstalledPluginInfo> {
        let entry = self.plugins.get(plugin_id)?;
        let registration = entry.value();
        let process_state = self
            .processes
            .get(plugin_id)
            .map(|p| PluginRuntimeState::from(&*p.state.read()))
            .unwrap_or_default();
        Some(build_plugin_info(
            registration,
            process_state,
            enabled_fn(registration),
        ))
    }
}

/// 由 PluginRegistration 构造运行时信息条目（list_plugin_info / get_plugin_info 共用）。
fn build_plugin_info(
    registration: &PluginRegistration,
    process_state: PluginRuntimeState,
    enabled: bool,
) -> InstalledPluginInfo {
    // 优先级统一取插件级元数据声明值（上层 list_plugins/plugin_info 不再覆盖，
    // 此即全部路径的唯一来源），不再取组件 priority 最小值。
    let priority = registration.metadata.priority;
    InstalledPluginInfo {
        plugin_id: registration.plugin_id.clone(),
        name: registration.manifest.plugin.name.clone(),
        version: registration.manifest.plugin.version.clone(),
        description: registration.manifest.plugin.description.clone(),
        author: registration.manifest.plugin.author.clone(),
        state: process_state,
        enabled,
        kind: PluginKind::ThirdParty,
        priority,
        component_ids: registration
            .components
            .iter()
            .map(|c| c.component_id().to_string())
            .collect(),
        hotkey: registration.metadata.hotkey.clone(),
        icon: registration.metadata.icon.clone(),
        mode: registration.metadata.mode,
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
    pub state: PluginRuntimeState,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    /// 插件种类：内置或第三方（内置条目由 src-tauri 合并时填充）。
    #[serde(rename = "kind", default)]
    pub kind: PluginKind,
    #[serde(rename = "priority")]
    pub priority: u32,
    /// 该插件注册的组件 id 列表（前端据此关联 ConfigManager 中的组件配置）。
    #[serde(rename = "componentIds", default)]
    pub component_ids: Vec<String>,
    /// 全局唤醒快捷键（如 "Ctrl+E"），可空。
    #[serde(rename = "hotkey", default)]
    pub hotkey: Option<String>,
    /// 插件显示图标（data URL，如 "data:image/png;base64,..."），可空表示无图标。
    /// 来源为 manifest [icon] 段，host 层无条件读取。
    #[serde(rename = "icon", default)]
    pub icon: Option<String>,
    /// 插件形态：inline = 行内插件；panel = 完全插件模式（trigger 类型）。
    /// 行内/trigger 判定以此字段为唯一权威依据。
    #[serde(rename = "mode", default)]
    pub mode: PluginMode,
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

    // Validate icon path does not escape the plugin directory (if declared).
    // 文件缺失/不可读不阻断加载（图标可选，由 read_plugin_icon 降级为 None）；
    // 仅当文件存在且能解析时校验逃逸（安全项硬拒绝）。
    if let Some(icon) = &manifest.icon {
        if let Ok(canonical_icon) = plugin_dir.join(&icon.path).canonicalize() {
            if !canonical_icon.starts_with(&canonical_plugin_dir) {
                return Err(PluginLoadError::Manifest(
                    "icon path escapes plugin directory".into(),
                ));
            }
        }
    }

    Ok(())
}

// ─── 辅助函数：按 component_id 从 Vec<(String, T)> 中查找值 ───

/// 读取 manifest [icon] 段声明的图标文件（相对插件目录）并转为 data URL。
/// 路径逃逸插件目录、文件缺失或超过大小上限返回 None（图标缺失不阻断加载）。
fn read_plugin_icon(plugin_dir: &Path, icon_path: &str) -> Option<String> {
    const MAX_ICON_BYTES: u64 = 1024 * 1024; // 1MB，避免超大图标膨胀 plugin_list 载荷

    let icon_abs = plugin_dir.join(icon_path);
    let canonical_icon = icon_abs.canonicalize().ok()?;
    let canonical_plugin_dir = plugin_dir.canonicalize().ok()?;
    // 防路径遍历：图标必须位于插件目录内
    if !canonical_icon.starts_with(&canonical_plugin_dir) {
        warn!("插件图标路径逃逸插件目录，忽略: {}", icon_path);
        return None;
    }
    let meta = std::fs::metadata(&canonical_icon).ok()?;
    if meta.len() > MAX_ICON_BYTES {
        warn!("插件图标超过 1MB 上限，忽略: {}", icon_path);
        return None;
    }
    let bytes = std::fs::read(&canonical_icon).ok()?;
    let mime = icon_mime_from_extension(&canonical_icon);
    Some(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

/// 根据图标文件扩展名推断 MIME 类型，未知扩展名回退 image/png。
fn icon_mime_from_extension(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/png",
    }
}

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

// ─── build_components ───────────────────────────────────────────────

/// 从组件 id 列表中找出第一个已被占用（与已加载插件或内置组件冲突）的 id。
///
/// 仅在本模块加载（load）预检中使用；返回 None 表示全部 id 可用。
/// 独立成纯函数便于单测。
fn find_component_id_collision(
    component_ids: &[String],
    component_id_taken: impl Fn(&str) -> bool,
) -> Option<String> {
    component_ids
        .iter()
        .find(|id| component_id_taken(id))
        .cloned()
}

/// 从 InitResult 构建所有 `RemoteComponent`。
///
/// 每个组件统一构造为 `RemoteComponent`；种类专属数据（target_types、result_actions、
/// metadata）根据 `ComponentKind` 放入 `RemoteComponentKind` 对应变体。
fn build_components(
    plugin_id: &str,
    plugin_dir: &Path,
    manifest: &Manifest,
    client: Arc<crate::client::JsonRpcClient>,
    init_result: &crate::process::InitResult,
) -> PluginRegistration {
    // 插件级元数据：以插件自声明为基础，仅覆盖需宿主保证一致性的字段。
    // 构造完成后包 Arc 成为唯一源：registration.metadata 与各 Plugin 组件共享同一数据，
    // 任何字段覆盖只能发生在此处（覆盖后不再可变）。
    let mut plugin_metadata = init_result.metadata.clone();
    plugin_metadata.id = plugin_id.to_string();
    plugin_metadata.version = manifest.plugin.version.clone();
    plugin_metadata.author = manifest.plugin.author.clone();
    // 第三方插件由宿主强制标注，插件自声明的 kind 不可信
    if plugin_metadata.kind != PluginKind::ThirdParty {
        warn!(
            plugin_id = plugin_id,
            declared = ?plugin_metadata.kind,
            "插件自声明 kind 与宿主强制值不符，已强制为 ThirdParty"
        );
    }
    plugin_metadata.kind = PluginKind::ThirdParty;
    // mode 一致性：声明热键（hotkey 有值）即完全插件模式（panel 形态）——
    // 热键唤醒契约要求全页面接管，行内形态（keep_search_bar=true）是不存在路径。
    // 与 kind 同策略由宿主强制，消除插件自报 Inline+hotkey 的矛盾状态。
    if plugin_metadata.hotkey.is_some() {
        if plugin_metadata.mode != PluginMode::Panel {
            warn!(
                plugin_id = plugin_id,
                declared = ?plugin_metadata.mode,
                "插件声明热键但形态非 panel，已强制为 Panel"
            );
        }
        plugin_metadata.mode = PluginMode::Panel;
    }
    // 图标：从 manifest [icon] 段读取（宿主唯一源，插件 RPC 自上报不采信）
    plugin_metadata.icon = manifest
        .icon
        .as_ref()
        .and_then(|icon| read_plugin_icon(plugin_dir, &icon.path));
    // name, description, supported_os, trigger_keywords, priority
    // 保留插件通过 plugin/get_metadata 自声明的值
    let plugin_metadata = Arc::new(plugin_metadata);

    let components: Vec<Arc<RemoteComponent>> = init_result
        .components
        .iter()
        .map(|comp| {
            let schema = find_by_id(&init_result.settings_schemas, &comp.component_id);
            let settings = find_settings_value(&init_result.settings_values, &comp.component_id);
            let config_actions = find_by_id(&init_result.config_actions_map, &comp.component_id);
            let default_enabled = find_by_id(&init_result.default_enabled_map, &comp.component_id);

            // 组件优先级为 u32（协议层已统一），无负数可能，无需钳制
            let priority = comp.priority;

            let kind = match &comp.kind {
                ComponentKind::Plugin { .. } => {
                    let interaction_policy =
                        find_by_id(&init_result.interaction_policy_map, &comp.component_id);
                    RemoteComponentKind::Plugin {
                        metadata: Arc::clone(&plugin_metadata),
                        interaction_policy: parking_lot::RwLock::new(interaction_policy),
                    }
                }
                ComponentKind::DataSource => RemoteComponentKind::DataSource,
                ComponentKind::ActionExecutor { target_types } => {
                    let result_actions =
                        find_by_id(&init_result.executor_actions_map, &comp.component_id);
                    RemoteComponentKind::ActionExecutor {
                        target_types: target_types.clone(),
                        result_actions,
                    }
                }
            };

            Arc::new(RemoteComponent::new(
                comp.component_id.clone(),
                comp.component_name.clone(),
                comp.component_description.clone(),
                comp.component_type,
                priority,
                client.clone(),
                schema,
                settings,
                config_actions,
                default_enabled,
                kind,
            ))
        })
        .collect();

    let registration = PluginRegistration {
        plugin_id: plugin_id.to_string(),
        manifest: manifest.clone(),
        metadata: plugin_metadata,
        components,
    };

    #[cfg(debug_assertions)]
    {
        // 唯一源断言：所有 Plugin 组件必须与 registration.metadata 共享同一 Arc。
        // 若未来某处改为独立 clone，此处立即失败，防止两副本单边漂移。
        for comp in &registration.components {
            if let RemoteComponentKind::Plugin { metadata, .. } = &comp.kind {
                debug_assert!(
                    Arc::ptr_eq(metadata, &registration.metadata),
                    "组件元数据与 PluginRegistration.metadata 必须同源共享（Arc::clone）"
                );
            }
        }
    }

    registration
}

/// 崩溃处理主循环：监听 crash channel，串行处理每个崩溃事件。
///
/// 每次 load 创建新 channel 时 spawn 一个；channel 关闭（进程与上下文均销毁）即退出。
async fn crash_loop(mgr: Arc<PluginHostManager>, mut crash_rx: mpsc::Receiver<String>) {
    while let Some(plugin_id) = crash_rx.recv().await {
        handle_crash(&mgr, &plugin_id).await;
    }
}

/// 处理单个插件崩溃：崩溃即解注册 → 计数/上限检查 → 复用 load() 重启。
///
/// 旧注册包在第一步交还 src-tauri 解注册（CM/SR/host_api），
/// 之后无论重启成败都不再有残留，无需任何「放弃清理」路径。
async fn handle_crash(mgr: &PluginHostManager, plugin_id: &str) {
    warn!("Watchdog triggered restart for plugin: {}", plugin_id);

    // 崩溃即解注册：从登记中移除旧进程与旧注册包，并把旧注册包
    // 立即交还 src-tauri 清理 CM/SR/host_api。自身组件因此不再
    // 出现在预检数据源中，后续 load 的重启预检不会自碰撞。
    let prev = mgr.plugins.remove(plugin_id);
    mgr.processes.remove(plugin_id);

    // 取出 owned Arc：DashMap 读守卫仅在闭包内存活，map 返回即释放，
    // 不跨后续 load（可能耗时数秒）的 .await。
    let Some(ctx) = mgr
        .restart_contexts
        .get(plugin_id)
        .map(|r| Arc::clone(r.value()))
    else {
        warn!(
            "Crash notification for plugin '{}' but no restart context found — \
             plugin may have been unloaded concurrently",
            plugin_id
        );
        return;
    };
    if let Some((_, prev)) = prev {
        (ctx.on_crash)(prev);
    }

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
        mgr.restart_contexts.remove(plugin_id);
        return;
    }

    // 复用 load()：spawn/discover/预检/登记与初次加载同一路径，
    // 失败路径由 load 自行清理（冲突/discover 失败会 teardown），
    // 此处只需在 load 失败时移除旧上下文。
    // 读取最新语言并克隆：RwLock guard 不得跨 .await 持有
    // （load 内部会 await spawn 握手）。
    let locale = ctx.locale.read().clone();
    match mgr
        .load(
            &ctx.plugin_dir,
            ctx.host_call_handler.clone(),
            ctx.on_restart.clone(),
            ctx.on_crash.clone(),
            new_count,
            &locale,
        )
        .await
    {
        Ok(registered) => {
            // 重启成功：通知 src-tauri 注册新适配器（旧组件已在崩溃第一步解注册）
            (ctx.on_restart)(registered).await;
            info!("Plugin {} successfully restarted", plugin_id);
        }
        Err(e) => {
            error!("Failed to restart plugin {}: {}", plugin_id, e);
            mgr.restart_contexts.remove(plugin_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::find_component_id_collision;

    /// 冲突预检契约：返回第一个被占用的组件 id；全部可用时返回 None。
    #[test]
    fn collision_check_finds_first_taken_id() {
        let ids = vec!["free".to_string(), "taken".to_string(), "other".to_string()];
        let taken = |id: &str| id == "taken";
        assert_eq!(
            find_component_id_collision(&ids, taken),
            Some("taken".to_string())
        );
    }

    /// 无冲突时返回 None，加载预检可放行。
    #[test]
    fn collision_check_returns_none_when_all_free() {
        let ids = vec!["a".to_string(), "b".to_string()];
        assert_eq!(find_component_id_collision(&ids, |_| false), None);
    }
}
