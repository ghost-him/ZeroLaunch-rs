//! 无窗口内核集成测试：装配、查询、确认执行与 CLI HTTP 端点。

use std::path::Path;
use std::sync::Arc;

use serde_json::json;
use zerolaunch_platform_windows::{windows_platform_services, WindowsPathResolver};
use zerolaunch_plugin_api::plugin::{Query, QueryResponse};
use zerolaunch_rs_lib::cli_server::server::apply_enabled;
use zerolaunch_rs_lib::kernel::{self, KernelOptions, KernelPaths};
use zerolaunch_rs_lib::plugin_framework::{ConfirmOutcome, ConfirmRequest};
use zerolaunch_rs_lib::state::app_state::AppState;

/// 无窗口装配后完成一次查询、一次插件动作确认，并验证 CLI ping。
#[test]
fn kernel_headless_query_confirm_and_cli() {
    zerolaunch_rs_lib::runtime::block_on(async {
        let root = tempfile::tempdir().unwrap();
        let app_data = root.path().join("data");
        let icon_cache = root.path().join("icons-cache");
        let config = root.path().join("config");
        let resource = root.path().join("resource");
        for dir in [&app_data, &icon_cache, &config, &resource] {
            std::fs::create_dir_all(dir).unwrap();
        }

        let state = Arc::new(AppState::new());
        let resolver = Arc::new(WindowsPathResolver::new());
        let platform = windows_platform_services(0, resolver, ".".to_string(), ".".to_string());
        kernel::init(
            &state,
            platform,
            KernelPaths {
                app_data_dir: app_data.to_string_lossy().to_string(),
                icon_cache_dir: icon_cache.to_string_lossy().to_string(),
                config_dir: config.to_string_lossy().to_string(),
                resource_dir: resource,
            },
            KernelOptions {
                start_background_tasks: false,
            },
        )
        .await;

        let dispatcher = state.get_session_dispatcher();
        let trace = "headless-kernel-test";
        let query = Query {
            id: trace.to_string(),
            raw_query: "= 1+1".to_string(),
            search_term: "= 1+1".to_string(),
            confirm: false,
        };
        let routed = dispatcher
            .route_query_ui(trace, &query)
            .await
            .expect("无窗口查询应成功");
        assert!(
            matches!(routed.response, QueryResponse::CustomPanel { .. }),
            "计算器应返回面板响应"
        );

        let confirmed = dispatcher
            .route_confirm(
                trace,
                ConfirmRequest::PluginAction {
                    plugin_id: "calculator".to_string(),
                    action: "copy_result".to_string(),
                    args: json!({}),
                    generation: routed.generation,
                },
            )
            .await
            .expect("无窗口确认应成功");
        assert!(matches!(confirmed.outcome, ConfirmOutcome::Executed));

        let token = state.get_cli_token().expect("CLI 服务应已启动");
        let client = reqwest::Client::new();
        let base = format!("http://127.0.0.1:{}", token.port);
        let response = client
            .get(format!("{base}/v1/ping"))
            .bearer_auth(&token.token)
            .send()
            .await
            .expect("CLI ping 请求失败");
        assert!(response.status().is_success(), "CLI ping 非 2xx");

        let response = client
            .post(format!("{base}/v1/query"))
            .bearer_auth(&token.token)
            .json(&json!({ "rawQuery": "= 1+1" }))
            .send()
            .await
            .expect("CLI query 请求失败");
        assert!(response.status().is_success(), "CLI query 非 2xx");
        let body: serde_json::Value = response.json().await.expect("CLI query 响应不是 JSON");
        assert!(
            body.get("customPanel").is_some(),
            "CLI query 应返回计算器面板: {body}"
        );

        apply_enabled(state.clone(), Path::new(&app_data), false)
            .await
            .expect("CLI 服务应可停止");
    });
}
