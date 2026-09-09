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
use zerolaunch_plugin_protocol::Manifest;

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

/// 安装仓库最新发布中的插件包：解析 release → 定位/下载 zip → 交给 PluginManager 安装。
///
/// zip 经 core PluginHandle 缓存接口暂存于 plugin-cache/core/market/<zip 名>
/// （cache_put 内部完成路径解析与建目录）。若该文件已存在（安装确认弹窗预检时
/// 下载暂存）则直接复用、不再重复下载；安装完成后经 cache_delete 清理。
/// 缓存复用安全依据：GitHub release asset 同名不可重复上传（同名即同内容），
/// 且插件包文件名含版本号，发布新版本时 key 随之变化、自动失效。
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

    // 1. 解析最新发布（轻量 API 调用），确定插件包附件与暂存路径。
    let client = GitHubMarketClient::new();
    let release = client
        .get_release(&full_name)
        .await
        .with_trace_id(&trace_id)?;
    let asset = &release.asset;
    let core_handle = state.get_core_handle();
    let dest = Path::new(
        &core_handle
            .resolve_path(KnownPath::AppCacheDir)
            .with_trace_id(&trace_id)?,
    )
    .join(core_handle.plugin_id())
    .join("market")
    .join(&asset.name);

    // 2. 命中预检暂存则直接复用；未命中才下载（整包入内存后经缓存接口落盘）。
    if !dest.exists() {
        let bytes = client.download(asset).await.with_trace_id(&trace_id)?;
        core_handle
            .cache_put("market", &asset.name, &bytes)
            .await
            .with_trace_id(&trace_id)?;
    } else {
        tracing::info!("命中预检暂存，跳过重复下载: {}", dest.display());
    }

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

/// 市场安装预检响应 —— 仓库最新发布的插件包信息 + 包内 manifest。
///
/// 仅由安装确认弹窗（PluginMarket.vue）消费：点击「安装」后先预检展示，
/// 确认后由 market_install 复用本次暂存安装（整包仅下载一次）；取消时经
/// market_discard_preview 释放暂存。assetName 即暂存缓存 key。
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarketPackagePreview {
    /// 发布 tag 名（如 `v1.2.3`），来源 GitHub release。
    #[serde(rename = "tagName")]
    pub tag_name: String,
    /// 插件包附件文件名（`zerolaunch-plugin-*.zip`），非空。
    #[serde(rename = "assetName")]
    pub asset_name: String,
    /// 插件包 manifest（zip 根 manifest.toml 解析结果，供用户判断是否安装）。
    pub manifest: Manifest,
}

/// 预检仓库最新发布的插件包：下载 zip → 暂存 cache → 读取 manifest。
///
/// 预检成功时保留暂存文件供确认后的 market_install 直接复用（避免整包二次下载）；
/// 用户取消时由 market_discard_preview 释放。解析失败说明包不完整（如缺
/// manifest），此时才主动删除暂存，避免坏包残留。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn market_preview_package(
    full_name: String,
    state: State<'_, Arc<AppState>>,
) -> Result<MarketPackagePreview, BridgeError> {
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

    // 2. 经 core PluginHandle 缓存接口暂存后交给 PluginManager 解析
    //    （zip 路径复用与 market_install 相同的解析/建目录逻辑）。
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

    let preview_result = state
        .get_plugin_manager()
        .inspect_package(&dest)
        .with_trace_id(&trace_id);

    // 解析失败（包不完整/缺 manifest）时清理暂存，避免坏包残留；成功则保留供安装复用。
    if preview_result.is_err() {
        if let Err(e) = core_handle.cache_delete("market", &asset.name).await {
            tracing::warn!("清理市场预检暂存文件失败: {}", e);
        }
    }

    let manifest = preview_result?;
    Ok(MarketPackagePreview {
        tag_name: release.tag_name,
        asset_name: asset.name.clone(),
        manifest,
    })
}

/// 释放市场安装预检暂存的插件包 —— 安装确认弹窗取消/关闭时调用。
///
/// 幂等：缓存不存在视为成功；安装成功路径由 market_install 自行删除，
/// 前端无需区分场景，统一在弹窗关闭时释放即可。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn market_discard_preview(
    asset_name: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    state
        .get_core_handle()
        .cache_delete("market", &asset_name)
        .await
        .with_trace_id(&trace_id)
}
