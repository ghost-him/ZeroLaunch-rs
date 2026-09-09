//! HTTP 客户端，与 ZeroLaunch 主进程的 CLI HTTP 服务通信。

use anyhow::{Context, Result};
use serde_json::Value;

/// 连接失败时的提示文案：提醒用户保持 ZeroLaunch 主程序运行。
pub const CONNECTION_HINT: &str = "\
无法连接到 ZeroLaunch 主程序。

ZeroLaunch 主程序当前未在运行。请先启动 ZeroLaunch 并保持其运行，然后再执行本命令。";

/// 判断错误是否属于「无法连接 ZeroLaunch 主进程」类：
/// token 文件缺失（主程序从未启动/数据目录被清理）或网络层连接失败，
/// 两者都说明主程序当前未在运行。
pub fn is_connection_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause.downcast_ref::<reqwest::Error>().is_some()
            || cause.downcast_ref::<std::io::Error>().is_some()
    })
}

/// 连接到 ZeroLaunch 主进程 HTTP 服务的客户端。
///
/// 仅在 zerolaunch-cli 的 main.rs 中使用：加载 cli-token.json
/// 得到连接信息，再向主进程 CLI HTTP 服务发送带 Bearer 鉴权的请求。
pub struct CliClient {
    /// CLI HTTP 服务监听地址（默认 127.0.0.1）。
    host: String,
    /// CLI HTTP 服务端口（默认 51429，实际以 cli-token.json 为准）。
    port: u16,
    /// Bearer 鉴权 token，由主进程启动时生成并写入 cli-token.json。
    token: String,
    /// 底层 blocking HTTP 客户端。
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
            inner: reqwest::blocking::Client::builder()
                // 只访问本机 loopback，禁用系统/环境代理，避免代理拦截本地请求
                .no_proxy()
                .build()
                .context("无法初始化 HTTP 客户端")?,
        })
    }

    /// 健康检查：GET /v1/ping，主程序在线时返回 `{"pong": true}`。
    pub fn ping(&self) -> Result<Value> {
        self.get("/v1/ping")
    }

    /// 发送 GET 请求并解析 JSON 响应。
    pub fn get(&self, path: &str) -> Result<Value> {
        let url = format!("http://{}:{}{}", self.host, self.port, path);
        let resp = self
            .inner
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()?;
        parse_response(resp)
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
        parse_response(resp)
    }
}

/// 检查 HTTP 状态码并解析 JSON 响应体；非成功状态码给出包含状态码与响应体的明确报错。
fn parse_response(resp: reqwest::blocking::Response) -> Result<Value> {
    let status = resp.status();
    let text = resp.text()?;
    if !status.is_success() {
        anyhow::bail!("HTTP {}：{}", status.as_u16(), text.trim());
    }
    serde_json::from_str(&text).context("响应不是有效 JSON")
}

/// 解析 ZeroLaunch 应用数据目录（$HOME/.ZeroLaunch-rs）。
fn dirs_data() -> Result<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().context("无法获取用户 Home 目录")?;
        return Ok(home.join("Library/Application Support/ZeroLaunch-rs"));
    }
    #[cfg(target_os = "windows")]
    {
        let home = dirs::home_dir().context("无法获取用户 Home 目录")?;
        Ok(home.join(".ZeroLaunch-rs"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let home = dirs::home_dir().context("无法获取用户 Home 目录")?;
        Ok(home.join(".ZeroLaunch-rs"))
    }
}
