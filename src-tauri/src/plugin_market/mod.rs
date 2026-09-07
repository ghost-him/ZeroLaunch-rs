//! 插件市场 — GitHub topic 仓库发现、最新发布解析与插件包下载。
//!
//! 纯网络/解析域：不依赖 plugin_framework（安装编排在 commands 层组合
//! PluginManager），只提供 GitHub REST 客户端与可离线的纯解析函数。

use reqwest::Client as HttpClient;
use reqwest::StatusCode;
use serde::Deserialize;
use std::time::Duration;

/// 插件仓库 topic：`https://github.com/topics/zerolaunch-plugin` 背后的主题标签。
const TOPIC: &str = "zerolaunch-plugin";
/// 从市场列表中排除的仓库（主程序自身也打了该 topic）。
const EXCLUDED_REPO: &str = "ghost-him/ZeroLaunch-rs";
/// 插件包压缩包命名前缀：release 附件中以此开头、`.zip` 结尾的才是可安装插件包。
const PLUGIN_ZIP_PREFIX: &str = "zerolaunch-plugin";
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
    /// 仓库尚无任何发布版本（`/releases/latest` 404）。
    #[error("该仓库暂无发布版本，无法自动安装: {0}")]
    NoRelease(String),
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

/// 最新发布解析结果：已确保存在 `zerolaunch-plugin-*.zip` 附件。
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarketRelease {
    /// 发布 tag 名（如 `v1.2.3`）。
    #[serde(rename = "tagName")]
    pub tag_name: String,
    /// 匹配的插件包附件。
    pub asset: MarketAsset,
}

/// 插件包附件信息。
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarketAsset {
    /// 附件文件名（`zerolaunch-plugin-*.zip`）。
    pub name: String,
    /// 附件下载地址（HTTP 302 跳转到对象存储）。
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
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

/// GitHub REST API 客户端。
pub struct GitHubMarketClient {
    client: HttpClient,
}

impl Default for GitHubMarketClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubMarketClient {
    /// 创建客户端（带 UA 与超时）。
    pub fn new() -> Self {
        let client = HttpClient::builder()
            .user_agent(USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("构建 GitHub HTTP 客户端失败");
        Self { client }
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

    /// 解析仓库最新发布并挑选插件包附件。
    ///
    /// 无发布版本（API 404）→ `NoRelease`；发布存在但无 `zerolaunch-plugin-*.zip`
    /// 附件 → `NoPluginZip`。
    pub async fn get_release(&self, full_name: &str) -> Result<MarketRelease, MarketError> {
        let url = format!("https://api.github.com/repos/{}/releases/latest", full_name);
        let body = self.get_json(&url).await?;
        let release: ReleaseResponse =
            serde_json::from_slice(&body).map_err(|e| MarketError::Parse(e.to_string()))?;
        let asset = pick_plugin_zip(&release.assets).ok_or_else(|| {
            MarketError::NoPluginZip(full_name.to_string(), release.tag_name.clone())
        })?;
        Ok(MarketRelease {
            tag_name: release.tag_name,
            asset: MarketAsset {
                name: asset.name.clone(),
                download_url: asset.browser_download_url.clone(),
            },
        })
    }

    /// 下载附件内容（整包读入内存后返回；插件 zip 体量小）。
    /// 落盘由调用方经 PluginHandle 缓存接口完成，避免在此重复路径解析/建目录。
    pub async fn download(&self, asset: &MarketAsset) -> Result<Vec<u8>, MarketError> {
        let response = self
            .client
            .get(&asset.download_url)
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

    /// 发送 GET 请求并校验状态码，返回响应体字节。
    async fn get_json(&self, url: &str) -> Result<Vec<u8>, MarketError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| MarketError::Network(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            if status == StatusCode::NOT_FOUND {
                return Err(MarketError::NotFound(url.to_string()));
            }
            return Err(MarketError::Network(format!(
                "GitHub API 响应状态 {} ({} 请求失败)",
                status.as_u16(),
                url
            )));
        }
        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| MarketError::Network(format!("读取响应失败: {}", e)))
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
}
