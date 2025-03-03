use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

pub struct VersionChecker {}

impl VersionChecker {
    /// 获得当前软件最新的版本
    pub async fn get_latest_release_version() -> Result<String> {
        // 硬编码仓库信息
        const OWNER: &str = "ghost-him";
        const REPO: &str = "ZeroLaunch-rs";
        let client = Client::new();
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            OWNER, REPO
        );
        let response = client
            .get(&url)
            .header("User-Agent", "ZeroLaunch-rs-Version-Checker")
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch GitHub release: HTTP {}",
                response.status()
            ));
        }
        let release: GitHubRelease = response.json().await?;
        Ok(release.tag_name)
    }
}
