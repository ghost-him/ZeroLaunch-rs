//! 宿主能力暴露审计 —— 防止「能力已在 `PluginHandle` 上实现，却忘了让第三方插件用到」。
//!
//! ## 约定（本审计的判据）
//!
//! `PluginHandle`（`crates/plugin-api/src/host/plugin_handle.rs`）上的公开方法，
//! **除 `new` 与 `plugin_id` 之外，都应当在宿主侧存在** —— 也就是第三方插件能经协议调到：
//!
//! 1. 协议方法常量（`crates/plugin-protocol/src/methods.rs`：`host/...`）
//! 2. 宿主分发分支（`src-tauri/src/plugin_framework/host_handler.rs` 调 `handle.<方法>`）
//! 3. SDK 代理方法（`crates/plugin-sdk-rust/src/host_proxy.rs`，插件作者实际调用的入口）
//!
//! **本审计无豁免清单**：任何一层缺失即失败，必须补齐三层后才能通过；
//! 唯一例外是 `NO_HOST_COUNTERPART`（不需要宿主对照的方法，语义例外而非豁免，
//! 其清单自身也受双向断言守护）。
//!
//! 只做静态文本审计（不编译插件作者侧代码），因此判定偏保守：**宁可要求你显式声明**。
//! 判定依据是「无空白源码」扫描，能识别 `handle\n    .shell_open(...)` 这类多行链式调用；
//! 并对条数设下限断言 —— 解析器一旦失灵，本审计必须失败而不是静默全绿。
//! `audit_detects_*` / `audit_accepts_*` 四支自测喂合成源码验证上述方向都成立。
//!
//! 运行：`cargo test -p zerolaunch-rs --test host_api_exposure`

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

// ============================ 清单（唯一需要人工维护的部分）============================

/// 不需要在宿主侧存在的例外（用户约定）。
///
/// - `new`：构造函数，没有 `self` 参数，不在本审计的方法集合内；
/// - `plugin_id`：插件身份元数据，进程内直接可取，无宿主能力可委托。
const NO_HOST_COUNTERPART: &[&str] = &["plugin_id"];

// ============================ 待审计源码 ============================

const PLUGIN_HANDLE_SRC: &str = "crates/plugin-api/src/host/plugin_handle.rs";
const PROTOCOL_METHODS_SRC: &str = "crates/plugin-protocol/src/methods.rs";
const HOST_HANDLER_SRC: &str = "src-tauri/src/plugin_framework/host_handler.rs";
const SDK_PROXY_SRC: &str = "crates/plugin-sdk-rust/src/host_proxy.rs";

/// 一次审计的输入（四个源文件内容）。抽成结构体便于用合成源码自测审计逻辑。
struct Sources<'a> {
    /// `PluginHandle` 实现文件。
    handle: &'a str,
    /// 协议方法常量文件。
    protocol: &'a str,
    /// 宿主 `host/*` 分发文件。
    handler: &'a str,
    /// SDK 代理文件。
    proxy: &'a str,
}

/// 解析结果：四层各自抽出的条目，供断言与自测复用。
struct Parsed {
    /// `PluginHandle` 的公开方法（首个参数为 self）。
    handle_methods: BTreeSet<String>,
    /// 协议里 `host/...` 方向的方法：常量名 → 方法字符串。
    protocol_methods: BTreeMap<String, String>,
    /// `host_handler` 引用的协议常量名。
    dispatched: BTreeSet<String>,
    /// SDK 代理引用的协议常量名。
    proxied: BTreeSet<String>,
    /// `host_handler` 里对 `handle` 的调用方法名（= 第三方可达的 PluginHandle 方法）。
    handler_calls: BTreeSet<String>,
}

/// 执行审计，返回全部问题（空 = 通过）。
///
/// 纯函数：只依赖入参与模块级清单，便于用合成源码自测四个方向。
fn audit(sources: &Sources<'_>) -> Vec<String> {
    let parsed = parse(sources);
    let mut problems = Vec::new();

    // ① 协议方法必须在 host_handler 有分发分支。
    let undispatched = diff(&parsed.protocol_methods, |k| parsed.dispatched.contains(k));
    if !undispatched.is_empty() {
        problems.push(format!(
            "以下 host/* 协议方法没有 host_handler 分发分支：\n{}",
            dump(&undispatched)
        ));
    }

    // ② 协议方法必须有 SDK 代理（第三方插件才知道怎么调）。
    let no_proxy = diff(&parsed.protocol_methods, |k| parsed.proxied.contains(k));
    if !no_proxy.is_empty() {
        problems.push(format!(
            "以下 host/* 协议方法没有 SDK 代理方法（第三方插件无法调用）：\n{}\n\
             请补 crates/plugin-sdk-rust/src/host_proxy.rs 的代理方法。",
            dump(&no_proxy)
        ));
    }

    // ③ SDK 代理引用的协议方法必须真实存在（防止方法被删/改名后悬空）。
    let ghost_refs: BTreeSet<String> = parsed
        .proxied
        .iter()
        .filter(|n| !parsed.protocol_methods.contains_key(*n))
        .cloned()
        .collect();
    if !ghost_refs.is_empty() {
        problems.push(format!(
            "SDK 代理引用了协议里不存在的 host:: 常量：\n{}",
            dump(&ghost_refs)
        ));
    }

    // ④ 核心：除 NO_HOST_COUNTERPART 外，PluginHandle 的公开方法必须被 host_handler 调用
    //    （第三方可达）；没有豁免清单，不可达即失败。
    let unexposed: BTreeSet<String> = parsed
        .handle_methods
        .iter()
        .filter(|name| {
            !NO_HOST_COUNTERPART.contains(&name.as_str()) && !parsed.handler_calls.contains(*name)
        })
        .cloned()
        .collect();

    if !unexposed.is_empty() {
        problems.push(format!(
            "以下 PluginHandle 方法在宿主侧不可达（host_handler 未调用 handle.<方法>）：\n{}\n\
             请补齐三层：host/* 协议方法 + 分发分支 + SDK 代理；\
             确有宿主对照的语义例外须加进 NO_HOST_COUNTERPART 并在注释里写明理由。",
            dump(&unexposed)
        ));
    }

    // ⑤ 例外清单不许腐烂：条目必须仍是真实方法，且确实不可达。
    let ghost_exceptions: BTreeSet<String> = NO_HOST_COUNTERPART
        .iter()
        .map(|n| n.to_string())
        .filter(|n| !parsed.handle_methods.contains(n))
        .collect();
    if !ghost_exceptions.is_empty() {
        problems.push(format!(
            "NO_HOST_COUNTERPART 里的名字在 PluginHandle 上已不存在，请删除：\n{}",
            dump(&ghost_exceptions)
        ));
    }
    let stale_exceptions: BTreeSet<String> = NO_HOST_COUNTERPART
        .iter()
        .map(|n| n.to_string())
        .filter(|n| parsed.handler_calls.contains(n))
        .collect();
    if !stale_exceptions.is_empty() {
        problems.push(format!(
            "NO_HOST_COUNTERPART 里的方法其实已可达，不该留在例外清单：\n{}",
            dump(&stale_exceptions)
        ));
    }

    problems
}

/// 解析四层源码。
fn parse(sources: &Sources<'_>) -> Parsed {
    Parsed {
        handle_methods: pub_methods(sources.handle),
        protocol_methods: protocol_host_methods(sources.protocol),
        dispatched: idents_after(sources.handler, "host::"),
        proxied: idents_after(sources.proxy, "host::"),
        handler_calls: calls_on(sources.handler, "handle"),
    }
}

/// 协议方法名集合中，`keep` 判定为 false 的条目。
fn diff(protocol: &BTreeMap<String, String>, keep: impl Fn(&str) -> bool) -> BTreeSet<String> {
    protocol
        .keys()
        .filter(|k| !keep(k.as_str()))
        .cloned()
        .collect()
}

// ============================ 解析与渲染工具 ============================

/// 仓库根（本文件位于 `src-tauri/tests/`）。
fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel)
}

/// 读取待审计源码（相对仓库根）。
fn read(rel: &str) -> String {
    let path = repo_path(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("无法读取待审计源码 {}: {e}", path.display()))
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// 扫描 `recv . name (` 调用（允许跨空白/换行），返回被调方法名集合。
///
/// `recv` 前一个字符必须不是标识符字节，避免把 `core_handle.xxx` 当成 `handle.xxx`。
fn calls_on(src: &str, recv: &str) -> BTreeSet<String> {
    let bytes = src.as_bytes();
    let mut out = BTreeSet::new();
    let mut from = 0usize;
    while let Some(idx) = src[from..].find(recv) {
        let at = from + idx;
        from = at + recv.len();
        if at > 0 && is_ident_byte(bytes[at - 1]) {
            continue;
        }
        let mut i = skip_ws(bytes, at + recv.len());
        if bytes.get(i) != Some(&b'.') {
            continue;
        }
        i = skip_ws(bytes, i + 1);
        let start = i;
        while i < bytes.len() && is_ident_byte(bytes[i]) {
            i += 1;
        }
        if i == start {
            continue;
        }
        let name = src[start..i].to_string();
        if bytes.get(skip_ws(bytes, i)) == Some(&b'(') {
            out.insert(name);
        }
    }
    out
}

/// 扫描 `prefix` 之后紧跟的标识符（如 `host::SHELL_OPEN` → `SHELL_OPEN`）。
///
/// 前缀前紧跟 `::` 的匹配被跳过：那是模块路径片段（如
/// `use zerolaunch_plugin_api::host::PluginSdkConfig;` 里的 `host::PluginSdkConfig`），
/// 不是协议常量引用。
fn idents_after(src: &str, prefix: &str) -> BTreeSet<String> {
    let bytes = src.as_bytes();
    let mut out = BTreeSet::new();
    let mut from = 0usize;
    while let Some(idx) = src[from..].find(prefix) {
        let at = from + idx;
        from = at + prefix.len();
        if at >= 2 && bytes[at - 1] == b':' && bytes[at - 2] == b':' {
            continue;
        }
        let mut i = from;
        while i < bytes.len() && is_ident_byte(bytes[i]) {
            i += 1;
        }
        if i > from {
            out.insert(src[from..i].to_string());
        }
    }
    out
}

/// 抽取 `pub (async) fn NAME(...)` 中「首个参数为 self」的方法名（排除同文件自由函数）。
fn pub_methods(src: &str) -> BTreeSet<String> {
    let bytes = src.as_bytes();
    let mut out = BTreeSet::new();
    let mut from = 0usize;
    while let Some(idx) = src[from..].find("pub ") {
        let at = from + idx;
        from = at + 4;
        let mut i = skip_ws(bytes, at + 3);
        if src[i..].starts_with("async") {
            i = skip_ws(bytes, i + "async".len());
        }
        if !src[i..].starts_with("fn") {
            continue;
        }
        i = skip_ws(bytes, i + 2);
        let start = i;
        while i < bytes.len() && is_ident_byte(bytes[i]) {
            i += 1;
        }
        if i == start {
            continue;
        }
        let name = src[start..i].to_string();
        let mut j = skip_ws(bytes, i);
        if bytes.get(j) != Some(&b'(') {
            continue;
        }
        j += 1;
        let param_start = j;
        while j < bytes.len() && bytes[j] != b',' && bytes[j] != b')' {
            j += 1;
        }
        let first_param: String = src[param_start..j]
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        // 首个参数为 self 家族即视为方法；自由函数（如 build_cache_path）在此被排除。
        if matches!(first_param.as_str(), "&self" | "self" | "&mutself") {
            out.insert(name);
        }
    }
    out
}

/// `host/...` 方向的协议方法：常量名 → 方法字符串。
fn protocol_host_methods(src: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in src.lines() {
        let Some(rest) = line.trim().strip_prefix("pub const ") else {
            continue;
        };
        let Some((name, tail)) = rest.split_once(": &str = ") else {
            continue;
        };
        let value = tail.trim().trim_end_matches(';').trim();
        if value.starts_with("\"host/") {
            out.insert(name.trim().to_string(), value.trim_matches('"').to_string());
        }
    }
    out
}

/// 把集合渲染成便于阅读的多行清单（断言失败时贴出来）。
fn dump(names: &BTreeSet<String>) -> String {
    names
        .iter()
        .map(|n| format!("  - {n}"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ============================ 测试 ============================

#[test]
fn every_plugin_handle_capability_is_reachable() {
    let handle = read(PLUGIN_HANDLE_SRC);
    let protocol = read(PROTOCOL_METHODS_SRC);
    let handler = read(HOST_HANDLER_SRC);
    let proxy = read(SDK_PROXY_SRC);
    let sources = Sources {
        handle: &handle,
        protocol: &protocol,
        handler: &handler,
        proxy: &proxy,
    };
    let parsed = parse(&sources);

    // 解析器自检：条数低于下限说明扫描逻辑失灵，必须失败而不是静默全绿。
    assert!(
        parsed.handle_methods.len() >= 45,
        "解析异常：只从 {PLUGIN_HANDLE_SRC} 解出 {} 个公开方法",
        parsed.handle_methods.len()
    );
    assert!(
        parsed.protocol_methods.len() >= 45,
        "解析异常：只从 {PROTOCOL_METHODS_SRC} 解出 {} 个 host/* 方法",
        parsed.protocol_methods.len()
    );
    assert!(
        parsed.dispatched.len() >= 45,
        "解析异常：{HOST_HANDLER_SRC} 只引用 {} 个 host:: 常量",
        parsed.dispatched.len()
    );
    assert!(
        parsed.proxied.len() >= 45,
        "解析异常：{SDK_PROXY_SRC} 只引用 {} 个 host:: 常量",
        parsed.proxied.len()
    );
    assert!(
        parsed.handler_calls.len() >= 40,
        "解析异常：host_handler 中的 handle 调用解出过少（{}）",
        parsed.handler_calls.len()
    );
    assert!(
        pub_methods(&proxy).len() >= 45,
        "解析异常：{SDK_PROXY_SRC} 解出的公开方法过少"
    );

    let problems = audit(&sources);
    assert!(
        problems.is_empty(),
        "宿主能力暴露审计失败（{} 项）：\n\n{}",
        problems.len(),
        problems.join("\n\n")
    );
}

/// 自测①：新增一个未暴露的方法 → 审计必须报出它（挡住「忘暴露」）。
#[test]
fn audit_detects_new_unexposed_capability() {
    let inputs = real_sources();
    let probe_handle = format!(
        "{}\n impl Probe {{ pub async fn probe_undeclared_capability(&self) {{}} }}\n",
        inputs.0
    );
    let problems = audit(&Sources {
        handle: &probe_handle,
        protocol: &inputs.1,
        handler: &inputs.2,
        proxy: &inputs.3,
    });
    assert!(
        problems
            .iter()
            .any(|p| p.contains("probe_undeclared_capability")),
        "审计没能报出未暴露的新能力（漏检）：\n{}",
        problems.join("\n")
    );
}

/// 自测②：新增 `host/*` 协议常量但没有分发分支 → 审计必须报出它。
#[test]
fn audit_detects_protocol_method_without_dispatch() {
    let inputs = real_sources();
    let probe_protocol = format!(
        "{}\npub const PROBE_UNWIRED: &str = \"host/probe.unwired\";\n",
        inputs.1
    );
    let problems = audit(&Sources {
        handle: &inputs.0,
        protocol: &probe_protocol,
        handler: &inputs.2,
        proxy: &inputs.3,
    });
    assert!(
        problems.iter().any(|p| p.contains("PROBE_UNWIRED")),
        "审计没能报出没有分发分支的协议方法（漏检）：\n{}",
        problems.join("\n")
    );
}

/// 自测③：新增一个**已暴露**的方法（分发分支里真的调它）→ 审计不得误报。
#[test]
fn audit_accepts_properly_exposed_capability() {
    let inputs = real_sources();
    let probe_handle = format!(
        "{}\n impl Probe {{ pub async fn probe_exposed_capability(&self) {{}} }}\n",
        inputs.0
    );
    // 分发侧补上同名调用（模拟「三层齐备」）。
    let probe_handler = format!(
        "{}\n host::PROBE => {{ let _ = handle.probe_exposed_capability(); }}\n",
        inputs.2
    );
    let problems = audit(&Sources {
        handle: &probe_handle,
        protocol: &inputs.1,
        handler: &probe_handler,
        proxy: &inputs.3,
    });
    assert!(
        !problems
            .iter()
            .any(|p| p.contains("probe_exposed_capability")),
        "审计误报：已暴露的能力被当成缺陷：\n{}",
        problems.join("\n")
    );
}

/// 自测④：新增 `host/*` 协议常量（已写分发分支）但没有 SDK 代理 → 审计必须报出它。
#[test]
fn audit_detects_protocol_method_without_sdk_proxy() {
    let inputs = real_sources();
    let probe_protocol = format!(
        "{}\npub const PROBE_NO_PROXY: &str = \"host/probe.no_proxy\";\n",
        inputs.1
    );
    // 分发侧补上分支（这条不再缺分发），但 SDK 侧不补代理 → 只应报「缺 SDK 代理」。
    let probe_handler = format!(
        "{}\n host::PROBE_NO_PROXY => {{ let _ = handle.capabilities(); }}\n",
        inputs.2
    );
    let problems = audit(&Sources {
        handle: &inputs.0,
        protocol: &probe_protocol,
        handler: &probe_handler,
        proxy: &inputs.3,
    });
    assert!(
        problems
            .iter()
            .any(|p| p.contains("SDK 代理") && p.contains("PROBE_NO_PROXY")),
        "审计没能报出缺少 SDK 代理的协议方法（漏检）：\n{}",
        problems.join("\n")
    );
    assert!(
        !problems
            .iter()
            .any(|p| p.contains("没有 host_handler 分发分支")),
        "审计误报：已写分发分支的协议方法被当成缺分发：\n{}",
        problems.join("\n")
    );
}

/// 读取四个真实源文件（自测复用）。
fn real_sources() -> (String, String, String, String) {
    (
        read(PLUGIN_HANDLE_SRC),
        read(PROTOCOL_METHODS_SRC),
        read(HOST_HANDLER_SRC),
        read(SDK_PROXY_SRC),
    )
}
