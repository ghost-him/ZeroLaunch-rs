//! 人可读的输出格式化器。
//!
//! 每个命令对应一个 `format_*` 函数，接收 HTTP 响应的 JSON Value，
//! 返回格式化后的纯文本字符串。加 `--json` 参数时跳过此模块直接输出 raw JSON。

use serde_json::Value;

// ─── Query ──────────────────────────────────────────────────────────

/// 格式化搜索查询结果。
pub fn format_query(value: &Value) -> String {
    match value {
        Value::String(s) if s == "empty" => "  无结果\n".into(),
        Value::Object(obj) => {
            if let Some(list) = obj.get("list") {
                format_query_list(list)
            } else if let Some(panel) = obj.get("customPanel") {
                format_query_panel(panel)
            } else if let Some(param) = obj.get("inlineParam") {
                format_query_inline_param(param)
            } else {
                // 兜底：展示 JSON 结构摘要
                format!("  （未知响应格式）\n{}", pretty_raw(value))
            }
        }
        _ => pretty_raw(value),
    }
}

fn format_query_list(list: &Value) -> String {
    let results = list.get("results").and_then(|v| v.as_array());
    let Some(results) = results else {
        return "  无结果\n".into();
    };
    if results.is_empty() {
        return "  无结果\n".into();
    }

    let mut out = String::new();
    out.push_str(&format!("  找到 {} 个结果:\n\n", results.len()));
    // 列头
    out.push_str(&format!(
        "  {:<4} {:<28} {:<10} {:>6}  目标路径\n",
        "#", "标题", "类型", "得分"
    ));
    out.push_str("  ");
    out.push_str(&"-".repeat(70));
    out.push('\n');
    for (i, item) in results.iter().enumerate() {
        let title = item["title"].as_str().unwrap_or("?");
        let subtitle = item["subtitle"].as_str().unwrap_or("");
        let target_type = item["targetType"].as_str().unwrap_or("?");
        let score = item["score"].as_f64().unwrap_or(0.0);

        out.push_str(&format!(
            "  {:<4} {:<28} {:<10} {:>6.1}  {}\n",
            format!("{}.", i + 1),
            truncate(title, 26),
            target_type,
            score,
            subtitle,
        ));
    }
    out
}

fn format_query_panel(panel: &Value) -> String {
    let panel_type = panel["panelType"].as_str().unwrap_or("?");
    let data = panel.get("data").unwrap_or(&Value::Null);
    let actions = panel["actions"].as_array().map(|a| a.len()).unwrap_or(0);

    let mut out = format!("  自定义面板 (type: {})\n", panel_type);
    out.push_str(&format!("  动作数量: {}\n", actions));
    if !data.is_null() && data.is_object() {
        for (k, v) in data.as_object().unwrap() {
            out.push_str(&format!("    {}: {}\n", k, val_to_line(v)));
        }
    }
    out
}

fn format_query_inline_param(param: &Value) -> String {
    let keyword = param["triggerKeyword"].as_str().unwrap_or("?");
    let arg_count = param["userArgCount"].as_u64().unwrap_or(0);
    let candidate_id = param["candidateId"].as_u64().unwrap_or(0);
    format!(
        "  行内参数模式\n  触发关键词: {}\n  参数数量: {}\n  候选项 ID: {}\n",
        keyword, arg_count, candidate_id
    )
}

// ─── Session ────────────────────────────────────────────────────────

/// 格式化会话模式查询结果。
pub fn format_session(value: &Value) -> String {
    let mode = value["mode"].as_str().unwrap_or("?");
    format!("  会话模式: {}\n", mode)
}

// ─── Plugins ─────────────────────────────────────────────────────────

/// 格式化插件列表。
pub fn format_plugins_list(value: &Value) -> String {
    let Some(arr) = value.as_array() else {
        return "  无法解析插件列表\n".into();
    };
    if arr.is_empty() {
        return "  没有已安装的插件\n".into();
    }

    let mut out = format!("  已安装插件 ({}):\n\n", arr.len());
    // 表头
    out.push_str(&format!(
        "  {:<36} {:<10} {:<30} {:<10}\n",
        "ID", "Version", "Name", "State"
    ));
    out.push_str("  ");
    out.push_str(&"-".repeat(90));
    out.push('\n');

    for item in arr {
        let id = item["pluginId"].as_str().unwrap_or("?");
        let ver = item["version"].as_str().unwrap_or("?");
        let name = item["name"].as_str().unwrap_or("?");
        let state = if item["enabled"].as_bool().unwrap_or(false) {
            "enabled"
        } else {
            "disabled"
        };
        out.push_str(&format!(
            "  {:<36} {:<10} {:<30} {:<10}\n",
            truncate(id, 34),
            ver,
            truncate(name, 28),
            state,
        ));
    }
    out
}

/// 格式化插件 Manifest 信息。
pub fn format_plugin_info(value: &Value) -> String {
    if value.is_null() {
        return "  插件不存在\n".into();
    }

    let plugin = value.get("plugin");
    let runtime = value.get("runtime");
    let components = value.get("components");

    let mut out = String::new();

    // 插件元信息
    if let Some(p) = plugin {
        out.push_str("  ── 插件信息 ──\n");
        out.push_str(&fmt_field(p, "ID", "id"));
        out.push_str(&fmt_field(p, "名称", "name"));
        out.push_str(&fmt_field(p, "版本", "version"));
        out.push_str(&fmt_field(p, "作者", "author"));
        out.push_str(&fmt_field(p, "描述", "description"));
        out.push_str(&fmt_field_opt(p, "主页", "homepage"));
        out.push_str(&fmt_field_opt(p, "许可证", "license"));
        out.push_str(&fmt_field(p, "最低宿主版本", "minHostVersion"));
    }

    // 运行时配置
    if let Some(r) = runtime {
        out.push_str("  ── 运行时配置 ──\n");
        out.push_str(&fmt_field(r, "命令", "command"));
        let args = r["args"].as_array();
        if let Some(args) = args {
            if !args.is_empty() {
                let args_str: Vec<&str> = args.iter().filter_map(|v| v.as_str()).collect();
                out.push_str(&format!("    参数: {}\n", args_str.join(" ")));
            }
        }
        out.push_str(&fmt_field(r, "启动超时", "startupTimeout"));
        out.push_str(&fmt_field(r, "自动重启", "autoRestart"));
        out.push_str(&fmt_field(r, "最大重启次数", "maxRestart"));
    }

    // 组件声明
    if let Some(c) = components {
        out.push_str("  ── 组件声明 ──\n");
        let provides = c["provides"].as_array();
        if let Some(provides) = provides {
            let list: Vec<&str> = provides.iter().filter_map(|v| v.as_str()).collect();
            out.push_str(&format!("    能力: {}\n", list.join(", ")));
        } else {
            out.push_str("    能力: (无)\n");
        }
    }

    // 前端 UI
    if let Some(ui) = value.get("ui") {
        if let Some(obj) = ui.as_object() {
            if !obj.is_empty() {
                out.push_str("  ── 前端 UI ──\n");
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        out.push_str(&format!("    {}: {}\n", k, s));
                    }
                }
            }
        }
    }

    // 图标
    if let Some(icon) = value.get("icon") {
        if let Some(path) = icon["path"].as_str() {
            out.push_str("  ── Icon ──\n");
            out.push_str(&format!("    路径: {}\n", path));
        }
    }

    out
}

/// 格式化插件日志。
pub fn format_plugin_logs(value: &Value) -> String {
    let logs = value["logs"].as_str().unwrap_or("");
    if logs.is_empty() {
        return "  (无日志)\n".into();
    }
    // 日志已经是行文本，直接返回
    let mut out = String::new();
    for line in logs.lines() {
        out.push_str(&format!("  {}\n", line));
    }
    out
}

// ─── Config ─────────────────────────────────────────────────────────

/// 格式化配置组件列表。
pub fn format_config_list(value: &Value) -> String {
    let Some(arr) = value.as_array() else {
        return "  无法解析配置组件列表\n".into();
    };
    if arr.is_empty() {
        return "  没有配置组件\n".into();
    }

    let mut out = format!("  配置组件 ({}):\n\n", arr.len());
    out.push_str(&format!(
        "  {:<28} {:<28} {:<10} {:<10}\n",
        "ID", "名称", "类型", "状态"
    ));
    out.push_str("  ");
    out.push_str(&"-".repeat(80));
    out.push('\n');

    for item in arr {
        let id = item["componentId"].as_str().unwrap_or("?");
        let name = item["componentName"].as_str().unwrap_or("?");
        let ctype = item["componentType"].as_str().unwrap_or("?");
        let enabled = item["enabled"].as_bool().unwrap_or(false);
        let state = if enabled { "enabled" } else { "disabled" };
        out.push_str(&format!(
            "  {:<28} {:<28} {:<10} {:<10}\n",
            truncate(id, 26),
            truncate(name, 26),
            ctype,
            state,
        ));
    }
    out
}

/// 格式化配置组件 schema。
pub fn format_config_schema(value: &Value) -> String {
    if value.is_null() {
        return "  组件不存在\n".into();
    }

    let component_id = value["componentId"].as_str().unwrap_or("?");
    let component_name = value["componentName"].as_str().unwrap_or("?");
    let component_type = value["componentType"].as_str().unwrap_or("?");

    let mut out = format!(
        "  Schema — {} ({}, {})\n\n",
        component_id, component_name, component_type
    );

    let settings = value["settings"].as_array();
    let Some(settings) = settings else {
        out.push_str("  (无配置项)\n");
        return out;
    };

    if settings.is_empty() {
        out.push_str("  (无配置项)\n");
        return out;
    }

    // 表头
    out.push_str(&format!(
        "  {:<24} {:<12} {:<16}  {}\n",
        "字段", "类型", "默认值", "描述"
    ));
    out.push_str("  ");
    out.push_str(&"-".repeat(80));
    out.push('\n');

    for setting in settings {
        let field = setting.get("field");
        if let Some(field) = field {
            let name = field["name"].as_str().unwrap_or("?");
            let ftype = field["fieldType"].as_str().unwrap_or("?");
            let default = field.get("defaultValue");
            let description = field["description"].as_str().unwrap_or("");

            let default_str = default.map_or_else(|| "-".to_string(), val_compact);

            out.push_str(&format!(
                "  {:<24} {:<12} {:<16}  {}\n",
                truncate(name, 22),
                ftype,
                truncate(&default_str, 14),
                description,
            ));
        }
    }
    out
}

/// 格式化配置设置值。
pub fn format_config_get(value: &Value) -> String {
    if value.is_null() {
        return "  (空设置)\n".into();
    }

    match value {
        Value::Object(obj) => {
            if obj.is_empty() {
                return "  (空设置)\n".into();
            }
            let mut out = String::new();
            for (k, v) in obj {
                out.push_str(&format!("  {}: {}\n", k, val_to_line(v)));
            }
            out
        }
        _ => {
            format!("  (设置值)\n  {}\n", val_to_line(value))
        }
    }
}

// ─── 辅助函数 ──────────────────────────────────────────────────────

/// 截断字符串到最大长度（超过时末尾加 `…`）。
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let end = max.saturating_sub(1);
        format!("{}…", &s[..end])
    }
}

/// 从 JSON 对象中提取字段值并格式化。
fn fmt_field(obj: &Value, label: &str, key: &str) -> String {
    let val = obj.get(key);
    match val {
        Some(Value::String(s)) => format!("  {}: {}\n", pad_label(label), s),
        Some(v) => format!("  {}: {}\n", pad_label(label), val_compact(v)),
        None => String::new(),
    }
}

/// 从 JSON 对象中提取可选字段值并格式化。
fn fmt_field_opt(obj: &Value, label: &str, key: &str) -> String {
    let val = obj.get(key);
    match val {
        Some(Value::Null) | None => String::new(),
        Some(Value::String(s)) => format!("  {}: {}\n", pad_label(label), s),
        Some(v) => format!("  {}: {}\n", pad_label(label), val_compact(v)),
    }
}

/// 填充标签到固定宽度。
fn pad_label(label: &str) -> String {
    format!("{:<12}", label)
}

/// 将 JSON 值转为紧凑的一行字符串表示。
fn val_compact(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("\"{}\"", s),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(val_compact).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                "{}".into()
            } else {
                let items: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, val_compact(v)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
        }
    }
}

/// 将 JSON 值转为多行友好表示（用于值本身）。
fn val_to_line(v: &Value) -> String {
    match v {
        Value::String(s) => {
            // 多行字符串缩进显示
            if s.contains('\n') {
                let indented = s
                    .lines()
                    .map(|l| format!("    {}", l))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("\n{}", indented)
            } else {
                format!("\"{}\"", s)
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                "[]".into()
            } else if arr.len() <= 5 {
                let items: Vec<String> = arr.iter().map(val_compact).collect();
                format!("[{}]", items.join(", "))
            } else {
                let mut out = String::from("[\n");
                for item in arr {
                    out.push_str(&format!("    {}\n", val_compact(item)));
                }
                out.push(']');
                out
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                "{}".into()
            } else {
                let mut out = String::new();
                for (k, v2) in obj {
                    out.push_str(&format!("    {}: {}\n", k, val_compact(v2)));
                }
                out
            }
        }
        _ => val_compact(v),
    }
}

/// 兜底：将值作为多行 JSON 输出（缩进 2 空格）。
fn pretty_raw(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| "? (序列化失败)".to_string())
}
