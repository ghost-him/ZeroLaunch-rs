//! PluginManager — 插件身份与生命周期的统一入口。
//!
//! PluginManager 是「有哪些插件」的唯一权威来源。
//! 它管理 PluginInfo[] 统一视图，连接内置组件提供者和第三方插件提供者，
//! 并将组件注册到 ConfigManager / SessionRouter（这两个管理器不知道「插件」概念）。
//!
//! 同时负责：第三方插件子进程生命周期、zip 安装/卸载、崩溃恢复、
//! zlplugin:// 协议处理，以及插件目录发现。

use parking_lot::RwLock;
use regex::Regex;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tracing::{debug, error, info};
use walkdir::WalkDir;
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_api::host::PluginSdkConfig;
use zerolaunch_plugin_host::host_dispatch::HostCallHandler;
use zerolaunch_plugin_host::manager::{InstalledPluginInfo, PluginHostManager, RegisteredAdapters};
use zerolaunch_plugin_protocol::Manifest;

use crate::core::config::ConfigManager;
use crate::plugin_system::SessionRouter;
use crate::sdk::HostApi;

use super::builtin::{BuiltinInitResult, BuiltinProvider};
use super::host_handler::TauriHostCallHandler;
use super::types::{InstallError, PluginInfo, PluginStatus};

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
    /// ConfigManager 引用
    config_manager: RwLock<Option<Arc<ConfigManager>>>,
    /// SessionRouter 引用
    session_router: RwLock<Option<Arc<SessionRouter>>>,
    /// HostApi 引用
    host_api: RwLock<Option<Arc<HostApi>>>,
    /// PluginHostManager（内部构造，管理子进程生命周期）
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
            host_api: RwLock::new(None),
            host_manager: RwLock::new(None),
            old_adapters: RwLock::new(std::collections::HashMap::new()),
        }
    }

    // ── 依赖注入（在 init_app_state 中各调用一次） ─────────────

    /// 设置 ConfigManager 引用。
    pub fn set_config_manager(&self, cm: Arc<ConfigManager>) {
        *self.config_manager.write() = Some(cm);
    }

    /// 设置 SessionRouter 引用。
    pub fn set_session_router(&self, sr: Arc<SessionRouter>) {
        *self.session_router.write() = Some(sr);
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

    fn config_manager(&self) -> Arc<ConfigManager> {
        self.config_manager
            .read()
            .as_ref()
            .cloned()
            .expect("ConfigManager not set in PluginManager")
    }

    fn session_router(&self) -> Arc<SessionRouter> {
        self.session_router
            .read()
            .as_ref()
            .cloned()
            .expect("SessionRouter not set in PluginManager")
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

    // ── 公开访问器 ──────────────────────────────────────────────

    /// 获取 ConfigManager 引用（panic 如果未初始化）。
    pub fn get_config_manager(&self) -> Arc<ConfigManager> {
        self.config_manager()
    }

    /// 获取 SessionRouter 引用（panic 如果未初始化）。
    pub fn get_session_router(&self) -> Arc<SessionRouter> {
        self.session_router()
    }

    // ── 初始化 API ──────────────────────────────────────────────

    /// 初始化所有内置组件。
    pub fn init_builtins(&self) -> BuiltinInitResult {
        let mut infos = Vec::new();
        let host_api = self.host_api();
        let result = self.builtin_provider.init(
            &host_api,
            &self.config_manager(),
            &self.session_router(),
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

    /// 返回插件安装根目录。
    pub fn plugins_dir(&self) -> PathBuf {
        self.host_manager().plugins_dir().to_path_buf()
    }

    /// 获取第三方插件的完整 manifest。
    pub fn get_manifest(&self, plugin_id: &str) -> Option<Manifest> {
        let hm = self.host_manager();
        let adapters = hm.adapters.get(plugin_id)?;
        Some(adapters.manifest.clone())
    }

    /// 获取第三方插件的日志文件最近 N 行。
    pub fn get_logs(&self, plugin_id: &str, tail_lines: usize) -> Result<Vec<String>, String> {
        let hm = self.host_manager();
        let log_file = hm.log_dir_root.join(format!("{}.log", plugin_id));

        let mut file = match std::fs::File::open(&log_file) {
            Ok(f) => f,
            Err(_) => return Ok(Vec::new()),
        };

        let file_size = file.metadata().map_err(|e| e.to_string())?.len();
        if file_size == 0 {
            return Ok(Vec::new());
        }

        // 从文件末尾读取最多 64KB，提取最后 tail_lines 行
        let tail_size = (64 * 1024).min(file_size);
        let mut buf = vec![0u8; tail_size as usize];
        file.seek(SeekFrom::End(-(tail_size as i64)))
            .map_err(|e| e.to_string())?;
        file.read_exact(&mut buf).map_err(|e| e.to_string())?;

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
        let uri = uri
            .strip_prefix("zlplugin://")
            .ok_or("not a zlplugin URI")?;
        let (host, path) = uri.split_once('/').unwrap_or((uri, ""));

        if host.is_empty() || !is_valid_plugin_id(host) {
            return Err("invalid plugin id".into());
        }

        if !path.starts_with("ui/") {
            return Err("access denied: only ui/ path allowed".into());
        }

        let plugins_dir = self.plugins_dir();
        let plugin_dir = plugins_dir.join(host);
        let asset_path = plugin_dir.join(path);

        // Canonicalize 防路径遍历
        let canonical = asset_path.canonicalize()?;
        let plugin_canonical = plugin_dir.canonicalize()?;
        if !canonical.starts_with(&plugin_canonical) {
            return Err("access denied: path traversal detected".into());
        }

        let bytes = std::fs::read(&canonical)?;
        let mime = mime_from_extension(&canonical).to_string();

        Ok((bytes, mime))
    }

    // ── 第三方插件生命周期方法 ─────────────────────────────────

    /// 从 .zip 文件或目录安装插件。
    /// 成功时发送 `plugin-installed` 事件。
    pub async fn install(
        &self,
        source_path: &Path,
        app_handle: Arc<AppHandle>,
    ) -> Result<InstalledPluginInfo, String> {
        if !source_path.exists() {
            return Err(format!("File not found: {}", source_path.display()));
        }

        let plugins_dir = self.plugins_dir();
        std::fs::create_dir_all(&plugins_dir).map_err(|e| e.to_string())?;

        let installed_dir = if source_path.is_dir() {
            self.install_from_dir(source_path)
                .map_err(|e| e.to_string())?
        } else if source_path.extension().map(|e| e == "zip").unwrap_or(false) {
            self.install_from_zip(source_path)
                .map_err(|e| e.to_string())?
        } else {
            return Err("Unsupported file format. Use .zip or directory.".into());
        };

        self.load_single_plugin(&installed_dir, app_handle.clone())
            .await?;

        let manifest_bytes = std::fs::read_to_string(installed_dir.join("manifest.toml"))
            .map_err(|e| format!("Failed to read manifest after install: {}", e))?;
        let manifest: Manifest = toml::from_str(&manifest_bytes)
            .map_err(|e| format!("Failed to parse manifest: {}", e))?;
        let plugin_id = &manifest.plugin.id;
        let hm = self.host_manager();
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

    /// 重载第三方插件。
    /// 成功时发送 `plugin-installed` 事件。
    pub async fn reload(&self, plugin_id: &str, app_handle: Arc<AppHandle>) -> Result<(), String> {
        info!("Reloading plugin: {}", plugin_id);

        let hm = self.host_manager();

        let adapters = hm
            .adapters
            .get(plugin_id)
            .ok_or(format!("Plugin not found: {}", plugin_id))?;
        let plugin_dir = hm.plugins_dir().join(plugin_id);

        self.unregister_adapters(&adapters).await;
        self.forget_adapters(plugin_id);

        if let Err(e) = hm.unload(plugin_id).await {
            error!("Unload during reload failed: {}", e);
        }

        self.load_single_plugin(&plugin_dir, app_handle)
            .await
            .map_err(|e| format!("Reload failed: {}", e))?;

        info!("Plugin {} reloaded successfully", plugin_id);
        Ok(())
    }

    /// 卸载第三方插件。
    /// 成功时发送 `plugin-uninstalled` 事件。
    pub async fn uninstall(
        &self,
        plugin_id: &str,
        app_handle: Arc<AppHandle>,
    ) -> Result<(), String> {
        info!("Uninstalling plugin: {}", plugin_id);

        let hm = self.host_manager();

        if let Some(adapters) = hm.adapters.get(plugin_id) {
            self.unregister_adapters(&adapters).await;
        }
        self.forget_adapters(plugin_id);
        self.remove_third_party_info(plugin_id);

        if let Err(e) = hm.unload(plugin_id).await {
            error!("Unload during uninstall failed: {}", e);
        }

        let plugin_dir = hm.plugins_dir().join(plugin_id);
        if plugin_dir.exists() {
            std::fs::remove_dir_all(&plugin_dir)
                .map_err(|e| format!("Cannot remove plugin dir: {}", e))?;
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
    pub async fn load_all_third_party(&self, app_handle: Arc<AppHandle>) {
        let plugins_dir = self.plugins_dir();
        let dirs = Self::scan_plugins_dir(&plugins_dir);

        if dirs.is_empty() {
            info!("No third-party plugins found in {}", plugins_dir.display());
            return;
        }

        info!("Found {} third-party plugin(s)", dirs.len());

        for dir in &dirs {
            if let Err(e) = self.load_single_plugin(dir, app_handle.clone()).await {
                error!("Failed to load plugin from {}: {}", dir.display(), e);
            }
        }

        self.session_router().refresh_candidates().await;
    }

    // ── 内部：第三方插件加载/安装/发现 ─────────────────────────

    /// 加载单个第三方插件。
    ///
    /// 成功时发送 `plugin-installed` Tauri 事件。
    async fn load_single_plugin(
        &self,
        plugin_dir: &Path,
        app_handle: Arc<AppHandle>,
    ) -> Result<(), String> {
        let host_manager = self.host_manager();
        let host_api = self.host_api();

        let manifest_path = plugin_dir.join("manifest.toml");
        let manifest_bytes =
            std::fs::read_to_string(&manifest_path).map_err(|e| format!("read manifest: {}", e))?;
        let manifest: Manifest =
            toml::from_str(&manifest_bytes).map_err(|e| format!("parse manifest: {}", e))?;
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
            .map_err(|e| format!("plugin-host load: {}", e))?;

        self.register_adapters(&registered).await;
        self.cache_adapters(&plugin_id, registered.clone());

        let enabled = !registered.configurables.is_empty()
            && registered.configurables.iter().all(|c| c.default_enabled());
        let component_count = registered.configurables.len();
        self.register_third_party_info(PluginInfo::third_party(
            plugin_id.clone(),
            manifest.plugin.name.clone(),
            manifest.plugin.version.clone(),
            manifest.plugin.description.clone(),
            manifest.plugin.author.clone(),
            component_count,
            enabled,
            true,
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

    /// 扫描插件目录，返回包含 manifest.toml 的子目录列表。
    fn scan_plugins_dir(plugins_dir: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        if !plugins_dir.exists() {
            return found;
        }
        if let Ok(entries) = std::fs::read_dir(plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let manifest = path.join("manifest.toml");
                    if manifest.exists() {
                        found.push(path);
                    }
                }
            }
        }
        found
    }

    /// 从 .zip 文件安装插件到 `plugins_dir/<plugin_id>/`。
    fn install_from_zip(&self, zip_path: &Path) -> Result<PathBuf, InstallError> {
        let plugins_dir = self.plugins_dir();
        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        // 第一遍：找 manifest + 收集所有条目名（用于计算公共前缀）
        let mut manifest_content = String::new();
        let mut find_manifest = false;
        let mut names = Vec::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if name == "manifest.toml" {
                entry.read_to_string(&mut manifest_content)?;
                find_manifest = true;
            }
            names.push(name);
        }

        if !find_manifest {
            return Err(InstallError::Manifest(format!(
                "manifest.toml not found in zip: {}",
                zip_path.to_string_lossy()
            )));
        }

        let manifest: Manifest = toml::from_str(&manifest_content)
            .map_err(|e| InstallError::Manifest(format!("invalid manifest: {}", e)))?;

        let plugin_id = &manifest.plugin.id;
        let target_dir = plugins_dir.join(plugin_id);

        if target_dir.exists() {
            return Err(InstallError::AlreadyInstalled(plugin_id.clone()));
        }

        std::fs::create_dir_all(&target_dir)?;

        let common_prefix = find_common_prefix(&names);

        // 第二遍：解压所有文件
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();

            let relative = if let Some(rest) = name.strip_prefix(&common_prefix) {
                let trimmed = rest.trim_start_matches('/');
                if trimmed.is_empty() {
                    continue;
                }
                trimmed
            } else {
                &name
            };

            if relative.is_empty() {
                continue;
            }

            let normalized = std::path::Path::new(relative);
            if normalized.is_absolute() {
                return Err(InstallError::Manifest("absolute path in zip".into()));
            }
            for c in normalized.components() {
                if matches!(c, std::path::Component::ParentDir) {
                    return Err(InstallError::Manifest("parent-dir traversal in zip".into()));
                }
            }

            let out_path = target_dir.join(relative);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            if entry.is_dir() {
                std::fs::create_dir_all(&out_path)?;
            } else {
                let mut out_file = std::fs::File::create(&out_path)?;
                std::io::copy(&mut entry, &mut out_file)?;
            }
        }

        verify_install_dir(&target_dir)?;

        info!(
            "Installed plugin {} from {} to {}",
            plugin_id,
            zip_path.display(),
            target_dir.display()
        );

        Ok(target_dir)
    }

    /// 从目录复制安装插件到 `plugins_dir/<plugin_id>/`。
    fn install_from_dir(&self, source_dir: &Path) -> Result<PathBuf, InstallError> {
        let plugins_dir = self.plugins_dir();
        let manifest_path = source_dir.join("manifest.toml");
        if !manifest_path.exists() {
            return Err(InstallError::Manifest(
                "manifest.toml not found in source directory".into(),
            ));
        }

        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: Manifest = toml::from_str(&manifest_content)
            .map_err(|e| InstallError::Manifest(format!("invalid manifest: {}", e)))?;

        let plugin_id = &manifest.plugin.id;
        let target_dir = plugins_dir.join(plugin_id);

        if target_dir.exists() {
            return Err(InstallError::AlreadyInstalled(plugin_id.clone()));
        }

        copy_dir_recursive(source_dir, &target_dir)?;
        verify_install_dir(&target_dir)?;

        Ok(target_dir)
    }

    // ── 适配器注册/解注册 ──────────────────────────────────────

    /// 将第三方插件的 RegisteredAdapters 注册到 ConfigManager / SessionRouter。
    pub async fn register_adapters(&self, adapters: &RegisteredAdapters) {
        let cm = self.config_manager();
        let sr = self.session_router();
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
        let cm = self.config_manager();
        let sr = self.session_router();
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
        let cm = self.config_manager();
        let sr = self.session_router();
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

            handle.block_on(async {
                sr.refresh_candidates().await;
            });

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

    /// 返回所有第三方插件的详细信息（前端展示用 DTO）。
    pub fn list_third_party_details(&self) -> Vec<InstalledPluginInfo> {
        let hm = self.host_manager();
        let cm = self.config_manager();
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

    /// 启用或禁用插件的所有组件。
    pub fn set_enabled(&self, plugin_id: &str, enabled: bool) -> Result<(), String> {
        let cm = self.config_manager();
        let hm = self.host_manager();

        if let Some(adapters) = hm.adapters.get(plugin_id) {
            for c in &adapters.configurables {
                cm.set_enabled(c.component_id(), enabled)
                    .map_err(|e| e.to_string())?;
            }
            return Ok(());
        }

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

// ── 私有辅助函数 ─────────────────────────────────────────────────

/// 校验插件 ID 是否符合反向域名格式。
fn is_valid_plugin_id(id: &str) -> bool {
    use std::sync::LazyLock;
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(zerolaunch_plugin_protocol::manifest::PLUGIN_ID_RE).unwrap());
    RE.is_match(id)
}

/// 根据文件扩展名确定 MIME 类型。
fn mime_from_extension(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("mjs") | Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("html") => "text/html",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") | Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// 校验安装目录内无符号链接和路径遍历（使用 walkdir 递归遍历）。
fn verify_install_dir(target_dir: &Path) -> Result<(), InstallError> {
    let canonical_target = target_dir
        .canonicalize()
        .map_err(|e| InstallError::Manifest(format!("cannot canonicalize target dir: {}", e)))?;

    for entry in WalkDir::new(target_dir).follow_links(false) {
        let entry = entry.map_err(|e| InstallError::Manifest(format!("walk error: {}", e)))?;
        let path = entry.path();

        if entry.file_type().is_symlink() {
            return Err(InstallError::Manifest(format!(
                "symlinks not allowed: {}",
                path.display()
            )));
        }

        let canonical = path.canonicalize().map_err(|e| {
            InstallError::Manifest(format!("cannot canonicalize {}: {}", path.display(), e))
        })?;
        if !canonical.starts_with(&canonical_target) {
            return Err(InstallError::Manifest(format!(
                "path traversal detected: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

/// 找到 zip 内所有条目的公共路径前缀。
///
/// 例如所有条目都以 `my-plugin/` 开头 → 返回 `"my-plugin"`，
/// 解压时会剥掉这层目录。如果条目没有公共前缀（比如所有文件都
/// 在 zip 根目录），返回空字符串，解压时不剥任何前缀。
fn find_common_prefix(names: &[String]) -> String {
    if names.is_empty() {
        return String::new();
    }

    let first = &names[0];
    let first_dir = match first.find('/') {
        Some(idx) => &first[..idx],
        None => {
            debug!("find_common_prefix: first entry has no '/', no prefix to strip ({first})");
            return String::new();
        }
    };

    let prefix_with_slash = format!("{}/", first_dir);
    for name in &names[1..] {
        if !name.starts_with(&prefix_with_slash) {
            debug!("find_common_prefix: no common prefix (\"{name}\" does not start with \"{prefix_with_slash}\")");
            return String::new();
        }
    }

    debug!("find_common_prefix: using prefix \"{first_dir}\"");
    first_dir.to_string()
}

/// 递归复制目录（使用 walkdir 避免手动递归）。
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(src).unwrap();
        let dst_path = dst.join(relative);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dst_path)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}
