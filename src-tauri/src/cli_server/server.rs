//! CLI HTTP server — local HTTP API with bearer token auth.

use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use std::path::Path;
use std::sync::{Arc, LazyLock};
use tauri::async_runtime::JoinHandle;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{error, info};

use super::middleware::trace_middleware;
use super::routes;
use crate::core::cli_token::{generate_token_string, persist_cli_token, CliToken};
use crate::state::app_state::AppState;

/// 运行中的服务句柄（仅本模块使用）：停用时取消 serve 任务并释放端口。
struct RunningServer {
    /// 监听端口（已落盘到 cli-token.json）。
    port: u16,
    /// serve 任务句柄；abort 并 await 后端口已释放。
    task: JoinHandle<()>,
}

/// 进程内唯一的运行句柄；`None` 表示未监听。
static RUNNING: LazyLock<Mutex<Option<RunningServer>>> = LazyLock::new(|| Mutex::new(None));

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

/// 按设置启用或停用 CLI HTTP 服务（幂等，启动阶段与设置变更时调用）。
///
/// 返回：启用时绑定/落盘失败的错误；停用恒成功。
pub async fn apply_enabled(
    state: Arc<AppState>,
    data_dir: &Path,
    enabled: bool,
) -> Result<(), anyhow::Error> {
    let mut slot = RUNNING.lock().await;
    if enabled {
        if slot.is_none() {
            *slot = Some(start(state, data_dir).await?);
        }
    } else if let Some(server) = slot.take() {
        // 取消任务即释放监听器，返回时端口已关闭。
        server.task.abort();
        let _ = server.task.await;
        info!("CLI HTTP 服务器已停止（原端口 {}）", server.port);
    }
    Ok(())
}

/// 在 127.0.0.1:0 启动服务并返回可停止的运行句柄。
async fn start(state: Arc<AppState>, data_dir: &Path) -> Result<RunningServer, anyhow::Error> {
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
        // Health check
        .route("/v1/ping", get(routes::ping::handle))
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
        .layer(middleware::from_fn(trace_middleware));

    info!("CLI HTTP server listening on 127.0.0.1:{}", port);

    let task = tauri::async_runtime::spawn(async move {
        // serve 只在 accept 异常或被停用时退出；失败仅记日志，避免静默杀掉任务。
        if let Err(e) = axum::serve(listener, app).await {
            error!("CLI HTTP server exited with error: {e}");
        }
    });

    Ok(RunningServer { port, task })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 启停开关：启用后落盘 token 并监听；停用后端口释放；可重复启停。
    ///
    /// 单测试串行覆盖（运行句柄是进程级单例），并在 tauri runtime 内执行以贴合生产链路。
    #[test]
    fn apply_enabled_starts_and_stops_listening() {
        tauri::async_runtime::block_on(async {
            let dir = tempfile::tempdir().unwrap();
            let state = Arc::new(AppState::new());

            apply_enabled(state.clone(), dir.path(), true)
                .await
                .unwrap();
            let port = state.get_cli_token().expect("启用后应缓存连接信息").port;
            assert!(dir.path().join("cli-token.json").exists());
            assert!(probe(port), "启用后端口 {port} 未监听");

            apply_enabled(state.clone(), dir.path(), false)
                .await
                .unwrap();
            assert!(!probe(port), "停用后端口 {port} 仍在监听");

            apply_enabled(state.clone(), dir.path(), true)
                .await
                .unwrap();
            let reopened = state
                .get_cli_token()
                .expect("重新启用后应缓存连接信息")
                .port;
            assert!(probe(reopened), "重新启用后端口 {reopened} 未监听");
            apply_enabled(state, dir.path(), false).await.unwrap();
        });
    }

    /// 同步探测端口是否在监听（async connect 在 block_on 下不可用）。
    fn probe(port: u16) -> bool {
        std::net::TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}").parse().unwrap(),
            std::time::Duration::from_millis(500),
        )
        .is_ok()
    }
}
