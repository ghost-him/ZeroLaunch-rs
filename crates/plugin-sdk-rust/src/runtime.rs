//! JSON-RPC runtime for third-party Rust plugins.
//!
//! Reads LSP-framed messages from stdin, dispatches to the user's Plugin trait
//! implementation, and writes responses to stdout.

use std::io::{BufRead, BufReader, Read, Write};

use zerolaunch_plugin_api::Plugin;
use zerolaunch_plugin_protocol::jsonrpc::{Message, Request, Response};
use zerolaunch_plugin_protocol::messages::*;
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;
use zerolaunch_plugin_protocol::{codes, JsonRpcError, PROTOCOL_VERSION};

/// Run the plugin JSON-RPC stdio loop with the given Plugin impl.
pub fn run(plugin: impl Plugin + 'static) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    rt.block_on(async move {
        run_async(plugin).await;
    });
}

async fn run_async(mut plugin: impl Plugin + 'static) {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    // Plugin state
    let mut plugin_context: Option<zerolaunch_plugin_api::PluginContext> = None;

    loop {
        // Read Content-Length header
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                return;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some(value) = trimmed.strip_prefix("Content-Length:") {
                content_length = value.trim().parse().ok();
            }
        }

        let len = match content_length {
            Some(l) if l <= 16 * 1024 * 1024 => l,
            _ => continue,
        };

        let mut body = vec![0u8; len];
        if reader.read_exact(&mut body).is_err() {
            return;
        }

        let msg: Message = match serde_json::from_slice(&body) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if let Message::Request(req) = msg {
            let result = handle_request(&mut plugin, &req, &mut plugin_context).await;
            let payload = serde_json::to_vec(&result).unwrap_or_default();
            let header = format!("Content-Length: {}\r\n\r\n", payload.len());
            let _ = writer.write_all(header.as_bytes());
            let _ = writer.write_all(&payload);
            let _ = writer.flush();
        }
    }
}

async fn handle_request(
    plugin: &mut impl Plugin,
    req: &Request,
    plugin_ctx: &mut Option<zerolaunch_plugin_api::PluginContext>,
) -> Message {
    let id = req.id;
    let result = dispatch(plugin, &req.method, &req.params, plugin_ctx).await;
    match result {
        Ok(value) => Message::Response(Response::ok(id, value)),
        Err(err) => Message::Response(Response::err(id, err)),
    }
}

async fn dispatch(
    plugin: &mut impl Plugin,
    method: &str,
    params: &serde_json::Value,
    plugin_ctx: &mut Option<zerolaunch_plugin_api::PluginContext>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        plugin_methods::INITIALIZE => {
            let p: InitializeParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            *plugin_ctx = Some(zerolaunch_plugin_api::PluginContext {
                trace_id: "init".into(),
                query_id: None,
                plugin_id: Some(p.plugin_id),
            });
            let result = InitializeResult {
                plugin_version: plugin.metadata().version.clone(),
                protocol_version: PROTOCOL_VERSION.to_string(),
            };
            Ok(serde_json::to_value(result).unwrap())
        }

        plugin_methods::SHUTDOWN => Ok(serde_json::Value::Null),

        plugin_methods::GET_METADATA => {
            Ok(serde_json::to_value(plugin.metadata()).unwrap_or(serde_json::Value::Null))
        }

        plugin_methods::GET_COMPONENTS => {
            let components = vec![ComponentDescriptor {
                component_id: plugin.component_id().to_string(),
                component_name: plugin.component_name().to_string(),
                component_type: plugin.component_type(),
                kind: ComponentKind::Plugin {
                    trigger_keywords: plugin.metadata().trigger_keywords.clone(),
                },
                priority: plugin.metadata().priority,
            }];
            Ok(serde_json::to_value(components).unwrap())
        }

        plugin_methods::GET_SETTINGS_SCHEMA => {
            Ok(serde_json::to_value(plugin.setting_schema()).unwrap_or(serde_json::Value::Null))
        }

        plugin_methods::GET_SETTINGS => Ok(plugin.get_settings()),

        plugin_methods::APPLY_SETTINGS => {
            let p: ApplySettingsParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            plugin
                .apply_settings(p.settings)
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
            Ok(serde_json::Value::Null)
        }

        plugin_methods::VALIDATE_SETTINGS => {
            let p: ValidateSettingsParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let result = match plugin.validate_settings(&p.settings) {
                Ok(()) => ValidateSettingsResult { error: None },
                Err(e) => ValidateSettingsResult {
                    error: Some(e.to_string()),
                },
            };
            Ok(serde_json::to_value(result).unwrap())
        }

        plugin_methods::CONFIG_ACTIONS => Ok(serde_json::to_value(ConfigActionsResult {
            actions: plugin.config_actions(),
        })
        .unwrap()),

        plugin_methods::EXECUTE_CONFIG_ACTION => {
            let p: ExecuteConfigActionParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            plugin
                .execute_config_action(&p.action, &p.params)
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))
        }

        plugin_methods::QUERY => {
            let p: QueryParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let _ctx = plugin_ctx
                .clone()
                .unwrap_or_else(|| zerolaunch_plugin_api::PluginContext::new("query"));
            let response = plugin
                .query(&p.ctx, &p.query)
                .await
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
            Ok(serde_json::to_value(response).unwrap())
        }

        plugin_methods::EXECUTE_ACTION => {
            let p: ExecuteActionParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            plugin
                .execute_action(&p.ctx, &p.action_id, p.payload)
                .await
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
            Ok(serde_json::Value::Null)
        }

        _ => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("method not found: {}", method),
        )),
    }
}
