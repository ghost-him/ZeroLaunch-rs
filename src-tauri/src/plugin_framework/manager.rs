//! PluginManager — 插件身份与生命周期的统一入口。
//!
//! PluginManager 是「有哪些插件」的唯一权威来源。
//! 它管理 PluginInfo[] 统一视图，连接内置组件提供者和第三方插件提供者。
//!
//! 注册/解注册通过 PluginRuntimeEvent 广播通道（PM → CM 解耦管道）完成，
//! ConfigManager 处理配置侧（Configurable）+ 转发 ConfigEvent 到 SessionRouter。

use dashmap::DashMap;
use parking_lot::RwLock;
use std::fmt;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tracing::{error, info};
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_api::host::PluginSdkConfig;
use zerolaunch_plugin_host::host_dispatch::HostCallHandler;
use zerolaunch_plugin_host::manager::{
    InstalledPluginInfo, PluginHostManager, PluginRegistration, RestartCallback,
};
use zerolaunch_plugin_protocol::Manifest;

use crate::core::config::event::{PluginEventSender, PluginRuntimeEvent};
use crate::plugin_framework::builtin_registry;
use crate::plugin_framework::builtin_registry::{CollectedBuiltins, InventoryContext};
use crate::plugin_framework::zlplugin_protocol::ZlpluginProtocolHandler;
use crate::sdk::HostApi;

use super::builtin;
use super::host_handler::TauriHostCallHandler;
use super::plugin_info::{PluginInfo, PluginStatus};
use super::plugin_installer::PluginInstaller;

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
    /// 常规内部错误
    Internal(String),
}

impl fmt::Display for PluginManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginManagerError::PluginNotFound(msg) => write!(f, "插件未找到: {}", msg),
            PluginManagerError::FileNotFound(msg) => write!(f, "文件未找到: {}", msg),
            PluginManagerError::UnsupportedFormat(msg) => write!(f, "不支持的文件格式: {}", msg),
            PluginManagerError::Internal(msg) => write!(f, "插件管理器内部错误: {}", msg),
        }
    }
}

impl std::error::Error for PluginManagerError {}

/// 插件管理器，统一管理内置组件和第三方插件。
///
/// 所有方法使用 `&self`（内部 RwLock 实现可变性），
/// 与 SessionRouter / ConfigManager 的模式一致。
///
/// 不再直接依赖 ConfigManager，通过 PluginRuntimeEvent 广播通道与 CM 通信。
pub struct PluginManager {
    /// 内置组件信息缓存
    builtin_infos: RwLock<Vec<PluginInfo>>,
    /// 第三方插件信息缓存
    third_party_infos: RwLock<Vec<PluginInfo>>,
    /// PluginRuntimeEvent 通道发送端（PM → CM 解耦管道）
    plugin_event_tx: RwLock<Option<PluginEventSender>>,
    /// HostApi 引用
    host_api: RwLock<Option<Arc<HostApi>>>,
    /// PluginHostManager（内部构造，管理子进程生命周期）
    host_manager: RwLock<Option<Arc<PluginHostManager>>>,
    /// 第三方插件 adapters 缓存（用于崩溃恢复时解注册已失效的旧适配器），按 plugin_id 索引。
    /// 使用 DashMap 避免 RwLock 守卫跨 .await 的 !Send 问题。
    adapters_cache: Arc<DashMap<String, PluginRegistration>>,
}

impl PluginManager {
    /// 创建 PluginManager 实例。
    pub fn new() -> Self {
        Self {
            builtin_infos: RwLock::new(Vec::new()),
            third_party_infos: RwLock::new(Vec::new()),
            plugin_event_tx: RwLock::new(None),
            host_api: RwLock::new(None),
            host_manager: RwLock::new(None),
            adapters_cache: Arc::new(DashMap::new()),
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

    /// 初始化 PluginHostManager（PluginManager 内部构造，不从外部注入）。
    /// 子目录命名（plugins / plugin-data / plugin-logs）是 PluginManager 的内部实现细节，
    /// 调用方只需提供 app_data_dir。
    pub fn init_host_manager(&self, app_data_dir: &Path) {
        let plugins_dir = app_data_dir.join("plugins");
        let plugin_data_dir = app_data_dir.join("plugin-data");
        let plugin_log_dir = app_data_dir.join("plugin-logs");
        let hm = Arc::new(PluginHostManager::new(
            plugins_dir,
            plugin_data_dir,
            plugin_log_dir,
        ));
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

    /// 收集所有内置组件、创建 PluginInfo 条目。
    ///
    /// 返回 `CollectedBuiltins`，调用方负责将各部分注册到 ConfigManager / SessionRouter。
    pub(crate) fn init_builtins(&self) -> CollectedBuiltins {
        let host_api = self.host_api();
        let ctx = InventoryContext::new(host_api.clone());
        let collected = builtin_registry::collect_all_builtin_entries(&ctx);

        // 为所有内置组件创建 PluginInfo 条目
        let mut infos = Vec::new();
        for (c, _) in &collected.executors {
            infos.push(builtin::make_builtin_info(c, c.default_enabled()));
        }
        for (c, _) in &collected.data_sources {
            infos.push(builtin::make_builtin_info(c, c.default_enabled()));
        }
        for (c, _) in &collected.keyword_optimizers {
            infos.push(builtin::make_builtin_info(c, c.default_enabled()));
        }
        for (c, _) in &collected.search_engines {
            infos.push(builtin::make_builtin_info(c, c.default_enabled()));
        }
        for (c, _) in &collected.score_boosters {
            infos.push(builtin::make_builtin_info(c, c.default_enabled()));
        }
        for (c, _) in &collected.plugins {
            infos.push(builtin::make_builtin_info(c, c.default_enabled()));
        }
        for c in &collected.config_components {
            infos.push(builtin::make_builtin_info(c, c.default_enabled()));
        }

        self.builtin_infos.write().append(&mut infos);

        collected
    }

    // ── 查询 API ────────────────────────────────────────────────

    /// 返回所有插件的统一列表（内置 + 第三方）。
    pub fn list_all(&self) -> Vec<PluginInfo> {
        let mut all = Vec::new();
        all.extend(self.builtin_infos.read().iter().cloned());
        all.extend(self.third_party_infos.read().iter().cloned());
        all.sort_by_key(|p| (p.priority, p.id.clone()));
        all
    }

    /// 按 plugin_id 查找插件信息。
    pub fn get(&self, plugin_id: &str) -> Option<PluginInfo> {
        self.builtin_infos
            .read()
            .iter()
            .find(|i| i.id == plugin_id)
            .cloned()
            .or_else(|| {
                self.third_party_infos
                    .read()
                    .iter()
                    .find(|i| i.id == plugin_id)
                    .cloned()
            })
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
                .map_err(|e| PluginManagerError::Internal(e.to_string()))?
        } else if source_path.extension().map(|e| e == "zip").unwrap_or(false) {
            self.installer()
                .install_from_zip(source_path)
                .map_err(|e| PluginManagerError::Internal(e.to_string()))?
        } else {
            return Err(PluginManagerError::UnsupportedFormat(
                "Unsupported file format. Use .zip or directory.".to_string(),
            ));
        };

        self.load_single_plugin(&installed_dir, app_handle.clone())
            .await?;

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

        let priority = adapters.compute_priority();

        Ok(InstalledPluginInfo {
            plugin_id: adapters.plugin_id.clone(),
            name: adapters.manifest.plugin.name.clone(),
            version: adapters.manifest.plugin.version.clone(),
            description: adapters.manifest.plugin.description.clone(),
            author: adapters.manifest.plugin.author.clone(),
            state: "running".to_string(),
            enabled: !adapters.configurables.is_empty()
                && adapters.configurables.iter().all(|c| c.default_enabled()),
            priority,
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
        self.forget_adapters(plugin_id);

        if let Err(e) = hm.unload(plugin_id).await {
            error!("Unload during reload failed: {}", e);
        }

        self.load_single_plugin(&plugin_dir, app_handle)
            .await
            .map_err(|e| PluginManagerError::Internal(format!("Reload failed: {}", e)))?;

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
        self.forget_adapters(plugin_id);
        self.remove_third_party_info(plugin_id);

        if let Err(e) = hm.unload(plugin_id).await {
            error!("Unload during uninstall failed: {}", e);
        }

        let plugin_dir = hm.plugins_dir().join(plugin_id);
        if plugin_dir.exists() {
            std::fs::remove_dir_all(&plugin_dir).map_err(|e| {
                PluginManagerError::Internal(format!("Cannot remove plugin dir: {}", e))
            })?;
        }

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
    /// CM 收到后注册 Configurable 并转发 ConfigEvent 到 SessionRouter。
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
    /// 崩溃恢复回调通过 adapters_cache 解注册旧组件。
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

        let _handle = host_api.register(&plugin_id, PluginSdkConfig::default());

        let handler: Arc<dyn HostCallHandler> = Arc::new(TauriHostCallHandler {
            host_api: host_api.clone(),
            plugin_id: plugin_id.clone(),
            app_handle: Some(app_handle.clone()),
        });

        let on_restart = self.make_restart_callback(plugin_id.clone());

        let registered = host_manager
            .load(plugin_dir, handler, on_restart)
            .await
            .map_err(|e| PluginManagerError::Internal(format!("plugin-host load: {}", e)))?;

        self.plugin_event_tx()
            .send(PluginRuntimeEvent::PluginLoaded(registered.clone()))
            .ok();
        self.cache_adapters(&plugin_id, registered.clone());

        let enabled = !registered.configurables.is_empty()
            && registered.configurables.iter().all(|c| c.default_enabled());
        let component_count = registered.configurables.len();
        let priority = registered.compute_priority();
        self.register_third_party_info(PluginInfo::third_party(
            plugin_id.clone(),
            manifest.plugin.name.clone(),
            manifest.plugin.version.clone(),
            manifest.plugin.description.clone(),
            manifest.plugin.author.clone(),
            component_count,
            enabled,
            true,
            priority,
        ));

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
    fn make_restart_callback(&self, plugin_id: String) -> RestartCallback {
        let tx = self.plugin_event_tx();
        let adapters_cache = self.adapters_cache.clone();

        Arc::new(move |new_adapters: PluginRegistration| {
            let tx = tx.clone();
            let adapters_cache = adapters_cache.clone();
            let pid = plugin_id.clone();

            Box::pin(async move {
                // 解注册旧适配器（如果存在缓存）
                if let Some((_, prev)) = adapters_cache.remove(&pid) {
                    tx.send(PluginRuntimeEvent::PluginUnloaded(prev)).ok();
                }

                // 注册新适配器
                tx.send(PluginRuntimeEvent::PluginLoaded(new_adapters.clone()))
                    .ok();

                // 更新缓存为新适配器
                adapters_cache.insert(pid.clone(), new_adapters);

                info!(
                    "Restarted third-party plugin: {} (adapters re-registered)",
                    pid
                );
            })
        })
    }

    /// 存储插件的 adapters 快照，供崩溃恢复时解注册。
    pub fn cache_adapters(&self, plugin_id: &str, adapters: PluginRegistration) {
        self.adapters_cache.insert(plugin_id.to_string(), adapters);
    }

    /// 清除插件的 adapters 缓存（卸载时调用）。
    pub fn forget_adapters(&self, plugin_id: &str) {
        self.adapters_cache.remove(plugin_id);
    }

    // ── 信息管理 ───────────────────────────────────────────────

    pub fn register_third_party_info(&self, info: PluginInfo) {
        self.third_party_infos.write().push(info);
    }

    pub fn remove_third_party_info(&self, plugin_id: &str) {
        self.third_party_infos.write().retain(|i| i.id != plugin_id);
    }

    pub fn update_third_party_status(
        &self,
        plugin_id: &str,
        is_running: bool,
        error_msg: Option<String>,
    ) {
        if let Some(info) = self
            .third_party_infos
            .write()
            .iter_mut()
            .find(|i| i.id == plugin_id)
        {
            info.status = if is_running {
                PluginStatus::Active
            } else if let Some(msg) = error_msg {
                PluginStatus::Error(msg)
            } else {
                PluginStatus::Inactive
            };
        }
    }

    pub fn list_third_party(&self) -> Vec<PluginInfo> {
        self.third_party_infos.read().clone()
    }

    pub fn register_builtin_info(&self, info: PluginInfo) {
        self.builtin_infos.write().push(info);
    }

    pub fn list_builtins(&self) -> Vec<PluginInfo> {
        self.builtin_infos.read().clone()
    }

    pub fn update_builtin_enabled(&self, component_id: &str, enabled: bool) {
        if let Some(info) = self
            .builtin_infos
            .write()
            .iter_mut()
            .find(|i| i.id == component_id)
        {
            info.enabled = enabled;
        }
    }

    // ── 安装器（委托至 PluginInstaller） ────────────────────────────

    /// 返回一个临时安装器实例（创建轻量，每次从 PluginManager 的 plugins_dir 新鲜构造）。
    fn installer(&self) -> PluginInstaller {
        PluginInstaller::new(self.plugins_dir())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
