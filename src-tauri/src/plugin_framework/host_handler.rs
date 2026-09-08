//! TauriHostCallHandler — 将插件的 `host/*` RPC 调用分发给插件句柄。
//! PluginHandle 为插件接口层（薄视图），方法委托宿主契约 PluginHost；
//! 因此经句柄的调用最终由宿主（HostApi）执行——直接处理或转发平台端口。

use base64::Engine;
use std::sync::Arc;
use tauri::AppHandle;
use zerolaunch_plugin_api::host::OpenTarget;
use zerolaunch_plugin_api::services::Theme;
use zerolaunch_plugin_host::host_dispatch::HostCallHandler;
use zerolaunch_plugin_protocol::{codes, JsonRpcError};

use crate::core::i18n::I18nManager;
use crate::sdk::HostApi;

/// 将插件的 `host/*` RPC 调用分发给本地的 PluginHandle。
pub(crate) struct TauriHostCallHandler {
    pub(crate) host_api: Arc<HostApi>,
    pub(crate) plugin_id: String,
    pub(crate) app_handle: Option<Arc<AppHandle>>,
    /// 后端翻译服务（host/i18n.get_locale 查询当前语言）
    pub(crate) i18n: Arc<I18nManager>,
}

#[async_trait::async_trait]
impl HostCallHandler for TauriHostCallHandler {
    async fn handle_host_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        tracing::debug!(
            "收到插件 host/* 调用: plugin={} method={} params={}",
            self.plugin_id,
            method,
            zerolaunch_plugin_protocol::codec::summarize_value(&params)
        );
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
                    .activate_window_by_pid(p.pid)
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
                    "AppCacheDir" => zerolaunch_plugin_api::services::path::KnownPath::AppCacheDir,
                    "AppLogDir" => zerolaunch_plugin_api::services::path::KnownPath::AppLogDir,
                    "AppIconCacheDir" => {
                        zerolaunch_plugin_api::services::path::KnownPath::AppIconCacheDir
                    }
                    // 未知 kind：返回错误而非静默回退 AppDataDir，避免路径语义漂移。
                    _ => {
                        return Err(JsonRpcError::new(
                            codes::INVALID_PARAMS,
                            format!("未知的路径类型: {}", p.kind),
                        ));
                    }
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
                    .resolve_parameters(&p.template, &p.user_args, &snapshot)
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
            // 查询宿主当前界面语言（如 "zh-Hans"），插件按需本地化动态文本。
            host::GET_LOCALE => Ok(serde_json::json!(self.i18n.current_language())),
            // 查询宿主当前实际生效主题，system 模式由 PluginHandle 解析。
            host::GET_THEME => {
                let theme = match handle
                    .get_theme()
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?
                {
                    Theme::Light => "light",
                    Theme::Dark => "dark",
                };
                Ok(serde_json::json!(theme))
            }
            // 全网模型清单（聚合缓存）。
            host::MODEL_LIST => Ok(serde_json::to_value(handle.model_list()).unwrap_or_default()),
            // 按 model_id 调用文本生成。
            host::MODEL_CHAT => {
                let req: zerolaunch_plugin_api::services::model::ModelChatRequest =
                    from_value(params)
                        .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let resp = handle.model_chat(req).await.map_err(model_error_to_rpc)?;
                Ok(serde_json::to_value(resp).unwrap_or_default())
            }
            // 按 model_id 调用文本向量化。
            host::MODEL_EMBEDDING => {
                let req: zerolaunch_plugin_api::services::model::ModelEmbeddingRequest =
                    from_value(params)
                        .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let resp = handle
                    .model_embedding(req)
                    .await
                    .map_err(model_error_to_rpc)?;
                Ok(serde_json::to_value(resp).unwrap_or_default())
            }
            // 按 model_id 计算查询向量与多个目标向量的相似度。
            host::MODEL_SIMILARITY => {
                let req: zerolaunch_plugin_api::services::model::ModelSimilarityRequest =
                    from_value(params)
                        .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let resp = handle
                    .model_similarity(req)
                    .await
                    .map_err(model_error_to_rpc)?;
                Ok(serde_json::to_value(resp).unwrap_or_default())
            }
            _ => Err(JsonRpcError::new(
                codes::METHOD_NOT_FOUND,
                format!("host method not found: {}", method),
            )),
        }
    }
}
/// 将模型服务错误映射为 JSON-RPC 错误（专用错误码 + 原始消息）。
/// 插件经 host/model.* 调用可区分 ModelNotFound / ProviderUnavailable 等语义。
fn model_error_to_rpc(e: zerolaunch_plugin_api::services::model::ModelError) -> JsonRpcError {
    use zerolaunch_plugin_api::services::model::ModelError;
    let (code, message) = match e {
        ModelError::NotSupported => (codes::MODEL_NOT_SUPPORTED, e.to_string()),
        ModelError::ModelNotFound(_) => (codes::MODEL_NOT_FOUND, e.to_string()),
        ModelError::ProviderUnavailable(_) => (codes::MODEL_PROVIDER_UNAVAILABLE, e.to_string()),
        ModelError::InvalidRequest(_) => (codes::MODEL_INVALID_REQUEST, e.to_string()),
        ModelError::Transport(_) => (codes::MODEL_TRANSPORT, e.to_string()),
        ModelError::Internal(_) => (codes::INTERNAL_ERROR, e.to_string()),
    };
    JsonRpcError::new(code, message)
}
