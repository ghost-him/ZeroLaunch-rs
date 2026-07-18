//! ZeroLaunch CLI — 通过本地 HTTP API 与 ZeroLaunch 主进程通信。

mod client;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::Value;

use client::CliClient;
use output::*;

#[derive(Parser)]
#[command(name = "zl", about = "ZeroLaunch CLI 工具")]
struct Cli {
    /// 以 JSON 格式输出（默认输出人可读格式）
    #[arg(short = 'j', long = "json")]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 搜索项目
    Query { text: String },
    /// 获取当前会话模式
    Session,
    /// 已安装插件管理
    Plugins {
        #[command(subcommand)]
        sub: PluginCmd,
    },
    /// 配置组件管理
    Config {
        #[command(subcommand)]
        sub: ConfigCmd,
    },
}

#[derive(Subcommand)]
enum PluginCmd {
    /// 列出所有已安装的插件
    List,
    /// 获取插件详细信息
    Info { id: String },
    /// 获取插件日志
    Logs {
        id: String,
        #[arg(long, default_value = "50", help = "显示最后 N 行日志")]
        tail: usize,
    },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// 列出所有配置组件
    List,
    /// 获取配置组件的 Schema
    Schema { id: String },
    /// 获取配置组件的当前设置
    Get { id: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = CliClient::load()?;

    let result = dispatch(&cli, &client)?;

    if cli.json {
        // --json: 输出 raw JSON
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        // 默认：人可读格式
        let text = format_human(&cli.command, &result);
        print!("{}", text);
    }

    Ok(())
}

/// 根据 CLI 命令调用 HTTP 接口，返回 JSON 响应。
fn dispatch(cli: &Cli, client: &CliClient) -> Result<Value> {
    match &cli.command {
        Commands::Query { text } => {
            client.post("/v1/query", serde_json::json!({ "rawQuery": text }))
        }
        Commands::Session => client.get("/v1/session/mode"),
        Commands::Plugins { sub } => dispatch_plugins(sub, client),
        Commands::Config { sub } => dispatch_config(sub, client),
    }
}

fn dispatch_plugins(sub: &PluginCmd, client: &CliClient) -> Result<Value> {
    match sub {
        PluginCmd::List => client.get("/v1/plugins"),
        PluginCmd::Info { id } => client.get(&format!("/v1/plugins/{}/manifest", id)),
        PluginCmd::Logs { id, .. } => client.get(&format!("/v1/plugins/{}/logs", id)),
    }
}

fn dispatch_config(sub: &ConfigCmd, client: &CliClient) -> Result<Value> {
    match sub {
        ConfigCmd::List => client.get("/v1/config/components"),
        ConfigCmd::Schema { id } => client.get(&format!("/v1/config/{}/schema", id)),
        ConfigCmd::Get { id } => client.get(&format!("/v1/config/{}/settings", id)),
    }
}

/// 根据命令类型选择对应的格式化函数。
fn format_human(cmd: &Commands, value: &Value) -> String {
    match cmd {
        Commands::Query { .. } => format_query(value),
        Commands::Session => format_session(value),
        Commands::Plugins { sub } => match sub {
            PluginCmd::List => format_plugins_list(value),
            PluginCmd::Info { .. } => format_plugin_info(value),
            PluginCmd::Logs { tail, .. } => {
                let full = format_plugin_logs(value);
                // 如果原始格式化未处理 tail，在此截取
                if full.is_empty() {
                    return full;
                }
                let lines: Vec<&str> = full.lines().collect();
                let start = if lines.len() > *tail {
                    lines.len() - tail
                } else {
                    0
                };
                let mut out = String::new();
                for line in &lines[start..] {
                    out.push_str(line);
                    out.push('\n');
                }
                out
            }
        },
        Commands::Config { sub } => match sub {
            ConfigCmd::List => format_config_list(value),
            ConfigCmd::Schema { .. } => format_config_schema(value),
            ConfigCmd::Get { .. } => format_config_get(value),
        },
    }
}
