//! 市场卡片元数据：走 `github.com` 的 release 网页路由与 CDN，**不消耗 REST API 额度**。
//!
//! 与 [`super`]（REST 客户端 + 安装/预检支撑）的分工：这里只负责“卡片要展示什么”，
//! 数据来自发布 CI 随 zip 一同上传的 release 附件（`manifest.toml` 与清单
//! `[icon].path` 同名图标附件）。之所以绕开 REST：匿名额度按**出口 IP** 计
//! （核心 60 次/小时），走系统代理的用户可能与其他人共享出口 IP 而被拖垮，
//! 而卡片信息是市场页每次进入都要拉的。
//!
//! 展示元数据是**尽力而为**：缺失/失败只告警并置 None 或原因，绝不阻断安装。

use super::{header_str, GitHubMarketClient, MarketCardMeta, MarketError, OWNER_AVATAR_SIZE};
use reqwest::StatusCode;
use std::path::Path;
use tracing::warn;
use zerolaunch_plugin_protocol::manifest::Manifest;

/// 发布侧市场元数据附件固定名：清单元文（由插件仓发布 CI 随 zip 一同上传）。
const MANIFEST_ASSET_NAME: &str = "manifest.toml";

impl GitHubMarketClient {
    /// 取仓库最新发布的卡片元数据（tag + 清单 + 图标/兜底头像）。
    ///
    /// ① `/releases/latest` 的 302 Location 给出 tag（跳到 `/releases` 即无发布版本）；
    /// ② `releases/download/<tag>/manifest.toml` 取清单元数据附件（404 = 旧版本发布未附带）；
    /// ③ 清单声明 `[icon]` 时取同名图标附件；④ 无图标时取仓库所有者头像兜底。
    pub async fn get_card_meta(&self, full_name: &str) -> Result<MarketCardMeta, MarketError> {
        let Some(tag_name) = self.fetch_latest_tag(full_name).await? else {
            // 无任何发布版本：卡片只显示仓库自身信息，安装按钮由前端置灰
            return Ok(MarketCardMeta {
                tag_name: None,
                manifest: None,
                metadata_error: None,
                icon: None,
                owner_avatar: None,
            });
        };

        let (manifest, metadata_error) = match self.fetch_manifest_asset(full_name, &tag_name).await
        {
            Ok(manifest) => (manifest, None),
            Err(reason) => (None, Some(reason)),
        };
        let icon = match manifest.as_ref().and_then(|m| m.icon.as_ref()) {
            Some(section) => {
                self.fetch_icon_asset(full_name, &tag_name, &section.path)
                    .await
            }
            None => None,
        };
        let owner_avatar = if icon.is_some() {
            None
        } else {
            self.fetch_owner_avatar(full_name).await
        };

        Ok(MarketCardMeta {
            tag_name: Some(tag_name),
            manifest,
            metadata_error,
            icon,
            owner_avatar,
        })
    }

    /// 读取最新发布 tag：`/releases/latest` 302 到 `/releases/tag/<tag>`；
    /// 无任何发布版本的仓库跳到 `/releases`（解析不出 tag）→ `Ok(None)`。
    async fn fetch_latest_tag(&self, full_name: &str) -> Result<Option<String>, MarketError> {
        let url = format!("https://github.com/{}/releases/latest", full_name);
        // probe_client 不跟随 302：只为读 Location（跟随会白拉一个 HTML 页面）
        let response = self
            .probe_client
            .get(&url)
            .send()
            .await
            .map_err(|e| MarketError::Network(e.to_string()))?;
        let status = response.status();
        if status.is_redirection() {
            return Ok(
                header_str(response.headers(), "location").and_then(tag_from_release_location)
            );
        }
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(MarketError::Network(format!(
                "读取最新发布失败: GitHub 响应状态 {}",
                status.as_u16()
            )));
        }
        // 200 = 该 URL 未被重定向（异常形态），无法确定 tag
        Err(MarketError::Parse(format!(
            "无法从 {} 解析最新发布 tag",
            url
        )))
    }

    /// 下载发布侧 `manifest.toml` 元数据附件并解析为清单。
    ///
    /// `Ok(None)` = 该 tag 没有这个附件（旧版本发布，属预期）；`Err(原因)` = 附件
    /// 存在但下载/解析失败（清单与该宿主不兼容等）——卡片据此区分“没有元数据”
    /// 与“元数据不可用”。
    async fn fetch_manifest_asset(
        &self,
        full_name: &str,
        tag_name: &str,
    ) -> Result<Option<Manifest>, String> {
        let url = release_asset_url(full_name, tag_name, MANIFEST_ASSET_NAME);
        let Some(bytes) = self
            .download_optional(&url)
            .await
            .map_err(|e| format!("下载元数据附件 {} 失败: {}", MANIFEST_ASSET_NAME, e))?
        else {
            return Ok(None);
        };
        match parse_manifest(&bytes) {
            Ok(manifest) => Ok(Some(manifest)),
            Err(e) => {
                warn!("解析市场元数据附件 {} 失败: {}", MANIFEST_ASSET_NAME, e);
                Err(e)
            }
        }
    }

    /// 下载清单 `[icon].path` 同名图标附件并转为 data URL；无该附件/超限返回 None。
    async fn fetch_icon_asset(
        &self,
        full_name: &str,
        tag_name: &str,
        icon_path: &str,
    ) -> Option<String> {
        let basename = icon_basename(icon_path)?;
        let url = release_asset_url(full_name, tag_name, basename);
        let bytes = match self.download_optional(&url).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return None,
            Err(e) => {
                warn!("下载市场图标附件 {} 失败: {}", basename, e);
                return None;
            }
        };
        let mime = zerolaunch_plugin_host::icon::mime_from_extension(Path::new(icon_path));
        zerolaunch_plugin_host::icon::to_data_url(mime, &bytes)
    }

    /// 拉取仓库所有者头像作为卡片兜底占位图（`https://github.com/<owner>.png`）。
    async fn fetch_owner_avatar(&self, full_name: &str) -> Option<String> {
        let owner = full_name.split('/').next().unwrap_or_default();
        // 仓库全名来自前端参数：登录名格式校验顺带挡住 URL 路径注入（非法即放弃兜底图）
        if !is_valid_owner_login(owner) {
            return None;
        }
        let url = format!(
            "https://github.com/{}.png?size={}",
            owner, OWNER_AVATAR_SIZE
        );
        let bytes = match self.download_optional(&url).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return None,
            Err(e) => {
                warn!("下载市场兜底头像 {} 失败: {}", owner, e);
                return None;
            }
        };
        zerolaunch_plugin_host::icon::to_data_url("image/png", &bytes)
    }
}

// ── 纯解析函数（可离线单测） ───────────────────────────────────

/// 从 `/releases/latest` 的 302 Location 解析 tag：`…/releases/tag/v1.2.3` → `v1.2.3`；
/// 无发布版本的仓库跳到 `…/releases`（无 `/tag/` 段）→ None。
///
/// 返回 GitHub 给出的原文（已 URL 编码），可直接用于 `releases/download/<tag>/…` 拼装。
fn tag_from_release_location(location: &str) -> Option<String> {
    let (_, tag) = location.split_once("/releases/tag/")?;
    let tag = tag.split(['?', '#']).next().unwrap_or(tag);
    if tag.is_empty() {
        None
    } else {
        Some(tag.to_string())
    }
}

/// 发布附件下载地址（tag 级路由）。
///
/// 用 tag 级而非 `releases/latest/download/<附件>`：后者在附件缺失时返回 302，
/// 而 tag 级路由稳定返回 404，宿主才能区分“该 tag 没有这个附件”。
fn release_asset_url(full_name: &str, tag_name: &str, asset_name: &str) -> String {
    format!(
        "https://github.com/{}/releases/download/{}/{}",
        full_name, tag_name, asset_name
    )
}

/// 清单 `[icon].path` 对应的图标附件名：取路径末段（发布侧以文件名上传附件，
/// Release 附件名不含目录分隔符）；路径为空返回 None。
fn icon_basename(icon_path: &str) -> Option<&str> {
    let basename = icon_path.rsplit(['/', '\\']).next().unwrap_or(icon_path);
    if basename.is_empty() {
        None
    } else {
        Some(basename)
    }
}

/// 校验仓库所有者登录名（`[A-Za-z0-9-]`，1..=39 字符）：仅用于拼装兜底头像 URL。
fn is_valid_owner_login(login: &str) -> bool {
    !login.is_empty()
        && login.len() <= 39
        && login
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// 解析发布侧 `manifest.toml` 附件字节为清单，错误返回人类可读原因（调用方只告警）。
fn parse_manifest(bytes: &[u8]) -> Result<Manifest, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("清单不是合法 UTF-8: {}", e))?;
    toml::from_str::<Manifest>(text).map_err(|e| format!("清单 TOML 解析失败: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_from_release_location_handles_release_and_empty_repo() {
        assert_eq!(
            tag_from_release_location(
                "https://github.com/ghost-him/ZeroLaunch-plugin-everything/releases/tag/v0.1.0"
            )
            .as_deref(),
            Some("v0.1.0")
        );
        // 无任何发布版本：GitHub 跳到 /releases（无 /tag/ 段）
        assert_eq!(
            tag_from_release_location(
                "https://github.com/ghost-him/ZeroLaunch-plugin-template/releases"
            ),
            None
        );
        // 带查询串/片段时只取 tag 段
        assert_eq!(
            tag_from_release_location("https://github.com/o/r/releases/tag/v1.0.0?x=1").as_deref(),
            Some("v1.0.0")
        );
        assert_eq!(tag_from_release_location("/releases/tag/"), None);
    }

    #[test]
    fn release_asset_url_uses_tag_scoped_route() {
        assert_eq!(
            release_asset_url("o/r", "v1.0.0", "manifest.toml"),
            "https://github.com/o/r/releases/download/v1.0.0/manifest.toml"
        );
    }

    #[test]
    fn icon_basename_takes_last_segment() {
        assert_eq!(icon_basename("icon.svg"), Some("icon.svg"));
        assert_eq!(icon_basename("assets/brand/logo.png"), Some("logo.png"));
        assert_eq!(icon_basename("assets\\logo.png"), Some("logo.png"));
        assert_eq!(icon_basename(""), None);
        assert_eq!(icon_basename("assets/"), None);
    }

    #[test]
    fn owner_login_validation_rejects_url_injection() {
        assert!(is_valid_owner_login("ghost-him"));
        assert!(is_valid_owner_login("a1-2B"));
        assert!(!is_valid_owner_login(""));
        assert!(!is_valid_owner_login("../evil"));
        assert!(!is_valid_owner_login("a?x=1"));
        assert!(!is_valid_owner_login(&"a".repeat(40)));
    }

    const SAMPLE_MANIFEST: &str = r#"
[plugin]
id = "com.example.sample"
name = "Sample"
version = "1.2.3"
description = "样例"
author = "tester"
mode = "panel"
triggerKeywords = ["sample"]
supportedOs = ["windows"]
priority = 100

[runtime]
command = "bin/sample.exe"

[icon]
path = "icon.svg"
"#;

    #[test]
    fn parse_manifest_reads_release_manifest_asset() {
        let manifest = parse_manifest(SAMPLE_MANIFEST.as_bytes()).expect("解析成功");
        assert_eq!(manifest.plugin.id, "com.example.sample");
        assert_eq!(manifest.plugin.version, "1.2.3");
        assert_eq!(
            manifest.icon.as_ref().map(|i| i.path.as_str()),
            Some("icon.svg")
        );
    }

    #[test]
    fn parse_manifest_reports_invalid_input() {
        // 非 UTF-8 与非法 TOML 都必须报错（调用方据此丢弃元数据，不阻断安装）
        assert!(parse_manifest(&[0xff, 0xfe]).is_err());
        assert!(parse_manifest(b"not = = toml").is_err());
    }
}
