//! TauriHostCallHandler — 将插件的 `host/*` RPC 调用分发给本地 PluginHandle。

use base64::Engine;
use std::sync::Arc;
use tauri::AppHandle;
use zerolaunch_plugin_api::host::OpenTarget;
use zerolaunch_plugin_host::host_dispatch::HostCallHandler;
use zerolaunch_plugin_protocol::{codes, JsonRpcError};

use crate::sdk::HostApi;

/// 将插件的 `host/*` RPC 调用分发给本地的 PluginHandle。
pub(crate) struct TauriHostCallHandler {
    pub(crate) host_api: Arc<HostApi>,
    pub(crate) plugin_id: String,
    pub(crate) app_handle: Option<Arc<AppHandle>>,
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
            //
            // 插件                           JSON-RPC 线上              宿主
            // ────                          ────────────              ────
            // upload:        传文件路径 → host/resource.upload ──→ resource_upload(file_path) → 读文件 → 存储
            // put:           raw bytes ──(encode)──→ base64 ──(decode)──→ resource_put(bytes) → 存储
            // get:           raw bytes ←──(decode)── base64 ←──(encode)── resource_get(id)
            //
            host::RESOURCE_UPLOAD => {
                let p: zerolaunch_plugin_protocol::ResourceUploadParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let result = handle
                    .resource_upload(&p.resource_id, &p.file_path, p.max_size.map(|s| s as u64))
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::Value::String(result))
            }
            host::RESOURCE_PUT => {
                let p: zerolaunch_plugin_protocol::ResourcePutParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&p.bytes_b64)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .resource_put(&p.resource_id, &bytes)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::Value::Null)
            }
            host::RESOURCE_GET => {
                let p: zerolaunch_plugin_protocol::ResourceGetParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let data = handle
                    .resource_get(&p.resource_id)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                Ok(serde_json::Value::String(b64))
            }
            host::RESOURCE_DELETE => {
                let p: zerolaunch_plugin_protocol::ResourceDeleteParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .resource_delete(&p.resource_id)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::Value::Null)
            }
            host::RESOURCE_LIST => {
                let _p: zerolaunch_plugin_protocol::ResourceListParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
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
