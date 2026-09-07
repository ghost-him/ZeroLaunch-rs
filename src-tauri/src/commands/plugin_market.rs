//! 插件市场 IPC 命令。
//!
//! 业务协调：组合 GitHubMarketClient（网络/解析）与 PluginManager（安装），
//! 市场模块不直接依赖 plugin_framework，安装编排在此层完成。

use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::plugin_market::{GitHubMarketClient, MarketRelease, MarketRepo};
use crate::state::app_state::AppState;
use std::path::Path;
use std::sync::Arc;
use tauri::State;
use zerolaunch_plugin_api::services::path::KnownPath;
use zerolaunch_plugin_host::manager::InstalledPluginInfo;

// ── Commands ─────────────────────────────────────────────────────

/// 拉取插件市场仓库列表（GitHub topic: zerolaunch-plugin，已排除主程序仓库）。
#[tauri::command]
#[tracing::instrument(fields(trace_id))]
pub async fn market_list() -> Result<Vec<MarketRepo>, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let client = GitHubMarketClient::new();
    client.list_repos().await.with_trace_id(&trace_id)
}

/// 查询仓库最新发布中的插件包附件（无发布/无 zip 附件返回对应错误）。
#[tauri::command]
#[tracing::instrument(fields(trace_id))]
pub async fn market_get_release(full_name: String) -> Result<MarketRelease, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let client = GitHubMarketClient::new();
    client
        .get_release(&full_name)
        .await
        .with_trace_id(&trace_id)
}

/// 安装仓库最新发布中的插件包：解析 release → 下载 zip → 交给 PluginManager 安装。
///
/// zip 经 core PluginHandle 缓存接口暂存于 plugin-cache/core/market/<zip 名>
/// （cache_put 内部完成路径解析与建目录），安装完成后经 cache_delete 清理。
/// `overwrite=true` 时若同名插件已安装则覆盖（复用 plugin_install_local 语义）。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn market_install(
    full_name: String,
    overwrite: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<InstalledPluginInfo, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());

    // 1. 解析最新发布并下载插件包（整包入内存）。
    let client = GitHubMarketClient::new();
    let release = client
        .get_release(&full_name)
        .await
        .with_trace_id(&trace_id)?;
    let asset = &release.asset;
    let bytes = client.download(asset).await.with_trace_id(&trace_id)?;

    // 2. 经 core PluginHandle 缓存接口暂存：plugin-cache/core/market/<zip 名>。
    let core_handle = state.get_core_handle();
    core_handle
        .cache_put("market", &asset.name, &bytes)
        .await
        .with_trace_id(&trace_id)?;
    let dest = Path::new(
        &core_handle
            .resolve_path(KnownPath::AppCacheDir)
            .with_trace_id(&trace_id)?,
    )
    .join(core_handle.plugin_id())
    .join("market")
    .join(&asset.name);

    // 3. 交给 PluginManager 安装（zip 解压复制进插件目录，此后 zip 可安全删除）。
    let plugin_manager = state.get_plugin_manager();
    let app_handle = state.get_main_handle();
    let install_result = plugin_manager.install(&dest, overwrite, app_handle).await;
    // 无论安装成败都清理暂存文件。
    if let Err(e) = core_handle.cache_delete("market", &asset.name).await {
        tracing::warn!("清理市场下载暂存文件失败: {}", e);
    }
    install_result.with_trace_id(&trace_id)
}
