//! CLI HTTP server — local HTTP API with bearer token auth.

use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use super::routes;
use crate::core::cli_token::{generate_token_string, persist_cli_token, CliToken};
use crate::state::app_state::AppState;

pub struct CliServerHandle {
    pub port: u16,
    pub token: String,
}

/// Auth middleware — validates Bearer token.
async fn auth_middleware(
    axum::extract::State(token): axum::extract::State<String>,
    headers: axum::http::HeaderMap,
    request: axum::extract::Request,
    next: middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    if auth_header == Some(&token) {
        Ok(next.run(request).await)
    } else {
        Err(axum::http::StatusCode::UNAUTHORIZED)
    }
}

/// Start the CLI HTTP server on 127.0.0.1:0 (OS-assigned port).
pub async fn start(
    state: Arc<AppState>,
    data_dir: &std::path::Path,
) -> Result<CliServerHandle, anyhow::Error> {
    let token = generate_token_string();
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    let cli_token = CliToken {
        host: "127.0.0.1".to_string(),
        port,
        token: token.clone(),
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    persist_cli_token(&cli_token, data_dir)?;

    // Cache the token in AppState so the `cli_get_info` IPC command can serve it.
    state.set_cli_token(cli_token);

    let app_state = state.clone();

    let app = Router::new()
        // Search & Session
        .route("/v1/query", post(routes::query::handle))
        .route("/v1/session/mode", get(routes::session::get_mode))
        .route(
            "/v1/candidates/count",
            get(routes::session::get_candidates_count),
        )
        // Config — read only
        .route(
            "/v1/config/components",
            get(routes::config::list_components),
        )
        .route("/v1/config/{id}/schema", get(routes::config::get_schema))
        .route(
            "/v1/config/{id}/settings",
            get(routes::config::get_settings),
        )
        .route("/v1/config/{id}/actions", get(routes::config::get_actions))
        // Plugin Management — read only
        .route("/v1/plugins", get(routes::plugins::handle_list))
        .route(
            "/v1/plugins/{id}/manifest",
            get(routes::plugins::handle_get_manifest),
        )
        .route(
            "/v1/plugins/{id}/logs",
            get(routes::plugins::handle_get_logs),
        )
        .with_state(app_state)
        .layer(middleware::from_fn_with_state(
            token.clone(),
            auth_middleware,
        ))
        .layer(middleware::from_fn(
            crate::cli_server::middleware::trace_middleware,
        ));

    info!("CLI HTTP server listening on 127.0.0.1:{}", port);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    Ok(CliServerHandle { port, token })
}
