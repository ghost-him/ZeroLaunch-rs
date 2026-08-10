use axum::Json;

/// GET /v1/ping — 健康检查端点。
///
/// 主程序在线时返回 `{"pong": true}`，供 CLI 探测 ZeroLaunch 是否在运行。
pub async fn handle() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "pong": true }))
}
