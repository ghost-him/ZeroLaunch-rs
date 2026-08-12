//! 人可读的输出格式化器。
//!
//! 每个命令对应一个 `format_*` 函数，接收 HTTP 响应的 JSON Value，
//! 返回格式化后的纯文本字符串。加 `--json` 参数时跳过此模块直接输出 raw JSON。
//!
//! 所有来自外部输入（HTTP 响应、插件元数据、日志等）的动态文本经过处理顺序：
//!   转义 → 按显示宽度截断 → 按显示宽度补齐
//! 确保终端控制字符不会被解释，且表格列对齐不受 CJK/emoji 影响。

use serde_json::Value;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

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

/// 格式化查询列表结果。使用 Unicode 显示宽度对齐表格列。
fn format_query_list(list: &Value) -> String {
    let results = list.get("results").and_then(|v| v.as_array());
    let Some(results) = results else {
        return "  无结果\n".into();
    };
    if results.is_empty() {
        return "  无结果\n".into();
    }

    const IDX_W: usize = 4;
    const TITLE_W: usize = 28;
    const TYPE_W: usize = 10;
    const SCORE_W: usize = 6;

    let mut out = format!("  找到 {} 个结果:\n\n", results.len());
    // 列头
    out.push_str("  ");
    out.push_str(&pad_display_width("#", IDX_W, Align::Right));
    out.push(' ');
    out.push_str(&pad_display_width("标题", TITLE_W, Align::Left));
    out.push(' ');
    out.push_str(&pad_display_width("类型", TYPE_W, Align::Left));
    out.push(' ');
    out.push_str(&pad_display_width("得分", SCORE_W, Align::Right));
    out.push_str("  目标路径\n");
    out.push_str("  ");
    out.push_str(&"-".repeat(70));
    out.push('\n');

    for (i, item) in results.iter().enumerate() {
        let title = escape_terminal_text(item["title"].as_str().unwrap_or("?"));
        let subtitle = escape_terminal_text(item["subtitle"].as_str().unwrap_or(""));
        let target_type = escape_terminal_text(item["targetType"].as_str().unwrap_or("?"));
        let score = item["score"].as_f64().unwrap_or(0.0);

        out.push_str("  ");
        out.push_str(&pad_display_width(
            &format!("{}.", i + 1),
            IDX_W,
            Align::Right,
        ));
        out.push(' ');
        out.push_str(&pad_display_width(&title, TITLE_W, Align::Left));
        out.push(' ');
        out.push_str(&pad_display_width(&target_type, TYPE_W, Align::Left));
        out.push(' ');
        out.push_str(&pad_display_width(
            &format!("{:.1}", score),
            SCORE_W,
            Align::Right,
        ));
        out.push_str("  ");
        out.push_str(&subtitle);
        out.push('\n');
    }
    out
}

/// 格式化自定义面板查询结果。
fn format_query_panel(panel: &Value) -> String {
    let panel_type = escape_terminal_text(panel["panelType"].as_str().unwrap_or("?"));
    let data = panel.get("data").unwrap_or(&Value::Null);
    let actions = panel["actions"].as_array().map(|a| a.len()).unwrap_or(0);

    let mut out = format!("  自定义面板 (type: {})\n", panel_type);
    out.push_str(&format!("  动作数量: {}\n", actions));
    if !data.is_null() && data.is_object() {
        for (k, v) in data.as_object().unwrap() {
            out.push_str(&format!(
                "    {}: {}\n",
                escape_terminal_text(k),
                val_to_line(v)
            ));
        }
    }
    out
}

/// 格式化行内参数模式查询结果。
fn format_query_inline_param(param: &Value) -> String {
    let keyword = escape_terminal_text(param["triggerKeyword"].as_str().unwrap_or("?"));
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
    let mode = escape_terminal_text(value["mode"].as_str().unwrap_or("?"));
    format!("  会话模式: {}\n", mode)
}

// ─── Plugins ─────────────────────────────────────────────────────────

/// 格式化插件列表。使用 Unicode 显示宽度对齐，分别展示 State 和 Enabled 列。
pub fn format_plugins_list(value: &Value) -> String {
    let Some(arr) = value.as_array() else {
        return "  无法解析插件列表\n".into();
    };
    if arr.is_empty() {
        return "  没有已安装的插件\n".into();
    }

    const ID_W: usize = 36;
    const VER_W: usize = 10;
    const NAME_W: usize = 30;
    const STATE_W: usize = 12;
    const ENABLED_W: usize = 9;

    let mut out = format!("  已安装插件 ({}):\n\n", arr.len());
    // 表头
    out.push_str("  ");
    out.push_str(&pad_display_width("ID", ID_W, Align::Left));
    out.push(' ');
    out.push_str(&pad_display_width("Version", VER_W, Align::Left));
    out.push(' ');
    out.push_str(&pad_display_width("Name", NAME_W, Align::Left));
    out.push(' ');
    out.push_str(&pad_display_width("State", STATE_W, Align::Left));
    out.push(' ');
    out.push_str(&pad_display_width("Enabled", ENABLED_W, Align::Left));
    out.push('\n');
    out.push_str("  ");
    out.push_str(&"-".repeat(100));
    out.push('\n');

    for item in arr {
        let id = escape_terminal_text(item["pluginId"].as_str().unwrap_or("?"));
        let ver = escape_terminal_text(item["version"].as_str().unwrap_or("?"));
        let name = escape_terminal_text(item["name"].as_str().unwrap_or("?"));
        let state = match item["state"].as_str() {
            Some("running") => "Running",
            Some("crashed") => "Crashed",
            Some("stopped") => "Stopped",
            Some("starting") => "Starting",
            Some("error") => "Error",
            _ => "?",
        };
        let enabled = item["enabled"].as_bool().unwrap_or(false);

        out.push_str("  ");
        out.push_str(&pad_display_width(&id, ID_W, Align::Left));
        out.push(' ');
        out.push_str(&pad_display_width(&ver, VER_W, Align::Left));
        out.push(' ');
        out.push_str(&pad_display_width(&name, NAME_W, Align::Left));
        out.push(' ');
        out.push_str(&pad_display_width(state, STATE_W, Align::Left));
        out.push(' ');
        out.push_str(&pad_display_width(
            if enabled { "✓" } else { "✗" },
            ENABLED_W,
            Align::Left,
        ));
        out.push('\n');
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
                let args_str: Vec<String> = args
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(escape_terminal_text)
                    .collect();
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
            let list: Vec<String> = provides
                .iter()
                .filter_map(|v| v.as_str())
                .map(escape_terminal_text)
                .collect();
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
                        out.push_str(&format!(
                            "    {}: {}\n",
                            escape_terminal_text(k),
                            escape_terminal_text(s)
                        ));
                    }
                }
            }
        }
    }

    // 图标
    if let Some(icon) = value.get("icon") {
        if let Some(path) = icon["path"].as_str() {
            out.push_str("  ── Icon ──\n");
            out.push_str(&format!("    路径: {}\n", escape_terminal_text(path)));
        }
    }

    out
}

/// 格式化插件日志。逐行转义控制字符，保持多行可读。
pub fn format_plugin_logs(value: &Value) -> String {
    let logs = value["logs"].as_str().unwrap_or("");
    if logs.is_empty() {
        return "  (无日志)\n".into();
    }
    let mut out = String::new();
    for line in logs.lines() {
        out.push_str(&format!("  {}\n", escape_terminal_text(line)));
    }
    out
}

// ─── Config ─────────────────────────────────────────────────────────

/// 格式化配置组件列表。使用 Unicode 显示宽度对齐表格列。
pub fn format_config_list(value: &Value) -> String {
    let Some(arr) = value.as_array() else {
        return "  无法解析配置组件列表\n".into();
    };
    if arr.is_empty() {
        return "  没有配置组件\n".into();
    }

    const ID_W: usize = 28;
    const NAME_W: usize = 28;
    const TYPE_W: usize = 10;
    const STATE_W: usize = 10;

    let mut out = format!("  配置组件 ({}):\n\n", arr.len());
    out.push_str("  ");
    out.push_str(&pad_display_width("ID", ID_W, Align::Left));
    out.push(' ');
    out.push_str(&pad_display_width("名称", NAME_W, Align::Left));
    out.push(' ');
    out.push_str(&pad_display_width("类型", TYPE_W, Align::Left));
    out.push(' ');
    out.push_str(&pad_display_width("状态", STATE_W, Align::Left));
    out.push('\n');
    out.push_str("  ");
    out.push_str(&"-".repeat(80));
    out.push('\n');

    for item in arr {
        let id = escape_terminal_text(item["componentId"].as_str().unwrap_or("?"));
        let name = escape_terminal_text(item["componentName"].as_str().unwrap_or("?"));
        let ctype = escape_terminal_text(item["componentType"].as_str().unwrap_or("?"));
        let enabled = item["enabled"].as_bool().unwrap_or(false);
        let state = if enabled { "enabled" } else { "disabled" };

        out.push_str("  ");
        out.push_str(&pad_display_width(&id, ID_W, Align::Left));
        out.push(' ');
        out.push_str(&pad_display_width(&name, NAME_W, Align::Left));
        out.push(' ');
        out.push_str(&pad_display_width(&ctype, TYPE_W, Align::Left));
        out.push(' ');
        out.push_str(&pad_display_width(state, STATE_W, Align::Left));
        out.push('\n');
    }
    out
}

/// 格式化配置组件的 Schema。使用 Unicode 显示宽度对齐表格列。
pub fn format_config_schema(value: &Value) -> String {
    if value.is_null() {
        return "  组件不存在\n".into();
    }

    let component_id = escape_terminal_text(value["componentId"].as_str().unwrap_or("?"));
    let component_name = escape_terminal_text(value["componentName"].as_str().unwrap_or("?"));
    let component_type = escape_terminal_text(value["componentType"].as_str().unwrap_or("?"));

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

    const FIELD_W: usize = 24;
    const TYPE_W: usize = 12;
    const DEFAULT_W: usize = 16;

    // 表头
    out.push_str("  ");
    out.push_str(&pad_display_width("字段", FIELD_W, Align::Left));
    out.push(' ');
    out.push_str(&pad_display_width("类型", TYPE_W, Align::Left));
    out.push(' ');
    out.push_str(&pad_display_width("默认值", DEFAULT_W, Align::Left));
    out.push_str("  描述\n");
    out.push_str("  ");
    out.push_str(&"-".repeat(80));
    out.push('\n');

    for setting in settings {
        let field = setting.get("field");
        if let Some(field) = field {
            let name = escape_terminal_text(field["name"].as_str().unwrap_or("?"));
            let ftype = escape_terminal_text(field["fieldType"].as_str().unwrap_or("?"));
            let default = field.get("defaultValue");
            let description = escape_terminal_text(field["description"].as_str().unwrap_or(""));

            let default_str = default.map_or_else(|| "-".to_string(), val_compact);

            out.push_str("  ");
            out.push_str(&pad_display_width(&name, FIELD_W, Align::Left));
            out.push(' ');
            out.push_str(&pad_display_width(&ftype, TYPE_W, Align::Left));
            out.push(' ');
            out.push_str(&pad_display_width(&default_str, DEFAULT_W, Align::Left));
            out.push_str("  ");
            out.push_str(&description);
            out.push('\n');
        }
    }
    out
}

/// 格式化配置设置值（key: value 对）。
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
                out.push_str(&format!(
                    "  {}: {}\n",
                    escape_terminal_text(k),
                    val_to_line(v)
                ));
            }
            out
        }
        _ => {
            format!("  (设置值)\n  {}\n", val_to_line(value))
        }
    }
}

// ─── Ping ─────────────────────────────────────────────────────────────

/// 格式化健康检查结果：主程序在线时输出「正在运行」提示。
pub fn format_ping(value: &Value) -> String {
    if value.get("pong").and_then(|v| v.as_bool()).unwrap_or(false) {
        "ZeroLaunch 正在运行。\n".into()
    } else {
        pretty_raw(value)
    }
}

// ─── 辅助函数 ─────────────────────────────────────────────────────────

/// 对齐方向。
enum Align {
    Left,
    Right,
}

/// 转义字符串中的终端控制字符为可见表示。
///
/// 将所有 C0 控制字符（U+0000–U+001F）和 DEL（U+007F）替换为可见转义形式，
/// 防止它们被终端解释。按规范顺序：先转义再测宽再截断。
fn escape_terminal_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\x1b' => out.push_str("\\x1b"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x00'..='\x08' | '\x0b'..='\x0c' | '\x0e'..='\x1f' => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{{{:04X}}}", c as u32);
            }
            '\x7f' => out.push_str("\\u{007F}"),
            // 其他 Unicode 控制字符（如 U+0085、U+200E 等）
            c if c.is_control() => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{{{:04X}}}", c as u32);
            }
            _ => out.push(c),
        }
    }
    out
}

/// 返回字符串在终端中的显示宽度（列数）。
///
#[allow(dead_code)]
fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// 按终端显示宽度截断字符串，超限时末尾追加省略号 `…`。
///
/// 调用方传入的文本应当已经过 `escape_terminal_text` 转义。
/// 使用 grapheme 分割避免从组合字符或 emoji 序列中间截断。
///
/// `max_width` 为 0 时返回空字符串；任何输入都不会 panic。
fn truncate_display_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let text_width = UnicodeWidthStr::width(text);
    if text_width <= max_width {
        return text.to_string();
    }

    // 预留省略号宽度
    let ellipsis = "…";
    let ellipsis_w = UnicodeWidthStr::width(ellipsis);
    let available = max_width.saturating_sub(ellipsis_w);

    let mut result = String::new();
    let mut cur_w = 0;
    for grapheme in UnicodeSegmentation::graphemes(text, true) {
        let g_w = UnicodeWidthStr::width(grapheme);
        if cur_w + g_w > available {
            break;
        }
        result.push_str(grapheme);
        cur_w += g_w;
    }
    result.push_str(ellipsis);
    result
}

/// 按终端显示宽度补齐或截断字符串到目标宽度。
///
/// 1. 文本显示宽度超过 `target_width` 时，先按 `truncate_display_width` 截断。
/// 2. 计算剩余列数，按指定对齐方向补齐空格。
///
/// 字符串列使用左对齐，数字列使用右对齐。
fn pad_display_width(text: &str, target_width: usize, align: Align) -> String {
    let text_w = UnicodeWidthStr::width(text);
    if text_w >= target_width {
        return truncate_display_width(text, target_width);
    }
    let padding = target_width - text_w;
    let spaces = " ".repeat(padding);
    match align {
        Align::Left => format!("{}{}", text, spaces),
        Align::Right => format!("{}{}", spaces, text),
    }
}

/// 从 JSON 对象中提取字段值并格式化。
fn fmt_field(obj: &Value, label: &str, key: &str) -> String {
    let val = obj.get(key);
    match val {
        Some(Value::String(s)) => {
            format!("  {}: {}\n", pad_label(label), escape_terminal_text(s))
        }
        Some(v) => format!("  {}: {}\n", pad_label(label), val_compact(v)),
        None => String::new(),
    }
}

/// 从 JSON 对象中提取可选字段值并格式化。
fn fmt_field_opt(obj: &Value, label: &str, key: &str) -> String {
    let val = obj.get(key);
    match val {
        Some(Value::Null) | None => String::new(),
        Some(Value::String(s)) => {
            format!("  {}: {}\n", pad_label(label), escape_terminal_text(s))
        }
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
        Value::String(s) => format!("\"{}\"", escape_terminal_text(s)),
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
                    .map(|(k, v)| format!("{}: {}", escape_terminal_text(k), val_compact(v)))
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
            // 先检查原始字符串是否有换行，决定是否多行展示
            if s.contains('\n') || s.contains('\r') {
                let indented = s
                    .lines()
                    .map(|l| format!("    {}", escape_terminal_text(l)))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("\n{}", indented)
            } else {
                format!("\"{}\"", escape_terminal_text(s))
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
                    out.push_str(&format!(
                        "    {}: {}\n",
                        escape_terminal_text(k),
                        val_compact(v2)
                    ));
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

// ─── 单元测试 ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── display_width ──

    #[test]
    fn test_display_width_ascii() {
        assert_eq!(display_width("hello"), 5);
    }

    #[test]
    fn test_display_width_cjk() {
        assert_eq!(display_width("中文"), 4);
        assert_eq!(display_width("A中B"), 4);
    }

    #[test]
    fn test_display_width_emoji() {
        // 常见 emoji 宽度 2
        assert_eq!(display_width("🙂"), 2);
    }

    // ── escape_terminal_text ──

    #[test]
    fn test_escape_normal_text() {
        assert_eq!(escape_terminal_text("hello"), "hello");
    }

    #[test]
    fn test_escape_esc() {
        assert_eq!(escape_terminal_text("\x1b[2J"), "\\x1b[2J");
    }

    #[test]
    fn test_escape_newline() {
        assert_eq!(escape_terminal_text("a\nb"), "a\\nb");
    }

    #[test]
    fn test_escape_cr() {
        assert_eq!(escape_terminal_text("a\rb"), "a\\rb");
    }

    #[test]
    fn test_escape_tab() {
        assert_eq!(escape_terminal_text("a\tb"), "a\\tb");
    }

    #[test]
    fn test_escape_bel() {
        assert_eq!(escape_terminal_text("\u{0007}"), "\\u{0007}");
    }

    #[test]
    fn test_escape_del() {
        assert_eq!(escape_terminal_text("\u{007f}"), "\\u{007F}");
    }

    #[test]
    fn test_escape_mixed() {
        assert_eq!(
            escape_terminal_text("hello\x1bworld\n"),
            "hello\\x1bworld\\n"
        );
    }

    #[test]
    fn test_escape_no_control_left() {
        let inputs = ["\x1b", "\n", "\r", "\t", "\u{0007}", "\u{007f}", "\u{0000}"];
        for input in inputs {
            let escaped = escape_terminal_text(input);
            assert!(
                !escaped.chars().any(|c| c.is_control()),
                "escape_terminal_text({:?}) still contains control char: {:?}",
                input,
                escaped
            );
        }
    }

    // ── truncate_display_width ──

    #[test]
    fn test_truncate_ascii_no_truncate() {
        assert_eq!(truncate_display_width("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_ascii_truncate() {
        let result = truncate_display_width("hello world", 5);
        assert_eq!(display_width(&result), 5);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_truncate_cjk_no_truncate() {
        let text = "你好";
        assert_eq!(truncate_display_width(text, 4), text);
    }

    #[test]
    fn test_truncate_cjk_truncate() {
        let result = truncate_display_width("你好世界", 5);
        assert!(display_width(&result) <= 5);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_truncate_zero() {
        assert_eq!(truncate_display_width("hello", 0), "");
    }

    #[test]
    fn test_truncate_emoji_no_panic() {
        let result = truncate_display_width("🙂emoji混合中文测试", 10);
        assert!(display_width(&result) <= 10);
    }

    #[test]
    fn test_truncate_boundary_equals() {
        let text = "hi";
        assert_eq!(truncate_display_width(text, 2), text);
    }

    // ── pad_display_width ──

    #[test]
    fn test_pad_left() {
        let result = pad_display_width("hello", 8, Align::Left);
        assert_eq!(display_width(&result), 8);
        assert!(result.starts_with("hello"));
    }

    #[test]
    fn test_pad_right() {
        let result = pad_display_width("hello", 8, Align::Right);
        assert_eq!(display_width(&result), 8);
        assert!(result.ends_with("hello"));
    }

    #[test]
    fn test_pad_cjk_left() {
        let result = pad_display_width("中文", 6, Align::Left);
        assert_eq!(display_width(&result), 6);
    }

    #[test]
    fn test_pad_over_width_truncates() {
        let result = pad_display_width("hello world", 5, Align::Left);
        assert!(display_width(&result) <= 5);
    }

    // ── format_plugins_list State/Enabled semantics ──

    #[test]
    fn test_plugins_list_state_enabled() {
        let json = serde_json::json!([
            {
                "pluginId": "test-plugin",
                "version": "1.0.0",
                "name": "测试插件",
                "state": "running",
                "enabled": true
            },
            {
                "pluginId": "broken-plugin",
                "version": "0.5.0",
                "name": "Broken",
                "state": "crashed",
                "enabled": true
            }
        ]);
        let output = format_plugins_list(&json);
        assert!(output.contains("Running"));
        assert!(output.contains("Crashed"));
        // State 列不应再显示 enabled/disabled
        assert!(
            !output.contains("enabled"),
            "State column should not show enabled/disabled"
        );
        // Enabled 列应有标记
        assert!(output.contains("✓"));
    }

    // ── format_ping ──

    #[test]
    fn test_ping_pong() {
        let json = serde_json::json!({ "pong": true });
        let output = format_ping(&json);
        assert!(output.contains("正在运行"));
    }

    #[test]
    fn test_ping_unexpected_shape() {
        // 非预期响应形状（如空 JSON）不应 panic，回退为 raw 输出
        let output = format_ping(&serde_json::json!({}));
        assert!(output.contains("{"));
    }

    // ── --json output unaffected ──

    #[test]
    fn test_json_output_not_affected() {
        let val = serde_json::json!({"key": "value\nwith\x1besc"});
        let json_str = serde_json::to_string_pretty(&val).unwrap();
        // JSON 输出应保留原始编码
        assert!(json_str.contains("\\n"), "JSON should keep literal \\n");
    }

    // ── format_config_list ──

    #[test]
    fn test_config_list() {
        let json = serde_json::json!([
            {
                "componentId": "appearance",
                "componentName": "外观设置",
                "componentType": "config",
                "enabled": true
            }
        ]);
        let output = format_config_list(&json);
        assert!(output.contains("appearance"));
        assert!(output.contains("外观设置"));
        assert!(output.contains("enabled"));
    }

    // ── format_config_schema object-type default value ──

    #[test]
    fn test_config_schema_object_default() {
        let json = serde_json::json!({
            "componentId": "test",
            "componentName": "Test",
            "componentType": "config",
            "settings": [
                {
                    "field": {
                        "name": "colors",
                        "fieldType": "object",
                        "defaultValue": {"bg": "#000", "fg": "#fff"},
                        "description": "颜色配置"
                    }
                }
            ]
        });
        // 不应 panic
        let output = format_config_schema(&json);
        assert!(output.contains("colors"));
    }

    // ── format_config_get multi-line string ──

    #[test]
    fn test_config_get_multiline() {
        let json = serde_json::json!({"description": "line1\nline2\nline3"});
        // 不应 panic
        let output = format_config_get(&json);
        assert!(output.contains("description"));
    }
}
