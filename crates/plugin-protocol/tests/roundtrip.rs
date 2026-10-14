use zerolaunch_plugin_api::KeywordInputSource;
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
        component_description: "A test plugin".into(),
        component_type: zerolaunch_plugin_api::config::ComponentType::Plugin,
        kind: ComponentKind::Plugin,
        priority: 100,
    };
    let json = serde_json::to_value(&comp).unwrap();
    let round: ComponentDescriptor = serde_json::from_value(json).unwrap();
    assert_eq!(round.component_id, "com.example.test");
    assert_eq!(round.priority, 100);
}
/// 固定全部组件种类变体的序列化键名（跨 RPC 契约，发布后永不改名）。
#[test]
fn test_component_kind_stable_serde_keys() {
    for (kind, expected_type) in [
        (ComponentKind::DataSource, "data_source"),
        (ComponentKind::SearchEngine, "search_engine"),
        (ComponentKind::ScoreBooster, "score_booster"),
        (ComponentKind::KeywordOptimizer, "keyword_optimizer"),
        (ComponentKind::KeywordInjector, "keyword_injector"),
        (ComponentKind::Plugin, "plugin"),
        (
            ComponentKind::ActionExecutor {
                target_types: vec![],
            },
            "action_executor",
        ),
    ] {
        let json = serde_json::to_value(&kind).unwrap();
        assert_eq!(
            json["type"], expected_type,
            "kind 序列化键名漂移: {:?}",
            kind
        );
    }
}

/// KeywordOptimizer 信息跨 RPC 往返（input_source / priority 为声明字段）。
#[test]
fn test_keyword_optimizer_info_roundtrip() {
    let info = KeywordOptimizerInfo {
        input_source: KeywordInputSource::OptimizerOutput {
            producer_id: "pinyin-converter".to_string(),
        },
        priority: 60,
    };
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(
        json["inputSource"],
        serde_json::json!({"optimizerOutput": {"producerId": "pinyin-converter"}})
    );
    assert_eq!(json["priority"], 60);
    let round: KeywordOptimizerInfo = serde_json::from_value(json).unwrap();
    assert_eq!(
        round.input_source,
        KeywordInputSource::OptimizerOutput {
            producer_id: "pinyin-converter".to_string(),
        }
    );
    assert_eq!(round.priority, 60);
}

/// 新增 host/* 与 plugin/* 载荷的 JSON 键名固定（跨 RPC 契约，发布后永不改名）。
#[test]
fn test_new_host_payload_serde_keys() {
    use zerolaunch_plugin_api::host::{CacheLevel, PluginSdkConfig};
    use zerolaunch_plugin_api::services::focus_monitor::FocusEvent;
    use zerolaunch_plugin_api::services::hotkey::types::{Hotkey, HotkeyEvent, HotkeyEventFilter};
    use zerolaunch_plugin_api::services::installation_monitor::types::{
        InstallationEvent, InstallationEventKind,
    };
    use zerolaunch_plugin_api::services::IconRequest;

    let sdk_config = serde_json::to_value(SdkConfigUpdateParams {
        config: PluginSdkConfig {
            icon_cache_level: Some(CacheLevel::SkipMemory),
        },
    })
    .unwrap();
    assert_eq!(
        sdk_config,
        serde_json::json!({"config": {"iconCacheLevel": "skipMemory"}})
    );

    let icon = serde_json::to_value(IconRequestParams {
        request: IconRequest::Path("C:/x/a.exe".into()),
    })
    .unwrap();
    assert_eq!(
        icon,
        serde_json::json!({"request": {"type": "path", "value": "C:/x/a.exe"}})
    );
    assert!(serde_json::from_value::<IconRequestParams>(icon).is_ok());

    let cache_put = serde_json::to_value(CachePutParams {
        domain: "d".into(),
        key: "k".into(),
        bytes_b64: "AA==".into(),
    })
    .unwrap();
    assert_eq!(
        cache_put,
        serde_json::json!({"domain": "d", "key": "k", "bytesB64": "AA=="})
    );
    assert_eq!(
        serde_json::to_value(CacheCleanupParams {
            domain: "d".into(),
            max_entries: 32,
        })
        .unwrap(),
        serde_json::json!({"domain": "d", "maxEntries": 32})
    );

    assert_eq!(
        serde_json::to_value(TimerSetTimeoutParams { delay_ms: 500 }).unwrap(),
        serde_json::json!({"delayMs": 500})
    );
    assert_eq!(
        serde_json::to_value(TimerSetIntervalParams { interval_ms: 1500 }).unwrap(),
        serde_json::json!({"intervalMs": 1500})
    );
    assert_eq!(
        serde_json::to_value(TimerFiredParams { id: 7 }).unwrap(),
        serde_json::json!({"id": 7})
    );

    let subscribe = serde_json::to_value(HotkeySubscribeParams {
        id: "k1".into(),
        filter: HotkeyEventFilter::All,
    })
    .unwrap();
    assert_eq!(
        subscribe,
        serde_json::json!({"id": "k1", "filter": {"kind": "all"}})
    );
    assert!(serde_json::from_value::<HotkeySubscribeParams>(subscribe).is_ok());

    let hotkey_event = serde_json::to_value(HotkeyEventParams {
        id: "k1".into(),
        event: HotkeyEvent::GlobalHotkey(Hotkey::new("Space").with_ctrl()),
    })
    .unwrap();
    assert_eq!(
        hotkey_event,
        serde_json::json!({
            "id": "k1",
            "event": {
                "kind": "globalHotkey",
                "key": "Space",
                "ctrl": true,
                "alt": false,
                "shift": false,
                "meta": false,
            }
        })
    );
    let round: HotkeyEventParams = serde_json::from_value(hotkey_event).unwrap();
    assert_eq!(round.id, "k1");
    assert_eq!(
        round.event,
        HotkeyEvent::GlobalHotkey(Hotkey::new("Space").with_ctrl())
    );

    let installation_event = serde_json::to_value(InstallationEventParams {
        id: "sub-1".into(),
        event: InstallationEvent {
            changed_paths: vec!["C:/start/a.lnk".into()],
            kind: InstallationEventKind::Created,
        },
    })
    .unwrap();
    assert_eq!(
        installation_event,
        serde_json::json!({
            "id": "sub-1",
            "event": {"changedPaths": ["C:/start/a.lnk"], "kind": "created"}
        })
    );
    let round: InstallationEventParams = serde_json::from_value(installation_event).unwrap();
    assert_eq!(round.event.kind, InstallationEventKind::Created);

    assert_eq!(
        serde_json::to_value(FocusEventParams {
            id: "sub-2".into(),
            event: FocusEvent::Lost
        })
        .unwrap(),
        serde_json::json!({"id": "sub-2", "event": "lost"})
    );
}
