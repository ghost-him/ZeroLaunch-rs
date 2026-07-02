use zerolaunch_plugin_protocol::jsonrpc::{Message, Notification, Request, Response};
use zerolaunch_plugin_protocol::messages::*;
use zerolaunch_plugin_protocol::{codes, JsonRpcError, PROTOCOL_VERSION};

#[test]
fn test_initialize_params_roundtrip() {
    let params = InitializeParams {
        host_version: "0.7.0".into(),
        protocol_version: PROTOCOL_VERSION.into(),
        data_dir: "/data/plugin".into(),
        log_dir: "/logs".into(),
        plugin_id: "com.example.test".into(),
        locale: "zh-CN".into(),
    };
    let json = serde_json::to_value(&params).unwrap();
    let round: InitializeParams = serde_json::from_value(json).unwrap();
    assert_eq!(round.host_version, "0.7.0");
    assert_eq!(round.plugin_id, "com.example.test");
}

#[test]
fn test_jsonrpc_request_envelope() {
    let req = Request::new(
        42,
        "plugin/initialize",
        serde_json::json!({
            "pluginId": "com.example.test"
        }),
    );
    let json = serde_json::to_string(&req).unwrap();
    let msg: Message = serde_json::from_str(&json).unwrap();
    match msg {
        Message::Request(r) => {
            assert_eq!(r.id, 42);
            assert_eq!(r.method, "plugin/initialize");
        }
        _ => panic!("expected Request"),
    }
}

#[test]
fn test_jsonrpc_response_envelope() {
    let resp = Response::ok(42, serde_json::json!({"result": "ok"}));
    let json = serde_json::to_string(&resp).unwrap();
    let msg: Message = serde_json::from_str(&json).unwrap();
    match msg {
        Message::Response(r) => {
            assert_eq!(r.id, 42);
            assert!(r.result.is_some());
            assert!(r.error.is_none());
        }
        _ => panic!("expected Response"),
    }
}

#[test]
fn test_jsonrpc_error_response() {
    let resp = Response::err(
        42,
        JsonRpcError::new(codes::METHOD_NOT_FOUND, "unknown method"),
    );
    let json = serde_json::to_string(&resp).unwrap();
    let msg: Message = serde_json::from_str(&json).unwrap();
    match msg {
        Message::Response(r) => {
            assert_eq!(r.id, 42);
            assert!(r.error.is_some());
            assert_eq!(r.error.as_ref().unwrap().code, codes::METHOD_NOT_FOUND);
        }
        _ => panic!("expected Response"),
    }
}

#[test]
fn test_notification_envelope() {
    let notif = Notification::new(
        "host/log",
        serde_json::json!({
            "level": "info",
            "message": "hello"
        }),
    );
    let json = serde_json::to_string(&notif).unwrap();
    let msg: Message = serde_json::from_str(&json).unwrap();
    match msg {
        Message::Notification(n) => {
            assert_eq!(n.method, "host/log");
        }
        _ => panic!("expected Notification"),
    }
}

#[test]
fn test_component_descriptor_roundtrip() {
    let comp = ComponentDescriptor {
        component_id: "com.example.test".into(),
        component_name: "Test".into(),
        component_type: zerolaunch_plugin_api::config::ComponentType::Plugin,
        kind: ComponentKind::Plugin {
            trigger_keywords: vec!["test".into()],
        },
        priority: 100,
    };
    let json = serde_json::to_value(&comp).unwrap();
    let round: ComponentDescriptor = serde_json::from_value(json).unwrap();
    assert_eq!(round.component_id, "com.example.test");
    assert_eq!(round.priority, 100);
}
