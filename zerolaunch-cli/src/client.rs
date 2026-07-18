//! HTTP 客户端，与 ZeroLaunch 主进程的 CLI HTTP 服务通信。

use anyhow::{Context, Result};
use serde_json::Value;

/// 连接到 ZeroLaunch 主进程 HTTP 服务的客户端。
pub struct CliClient {
    host: String,
    port: u16,
    token: String,
    inner: reqwest::blocking::Client,
}

impl CliClient {
    /// 从 cli-token.json 加载连接信息并初始化客户端。
    pub fn load() -> Result<Self> {
        let app_data = dirs_data()?;
        let token_path = app_data.join("cli-token.json");
        let content = std::fs::read_to_string(&token_path).with_context(|| {
            format!(
                "无法读取 CLI token 文件 {:?}。ZeroLaunch 是否正在运行？",
                token_path
            )
        })?;
        let token_data: Value = serde_json::from_str(&content)?;
        Ok(Self {
            host: token_data["host"]
                .as_str()
                .unwrap_or("127.0.0.1")
                .to_string(),
            port: token_data["port"].as_u64().unwrap_or(51429) as u16,
            token: token_data["token"].as_str().unwrap_or("").to_string(),
            inner: reqwest::blocking::Client::new(),
        })
    }

    /// 发送 GET 请求并解析 JSON 响应。
    pub fn get(&self, path: &str) -> Result<Value> {
        let url = format!("http://{}:{}{}", self.host, self.port, path);
        let resp = self
            .inner
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()?;
        Ok(resp.json()?)
    }

    /// 发送 POST 请求并解析 JSON 响应。
    pub fn post(&self, path: &str, body: Value) -> Result<Value> {
        let url = format!("http://{}:{}{}", self.host, self.port, path);
        let resp = self
            .inner
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?;
        Ok(resp.json()?)
    }
}

/// 解析 ZeroLaunch 应用数据目录（$HOME/.ZeroLaunch-rs）。
fn dirs_data() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("无法获取用户 Home 目录")?;
    Ok(home.join(".ZeroLaunch-rs"))
}
