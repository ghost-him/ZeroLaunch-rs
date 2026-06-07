//! PluginManager — 插件身份与生命周期的统一入口。
//!
//! PluginManager 是「有哪些插件」的唯一权威来源。
//! 它管理 PluginInfo[] 统一视图，连接内置组件提供者和第三方插件提供者，
//! 并将组件注册到 ConfigManager / SessionRouter（这两个管理器不知道「插件」概念）。

use parking_lot::RwLock;
use std::path::Path;
use std::sync::Arc;
use tauri::Emitter;
use tracing::{error, info};
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_host::manager::{InstalledPluginInfo, PluginHostManager, RegisteredAdapters};

use crate::core::config::ConfigManager;
use crate::plugin_system::SessionRouter;
use crate::sdk::HostApi;

use super::builtin::{BuiltinInitResult, BuiltinProvider};
use super::types::{PluginInfo, PluginStatus};

/// 插件管理器，统一管理内置组件和第三方插件。
///
/// 所有方法使用 `&self`（内部 RwLock 实现可变性），
/// 与 SessionRouter / ConfigManager 的模式一致。
pub struct PluginManager {
    /// 内置组件信息缓存
    builtin_infos: RwLock<Vec<PluginInfo>>,
    /// 第三方插件信息缓存
    third_party_infos: RwLock<Vec<PluginInfo>>,
    /// BuiltinProvider（inventory 收集 + 注册）
    builtin_provider: BuiltinProvider,
    /// ConfigManager 弱引用
    config_manager: RwLock<Option<Arc<ConfigManager>>>,
    /// SessionRouter 弱引用
    session_router: RwLock<Option<Arc<SessionRouter>>>,
    /// PluginHostManager 弱引用
    host_manager: RwLock<Option<Arc<PluginHostManager>>>,
    /// 第三方插件旧的 adapters 缓存（用于崩溃恢复时解注册），按 plugin_id 索引
    old_adapters: RwLock<std::collections::HashMap<String, RegisteredAdapters>>,
}

impl PluginManager {
    /// 创建 PluginManager 实例。
    pub fn new() -> Self {
        Self {
            builtin_infos: RwLock::new(Vec::new()),
            third_party_infos: RwLock::new(Vec::new()),
            builtin_provider: BuiltinProvider::new(),
            config_manager: RwLock::new(None),
            session_router: RwLock::new(None),
            host_manager: RwLock::new(None),
            old_adapters: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// 设置 ConfigManager 引用（在 init_app_state 中调用一次）。
    pub fn set_config_manager(&self, cm: Arc<ConfigManager>) {
        *self.config_manager.write() = Some(cm);
    }

    /// 设置 SessionRouter 引用（在 init_app_state 中调用一次）。
    pub fn set_session_router(&self, sr: Arc<SessionRouter>) {
        *self.session_router.write() = Some(sr);
    }

    /// 设置 PluginHostManager 引用（在 init_app_state 中调用一次）。
    pub fn set_host_manager(&self, hm: Arc<PluginHostManager>) {
        *self.host_manager.write() = Some(hm);
    }

    /// 获取 ConfigManager 引用（panic 如果未初始化）。
    pub fn get_config_manager(&self) -> Arc<ConfigManager> {
        self.config_manager
            .read()
            .as_ref()
            .cloned()
            .expect("ConfigManager not set in PluginManager")
    }

    /// 获取 SessionRouter 引用（panic 如果未初始化）。
    pub fn get_session_router(&self) -> Arc<SessionRouter> {
        self.session_router
            .read()
            .as_ref()
            .cloned()
            .expect("SessionRouter not set in PluginManager")
    }

    /// 获取 PluginHostManager 引用（panic 如果未初始化）。
    fn host_manager(&self) -> Arc<PluginHostManager> {
        self.host_manager
            .read()
            .as_ref()
            .cloned()
            .expect("PluginHostManager not set in PluginManager")
    }

    // ── 初始化 API ──────────────────────────────────────────────

    /// 初始化所有内置组件。
    pub fn init_builtins(&self, host_api: &Arc<HostApi>) -> BuiltinInitResult {
        let mut infos = Vec::new();
        let result = self.builtin_provider.init(
            host_api,
            &self.get_config_manager(),
            &self.get_session_router(),
            &mut |info| infos.push(info),
        );
        self.builtin_infos.write().append(&mut infos);
        result
    }

    // ── 查询 API ────────────────────────────────────────────────

    /// 返回所有插件的统一列表（内置 + 第三方）。
    pub fn list_all(&self) -> Vec<PluginInfo> {
        let mut all = Vec::new();
        all.extend(self.builtin_infos.read().iter().cloned());
        all.extend(self.third_party_infos.read().iter().cloned());
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

    // ── 第三方插件生命周期方法 ─────────────────────────────────
    //
    // install / reload / uninstall 是完整的编排方法。
    // commands 层只需调这些方法，不再手动编排多步骤。

    /// Install a plugin from a .zip file or directory.
    /// Emits `plugin-installed` on success.
    pub async fn install(
        &self,
        source_path: &Path,
        host_api: Arc<HostApi>,
        app_handle: tauri::AppHandle,
    ) -> Result<InstalledPluginInfo, String> {
        if !source_path.exists() {
            return Err(format!("File not found: {}", source_path.display()));
        }

        let hm = self.host_manager();
        let plugins_dir = hm.plugins_dir();
        std::fs::create_dir_all(plugins_dir).map_err(|e| e.to_string())?;

        let installed_dir = if source_path.is_dir() {
            super::third_party::installer::install_from_dir(source_path, plugins_dir)
                .map_err(|e| e.to_string())?
        } else if source_path.extension().map(|e| e == "zip").unwrap_or(false) {
            super::third_party::installer::install_from_zip(source_path, plugins_dir)
                .map_err(|e| e.to_string())?
        } else {
            return Err("Unsupported file format. Use .zip or directory.".into());
        };

        // Load the newly installed plugin
        super::third_party::loader::load_plugin(
            &installed_dir,
            self,
            &hm,
            host_api,
            app_handle.clone(),
        )
        .await?;

        // Read plugin_id from manifest (authoritative) — not from directory name
        let manifest_bytes = std::fs::read_to_string(installed_dir.join("manifest.toml"))
            .map_err(|e| format!("Failed to read manifest after install: {}", e))?;
        let manifest: zerolaunch_plugin_protocol::manifest::Manifest =
            toml::from_str(&manifest_bytes)
                .map_err(|e| format!("Failed to parse manifest: {}", e))?;
        let plugin_id = &manifest.plugin.id;
        let adapters = hm
            .adapters
            .get(plugin_id)
            .ok_or(format!("Plugin not found after load: {}", plugin_id))?;

        Ok(InstalledPluginInfo {
            plugin_id: adapters.plugin_id.clone(),
            name: adapters.manifest.plugin.name.clone(),
            version: adapters.manifest.plugin.version.clone(),
            description: adapters.manifest.plugin.description.clone(),
            author: adapters.manifest.plugin.author.clone(),
            state: "running".to_string(),
            enabled: true,
        })
    }

    /// Reload a third-party plugin.
    /// Emits `plugin-installed` on success.
    pub async fn reload(
        &self,
        plugin_id: &str,
        host_api: Arc<HostApi>,
        app_handle: tauri::AppHandle,
    ) -> Result<(), String> {
        info!("Reloading plugin: {}", plugin_id);

        let hm = self.host_manager();

        // Resolve plugin directory
        let adapters = hm
            .adapters
            .get(plugin_id)
            .ok_or(format!("Plugin not found: {}", plugin_id))?;
        let plugin_dir = hm.plugins_dir().join(plugin_id);

        // Step 1: Unregister
        self.unregister_adapters(&adapters).await;
        self.forget_adapters(plugin_id);

        // Step 2: Unload subprocess
        if let Err(e) = hm.unload(plugin_id).await {
            error!("Unload during reload failed: {}", e);
        }

        // Step 3: Reload (emits plugin-installed internally)
        super::third_party::loader::load_plugin(&plugin_dir, self, &hm, host_api, app_handle)
            .await
            .map_err(|e| format!("Reload failed: {}", e))?;

        info!("Plugin {} reloaded successfully", plugin_id);
        Ok(())
    }

    /// Uninstall a third-party plugin.
    /// Emits `plugin-uninstalled` on success.
    pub async fn uninstall(
        &self,
        plugin_id: &str,
        host_api: Arc<HostApi>,
        app_handle: tauri::AppHandle,
    ) -> Result<(), String> {
        info!("Uninstalling plugin: {}", plugin_id);

        let hm = self.host_manager();

        // Step 1: Unregister
        if let Some(adapters) = hm.adapters.get(plugin_id) {
            self.unregister_adapters(&adapters).await;
        }
        self.forget_adapters(plugin_id);
        self.remove_third_party_info(plugin_id);

        // Step 2: Unload subprocess
        if let Err(e) = hm.unload(plugin_id).await {
            error!("Unload during uninstall failed: {}", e);
        }

        // Step 3: Remove from filesystem
        let plugin_dir = hm.plugins_dir().join(plugin_id);
        if plugin_dir.exists() {
            std::fs::remove_dir_all(&plugin_dir)
                .map_err(|e| format!("Cannot remove plugin dir: {}", e))?;
        }

        // Step 4: Notify host API
        host_api.unregister(plugin_id);

        // Step 5: Notify frontend
        let _ = app_handle.emit(
            "plugin-uninstalled",
            serde_json::json!({
                "pluginId": plugin_id,
            }),
        );

        info!("Plugin {} uninstalled successfully", plugin_id);
        Ok(())
    }

    // ── 适配器注册/解注册（第三方插件内部编排用） ─────────────

    /// 将第三方插件的 RegisteredAdapters 注册到 ConfigManager / SessionRouter。
    pub async fn register_adapters(&self, adapters: &RegisteredAdapters) {
        let cm = self.get_config_manager();
        let sr = self.get_session_router();
        for c in &adapters.configurables {
            cm.register(c.clone());
        }
        for ds in &adapters.data_sources {
            sr.register_data_source(ds.clone()).await;
        }
        for ex in &adapters.executors {
            sr.register_executor(ex.clone());
        }
        if let Some(p) = &adapters.plugin {
            sr.register_remote_plugin(p.clone());
        }
    }

    /// 将第三方插件的 RegisteredAdapters 从 ConfigManager / SessionRouter 解注册。
    pub async fn unregister_adapters(&self, adapters: &RegisteredAdapters) {
        let cm = self.get_config_manager();
        let sr = self.get_session_router();
        sr.unregister_plugin(&adapters.plugin_id);
        for ds in &adapters.data_sources {
            sr.unregister_data_source(&ds.component_id).await;
        }
        for ex in &adapters.executors {
            sr.unregister_executor(&ex.component_id);
        }
        for c in &adapters.configurables {
            cm.unregister(&c.component_id);
        }
    }

    /// 为崩溃恢复场景构建 restart 回调。
    pub fn make_restart_callback(
        &self,
        plugin_id: String,
    ) -> Arc<dyn Fn(RegisteredAdapters) + Send + Sync> {
        let cm = self.get_config_manager();
        let sr = self.get_session_router();
        let old_adapters =
            parking_lot::Mutex::new(self.old_adapters.read().get(&plugin_id).cloned());
        let plugin_id_for_log = plugin_id.clone();

        Arc::new(move |new_adapters: RegisteredAdapters| {
            let handle = tokio::runtime::Handle::current();

            let mut old = old_adapters.lock();
            if let Some(prev) = old.take() {
                handle.block_on(async {
                    for ds in &prev.data_sources {
                        sr.unregister_data_source(&ds.component_id).await;
                    }
                });
                for ex in &prev.executors {
                    sr.unregister_executor(&ex.component_id);
                }
                if prev.plugin.is_some() {
                    sr.unregister_plugin(&prev.plugin_id);
                }
                for c in &prev.configurables {
                    cm.unregister(&c.component_id);
                }
            }

            for c in &new_adapters.configurables {
                cm.register(c.clone());
            }
            handle.block_on(async {
                for ds in &new_adapters.data_sources {
                    sr.register_data_source(ds.clone()).await;
                }
            });
            for ex in &new_adapters.executors {
                sr.register_executor(ex.clone());
            }
            if let Some(p) = &new_adapters.plugin {
                sr.register_remote_plugin(p.clone());
            }

            *old = Some(new_adapters);

            info!(
                "Restarted third-party plugin: {} (adapters re-registered)",
                plugin_id_for_log
            );
        })
    }

    /// 存储插件的 adapters 快照，供崩溃恢复时解注册。
    pub fn cache_adapters(&self, plugin_id: &str, adapters: RegisteredAdapters) {
        self.old_adapters
            .write()
            .insert(plugin_id.to_string(), adapters);
    }

    /// 清除插件的 adapters 缓存（卸载时调用）。
    pub fn forget_adapters(&self, plugin_id: &str) {
        self.old_adapters.write().remove(plugin_id);
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

    /// Return detailed info for all third-party plugins (frontend-facing DTOs).
    pub fn list_third_party_details(&self) -> Vec<InstalledPluginInfo> {
        let hm = self.host_manager();
        let cm = self.get_config_manager();
        hm.adapters
            .iter()
            .map(|entry| {
                let adapters = entry.value();
                let process_state = hm
                    .processes
                    .get(&adapters.plugin_id)
                    .map(|p| format!("{:?}", *p.state.read()))
                    .unwrap_or_else(|| "unknown".to_string());
                let enabled = adapters
                    .configurables
                    .iter()
                    .all(|c| cm.is_enabled(c.component_id()))
                    && !adapters.configurables.is_empty();
                InstalledPluginInfo {
                    plugin_id: adapters.plugin_id.clone(),
                    name: adapters.manifest.plugin.name.clone(),
                    version: adapters.manifest.plugin.version.clone(),
                    description: adapters.manifest.plugin.description.clone(),
                    author: adapters.manifest.plugin.author.clone(),
                    state: process_state,
                    enabled,
                }
            })
            .collect()
    }

    /// Enable or disable all components of a plugin.
    ///
    /// For third-party plugins, this iterates the plugin's adapters.
    /// Falls back to treating plugin_id as a single component id (supports builtin components).
    pub fn set_enabled(&self, plugin_id: &str, enabled: bool) -> Result<(), String> {
        let cm = self.get_config_manager();
        let hm = self.host_manager();

        if let Some(adapters) = hm.adapters.get(plugin_id) {
            for c in &adapters.configurables {
                cm.set_enabled(c.component_id(), enabled)
                    .map_err(|e| e.to_string())?;
            }
            return Ok(());
        }

        // Fallback: treat plugin_id itself as the component id (covers builtin components)
        cm.set_enabled(plugin_id, enabled)
            .map_err(|e| e.to_string())
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
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
