use axum::extract::Request;
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument;

/// 请求级 trace_id，通过 axum Extension 在中间件与 handler 间传递。
#[derive(Clone)]
pub struct TraceId(pub String);

/// Trace 中间件：读取 X-Trace-Id 请求头（支持客户端传入），不存在则生成；
/// 写入 Extension 供 handler 使用；响应时回写 X-Trace-Id 头。
pub async fn trace_middleware(headers: HeaderMap, mut request: Request, next: Next) -> Response {
    let trace_id = headers
        .get("x-trace-id")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty() && s.len() <= 64)
        .map(|s| s.to_string())
        .unwrap_or_else(crate::utils::trace_id::generate_trace_id);

    request.extensions_mut().insert(TraceId(trace_id.clone()));

    let span = tracing::info_span!("http_request", trace_id = %trace_id);
    let mut response = next.run(request).instrument(span).await;

    if let Ok(val) = trace_id.parse() {
        response.headers_mut().insert("x-trace-id", val);
    }
    response
}
