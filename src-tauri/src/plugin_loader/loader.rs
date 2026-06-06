use parking_lot;
use std::path::Path;
use std::sync::Arc;
use tauri::Emitter;
use tracing::{error, info};

use crate::core::config::ConfigManager;
use crate::plugin_system::SessionRouter;
use base64::Engine;
use zerolaunch_plugin_api::host::OpenTarget;
use zerolaunch_plugin_host::host_dispatch::HostCallHandler;
use zerolaunch_plugin_host::manager::PluginHostManager;
use zerolaunch_plugin_protocol::{codes, JsonRpcError};

/// A HostCallHandler that dispatches host/* calls to the local PluginHandle.
struct TauriHostCallHandler {
    host_api: Arc<crate::sdk::HostApi>,
    plugin_id: String,
    app_handle: Option<tauri::AppHandle>,
}

#[async_trait::async_trait]
impl HostCallHandler for TauriHostCallHandler {
    async fn handle_host_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use serde_json::from_value;
        use zerolaunch_plugin_protocol::methods::host;

        let handle = self
            .host_api
            .get_plugin_handle(&self.plugin_id)
            .ok_or_else(|| JsonRpcError::new(codes::PLUGIN_ERROR, "PluginHandle not found"))?;

        match method {
            host::LOG => {
                let p: zerolaunch_plugin_protocol::LogParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                match p.level.as_str() {
                    "error" => tracing::error!("[plugin {}] {}", self.plugin_id, p.message),
                    "warn" => tracing::warn!("[plugin {}] {}", self.plugin_id, p.message),
                    "debug" => tracing::debug!("[plugin {}] {}", self.plugin_id, p.message),
                    _ => tracing::info!("[plugin {}] {}", self.plugin_id, p.message),
                }
                Ok(serde_json::Value::Null)
            }
            host::NOTIFY => {
                let p: zerolaunch_plugin_protocol::NotifyParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                if let Some(app_handle) = &self.app_handle {
                    use tauri_plugin_notification::NotificationExt;
                    let _ = app_handle
                        .notification()
                        .builder()
                        .title(&p.title)
                        .body(&p.message)
                        .show();
                }
                Ok(serde_json::Value::Null)
            }
            host::SHELL_OPEN => {
                let p: zerolaunch_plugin_protocol::ShellOpenParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .shell_open(OpenTarget::File(p.target))
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::Value::Null)
            }
            host::SHELL_OPEN_FOLDER => {
                let p: zerolaunch_plugin_protocol::ShellOpenFolderParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .shell_open_folder(&p.path)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::Value::Null)
            }
            host::SHELL_EXECUTE_ELEVATION => {
                let p: zerolaunch_plugin_protocol::ShellExecuteElevationParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .shell_execute_elevation(&p.path)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::Value::Null)
            }
            host::SHELL_EXECUTE_COMMAND => {
                let p: zerolaunch_plugin_protocol::ShellExecuteCommandParams =
                    from_value(params)
                        .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .shell_execute_command(&p.cmd)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::Value::Null)
            }
            host::WINDOW_ACTIVATE_BY_PROCESS => {
                let p: zerolaunch_plugin_protocol::WindowActivateParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .activate_window_by_process(&p.pid.to_string())
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::Value::Null)
            }
            host::ICON_GET => {
                let p: zerolaunch_plugin_protocol::IconGetParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let icon_request: zerolaunch_plugin_api::services::icon_request::IconRequest =
                    serde_json::from_value(p.request)
                        .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let bytes = handle.get_icon_or_default(icon_request).await;
                let base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                Ok(serde_json::Value::String(base64))
            }
            host::APP_ENUMERATE => {
                let apps = handle.enumerate_apps().await;
                Ok(serde_json::to_value(apps).unwrap_or_default())
            }
            host::PATH_RESOLVE => {
                let p: zerolaunch_plugin_protocol::PathResolveParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let known_path = match p.kind.as_str() {
                    "AppDataDir" => zerolaunch_plugin_api::services::path::KnownPath::AppDataDir,
                    "AppConfigDir" => {
                        zerolaunch_plugin_api::services::path::KnownPath::AppConfigDir
                    }
                    "AppIconCacheDir" => {
                        zerolaunch_plugin_api::services::path::KnownPath::AppIconCacheDir
                    }
                    _ => zerolaunch_plugin_api::services::path::KnownPath::AppDataDir,
                };
                let path = handle
                    .resolve_path(known_path)
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::Value::String(path))
            }
            host::PARAMETER_RESOLVE => {
                let p: zerolaunch_plugin_protocol::ParameterResolveParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let snapshot = zerolaunch_plugin_api::services::ParameterSnapshot::empty();
                let result = handle
                    .resolve_parameters(&p.plugin_id, &p.user_args, &snapshot)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::to_value(result).unwrap_or_default())
            }
            host::RESOURCE_UPLOAD => {
                let p: zerolaunch_plugin_protocol::ResourceUploadParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                if p.plugin_id != self.plugin_id {
                    return Err(JsonRpcError::new(
                        codes::INVALID_PARAMS,
                        "plugin_id mismatch",
                    ));
                }
                let bytes = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    &p.bytes_b64,
                )
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                // Write temp file and upload
                let tmp = std::env::temp_dir().join(format!("zl_upload_{}", p.key));
                std::fs::write(&tmp, &bytes)
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                let uri = handle
                    .resource_upload(&p.key, &tmp.to_string_lossy(), p.max_size.map(|s| s as u64))
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                let _ = std::fs::remove_file(&tmp);
                Ok(serde_json::Value::String(uri))
            }
            host::RESOURCE_GET => {
                let p: zerolaunch_plugin_protocol::ResourceGetParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                if p.plugin_id != self.plugin_id {
                    return Err(JsonRpcError::new(
                        codes::INVALID_PARAMS,
                        "plugin_id mismatch",
                    ));
                }
                let data = handle
                    .resource_get(&p.key)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                Ok(serde_json::Value::String(b64))
            }
            host::RESOURCE_DELETE => {
                let p: zerolaunch_plugin_protocol::ResourceDeleteParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                if p.plugin_id != self.plugin_id {
                    return Err(JsonRpcError::new(
                        codes::INVALID_PARAMS,
                        "plugin_id mismatch",
                    ));
                }
                handle
                    .resource_delete(&p.key)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::Value::Null)
            }
            host::RESOURCE_LIST => {
                let p: zerolaunch_plugin_protocol::ResourceListParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                if p.plugin_id != self.plugin_id {
                    return Err(JsonRpcError::new(
                        codes::INVALID_PARAMS,
                        "plugin_id mismatch",
                    ));
                }
                let keys = handle
                    .resource_list()
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::to_value(keys).unwrap_or_default())
            }
            _ => Err(JsonRpcError::new(
                codes::METHOD_NOT_FOUND,
                format!("host method not found: {}", method),
            )),
        }
    }
}

/// Load a single third-party plugin from a directory and register it.
/// Emits the `plugin-installed` Tauri event on success so the frontend can
/// dynamically register panel/settings providers.
pub async fn load_plugin(
    plugin_dir: &Path,
    config_manager: &Arc<ConfigManager>,
    session_router: &Arc<SessionRouter>,
    host_manager: &Arc<PluginHostManager>,
    host_api: Arc<crate::sdk::HostApi>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Read manifest first to get plugin_id
    let manifest_path = plugin_dir.join("manifest.toml");
    let manifest_bytes =
        std::fs::read_to_string(&manifest_path).map_err(|e| format!("read manifest: {}", e))?;
    let manifest: zerolaunch_plugin_protocol::manifest::Manifest =
        toml::from_str(&manifest_bytes).map_err(|e| format!("parse manifest: {}", e))?;
    let plugin_id = manifest.plugin.id.clone();

    // Create PluginHandle via HostApi
    let _handle = host_api.register(
        &plugin_id,
        zerolaunch_plugin_api::host::PluginSdkConfig::default(),
    );

    // Build the host call handler
    let handler: Arc<dyn HostCallHandler> = Arc::new(TauriHostCallHandler {
        host_api: host_api.clone(),
        plugin_id: plugin_id.clone(),
        app_handle: Some(app_handle.clone()),
    });

    // Build the restart callback: when the plugin-host manager re-spawns a
    // crashed process, this callback unregisters the old adapters from
    // ConfigManager/SessionRouter and registers the new ones.
    let restart_cm = config_manager.clone();
    let restart_sr = session_router.clone();
    let old_adapters_ref: Arc<
        parking_lot::Mutex<Option<zerolaunch_plugin_host::manager::RegisteredAdapters>>,
    > = Arc::new(parking_lot::Mutex::new(None));
    let old_for_cb = old_adapters_ref.clone();

    let on_restart: Arc<dyn Fn(zerolaunch_plugin_host::manager::RegisteredAdapters) + Send + Sync> =
        Arc::new(move |new_adapters| {
            let handle = tokio::runtime::Handle::current();
            // Unregister old adapters if any
            let mut old = old_for_cb.lock();
            if let Some(prev) = old.take() {
                handle.block_on(async {
                    for ds in &prev.data_sources {
                        restart_sr.unregister_data_source(&ds.component_id).await;
                    }
                });
                for ex in &prev.executors {
                    restart_sr.unregister_executor(&ex.component_id);
                }
                if prev.plugin.is_some() {
                    restart_sr.unregister_plugin(&prev.plugin_id);
                }
                for c in &prev.configurables {
                    restart_cm.unregister(&c.component_id);
                }
            }
            // Register new adapters
            for c in &new_adapters.configurables {
                restart_cm.register(c.clone());
            }
            handle.block_on(async {
                for ds in &new_adapters.data_sources {
                    restart_sr.register_data_source(ds.clone()).await;
                }
            });
            for ex in &new_adapters.executors {
                restart_sr.register_executor(ex.clone());
            }
            if let Some(p) = &new_adapters.plugin {
                restart_sr.register_remote_plugin(p.clone());
            }
            // Store new as old for the next restart cycle
            *old = Some(new_adapters);
        });

    // Delegate to plugin-host for loading
    let registered = host_manager
        .load(plugin_dir, handler, on_restart)
        .await
        .map_err(|e| format!("plugin-host load: {}", e))?;

    // Register adapters for the first time
    for c in &registered.configurables {
        config_manager.register(c.clone());
    }
    for ds in &registered.data_sources {
        session_router.register_data_source(ds.clone()).await;
    }
    for ex in &registered.executors {
        session_router.register_executor(ex.clone());
    }
    if let Some(p) = &registered.plugin {
        session_router.register_remote_plugin(p.clone());
    }

    // Store for restart callback use
    *old_adapters_ref.lock() = Some(registered.clone());

    info!("Loaded third-party plugin: {}", plugin_id);

    // Notify frontend so it can dynamically register panel/settings providers
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

/// Load all third-party plugins from the given directory.
pub async fn load_all(
    plugins_dir: &Path,
    config_manager: Arc<ConfigManager>,
    session_router: Arc<SessionRouter>,
    host_manager: Arc<PluginHostManager>,
    host_api: Arc<crate::sdk::HostApi>,
    app_handle: tauri::AppHandle,
) {
    let dirs = super::discovery::scan_plugins_dir(plugins_dir);

    if dirs.is_empty() {
        info!("No third-party plugins found in {}", plugins_dir.display());
        return;
    }

    info!("Found {} third-party plugin(s)", dirs.len());

    for dir in &dirs {
        if let Err(e) = load_plugin(
            dir,
            &config_manager,
            &session_router,
            &host_manager,
            host_api.clone(),
            app_handle.clone(),
        )
        .await
        {
            error!("Failed to load plugin from {}: {}", dir.display(), e);
        }
    }

    // Rebuild search pipeline with newly added data sources
    session_router.refresh_candidates().await;
}
