//! 插件市场 IPC 命令。
//!
//! 业务协调：组合 GitHubMarketClient（网络/解析）与 PluginManager（安装），
//! 市场模块不直接依赖 plugin_framework，安装编排在此层完成。

use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::plugin_market::cache;
use crate::plugin_market::{GitHubMarketClient, MarketCardMeta, MarketError, MarketRepo};
use crate::state::app_state::AppState;
use std::path::Path;
use std::sync::Arc;
use tauri::State;
use zerolaunch_plugin_api::services::path::KnownPath;
use zerolaunch_plugin_host::manager::InstalledPluginInfo;
use zerolaunch_plugin_protocol::Manifest;

// ── 展示数据磁盘缓存（24h TTL，见 plugin_market::cache） ──────────

/// 列表缓存域：`plugin-cache/core/market-list/`。
const LIST_CACHE_DOMAIN: &str = "market-list";
/// 列表缓存键：整个列表一个文件。
const LIST_CACHE_KEY: &str = "index.json";
/// 卡片元数据缓存域：`plugin-cache/core/market-meta/`。
const META_CACHE_DOMAIN: &str = "market-meta";

// ── Commands ─────────────────────────────────────────────────────

/// 市场列表响应 —— 仓库列表 + 数据年龄 + 本次刷新失败原因（跨 IPC 序列化，前端
/// `MarketListResult` 一一对应）。
///
/// 列表可能来自 24h 磁盘缓存，故新鲜度必须随数据一并下发：前端据 `fetchedAt`
/// 展示"最后更新"，据 `staleReason` 提示当前展示的是旧数据（限流/断网回落）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarketListResult {
    /// 仓库列表（实时抓取或磁盘缓存）。
    #[serde(rename = "repos")]
    pub repos: Vec<MarketRepo>,
    /// 数据抓取时刻（unix 秒）：实时抓取为当前时间，回落缓存时为缓存写入时刻。
    #[serde(rename = "fetchedAt")]
    pub fetched_at: u64,
    /// 本次刷新失败而回落到缓存的原因（含限流重试提示）；实时抓取成功为 None。
    #[serde(rename = "staleReason")]
    pub stale_reason: Option<String>,
}

/// 拉取插件市场仓库列表（GitHub topic: zerolaunch-plugin，已排除主程序仓库）。
///
/// 先读 24h 磁盘缓存：非 `force` 且缓存未过期时直接命中、不发请求（`force` 为
/// 手动刷新）。刷新失败但手上有旧缓存时回落旧数据并把原因放进 `staleReason`，
/// 避免匿名额度被共享出口 IP 打空时（issue #93）市场页变成空白报错。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn market_list(
    force: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<MarketListResult, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let handle = state.get_core_handle();

    // 新鲜缓存（且非手动刷新）直接命中；过期条目留给刷新失败时回落。
    let fallback =
        match cache::read::<Vec<MarketRepo>>(&handle, LIST_CACHE_DOMAIN, LIST_CACHE_KEY).await {
            Some((repos, fetched_at)) if !force && cache::is_fresh(fetched_at) => {
                return Ok(MarketListResult {
                    repos,
                    fetched_at,
                    stale_reason: None,
                })
            }
            stale => stale,
        };

    let client = GitHubMarketClient::new();
    let result: Result<MarketListResult, MarketError> = match client.list_repos().await {
        Ok(repos) => {
            cache::write(&handle, LIST_CACHE_DOMAIN, LIST_CACHE_KEY, &repos).await;
            Ok(MarketListResult {
                repos,
                fetched_at: cache::now_unix(),
                stale_reason: None,
            })
        }
        Err(e) => match fallback {
            Some((repos, fetched_at)) => {
                tracing::warn!("市场列表刷新失败，回落展示缓存数据: {}", e);
                Ok(MarketListResult {
                    repos,
                    fetched_at,
                    stale_reason: Some(e.to_string()),
                })
            }
            None => Err(e),
        },
    };
    result.with_trace_id(&trace_id)
}

/// 查询仓库最新发布的卡片元数据（tag + 清单 + 图标/兜底头像）。
///
/// 走 github.com 的 release 网页路由与 CDN，不消耗 GitHub REST API 额度；
/// 元数据缺失（旧版本发布）不是错误，由返回结构里的 None/原因字段表达。
/// 先读 24h 磁盘缓存（键 `<owner>__<repo>.json`）：非 `force` 且未过期时直接命中。
/// 缓存只在成功时覆盖（失败不写）：任何失败都沿用上次成功结果，连缓存都没有才把
/// 本次结果（降级快照或错误）交给前端。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn market_get_meta(
    full_name: String,
    force: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<MarketCardMeta, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let handle = state.get_core_handle();

    // 仓库全名不合法（无法安全用作缓存文件名）时不做缓存，仍按实时请求返回。
    let cache_key = cache::repo_key(&full_name);
    let cached = match &cache_key {
        Some(key) => cache::read::<MarketCardMeta>(&handle, META_CACHE_DOMAIN, key).await,
        None => None,
    };
    // 新鲜缓存直接命中（手动刷新除外）；其余条目留作刷新失败时的回落。
    let cached = match cached {
        Some((meta, fetched_at)) if !force && cache::is_fresh(fetched_at) => return Ok(meta),
        other => other.map(|(meta, _)| meta),
    };

    let fresh = GitHubMarketClient::new().get_card_meta(&full_name).await;
    if let Err(e) = &fresh {
        tracing::warn!("拉取市场仓库 {} 的卡片元数据失败: {}", full_name, e);
    }
    // 只有成功结果才覆盖缓存：失败时旧数据原样保留
    if let Ok(meta) = &fresh {
        if cache::is_complete(meta) {
            if let Some(key) = &cache_key {
                cache::write(&handle, META_CACHE_DOMAIN, key, meta).await;
            }
        }
    }
    cache::pick_card_meta(fresh, cached).with_trace_id(&trace_id)
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
    let core_handle = state.get_core_handle();
    let dest = Path::new(
        &core_handle
            .resolve_path(KnownPath::AppCacheDir)
            .with_trace_id(&trace_id)?,
    )
    .join(core_handle.plugin_id())
    .join("market")
    .join(&release.asset_name);

    // 2. 命中预检暂存则直接复用；未命中才下载（整包入内存后经缓存接口落盘）。
    if !dest.exists() {
        let bytes = client
            .download(&release.download_url)
            .await
            .with_trace_id(&trace_id)?;
        core_handle
            .cache_put("market", &release.asset_name, &bytes)
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
    if let Err(e) = core_handle
        .cache_delete("market", &release.asset_name)
        .await
    {
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
    let bytes = client
        .download(&release.download_url)
        .await
        .with_trace_id(&trace_id)?;

    // 2. 经 core PluginHandle 缓存接口暂存后交给 PluginManager 解析
    //    （zip 路径复用与 market_install 相同的解析/建目录逻辑）。
    let core_handle = state.get_core_handle();
    core_handle
        .cache_put("market", &release.asset_name, &bytes)
        .await
        .with_trace_id(&trace_id)?;
    let dest = Path::new(
        &core_handle
            .resolve_path(KnownPath::AppCacheDir)
            .with_trace_id(&trace_id)?,
    )
    .join(core_handle.plugin_id())
    .join("market")
    .join(&release.asset_name);

    let preview_result = state
        .get_plugin_manager()
        .inspect_package(&dest)
        .with_trace_id(&trace_id);

    // 解析失败（包不完整/缺 manifest）时清理暂存，避免坏包残留；成功则保留供安装复用。
    if preview_result.is_err() {
        if let Err(e) = core_handle
            .cache_delete("market", &release.asset_name)
            .await
        {
            tracing::warn!("清理市场预检暂存文件失败: {}", e);
        }
    }

    let manifest = preview_result?;
    Ok(MarketPackagePreview {
        tag_name: release.tag_name,
        asset_name: release.asset_name,
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
