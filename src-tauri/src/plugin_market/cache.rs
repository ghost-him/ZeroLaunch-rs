//! 市场数据的本地磁盘缓存：24h 时限 + 手动刷新越过时限。
//!
//! 落在 core PluginHandle 的缓存域（`<AppCacheDir>/core/<domain>/<key>`，即
//! `plugin-cache/` 下），不经 StorageService。GitHub 匿名额度按**出口 IP** 计：
//! 走系统代理的用户与他人共享出口 IP，市场页每次进入都实时请求极易被拖垮
//! （issue #93），故列表与卡片元数据都先读缓存。
//!
//! 缓存即副本：失效点 = [`MARKET_CACHE_TTL_SECS`] 到期，或调用方传 force 绕过；
//! 触发者 = 下一次市场命令调用（无后台定时器）。`cache_*` 句柄没有修改时间读取
//! 接口，抓取时刻只能内嵌在载荷信封（[`CacheEnvelope`]）里。

use super::card_meta::is_valid_owner_login;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use zerolaunch_plugin_api::host::PluginHandle;

/// 市场数据缓存新鲜期：24 小时。
pub const MARKET_CACHE_TTL_SECS: u64 = 24 * 60 * 60;

/// 落盘信封 —— 数据本体 + 抓取时刻。
///
/// 仅本模块读写，不经 IPC 出网，故字段名不做 serde rename。
#[derive(Serialize, Deserialize)]
struct CacheEnvelope<T> {
    /// 抓取时刻（unix 秒），TTL 判定与前端展示"最后更新"均用它。
    fetched_at: u64,
    /// 数据本体（列表或单个仓库的卡片元数据）。
    data: T,
}

/// 当前 unix 秒；系统时钟早于 1970 时按 0 处理。
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 抓取时刻是否仍在新鲜期内（`now` 显式传入便于单测）。
///
/// 未来时间戳（时钟回拨、跨机拷贝缓存文件）按"新鲜"处理，避免被判过期后反复请求。
fn is_fresh_at(fetched_at: u64, now: u64) -> bool {
    now.saturating_sub(fetched_at) < MARKET_CACHE_TTL_SECS
}

/// 抓取时刻是否仍在新鲜期内（TTL = [`MARKET_CACHE_TTL_SECS`]）。
pub fn is_fresh(fetched_at: u64) -> bool {
    is_fresh_at(fetched_at, now_unix())
}

/// 读取缓存条目（不判新鲜期）→ 数据本体 + 抓取时刻（unix 秒）。
///
/// 缺失、读取失败、信封损坏均返回 None（调用方按未缓存处理）；读取失败与解析
/// 失败只告警——缓存是优化，不能把失败升级成命令错误。
pub async fn read<T: DeserializeOwned>(
    handle: &PluginHandle,
    domain: &str,
    key: &str,
) -> Option<(T, u64)> {
    let bytes = match handle.cache_get(domain, key).await {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return None,
        Err(e) => {
            tracing::warn!("读取市场缓存 {}/{} 失败: {}", domain, key, e);
            return None;
        }
    };
    match serde_json::from_slice::<CacheEnvelope<T>>(&bytes) {
        Ok(envelope) => Some((envelope.data, envelope.fetched_at)),
        Err(e) => {
            tracing::warn!("市场缓存 {}/{} 解析失败，按未缓存处理: {}", domain, key, e);
            None
        }
    }
}

/// 写入缓存条目（整块覆盖，内嵌当前时刻）；失败只告警，不影响本次结果。
pub async fn write<T: Serialize>(handle: &PluginHandle, domain: &str, key: &str, data: &T) {
    let envelope = CacheEnvelope {
        fetched_at: now_unix(),
        data,
    };
    let bytes = match serde_json::to_vec(&envelope) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!("序列化市场缓存 {}/{} 失败: {}", domain, key, e);
            return;
        }
    };
    if let Err(e) = handle.cache_put(domain, key, &bytes).await {
        tracing::warn!("写入市场缓存 {}/{} 失败: {}", domain, key, e);
    }
}

/// 仓库全名 → 卡片元数据缓存键 `<owner>__<repo>.json`；不合法时返回 None（不缓存）。
///
/// owner 登录名不允许下划线，故首个 `__` 即分隔符，键与仓库全名一一对应，不会串号。
pub fn repo_key(full_name: &str) -> Option<String> {
    let (owner, repo) = full_name.split_once('/')?;
    if !is_valid_owner_login(owner) || !is_valid_repo_name(repo) {
        return None;
    }
    Some(format!("{}__{}.json", owner, repo))
}

/// 校验仓库短名（`[A-Za-z0-9._-]`，1..=100 字符，且不为 `.`/`..`）：仅用于拼装缓存键。
fn is_valid_repo_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_fresh_at_boundary() {
        let now = 1_000_000_000;
        assert!(is_fresh_at(now, now), "刚写入应为新鲜");
        assert!(
            is_fresh_at(now - MARKET_CACHE_TTL_SECS + 1, now),
            "TTL 前 1 秒仍新鲜"
        );
        assert!(
            !is_fresh_at(now - MARKET_CACHE_TTL_SECS, now),
            "恰好 TTL 应过期"
        );
        assert!(
            is_fresh_at(now + 3600, now),
            "未来时间戳（时钟回拨）按新鲜处理"
        );
    }

    #[test]
    fn repo_key_maps_valid_full_name() {
        assert_eq!(
            repo_key("ghost-him/ZeroLaunch-plugin-ai-search").as_deref(),
            Some("ghost-him__ZeroLaunch-plugin-ai-search.json")
        );
        assert_eq!(
            repo_key("a1/B.c_d-e").as_deref(),
            Some("a1__B.c_d-e.json"),
            "短名允许点、下划线与连字符"
        );
    }

    #[test]
    fn repo_key_rejects_unsafe_full_name() {
        for bad in [
            "",
            "ghost-him",
            "ghost-him/",
            "/repo",
            "ghost-him/../etc",
            "ghost-him/a/b",
            "ghost_him/repo",
            "ghost-him/.",
            "ghost-him/..",
        ] {
            assert_eq!(repo_key(bad), None, "`{}` 不应被用作缓存键", bad);
        }
    }
}
