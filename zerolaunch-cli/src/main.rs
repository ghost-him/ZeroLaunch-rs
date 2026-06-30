//! ZeroLaunch CLI — talks to the local HTTP API server.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(name = "zl", about = "ZeroLaunch CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for items
    Query { text: String },
    /// Get session mode
    Session,
    /// Plugin management
    Plugins {
        #[command(subcommand)]
        sub: PluginCmd,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        sub: ConfigCmd,
    },
}

#[derive(Subcommand)]
enum PluginCmd {
    /// List all installed plugins
    List,
    /// Get plugin info
    Info { id: String },
    /// Get plugin logs
    Logs {
        id: String,
        #[arg(long, default_value = "50")]
        tail: usize,
    },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// List all config components
    List,
    /// Get config schema for a component
    Schema { id: String },
    /// Get settings for a component
    Get { id: String },
}

struct CliClient {
    host: String,
    port: u16,
    token: String,
}

impl CliClient {
    fn load() -> Result<Self> {
        let app_data = dirs_data()?;
        let token_path = app_data.join("cli-token.json");
        let content = std::fs::read_to_string(&token_path).with_context(|| {
            format!(
                "Cannot read CLI token at {:?}. Is ZeroLaunch running?",
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
        })
    }

    fn get(&self, path: &str) -> Result<Value> {
        let url = format!("http://{}:{}{}", self.host, self.port, path);
        let client = reqwest::blocking::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()?;
        Ok(resp.json()?)
    }

    fn post(&self, path: &str, body: Value) -> Result<Value> {
        let url = format!("http://{}:{}{}", self.host, self.port, path);
        let client = reqwest::blocking::Client::new();
        let resp = client
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
    let home = dirs::home_dir().context("无法获取用户 Home 目录（dirs::home_dir() 返回 None）")?;
    Ok(home.join(".ZeroLaunch-rs"))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Query { text } => {
            let client = CliClient::load()?;
            let result = client.post("/v1/query", serde_json::json!({ "rawQuery": text }))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Session => {
            let client = CliClient::load()?;
            let result = client.get("/v1/session/mode")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Plugins { sub } => {
            let client = CliClient::load()?;
            match sub {
                PluginCmd::List => {
                    let result = client.get("/v1/plugins")?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                PluginCmd::Info { id } => {
                    let result = client.get(&format!("/v1/plugins/{}/manifest", id))?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                PluginCmd::Logs { id, tail } => {
                    let result = client.get(&format!("/v1/plugins/{}/logs", id))?;
                    let logs = result["logs"].as_str().unwrap_or("");
                    let lines: Vec<&str> = logs.lines().collect();
                    let start = if lines.len() > tail {
                        lines.len() - tail
                    } else {
                        0
                    };
                    for line in &lines[start..] {
                        println!("{}", line);
                    }
                }
            }
        }
        Commands::Config { sub } => {
            let client = CliClient::load()?;
            match sub {
                ConfigCmd::List => {
                    let result = client.get("/v1/config/components")?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                ConfigCmd::Schema { id } => {
                    let result = client.get(&format!("/v1/config/{}/schema", id))?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                ConfigCmd::Get { id } => {
                    let result = client.get(&format!("/v1/config/{}/settings", id))?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
        }
    }

    Ok(())
}
