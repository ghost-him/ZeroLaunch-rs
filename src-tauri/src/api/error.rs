use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// HTTP API 统一错误类型
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub error: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
}

impl ApiError {
    pub fn not_found(message: impl Into<String>) -> Self {
        ApiError {
            error: message.into(),
            status: 404,
            component_id: None,
        }
    }

    pub fn invalid_query(message: impl Into<String>) -> Self {
        ApiError {
            error: message.into(),
            status: 400,
            component_id: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        ApiError {
            error: message.into(),
            status: 500,
            component_id: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}
