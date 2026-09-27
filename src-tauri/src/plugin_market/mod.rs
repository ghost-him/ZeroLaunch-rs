//! 插件市场 — GitHub topic 仓库发现、最新发布解析与插件包下载。
//!
//! 纯网络/解析域：不依赖 plugin_framework（安装编排在 commands 层组合
//! PluginManager），只提供 GitHub REST 客户端与可离线的纯解析函数；
//! 发布侧元数据（清单元文 / 图标）与发布包内的清单共用 `plugin-protocol`
//! 的 `Manifest`，图标载荷编码复用 `plugin-host::icon` 的单点实现。
//!
//! 两条互不依赖的数据链：
//! - **安装/预检**（本模块）：`api.github.com` REST，消耗匿名额度，产出 [`ReleasePackage`]。
//! - **卡片元数据**（[`card_meta`] 子模块）：`github.com` release 网页路由 + CDN，
//!   零 REST 额度，产出 [`MarketCardMeta`]。

use reqwest::Client as HttpClient;
use reqwest::StatusCode;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex as AsyncMutex;
use zerolaunch_plugin_protocol::manifest::Manifest;

mod card_meta;

/// 插件仓库 topic：`https://github.com/topics/zerolaunch-plugin` 背后的主题标签。
const TOPIC: &str = "zerolaunch-plugin";
/// 从市场列表中排除的仓库（主程序自身也打了该 topic）。
const EXCLUDED_REPO: &str = "ghost-him/ZeroLaunch-rs";
/// 插件包压缩包命名前缀：release 附件中以此开头、`.zip` 结尾的才是可安装插件包。
const PLUGIN_ZIP_PREFIX: &str = "zerolaunch-plugin";
/// 兜底头像尺寸（像素）：仓库所有者头像，插件未自带图标时用作卡片占位图。
const OWNER_AVATAR_SIZE: u32 = 96;
/// 市场 GET 响应缓存新鲜期：期内不再发起请求。
///
/// 走 `get_json` 的只剩两处：topic 搜索列表（桶限 10 次/分钟，市场页每次进入都会拉）
/// 与安装/预检的最新发布查询（同一 URL 会在“预检 → 确认安装”里连打两次）。
/// 没有这层缓存，反复进出设置页或反复确认安装就会把匿名额度打空。
const GET_CACHE_TTL: Duration = Duration::from_secs(60);

/// 市场 GET 响应缓存条目（仅存少量小响应：搜索列表 + 最新发布 JSON）。
struct CachedGet {
    /// 响应体。
    body: Vec<u8>,
    /// 写入时刻。
    fetched_at: Instant,
}

/// 进程内 GET 响应缓存：URL → 响应。
///
/// 失效点：TTL（[`GET_CACHE_TTL`]）到期。触发者：下一次同 URL 请求。
static GET_CACHE: LazyLock<AsyncMutex<HashMap<String, CachedGet>>> =
    LazyLock::new(|| AsyncMutex::new(HashMap::new()));
/// GitHub REST API 用户代理（API 要求显式 UA，缺省会 403）。
const USER_AGENT: &str = "ZeroLaunch-rs-plugin-market";
/// 单次 API 请求超时。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// 市场模块错误类型。
#[derive(Debug, thiserror::Error)]
pub enum MarketError {
    /// 网络请求失败（连接、超时、非 2xx 状态码等）。
    #[error("网络请求失败: {0}")]
    Network(String),
    /// GitHub 响应解析失败。
    #[error("解析 GitHub 响应失败: {0}")]
    Parse(String),
    /// 仓库不存在或无权访问（GitHub 404）。
    #[error("仓库不存在或无权访问: {0}")]
    NotFound(String),
    /// 仓库尚未有发布版本（`/releases/latest` 404）。
    #[error("该仓库暂无发布版本，无法自动安装: {0}")]
    NoRelease(String),
    /// GitHub 接口限流（匿名额度：核心 60 次/小时、搜索 10 次/分钟）。
    #[error("GitHub 接口访问频率超限: {0}")]
    RateLimited(String),
    /// 最新发布存在但缺少 `zerolaunch-plugin-*.zip` 附件。
    #[error("仓库 {0} 最新发布 {1} 中未找到 zerolaunch-plugin-*.zip 附件，无法自动安装")]
    NoPluginZip(String, String),
}

/// 插件市场条目（对应 topic 搜索结果中的一个仓库）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarketRepo {
    /// 仓库全名 `owner/name`。
    #[serde(rename = "fullName")]
    pub full_name: String,
    /// 仓库短名。
    pub name: String,
    /// 仓库描述（可能为空）。
    pub description: String,
    /// 仓库主页。
    #[serde(rename = "htmlUrl")]
    pub html_url: String,
}

/// 最新发布的插件包（安装/预检链路用；来自 GitHub REST API，消耗匿名接口额度）。
#[derive(Debug, Clone)]
pub struct ReleasePackage {
    /// 发布 tag 名（如 `v1.2.3`）。
    pub tag_name: String,
    /// 插件包附件文件名（`zerolaunch-plugin-*.zip`）。
    pub asset_name: String,
    /// 插件包附件下载地址。
    pub download_url: String,
}

/// 市场卡片元数据（IPC 载荷）：走 `github.com` 的 release 网页路由与 CDN 取，
/// **不消耗 GitHub REST API 额度** —— 匿名额度（核心 60 次/小时）按出口 IP 计，
/// 走系统代理的用户可能与其他人共享出口 IP 而被拖垮，卡片信息不该依赖它。
///
/// 元数据来源是发布 CI 随 zip 一同上传的 release 附件（`manifest.toml` 与清单
/// `[icon].path` 同名图标附件）：旧版本发布未附带这些附件时 manifest 为 None，
/// 卡片退化为仓库描述 + 所有者头像，安装不受影响。
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarketCardMeta {
    /// 最新发布 tag 名；`None` = 该仓库没有任何发布版本（安装按钮置灰）。
    #[serde(rename = "tagName")]
    pub tag_name: Option<String>,
    /// 发布侧 `manifest.toml` 附件解析出的清单；附件缺失或解析失败为 None。
    #[serde(rename = "manifest")]
    pub manifest: Option<Manifest>,
    /// 发布侧 `manifest.toml` 附件存在但不可用时的原因（解析失败等），供卡片直接提示；
    /// 附件缺失（旧版本发布）与解析正常均为 None。
    #[serde(rename = "metadataError")]
    pub metadata_error: Option<String>,
    /// 插件图标 data URL（清单声明 `[icon]` 且同名附件存在时）。
    #[serde(rename = "icon")]
    pub icon: Option<String>,
    /// 仓库所有者头像 data URL：插件未提供图标时的兜底占位图。
    #[serde(rename = "ownerAvatar")]
    pub owner_avatar: Option<String>,
}

// ── GitHub REST API 响应模型（私有） ────────────────────────────

#[derive(Deserialize)]
struct SearchResponse {
    items: Vec<SearchRepo>,
}

#[derive(Deserialize)]
struct SearchRepo {
    full_name: String,
    name: String,
    description: Option<String>,
    html_url: String,
}

#[derive(Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

/// GitHub 客户端（REST API + release 网页路由/CDN）。
pub struct GitHubMarketClient {
    /// 常规客户端：跟随 302 跳转（附件下载会跳到对象存储）。
    client: HttpClient,
    /// 不跟随跳转：`/releases/latest` 只需要读 302 的 Location 取 tag，
    /// 跟随会白拉一个 HTML 页面。
    probe_client: HttpClient,
}

impl Default for GitHubMarketClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubMarketClient {
    /// 创建客户端（带 UA 与超时）。
    pub fn new() -> Self {
        Self {
            client: build_http_client(reqwest::redirect::Policy::default()),
            probe_client: build_http_client(reqwest::redirect::Policy::none()),
        }
    }

    /// 拉取 topic 下的插件仓库列表（排除主程序仓库）。
    pub async fn list_repos(&self) -> Result<Vec<MarketRepo>, MarketError> {
        let url = format!(
            "https://api.github.com/search/repositories?q=topic:{}&per_page=100",
            TOPIC
        );
        let body = self.get_json(&url).await?;
        parse_repos(&body)
    }

    /// 解析仓库最新发布并定位插件包附件（安装/预检链路用，走 GitHub REST API）。
    ///
    /// 无发布版本（API 404）→ `NoRelease`；发布存在但无 `zerolaunch-plugin-*.zip`
    /// 附件 → `NoPluginZip`。卡片展示用的元数据不走这里（见 [`Self::get_card_meta`]）。
    pub async fn get_release(&self, full_name: &str) -> Result<ReleasePackage, MarketError> {
        let url = format!("https://api.github.com/repos/{}/releases/latest", full_name);
        let body = match self.get_json(&url).await {
            Ok(body) => body,
            // `/releases/latest` 对"无任何发布"的仓库返回 404（不是 200 + 空列表）：
            // 映射为"暂无发布版本"，而不是当成仓库不存在
            Err(MarketError::NotFound(_)) => {
                return Err(MarketError::NoRelease(full_name.to_string()))
            }
            Err(e) => return Err(e),
        };
        let release: ReleaseResponse =
            serde_json::from_slice(&body).map_err(|e| MarketError::Parse(e.to_string()))?;
        let asset = pick_plugin_zip(&release.assets).ok_or_else(|| {
            MarketError::NoPluginZip(full_name.to_string(), release.tag_name.clone())
        })?;
        Ok(ReleasePackage {
            tag_name: release.tag_name,
            asset_name: asset.name.clone(),
            download_url: asset.browser_download_url.clone(),
        })
    }

    /// 下载附件内容（整包读入内存后返回；插件 zip 体量小）。
    /// 落盘由调用方经 PluginHandle 缓存接口完成，避免在此重复路径解析/建目录。
    pub async fn download(&self, download_url: &str) -> Result<Vec<u8>, MarketError> {
        let response = self
            .client
            .get(download_url)
            .send()
            .await
            .map_err(|e| MarketError::Network(e.to_string()))?
            .error_for_status()
            .map_err(|e| MarketError::Network(format!("下载附件失败: {}", e)))?;
        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| MarketError::Network(format!("读取下载内容失败: {}", e)))
    }

    /// 下载可有可无的附件：200 → 内容；404 → None（该 tag 没有这个附件）；
    /// 限流 → `RateLimited`；其他非 2xx → 错误。
    async fn download_optional(&self, url: &str) -> Result<Option<Vec<u8>>, MarketError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| MarketError::Network(e.to_string()))?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if is_rate_limited(status, response.headers()) {
            return Err(MarketError::RateLimited(rate_limit_hint(
                response.headers(),
            )));
        }
        let response = response
            .error_for_status()
            .map_err(|e| MarketError::Network(format!("下载附件失败: {}", e)))?;
        response
            .bytes()
            .await
            .map(|b| Some(b.to_vec()))
            .map_err(|e| MarketError::Network(format!("读取下载内容失败: {}", e)))
    }

    /// 发送 GET 请求并校验状态码，返回响应体字节。
    ///
    /// [`GET_CACHE_TTL`] 内同 URL 直接回放缓存，避免反复进出市场页/反复确认安装
    /// 把 GitHub 匿名额度（核心 60 次/小时、搜索 10 次/分钟）打空。
    async fn get_json(&self, url: &str) -> Result<Vec<u8>, MarketError> {
        // 一次加锁取出缓存体，避免分离式加锁读到不同版本
        let cached = {
            let guard = GET_CACHE.lock().await;
            guard
                .get(url)
                .filter(|entry| entry.fetched_at.elapsed() < GET_CACHE_TTL)
                .map(|entry| entry.body.clone())
        };
        if let Some(body) = cached {
            return Ok(body);
        }

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| MarketError::Network(e.to_string()))?;
        let status = response.status();

        if !status.is_success() {
            if is_rate_limited(status, response.headers()) {
                return Err(MarketError::RateLimited(rate_limit_hint(
                    response.headers(),
                )));
            }
            if status == StatusCode::NOT_FOUND {
                return Err(MarketError::NotFound(url.to_string()));
            }
            let body = response.text().await.unwrap_or_default();
            return Err(MarketError::Network(format!(
                "GitHub API 响应状态 {}（{}）",
                status.as_u16(),
                github_error_message(&body).unwrap_or_else(|| url.to_string())
            )));
        }

        let body = response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| MarketError::Network(format!("读取响应失败: {}", e)))?;
        GET_CACHE.lock().await.insert(
            url.to_string(),
            CachedGet {
                body: body.clone(),
                fetched_at: Instant::now(),
            },
        );
        Ok(body)
    }
}

// ── 纯解析函数（可离线单测） ───────────────────────────────────

/// 判断仓库是否应从市场列表排除（主程序仓库自身）。
fn is_excluded_repo(full_name: &str) -> bool {
    full_name == EXCLUDED_REPO
}

/// 从 topic 搜索响应 JSON 解析仓库列表，过滤主程序仓库。
fn parse_repos(body: &[u8]) -> Result<Vec<MarketRepo>, MarketError> {
    let resp: SearchResponse =
        serde_json::from_slice(body).map_err(|e| MarketError::Parse(e.to_string()))?;
    Ok(resp
        .items
        .into_iter()
        .filter(|r| !is_excluded_repo(&r.full_name))
        .map(|r| MarketRepo {
            full_name: r.full_name,
            name: r.name,
            description: r.description.unwrap_or_default(),
            html_url: r.html_url,
        })
        .collect())
}

/// 从附件列表中挑选首个插件包附件。
///
/// 匹配大小写不敏感（发布者可能写成 `ZeroLaunch-plugin-*.zip` 等），
/// 条件：`zerolaunch-plugin` 开头且 `.zip` 结尾（字节级 ASCII 比较，零分配）。
fn pick_plugin_zip(assets: &[ReleaseAsset]) -> Option<&ReleaseAsset> {
    assets.iter().find(|a| {
        let name = a.name.as_bytes();
        name.len() >= PLUGIN_ZIP_PREFIX.len() + 4
            && name[..PLUGIN_ZIP_PREFIX.len()].eq_ignore_ascii_case(PLUGIN_ZIP_PREFIX.as_bytes())
            && name[name.len() - 4..].eq_ignore_ascii_case(b".zip")
    })
}

/// 构建 GitHub HTTP 客户端（统一 UA 与超时；跳转策略由调用方指定）。
fn build_http_client(redirect: reqwest::redirect::Policy) -> HttpClient {
    HttpClient::builder()
        .user_agent(USER_AGENT)
        .timeout(REQUEST_TIMEOUT)
        .redirect(redirect)
        .build()
        .expect("构建 GitHub HTTP 客户端失败")
}

/// 读取响应头字符串值（缺失或非 ASCII 返回 None）。
fn header_str<'a>(headers: &'a reqwest::header::HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

/// 判断响应是否为 GitHub 限流：429，或 403 且额度用尽 / 带 `Retry-After`（次级限流）。
fn is_rate_limited(status: StatusCode, headers: &reqwest::header::HeaderMap) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS
        || header_str(headers, "x-ratelimit-remaining") == Some("0")
        || header_str(headers, "retry-after").is_some()
}

/// 生成限流提示：优先 `Retry-After`（秒），其次按 `X-RateLimit-Reset`（epoch 秒）算剩余等待。
fn rate_limit_hint(headers: &reqwest::header::HeaderMap) -> String {
    if let Some(seconds) = header_str(headers, "retry-after").and_then(|v| v.parse::<u64>().ok()) {
        return format!("建议 {} 秒后重试", seconds);
    }
    let reset = header_str(headers, "x-ratelimit-reset").and_then(|v| v.parse::<u64>().ok());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    match reset {
        Some(reset) if reset > now => {
            format!(
                "建议约 {} 分钟后重试",
                reset.saturating_sub(now).div_ceil(60)
            )
        }
        _ => "建议稍后重试".to_string(),
    }
}

/// 从 GitHub 错误响应体（`{"message": "..."}`）取人类可读原因，取不到返回 None。
fn github_error_message(body: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let message = parsed.get("message")?.as_str()?;
    // 次级限流的说明可能很长，卡片只展示前 200 字符
    Some(message.chars().take(200).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_json(full_name: &str, name: &str, description: &str) -> String {
        format!(
            r#"{{"full_name":"{}","name":"{}","description":{},"html_url":"https://github.com/{}"}}"#,
            full_name,
            name,
            if description.is_empty() {
                "null".to_string()
            } else {
                format!("\"{}\"", description)
            },
            full_name
        )
    }

    #[test]
    fn parse_repos_filters_main_repo() {
        let body = format!(
            r#"{{"total_count":3,"incomplete_results":false,"items":[{}]}}"#,
            [
                repo_json("ghost-him/ZeroLaunch-rs", "ZeroLaunch-rs", "main app"),
                repo_json("alice/everything-plugin", "everything-plugin", "everything"),
                repo_json("bob/ai-search", "ai-search", "semantic"),
            ]
            .join(",")
        );
        let repos = parse_repos(body.as_bytes()).expect("parse ok");
        assert_eq!(repos.len(), 2);
        assert!(repos
            .iter()
            .all(|r| r.full_name != "ghost-him/ZeroLaunch-rs"));
        assert_eq!(repos[0].full_name, "alice/everything-plugin");
        assert_eq!(repos[1].description, "semantic");
    }

    #[test]
    fn parse_repos_null_description() {
        let body = format!(
            r#"{{"total_count":1,"incomplete_results":false,"items":[{}]}}"#,
            repo_json("alice/no-desc", "no-desc", "")
        );
        let repos = parse_repos(body.as_bytes()).expect("parse ok");
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].description, "");
    }

    fn asset(name: &str) -> String {
        format!(
            r#"{{"name":"{}","browser_download_url":"https://github.com/x/y/releases/download/t/{}"}}"#,
            name, name
        )
    }

    #[test]
    fn pick_plugin_zip_prefers_plugin_zip() {
        let body = format!(
            r#"{{"tag_name":"v1.2.3","assets":[{}]}}"#,
            [
                asset("README.txt"),
                asset("zerolaunch-plugin-everything-v1.2.3.zip"),
                asset("zerolaunch-plugin-everything-v1.2.3.zip.sha256"),
            ]
            .join(",")
        );
        let release: ReleaseResponse = serde_json::from_slice(body.as_bytes()).expect("parse ok");
        let zip = pick_plugin_zip(&release.assets).expect("zip found");
        assert_eq!(zip.name, "zerolaunch-plugin-everything-v1.2.3.zip");
    }

    #[test]
    fn pick_plugin_zip_none_without_plugin_zip() {
        // 有 release 但附件不含插件包：README + 非本前缀 zip + 校验和
        let body = format!(
            r#"{{"tag_name":"v1.0.0","assets":[{}]}}"#,
            [
                asset("README.txt"),
                asset("my-plugin.zip"),
                asset("zerolaunch-plugin-foo.zip.sha256"),
                asset("source.tar.gz"),
            ]
            .join(",")
        );
        let release: ReleaseResponse = serde_json::from_slice(body.as_bytes()).expect("parse ok");
        assert!(pick_plugin_zip(&release.assets).is_none());
    }

    #[test]
    fn pick_plugin_zip_matches_case_insensitively() {
        // 发布者大小写变体：前缀/后缀任意大小写都应收录
        let cases = [
            "ZeroLaunch-plugin-everything-v1.0.0.ZIP",
            "zerolaunch-plugin-everything-v1.0.0.zip",
            "ZEROLAUNCH-PLUGIN-everything-v1.0.0.Zip",
        ];
        for name in cases {
            let body = format!(r#"{{"tag_name":"v1.0.0","assets":[{}]}}"#, asset(name));
            let release: ReleaseResponse =
                serde_json::from_slice(body.as_bytes()).expect("parse ok");
            let zip = pick_plugin_zip(&release.assets).unwrap_or_else(|| panic!("应匹配 {}", name));
            assert_eq!(zip.name, name);
        }
    }

    #[test]
    fn pick_plugin_zip_matches_first_in_mixed_case_list() {
        // 列表中大小写混合：取首个大小写不敏感匹配项
        let body = format!(
            r#"{{"tag_name":"v1.0.0","assets":[{}]}}"#,
            [
                asset("readme.txt"),
                asset("ZeroLaunch-plugin-foo-v1.0.0.zip"),
                asset("zerolaunch-plugin-bar-v1.0.0.zip"),
            ]
            .join(",")
        );
        let release: ReleaseResponse = serde_json::from_slice(body.as_bytes()).expect("parse ok");
        let zip = pick_plugin_zip(&release.assets).expect("zip found");
        assert_eq!(zip.name, "ZeroLaunch-plugin-foo-v1.0.0.zip");
    }

    fn headers(pairs: &[(&str, &str)]) -> reqwest::header::HeaderMap {
        let mut map = reqwest::header::HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                reqwest::header::HeaderName::from_bytes(name.as_bytes()).expect("合法头名"),
                value.parse().expect("合法头值"),
            );
        }
        map
    }

    #[test]
    fn rate_limit_detection_covers_quota_and_secondary_limits() {
        // 额度用尽：403 + remaining 0（搜索桶与核心桶都这样表达）
        assert!(is_rate_limited(
            StatusCode::FORBIDDEN,
            &headers(&[("x-ratelimit-remaining", "0")])
        ));
        // 次级限流：429 或带 Retry-After
        assert!(is_rate_limited(
            StatusCode::TOO_MANY_REQUESTS,
            &headers(&[])
        ));
        assert!(is_rate_limited(
            StatusCode::FORBIDDEN,
            &headers(&[("retry-after", "60")])
        ));
        // 普通 403（无权限等）与 404 不算限流
        assert!(!is_rate_limited(
            StatusCode::FORBIDDEN,
            &headers(&[("x-ratelimit-remaining", "42")])
        ));
        assert!(!is_rate_limited(StatusCode::NOT_FOUND, &headers(&[])));
    }

    #[test]
    fn rate_limit_hint_prefers_retry_after_then_reset() {
        assert_eq!(
            rate_limit_hint(&headers(&[("retry-after", "30")])),
            "建议 30 秒后重试"
        );
        let future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时间正常")
            .as_secs()
            + 125;
        assert_eq!(
            rate_limit_hint(&headers(&[("x-ratelimit-reset", &future.to_string())])),
            "建议约 3 分钟后重试"
        );
        // 重置时间已过或缺头：退化为通用提示
        assert_eq!(rate_limit_hint(&headers(&[])), "建议稍后重试");
    }

    #[test]
    fn github_error_message_extracts_and_truncates() {
        assert_eq!(
            github_error_message(r#"{"message":"Not Found"}"#).as_deref(),
            Some("Not Found")
        );
        assert_eq!(github_error_message("not json"), None);
        assert_eq!(github_error_message(r#"{"other":1}"#), None);
        let long = format!(r#"{{"message":"{}"}}"#, "x".repeat(300));
        assert_eq!(github_error_message(&long).map(|m| m.len()), Some(200));
    }
}
