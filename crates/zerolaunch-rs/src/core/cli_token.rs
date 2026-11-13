//! CLI server bearer token generation and persistence.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliToken {
    pub host: String,
    pub port: u16,
    pub token: String,
    #[serde(rename = "startedAt")]
    pub started_at: String,
}

/// CLI HTTP 服务连接信息 —— `cli_get_info` 的下发投影。
///
/// 权威是落盘的 [`CliToken`]；投影只裁掉 `startedAt`（前端不需要落盘时间）。
#[derive(Debug, Clone, Serialize)]
pub struct CliInfo {
    /// HTTP 服务绑定主机。序列化键名 `host`。
    #[serde(rename = "host")]
    pub host: String,
    /// HTTP 服务端口。序列化键名 `port`。
    #[serde(rename = "port")]
    pub port: u16,
    /// 请求鉴权用的 bearer token。序列化键名 `token`。
    #[serde(rename = "token")]
    pub token: String,
}

impl CliToken {
    /// 导出 IPC 下发投影（`CliInfo` 的唯一构造入口）。
    ///
    /// 返回：只含 host/port/token 的连接信息，不含 `startedAt`。
    pub fn to_info(&self) -> CliInfo {
        CliInfo {
            host: self.host.clone(),
            port: self.port,
            token: self.token.clone(),
        }
    }
}

/// Generate a 32-byte random token, base64-urlsafe encoded.
pub fn generate_token_string() -> String {
    let bytes: [u8; 32] = rand::random();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Write the CLI token file to disk.
/// 原子写入：先写同目录临时文件并落盘，再 rename 替换，避免读取方读到半截 JSON。
pub fn persist_cli_token(token: &CliToken, data_dir: &Path) -> Result<(), std::io::Error> {
    let token_path = data_dir.join("cli-token.json");
    let json = serde_json::to_string_pretty(token)?;
    let tmp_path = token_path.with_extension("tmp");
    {
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp_path, &token_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 下发投影只含 host/port/token（不含落盘时间戳），键名与前端 `CliInfo` 一致。
    #[test]
    fn cli_info_projection_drops_started_at() {
        let token = CliToken {
            host: "127.0.0.1".to_string(),
            port: 12345,
            token: "abc".to_string(),
            started_at: "2026-10-29T00:00:00Z".to_string(),
        };
        let json = serde_json::to_value(token.to_info()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "host": "127.0.0.1", "port": 12345, "token": "abc" })
        );
    }
}
