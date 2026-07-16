//! CLI server bearer token generation and persistence.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliToken {
    pub host: String,
    pub port: u16,
    pub token: String,
    #[serde(rename = "startedAt")]
    pub started_at: String,
}

/// Generate a 32-byte random token, base64-urlsafe encoded.
pub fn generate_token_string() -> String {
    let bytes: [u8; 32] = rand::random();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Write the CLI token file to disk.
pub fn persist_cli_token(token: &CliToken, data_dir: &Path) -> Result<(), std::io::Error> {
    let token_path = data_dir.join("cli-token.json");
    let json = serde_json::to_string_pretty(token)?;
    std::fs::write(&token_path, json)
}
