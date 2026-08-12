//! PluginManager — 插件生命周期的统一入口。
//!
//! 管理第三方插件的加载/卸载/安装/崩溃恢复（经 PluginHostManager），
//! 内置组件由 builtin_registry 收集后由调用方（bootstrap）注册。
//! 插件级视图数据（列表/详情）由 commands 层经 host_runtime_infos + PluginRegistry 组装，
//! 不经本模块缓存。
//!
//! 注册/解注册通过 PluginRuntimeEvent 广播通道（PM → CM 解耦管道）完成，
//! ConfigManager 处理配置侧（Configurable）+ 转发 ConfigEvent 到 SessionDispatcher。

use parking_lot::RwLock;
use std::collections::HashSet;
use std::fmt;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tracing::{error, info};
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_api::host::PluginSdkConfig;
use zerolaunch_plugin_api::plugin::{PluginKind, PluginMetadata};
use zerolaunch_plugin_host::host_dispatch::HostCallHandler;
use zerolaunch_plugin_host::manager::{
    CrashCallback, InstalledPluginInfo, PluginHostManager, PluginLoadError, PluginRegistration,
    PluginRuntimeState, RestartCallback,
};
use zerolaunch_plugin_protocol::Manifest;

use crate::core::config::event::{PluginEventSender, PluginRuntimeEvent};
use crate::core::config::manager::ConfigManager;
use crate::plugin_framework::builtin_registry;
use crate::plugin_framework::builtin_registry::{CollectedBuiltins, InventoryContext};
use crate::plugin_framework::zlplugin_protocol::ZlpluginProtocolHandler;
use crate::plugin_framework::SessionDispatcher;
use crate::sdk::HostApi;

use super::host_handler::TauriHostCallHandler;
use super::plugin_info::InstallError;
use super::plugin_installer::PluginInstaller;
use crate::core::i18n::I18nManager;

/// PluginManager 内部错误类型。
/// 在 commands/ 层通过 From 转换为 BridgeError。
#[derive(Debug)]
pub enum PluginManagerError {
    /// 插件未找到
    PluginNotFound(String),
    /// 文件未找到
    FileNotFound(String),
    /// 不支持的文件格式
    UnsupportedFormat(String),
    /// 同名插件已安装
    AlreadyInstalled(String),
    /// 组件 id 与已注册组件冲突（加载被拒，插件未加载）
    ComponentIdCollision(String),
    /// 常规内部错误
    Internal(String),
}

impl fmt::Display for PluginManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginManagerError::PluginNotFound(msg) => write!(f, "插件未找到: {}", msg),
            PluginManagerError::FileNotFound(msg) => write!(f, "文件未找到: {}", msg),
            PluginManagerError::UnsupportedFormat(msg) => write!(f, "不支持的文件格式: {}", msg),
            PluginManagerError::AlreadyInstalled(id) => write!(f, "插件已安装: {}", id),
            PluginManagerError::ComponentIdCollision(id) => {
                write!(f, "组件 id 已被其他已注册组件占用: {}", id)
            }
            PluginManagerError::Internal(msg) => write!(f, "插件管理器内部错误: {}", msg),
        }
    }
}

impl std::error::Error for PluginManagerError {}

/// 插件管理器：统一管理内置组件与第三方插件的生命周期与注册编排。
///
/// 所有方法使用 `&self`（内部 RwLock 实现可变性），
/// 与 SessionDispatcher / ConfigManager 的模式一致。
///
/// 不再直接依赖 ConfigManager，通过 PluginRuntimeEvent 广播通道与 CM 通信。
pub struct PluginManager {
    /// PluginRuntimeEvent 通道发送端（PM → CM 解耦管道）
    plugin_event_tx: RwLock<Option<PluginEventSender>>,
    /// HostApi 引用
    host_api: RwLock<Option<Arc<HostApi>>>,
    /// 后端翻译服务（host/i18n.get_locale 与插件翻译目录注册）
    i18n: RwLock<Option<Arc<I18nManager>>>,
    /// PluginHostManager（内部构造，管理子进程生命周期）
    host_manager: RwLock<Option<Arc<PluginHostManager>>>,
}

impl PluginManager {
    /// 创建 PluginManager 实例。
    pub fn new() -> Self {
        Self {
            plugin_event_tx: RwLock::new(None),
            host_api: RwLock::new(None),
            i18n: RwLock::new(None),
            host_manager: RwLock::new(None),
        }
    }

    // ── 依赖注入（在 init_app_state 中各调用一次） ─────────────

    /// 设置 PluginRuntimeEvent 通道发送端。
    pub fn set_plugin_event_tx(&self, tx: PluginEventSender) {
        *self.plugin_event_tx.write() = Some(tx);
    }

    /// 设置 HostApi 引用。
    pub fn set_host_api(&self, api: Arc<HostApi>) {
        *self.host_api.write() = Some(api);
    }

    /// 注入后端翻译服务（bootstrap 在加载第三方插件前调用）。
    pub fn set_i18n_manager(&self, i18n: Arc<I18nManager>) {
        *self.i18n.write() = Some(i18n);
    }

    /// 读取后端翻译服务引用。
    ///
    /// # Panics
    ///
    /// 未注入时 panic：bootstrap 顺序不变式保证 `set_i18n_manager` 先于
    /// 第三方插件加载/卸载路径执行；None 意味着初始化流程被破坏。
    fn i18n_manager(&self) -> Arc<I18nManager> {
        self.i18n
            .read()
            .as_ref()
            .cloned()
            .expect("i18n manager 必须在加载第三方插件前注入（bootstrap 顺序不变式）")
    }

    /// 初始化 PluginHostManager（PluginManager 内部构造，不从外部注入）。
    /// 子目录命名（plugins / plugin-data / plugin-logs）是 PluginManager 的内部实现细节，
    /// 调用方只需提供 app_data_dir。
    /// `builtin_component_ids` 为 plugin-host 冲突预检数据源：由调用方在 init_builtins
    /// 完成后从 CollectedBuiltins 提取传入（内置组件集合启动后稳定，注入一次即可）。
    pub fn init_host_manager(&self, app_data_dir: &Path, builtin_component_ids: HashSet<String>) {
        let plugins_dir = app_data_dir.join("plugins");
        let plugin_data_dir = app_data_dir.join("plugin-data");
        let plugin_log_dir = app_data_dir.join("plugin-logs");
        let hm = PluginHostManager::new(plugins_dir, plugin_data_dir, plugin_log_dir);
        hm.set_builtin_component_ids(builtin_component_ids);
        *self.host_manager.write() = Some(hm);
    }

    // ── 内部访问器 ──────────────────────────────────────────────

    pub(crate) fn plugin_event_tx(&self) -> PluginEventSender {
        self.plugin_event_tx
            .read()
            .as_ref()
            .cloned()
            .expect("PluginEventSender not set in PluginManager")
    }

    fn host_api(&self) -> Arc<HostApi> {
        self.host_api
            .read()
            .as_ref()
            .cloned()
            .expect("HostApi not set in PluginManager")
    }

    pub(crate) fn host_manager(&self) -> Arc<PluginHostManager> {
        self.host_manager
            .read()
            .as_ref()
            .cloned()
            .expect("PluginHostManager not set in PluginManager")
    }

    // ── 初始化 API ──────────────────────────────────────────────

    /// 收集所有内置组件（inventory 自动发现）。
    ///
    /// 参数 `session_dispatcher` 用于传递给 InventoryContext，供需要 SessionDispatcher 的组件工厂使用。
    /// 返回 `CollectedBuiltins`，调用方负责将各部分注册到 ConfigManager / SessionDispatcher，
    /// 并从中提取内置组件 id 集合传给 init_host_manager（plugin-host 冲突预检数据源）。
    pub(crate) fn init_builtins(
        &self,
        session_dispatcher: Arc<SessionDispatcher>,
    ) -> CollectedBuiltins {
        let host_api = self.host_api();
        let ctx = InventoryContext::new(host_api.clone(), session_dispatcher);
        builtin_registry::collect_all_builtin_entries(&ctx)
    }

    // ── 查询 API ────────────────────────────────────────────────

    /// 内置插件条目：以 PluginMetadata 为插件级数据源（插件管理页实体是插件而非组件），
    /// state 恒为 Running（内置编译在程序内运行），种类由 kind 字段表达，组件 id 即插件 id。
    pub fn builtin_plugin_info(
        &self,
        meta: &PluginMetadata,
        cm: &ConfigManager,
    ) -> InstalledPluginInfo {
        InstalledPluginInfo {
            plugin_id: meta.id.clone(),
            name: meta.name.clone(),
            version: meta.version.clone(),
            description: meta.description.clone(),
            author: meta.author.clone(),
            state: PluginRuntimeState::Running,
            enabled: cm.is_enabled(&meta.id),
            kind: PluginKind::Builtin,
            priority: meta.priority,
            component_ids: vec![meta.id.clone()],
        }
    }

    /// 单个插件条目（详情用）：内置直接构造，第三方按 id 直查 host 运行时（O(1)）。
    ///
    /// 调用方已持有 metadata（registry.get），此处不重复查询；
    /// priority 由构造方统一取插件声明优先级（内置 meta.priority / 第三方插件级元数据）。
    pub fn plugin_info(
        &self,
        plugin_id: &str,
        meta: &PluginMetadata,
        cm: &ConfigManager,
    ) -> Option<InstalledPluginInfo> {
        let hm = self.host_manager();
        let info = if meta.kind == PluginKind::Builtin {
            self.builtin_plugin_info(meta, cm)
        } else {
            hm.get_plugin_info(plugin_id, |a| {
                a.components.iter().all(|c| cm.is_enabled(c.component_id()))
            })?
        };
        Some(info)
    }

    /// 插件统一列表（IPC / CLI 共用）：第三方运行时信息 + 内置条目合并。
    ///
    /// priority 统一为插件声明优先级（PluginMetadata.priority）：
    /// 第三方由 plugin-host 按插件级元数据直接产出，此处仅补充内置条目。
    /// 内置判定以 metadata.kind（宿主管辖的运行属性，插件注册时确定）为准：
    /// 内置插件由代码构造为 Builtin，第三方由 plugin-host 加载时强制为 ThirdParty。
    pub fn list_plugins(
        &self,
        cm: &ConfigManager,
        dispatcher: &SessionDispatcher,
    ) -> Vec<InstalledPluginInfo> {
        let hm = self.host_manager();
        let mut list = hm.list_plugin_info(|a| {
            a.components.iter().all(|c| cm.is_enabled(c.component_id())) && !a.components.is_empty()
        });
        for meta in dispatcher.plugin_registry().get_all_metadata() {
            // 内置条目不在 hm 列表（内置不经过 plugin-host），按 id 防重后追加；
            // 第三方 priority 已由 plugin-host 按插件级元数据产出，无需覆盖。
            // 非内置且不在 hm：注册/加载时序异常（理论不可达），不产出行，避免误标。
            if meta.kind == PluginKind::Builtin && !list.iter().any(|i| i.plugin_id == meta.id) {
                list.push(self.builtin_plugin_info(&meta, cm));
            }
        }
        list.sort_by_key(|p| (p.priority, p.plugin_id.clone()));
        list
    }

    /// 返回插件安装根目录。
    pub fn plugins_dir(&self) -> PathBuf {
        self.host_manager().plugins_dir().to_path_buf()
    }

    /// 获取第三方插件的完整 manifest。
    pub fn get_manifest(&self, plugin_id: &str) -> Option<Manifest> {
        let hm = self.host_manager();
        let adapters = hm.plugins.get(plugin_id)?;
        Some(adapters.manifest.clone())
    }

    /// 获取第三方插件的日志文件最近 N 行。
    pub fn get_logs(
        &self,
        plugin_id: &str,
        tail_lines: usize,
    ) -> Result<Vec<String>, PluginManagerError> {
        let hm = self.host_manager();
        let log_file = hm.log_dir_root.join(format!("{}.log", plugin_id));

        let mut file = match std::fs::File::open(&log_file) {
            Ok(f) => f,
            Err(_) => return Ok(Vec::new()),
        };

        let file_size = file
            .metadata()
            .map_err(|e| PluginManagerError::Internal(e.to_string()))?
            .len();
        if file_size == 0 {
            return Ok(Vec::new());
        }

        // 从文件末尾读取最多 64KB，提取最后 tail_lines 行
        let tail_size = (64 * 1024).min(file_size);
        let mut buf = vec![0u8; tail_size as usize];
        file.seek(SeekFrom::End(-(tail_size as i64)))
            .map_err(|e| PluginManagerError::Internal(e.to_string()))?;
        file.read_exact(&mut buf)
            .map_err(|e| PluginManagerError::Internal(e.to_string()))?;

        let content = String::from_utf8_lossy(&buf);
        let lines: Vec<&str> = if tail_size < file_size {
            // 未从文件开头读取，跳过第一个不完整的行
            match content.find('\n') {
                Some(pos) => content[pos + 1..].lines().collect(),
                None => return Ok(vec![content.to_string()]),
            }
        } else {
            content.lines().collect()
        };

        let start = if lines.len() > tail_lines {
            lines.len() - tail_lines
        } else {
            0
        };
        Ok(lines[start..].iter().map(|s| s.to_string()).collect())
    }

    // ── zlplugin:// 协议处理 ────────────────────────────────────

    /// 处理 `zlplugin://` 协议请求，返回 (文件字节, MIME 类型)。
    ///
    /// URI 格式：`zlplugin://<plugin-id>/ui/<sub-path>`
    pub fn handle_zlplugin_uri(
        &self,
        uri: &str,
    ) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
        let handler = ZlpluginProtocolHandler::new(self.plugins_dir());
        handler.handle(uri)
    }

    // ── 第三方插件生命周期方法 ─────────────────────────────────

    /// 从 .zip 文件或目录安装插件。
    /// 成功时发送 `plugin-installed` 事件。
    pub async fn install(
        &self,
        source_path: &Path,
        app_handle: Arc<AppHandle>,
    ) -> Result<InstalledPluginInfo, PluginManagerError> {
        if !source_path.exists() {
            return Err(PluginManagerError::FileNotFound(format!(
                "File not found: {}",
                source_path.display()
            )));
        }

        let plugins_dir = self.plugins_dir();
        std::fs::create_dir_all(&plugins_dir)
            .map_err(|e| PluginManagerError::Internal(e.to_string()))?;

        let installed_dir = if source_path.is_dir() {
            self.installer()
                .install_from_dir(source_path)
                .map_err(install_error_to_manager)?
        } else if source_path.extension().map(|e| e == "zip").unwrap_or(false) {
            self.installer()
                .install_from_zip(source_path)
                .map_err(install_error_to_manager)?
        } else {
            return Err(PluginManagerError::UnsupportedFormat(
                "Unsupported file format. Use .zip or directory.".to_string(),
            ));
        };

        if let Err(e) = self
            .load_single_plugin(&installed_dir, app_handle.clone())
            .await
        {
            // 回滚：加载失败（如组件 id 冲突）时删除已落盘的插件目录，
            // 避免 UI 不可见、不可卸载、每次启动重试加载的残留目录。
            let _ = std::fs::remove_dir_all(&installed_dir);
            return Err(e);
        }

        let manifest_bytes =
            std::fs::read_to_string(installed_dir.join("manifest.toml")).map_err(|e| {
                PluginManagerError::Internal(format!(
                    "Failed to read manifest after install: {}",
                    e
                ))
            })?;
        let manifest: Manifest = toml::from_str(&manifest_bytes).map_err(|e| {
            PluginManagerError::Internal(format!("Failed to parse manifest: {}", e))
        })?;
        let plugin_id = &manifest.plugin.id;
        let hm = self.host_manager();
        let adapters = hm.plugins.get(plugin_id).ok_or_else(|| {
            PluginManagerError::PluginNotFound(format!(
                "Plugin not found after load: {}",
                plugin_id
            ))
        })?;

        // priority 与 list_plugins/plugin_info 统一取插件级元数据声明值（不再用组件最小优先级）。
        // 安装成功即子进程已加载运行，state 恒为 Running。
        Ok(InstalledPluginInfo {
            plugin_id: adapters.plugin_id.clone(),
            name: adapters.manifest.plugin.name.clone(),
            version: adapters.manifest.plugin.version.clone(),
            description: adapters.manifest.plugin.description.clone(),
            author: adapters.manifest.plugin.author.clone(),
            state: PluginRuntimeState::Running,
            enabled: !adapters.components.is_empty()
                && adapters.components.iter().all(|c| c.default_enabled()),
            kind: PluginKind::ThirdParty,
            priority: adapters.metadata.priority,
            component_ids: adapters
                .components
                .iter()
                .map(|c| c.component_id().to_string())
                .collect(),
        })
    }

    /// 重载第三方插件。
    /// 成功时发送 `plugin-installed` 事件。
    pub async fn reload(
        &self,
        plugin_id: &str,
        app_handle: Arc<AppHandle>,
    ) -> Result<(), PluginManagerError> {
        info!("Reloading plugin: {}", plugin_id);

        let hm = self.host_manager();

        let adapters = hm
            .plugins
            .get(plugin_id)
            .ok_or_else(|| {
                PluginManagerError::PluginNotFound(format!("Plugin not found: {}", plugin_id))
            })?
            .clone();
        let plugin_dir = hm.plugins_dir().join(plugin_id);

        self.plugin_event_tx()
            .send(PluginRuntimeEvent::PluginUnloaded(adapters.clone()))
            .ok();

        if let Err(e) = hm.unload(plugin_id).await {
            error!("Unload during reload failed: {}", e);
        }

        // 预检数据源为 plugin-host 内部（已加载插件 + 内置组件），
        // unload 已移除自身组件，无需豁免集。
        self.load_single_plugin(&plugin_dir, app_handle)
            .await
            .map_err(|e| match e {
                PluginManagerError::ComponentIdCollision(id) => {
                    PluginManagerError::ComponentIdCollision(id)
                }
                other => PluginManagerError::Internal(format!("Reload failed: {}", other)),
            })?;

        info!("Plugin {} reloaded successfully", plugin_id);
        Ok(())
    }

    /// 卸载第三方插件。
    /// 成功时发送 `plugin-uninstalled` 事件。
    pub async fn uninstall(
        &self,
        plugin_id: &str,
        app_handle: Arc<AppHandle>,
    ) -> Result<(), PluginManagerError> {
        info!("Uninstalling plugin: {}", plugin_id);

        let hm = self.host_manager();

        if let Some(adapters) = hm.plugins.get(plugin_id) {
            let adapters = adapters.clone();
            self.plugin_event_tx()
                .send(PluginRuntimeEvent::PluginUnloaded(adapters))
                .ok();
        }

        if let Err(e) = hm.unload(plugin_id).await {
            error!("Unload during uninstall failed: {}", e);
        }

        let plugin_dir = hm.plugins_dir().join(plugin_id);
        if plugin_dir.exists() {
            std::fs::remove_dir_all(&plugin_dir).map_err(|e| {
                PluginManagerError::Internal(format!("Cannot remove plugin dir: {}", e))
            })?;
        }

        self.i18n_manager().unregister_plugin_catalog(plugin_id);

        self.host_api().unregister(plugin_id);

        let _ = app_handle.emit(
            "plugin-uninstalled",
            serde_json::json!({
                "pluginId": plugin_id,
            }),
        );

        info!("Plugin {} uninstalled successfully", plugin_id);
        Ok(())
    }

    /// 扫描并加载所有第三方插件。
    ///
    /// 每个插件的注册通过 PluginRuntimeEvent 广播通道（PM → CM）完成，
    /// CM 收到后注册 Configurable 并转发 ConfigEvent 到 SessionDispatcher。
    pub async fn load_all_third_party(&self, app_handle: Arc<AppHandle>) {
        let dirs = self.installer().scan_plugins_dir();

        if dirs.is_empty() {
            info!(
                "No third-party plugins found in {}",
                self.plugins_dir().display()
            );
            return;
        }

        info!("Found {} third-party plugin(s)", dirs.len());

        for dir in &dirs {
            if let Err(e) = self.load_single_plugin(dir, app_handle.clone()).await {
                error!("Failed to load plugin from {}: {}", dir.display(), e);
            }
        }
    }

    // ── 内部：第三方插件加载 ─────────────────────────────────────

    /// 加载单个第三方插件。
    ///
    /// 通过 PluginRuntimeEvent::PluginLoaded 广播通知 CM：
    /// CM 收到后注册 Configurable + 转发 ConfigEvent::PluginRegistered 到 SR。
    /// 崩溃即解注册回调（on_crash）在崩溃发生时以旧注册包解注册 CM/SR 并清理 HostApi。
    /// 成功时发送 `plugin-installed` Tauri 事件。
    async fn load_single_plugin(
        &self,
        plugin_dir: &Path,
        app_handle: Arc<AppHandle>,
    ) -> Result<(), PluginManagerError> {
        let host_manager = self.host_manager();
        let host_api = self.host_api();

        let manifest_path = plugin_dir.join("manifest.toml");
        let manifest_bytes = std::fs::read_to_string(&manifest_path)
            .map_err(|e| PluginManagerError::Internal(format!("read manifest: {}", e)))?;
        let manifest: Manifest = toml::from_str(&manifest_bytes)
            .map_err(|e| PluginManagerError::Internal(format!("parse manifest: {}", e)))?;
        let plugin_id = manifest.plugin.id.clone();

        // 重新加载/重复加载时先移除旧 catalog，避免残留
        self.i18n_manager().unregister_plugin_catalog(&plugin_id);

        let _handle = host_api.register(&plugin_id, PluginSdkConfig::default());

        let handler: Arc<dyn HostCallHandler> = Arc::new(TauriHostCallHandler {
            host_api: host_api.clone(),
            plugin_id: plugin_id.clone(),
            app_handle: Some(app_handle.clone()),
            i18n: self.i18n_manager(),
        });

        let on_restart = self.make_restart_callback(plugin_id.clone());
        let on_crash = self.make_crash_callback(plugin_id.clone());

        let registered = host_manager
            .load(
                plugin_dir,
                handler,
                on_restart,
                on_crash,
                0, // 初次加载，无先前重启记录
                &self.i18n_manager().current_language(),
            )
            .await
            .map_err(|e| match e {
                PluginLoadError::ComponentIdCollision { component_id, .. } => {
                    PluginManagerError::ComponentIdCollision(component_id)
                }
                other => PluginManagerError::Internal(format!("plugin-host load: {}", other)),
            })?;

        if let Err(e) = self
            .plugin_event_tx()
            .send(PluginRuntimeEvent::PluginLoaded(registered.clone()))
        {
            error!("广播 PluginLoaded 失败（无接收者？）: {}", e);
        }

        // 注册插件翻译目录（<plugin_dir>/i18n/<lang>.json），供前端经 IPC 合并
        self.i18n_manager()
            .register_plugin_catalog(&plugin_id, plugin_dir);

        info!("Loaded third-party plugin: {}", plugin_id);

        let _ = app_handle.emit(
            "plugin-installed",
            serde_json::json!({
                "pluginId": plugin_id,
                "name": manifest.plugin.name,
                "version": manifest.plugin.version,
            }),
        );

        Ok(())
    }

    // ── 崩溃恢复 ───────────────────────────────────────────────

    /// 为崩溃恢复场景构建 restart 回调。
    ///
    /// 返回的闭包接收 `PluginRegistration` 并返回一个 future，
    /// watchdog 会 `.await` 该 future 以完成重新注册。
    /// 通过 PluginRuntimeEvent 管道通知 CM 解注册旧组件 + 注册新组件。
    /// 为崩溃恢复场景构建 restart 回调。
    ///
    /// 返回的闭包接收重启后的 `PluginRegistration` 并返回一个 future，
    /// watchdog 会 `.await` 该 future 以完成重新注册。
    /// 旧组件已在崩溃处理第一步经 on_crash 解注册，此处只注册新组件。
    fn make_restart_callback(&self, plugin_id: String) -> RestartCallback {
        let tx = self.plugin_event_tx();

        Arc::new(move |new_adapters: PluginRegistration| {
            let tx = tx.clone();
            let pid = plugin_id.clone();

            Box::pin(async move {
                tx.send(PluginRuntimeEvent::PluginLoaded(new_adapters.clone()))
                    .ok();
                info!(
                    "Restarted third-party plugin: {} (adapters re-registered)",
                    pid
                );
            })
        })
    }

    /// 为崩溃恢复构建「崩溃即解注册」回调。
    ///
    /// 崩溃处理第一步以旧注册包调用：解注册 CM/SR 组件（PluginUnloaded 事件）
    /// 并清理 HostApi 句柄。无论后续重启成败，组件都不再残留。
    fn make_crash_callback(&self, plugin_id: String) -> CrashCallback {
        let tx = self.plugin_event_tx();
        let host_api = self.host_api();
        let host_manager = self.host_manager();
        let i18n = self.i18n_manager();

        Arc::new(move |prev: PluginRegistration| {
            tx.send(PluginRuntimeEvent::PluginUnloaded(prev)).ok();
            host_api.unregister(&plugin_id);
            // 崩溃时同步最新语言到重启上下文：崩溃重启的 initialize 握手
            // 携带实时 locale（而非首次加载时的快照）。
            host_manager.update_locale(&plugin_id, &i18n.current_language());
            info!(
                "Plugin {} crashed — stale components unregistered",
                plugin_id
            );
        })
    }

    // ── 安装器（委托至 PluginInstaller） ────────────────────────────

    /// 返回一个临时安装器实例（创建轻量，每次从 PluginManager 的 plugins_dir 新鲜构造）。
    fn installer(&self) -> PluginInstaller {
        PluginInstaller::new(self.plugins_dir())
    }
}

/// 将安装器错误转换为 PluginManagerError，保留「已安装」语义供 IPC 层区分。
fn install_error_to_manager(e: InstallError) -> PluginManagerError {
    match e {
        InstallError::AlreadyInstalled(id) => PluginManagerError::AlreadyInstalled(id),
        other => PluginManagerError::Internal(other.to_string()),
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
