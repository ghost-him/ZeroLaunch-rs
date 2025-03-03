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
        println!("调用了一次函数");
        // 硬编码仓库信息
        const OWNER: &str = "ghost-him";
        const REPO: &str = "ZeroLaunch-rs";
        println!("123");
        let client = Client::new();
        println!("123");
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            OWNER, REPO
        );
        println!("123");
        let response = client
            .get(&url)
            .header("User-Agent", "ZeroLaunch-rs-Version-Checker")
            .send()
            .await?;
        println!("123");
        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch GitHub release: HTTP {}",
                response.status()
            ));
        }
        println!("123");
        let release: GitHubRelease = response.json().await?;
        println!("123");
        Ok(release.tag_name)
    }
}
