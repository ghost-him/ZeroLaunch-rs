//! ZeroLaunch CLI — 命令行搜索工具
//!
//! 通过 HTTP 与 ZeroLaunch 主进程通信，支持搜索本机安装的软件、
//! 查看候选项、组件配置等只读操作。
//!
//! 用法：
//!   zl search <query>    搜索软件
//!   zl list              列出所有候选项
//!   zl info <id>         查看候选项详情
//!   zl components        列出可配置组件
//!   zl plugins           列出插件
//!   zl health            检查服务状态
//!
//! 全局标志：--json（机器可读输出）、--port（覆盖端口）

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::PathBuf;

// ============================================================================
// CLI 参数定义
// ============================================================================

#[derive(Parser)]
#[command(name = "zl", about = "ZeroLaunch CLI — 命令行搜索工具", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// 机器可读的 JSON 输出（适合 AI agent 消费）
    #[arg(long, global = true)]
    json: bool,

    /// 服务器端口（覆盖自动发现）
    #[arg(long, global = true, default_value_t = 0)]
    port: u16,
}

#[derive(Subcommand)]
enum Commands {
    /// 搜索已安装软件
    Search {
        /// 搜索关键词
        query: String,
    },
    /// 列出所有缓存的候选项
    List,
    /// 查看指定候选项的详细信息
    Info {
        /// 候选项 ID
        id: u64,
    },
    /// 列出所有可配置组件
    Components,
    /// 列出所有已注册插件
    Plugins,
    /// 检查 ZeroLaunch HTTP 服务状态
    Health,
}

// ============================================================================
// HTTP API 响应类型（精简，仅 CLI 需要的字段）
// ============================================================================

#[derive(Deserialize, Serialize)]
struct ApiSearchResult {
    id: u64,
    title: String,
    subtitle: String,
    score: f64,
    #[serde(rename = "targetType")]
    target_type: String,
    actions: Vec<ApiResultAction>,
}

#[derive(Deserialize, Serialize)]
struct ApiResultAction {
    id: String,
    label: String,
    #[serde(rename = "isDefault")]
    is_default: bool,
    #[serde(rename = "shortcutKey")]
    shortcut_key: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiSearchResponse {
    mode: String,
    result_count: usize,
    results: Vec<ApiSearchResult>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiCandidate {
    id: u64,
    name: String,
    target_type: String,
    keywords: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiCandidatesResponse {
    total_count: usize,
    candidates: Vec<ApiCandidate>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiComponentInfo {
    #[serde(rename = "componentId")]
    component_id: String,
    #[serde(rename = "componentName")]
    component_name: String,
    #[serde(rename = "componentType")]
    component_type: String,
    enabled: bool,
    #[serde(rename = "defaultEnabled")]
    default_enabled: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiComponentsResponse {
    total_count: usize,
    components: Vec<ApiComponentInfo>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiPluginInfo {
    id: String,
    name: String,
    version: String,
    description: String,
    author: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiPluginsResponse {
    total_count: usize,
    plugins: Vec<ApiPluginInfo>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiHealth {
    status: String,
    version: String,
    candidates_count: usize,
    session_mode: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiErrorResponse {
    error: String,
    status: u16,
}

// ============================================================================
// 端口发现
// ============================================================================

/// 获取端口文件所在目录的路径（跨平台）
fn port_file_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").ok()?;
        Some(PathBuf::from(appdata).join("ZeroLaunch-rs"))
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join("Library").join("Application Support").join("ZeroLaunch-rs"))
    }
    #[cfg(target_os = "linux")]
    {
        let data_dir = std::env::var("XDG_DATA_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".local/share"))
            })?;
        Some(data_dir.join("ZeroLaunch-rs"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

/// 发现服务器端口
///
/// 优先级：命令行 --port > .zl-port 文件 > 默认值 45678
fn discover_port(override_port: u16) -> u16 {
    if override_port != 0 {
        return override_port;
    }
    port_file_dir()
        .and_then(|dir| {
            let path = dir.join(".zl-port");
            std::fs::read_to_string(path).ok()
        })
        .and_then(|data| data.trim().parse::<u16>().ok())
        .unwrap_or(45678)
}

// ============================================================================
// HTTP 请求
// ============================================================================

fn api_url(port: u16, path: &str) -> String {
    format!("http://127.0.0.1:{}{}", port, path)
}

fn api_get<T: for<'de> Deserialize<'de>>(port: u16, path: &str) -> Result<T, String> {
    let url = api_url(port, path);
    let response = reqwest::blocking::get(&url).map_err(|e| format!("请求失败: {}", e))?;
    if !response.status().is_success() {
        let api_err: ApiErrorResponse = response
            .json()
            .unwrap_or(ApiErrorResponse {
                error: "unknown error".to_string(),
                status: 500,
            });
        return Err(format!("{} (状态码: {})", api_err.error, api_err.status));
    }
    response
        .json::<T>()
        .map_err(|e| format!("响应解析失败: {}", e))
}

// ============================================================================
// 输出函数
// ============================================================================

fn print_json<T: serde::Serialize>(value: &T) {
    let json =
        serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("{{error: \"{}\"}}", e));
    println!("{}", json);
}

fn bold(s: &str) -> String {
    if cfg!(target_os = "windows") {
        s.to_string()
    } else {
        format!("\x1b[1m{}\x1b[0m", s)
    }
}

// ============================================================================
// 命令处理
// ============================================================================

fn cmd_health(port: u16, json: bool) -> Result<(), String> {
    let health: ApiHealth = api_get(port, "/health")?;
    if json {
        print_json(&serde_json::json!({
            "command": "health",
            "response": health,
        }));
    } else {
        let status_icon = if health.status == "ok" { "✓" } else { "✗" };
        println!("{} ZeroLaunch HTTP API", bold("Status:"));
        println!("  {} {}", status_icon, health.status);
        println!("  Version:       {}", health.version);
        println!("  Candidates:    {}", health.candidates_count);
        println!("  Session Mode:  {}", health.session_mode);
    }
    Ok(())
}

fn cmd_search(port: u16, query: &str, json: bool) -> Result<(), String> {
    let encoded = urlencoding(query);
    let resp: ApiSearchResponse = api_get(port, &format!("/search?q={}", encoded))?;

    if json {
        print_json(&serde_json::json!({
            "command": "search",
            "query": query,
            "resultCount": resp.result_count,
            "results": resp.results,
        }));
    } else {
        println!("Found {} results for \"{}\":", resp.result_count, query);
        for item in &resp.results {
            let default_action = item
                .actions
                .iter()
                .find(|a| a.is_default)
                .map(|a| a.shortcut_key.as_str())
                .unwrap_or("");
            let shortcut = if default_action.is_empty() {
                String::new()
            } else {
                format!(" [{}]", default_action)
            };
            println!(
                "  #{:<5} {:<30} {:.2}  [{}]{}",
                item.id, item.title, item.score, item.target_type, shortcut
            );
        }
    }
    Ok(())
}

fn cmd_list(port: u16, json: bool) -> Result<(), String> {
    let resp: ApiCandidatesResponse = api_get(port, "/candidates")?;
    if json {
        print_json(&serde_json::json!({
            "command": "list",
            "response": resp,
        }));
    } else {
        println!("Total candidates: {}", resp.total_count);
        for item in &resp.candidates {
            println!(
                "  #{:<5} {:<30} [{}]  keywords: {}",
                item.id,
                item.name,
                item.target_type,
                item.keywords.join(", ")
            );
        }
    }
    Ok(())
}

fn cmd_info(port: u16, id: u64, json: bool) -> Result<(), String> {
    let candidate: ApiCandidate = api_get(port, &format!("/candidates/{}", id))
        .map_err(|e| format!("候选项 #{} 未找到: {}", id, e))?;

    if json {
        print_json(&serde_json::json!({
            "command": "info",
            "candidate": candidate,
        }));
    } else {
        println!(
            "{}",
            bold(&format!("#{} {}", candidate.id, candidate.name))
        );
        println!("  Type:     {}", candidate.target_type);
        println!("  Keywords: {}", candidate.keywords.join(", "));
    }
    Ok(())
}

fn cmd_components(port: u16, json: bool) -> Result<(), String> {
    let resp: ApiComponentsResponse = api_get(port, "/components")?;
    if json {
        print_json(&serde_json::json!({
            "command": "components",
            "response": resp,
        }));
    } else {
        println!("Total components: {}", resp.total_count);
        for c in &resp.components {
            let enabled_str = if c.enabled { "✓" } else { "✗" };
            println!(
                "  {} {:<30} [{:<15}] {}",
                enabled_str, c.component_name, c.component_type, c.component_id
            );
        }
    }
    Ok(())
}

fn cmd_plugins(port: u16, json: bool) -> Result<(), String> {
    let resp: ApiPluginsResponse = api_get(port, "/plugins")?;
    if json {
        print_json(&serde_json::json!({
            "command": "plugins",
            "response": resp,
        }));
    } else {
        println!("Total plugins: {}", resp.total_count);
        for p in &resp.plugins {
            println!("  {} v{} — {}", bold(&p.name), p.version, p.description);
            println!("    ID: {}, Author: {}", p.id, p.author);
        }
    }
    Ok(())
}

/// 简单的 URL 编码（仅对空格和特殊字符编码）
fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('#', "%23")
        .replace('?', "%3F")
        .replace('+', "%2B")
}

// ============================================================================
// 入口
// ============================================================================

fn main() {
    let cli = Cli::parse();
    let port = discover_port(cli.port);

    let result = match cli.command {
        Commands::Health => cmd_health(port, cli.json),
        Commands::Search { query } => cmd_search(port, &query, cli.json),
        Commands::List => cmd_list(port, cli.json),
        Commands::Info { id } => cmd_info(port, id, cli.json),
        Commands::Components => cmd_components(port, cli.json),
        Commands::Plugins => cmd_plugins(port, cli.json),
    };

    if let Err(e) = result {
        let _ = writeln!(io::stderr(), "错误: {}", e);
        std::process::exit(1);
    }
}
