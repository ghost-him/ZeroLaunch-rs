use crate::bridge_error::{BridgeError, WithTraceId};
use crate::state::app_state::AppState;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use std::sync::Arc;

/// 资源上传负载。
#[derive(Deserialize, Debug)]
pub struct ResourceUploadPayload {
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "resourceId")]
    pub resource_id: String,
    #[serde(rename = "maxSize")]
    pub max_size: Option<u64>,
}

/// 获取资源文件内容，返回 base64 data URL 供前端预览。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn resource_get(
    state: tauri::State<'_, Arc<AppState>>,
    resource_id: String,
) -> Result<String, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let core_handle = state.get_core_handle();
    let data = core_handle
        .resource_get(&resource_id)
        .await
        .with_trace_id(&trace_id)?;
    Ok(to_data_url(&data, &resource_id))
}

/// 上传资源文件，返回存储标识符。
#[tauri::command]
#[tracing::instrument(skip(state, payload), fields(trace_id))]
pub async fn resource_upload(
    state: tauri::State<'_, Arc<AppState>>,
    payload: ResourceUploadPayload,
) -> Result<String, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let core_handle = state.get_core_handle();
    core_handle
        .resource_upload(&payload.resource_id, &payload.file_path, payload.max_size)
        .await
        .with_trace_id(&trace_id)
}

/// 将字节数据转为 base64 data URL。
fn to_data_url(data: &[u8], resource_id: &str) -> String {
    let mime = mime_type(resource_id);
    let b64 = STANDARD.encode(data);
    format!("data:{};base64,{}", mime, b64)
}

/// 根据扩展名推断 MIME 类型。
fn mime_type(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}
