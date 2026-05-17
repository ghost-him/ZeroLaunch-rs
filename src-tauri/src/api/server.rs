use super::read_api::ReadApi;
use crate::utils::service_locator::ServiceLocator;
use axum::routing::get;
use axum::{Extension, Router};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

/// HTTP 服务器句柄，管理启动、优雅关闭和端口文件生命周期。
pub struct HttpServerHandle;

impl HttpServerHandle {
    /// 启动 HTTP 服务器。
    ///
    /// 绑定 127.0.0.1 随机端口，写入端口发现文件，
    /// 将关闭信号 sender 存入 AppState。
    ///
    /// 参数：read_api - ReadApi 实现；port_file_dir - 写入 .zl-port 的目录
    /// 返回：实际绑定的端口号
    pub async fn start(
        read_api: Arc<dyn ReadApi>,
        port_file_dir: PathBuf,
    ) -> Result<u16, io::Error> {
        let app = Router::new()
            .route("/health", get(super::routes::health_handler))
            .route("/search", get(super::routes::search_handler))
            .route("/candidates", get(super::routes::candidates_handler))
            .route("/candidates/{id}", get(super::routes::candidate_handler))
            .route(
                "/candidates/count",
                get(super::routes::candidates_count_handler),
            )
            .route("/session/mode", get(super::routes::session_mode_handler))
            .route("/components", get(super::routes::components_handler))
            .route(
                "/components/{id}/schema",
                get(super::routes::component_schema_handler),
            )
            .route(
                "/components/{id}/settings",
                get(super::routes::component_settings_handler),
            )
            .route("/plugins", get(super::routes::plugins_handler))
            .layer(Extension(read_api));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();

        Self::write_port_file(&port_file_dir, port)?;

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let state = ServiceLocator::get_state();
        state.set_http_server_shutdown(shutdown_tx);
        // 保存端口文件目录用于退出时清理
        state.set_port_file_dir(port_file_dir.to_string_lossy().to_string());

        let handle = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    shutdown_rx.await.ok();
                })
                .await
            {
                warn!("HTTP API 服务器退出错误: {}", e);
            }
        });
        state.set_http_server_handle(handle);

        info!("HTTP API 服务器已启动: 127.0.0.1:{}", port);
        Ok(port)
    }

    /// 将端口号写入发现文件。
    ///
    /// 文件位置：{port_file_dir}/.zl-port
    /// 格式：纯文本端口号，如 "45678"
    fn write_port_file(dir: &PathBuf, port: u16) -> Result<(), io::Error> {
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join(".zl-port"), port.to_string())?;
        info!("端口写入文件: {}", dir.join(".zl-port").display());
        Ok(())
    }
}
