//! HostProxy — provides methods for third-party plugins to call host/* APIs.
//!
//! Internally sends JSON-RPC requests to the host via stdout.

use std::io::{BufRead, Write};

/// Proxy for calling host-side APIs from a plugin subprocess.
/// Each method sends a `host/*` JSON-RPC request to stdout.
pub struct HostProxy {
    next_id: std::sync::atomic::AtomicU64,
}

impl HostProxy {
    pub fn new() -> Self {
        Self {
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Send a host/* request and read the response from stdin.
    fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let payload = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        let header = format!("Content-Length: {}\r\n\r\n", payload.len());

        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle
            .write_all(header.as_bytes())
            .map_err(|e| e.to_string())?;
        handle.write_all(&payload).map_err(|e| e.to_string())?;
        handle.flush().map_err(|e| e.to_string())?;

        // Read response from stdin (LSP-framed)
        let stdin = std::io::stdin();
        let mut reader = std::io::BufReader::new(stdin.lock());
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).map_err(|e| e.to_string())?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some(v) = trimmed.strip_prefix("Content-Length:") {
                content_length = v.trim().parse().ok();
            }
        }
        let len = content_length.ok_or("no Content-Length in response")?;
        let mut body = vec![0u8; len];
        use std::io::Read;
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;

        let response: serde_json::Value =
            serde_json::from_slice(&body).map_err(|e| e.to_string())?;
        if let Some(err) = response.get("error") {
            Err(format!("host error: {:?}", err))
        } else {
            Ok(response
                .get("result")
                .cloned()
                .unwrap_or(serde_json::Value::Null))
        }
    }

    pub fn log(&self, level: &str, message: &str) -> Result<(), String> {
        self.send_request(
            "host/log",
            serde_json::json!({ "level": level, "message": message }),
        )?;
        Ok(())
    }

    pub fn shell_open(&self, target: &str) -> Result<(), String> {
        self.send_request("host/shell.open", serde_json::json!({ "target": target }))?;
        Ok(())
    }

    pub fn get_icon(&self, path: &str) -> Result<String, String> {
        let result = self.send_request(
            "host/icon.get",
            serde_json::json!({ "request": { "path": path }, "level": "Full" }),
        )?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    pub fn shell_execute_command(&self, cmd: &str) -> Result<(), String> {
        self.send_request(
            "host/shell.execute_command",
            serde_json::json!({ "cmd": cmd }),
        )?;
        Ok(())
    }

    pub fn shell_open_folder(&self, path: &str) -> Result<(), String> {
        self.send_request(
            "host/shell.open_folder",
            serde_json::json!({ "path": path }),
        )?;
        Ok(())
    }

    pub fn shell_execute_elevation(&self, path: &str) -> Result<(), String> {
        self.send_request(
            "host/shell.execute_elevation",
            serde_json::json!({ "path": path }),
        )?;
        Ok(())
    }

    pub fn notify(&self, title: &str, message: &str) -> Result<(), String> {
        self.send_request(
            "host/notify",
            serde_json::json!({ "title": title, "message": message }),
        )?;
        Ok(())
    }

    pub fn enumerate_apps(&self) -> Result<serde_json::Value, String> {
        self.send_request("host/app.enumerate", serde_json::json!(null))
    }

    pub fn resolve_path(&self, kind: &str) -> Result<String, String> {
        let result = self.send_request("host/path.resolve", serde_json::json!({ "kind": kind }))?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    pub fn resource_upload(
        &self,
        plugin_id: &str,
        key: &str,
        bytes_b64: &str,
    ) -> Result<String, String> {
        let result = self.send_request(
            "host/resource.upload",
            serde_json::json!({
                "pluginId": plugin_id,
                "key": key,
                "bytesB64": bytes_b64,
            }),
        )?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    pub fn resource_get(&self, plugin_id: &str, key: &str) -> Result<String, String> {
        let result = self.send_request(
            "host/resource.get",
            serde_json::json!({
                "pluginId": plugin_id,
                "key": key,
            }),
        )?;
        Ok(result.as_str().unwrap_or("").to_string())
    }
}

impl Default for HostProxy {
    fn default() -> Self {
        Self::new()
    }
}
