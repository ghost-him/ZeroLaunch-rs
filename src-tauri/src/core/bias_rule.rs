use serde::Deserialize;
use tracing::{debug, warn};

/// 固定偏移量规则，按 target 精确匹配（target 已归一化为 lowercase）。
#[derive(Debug, Clone)]
pub struct BiasRule {
    pub target: String,
    pub bias: f64,
}

/// BiasConfig settings JSON 中 `entries` 数组元素的解析结构。
/// 仅在此模块内部用于从 JSON 边界安全反序列化，不与 BiasEntry 耦合。
#[derive(Deserialize)]
struct BiasSettingsEntry {
    target: String,
    bias: f64,
}

impl BiasRule {
    /// 从 BiasConfig 组件 `get_settings()` 返回的 JSON 中解析规则列表。
    ///
    /// `settings` 应为 BiasConfig 的完整 settings JSON（含 `entries` 数组字段）。
    /// 解析失败的条目被跳过并记 warn 日志，整个解析失败时返回空 Vec。
    pub fn from_settings_json(settings: &serde_json::Value) -> Vec<BiasRule> {
        let Some(entries_val) = settings.get("entries") else {
            debug!("BiasConfig: settings 中无 entries 字段");
            return Vec::new();
        };
        let entries: Vec<BiasSettingsEntry> = match serde_json::from_value(entries_val.clone()) {
            Ok(e) => e,
            Err(e) => {
                warn!("解析 BiasConfig entries 失败: {}", e);
                return Vec::new();
            }
        };
        if entries.is_empty() {
            debug!("BiasConfig entries 为空");
            return Vec::new();
        }
        let rules: Vec<BiasRule> = entries
            .into_iter()
            .map(|e| BiasRule {
                // apply_settings 中已做 to_ascii_lowercase，此处再确保一次
                target: e.target.to_ascii_lowercase(),
                bias: e.bias,
            })
            .collect();
        debug!("从 BiasConfig 加载 {} 条偏移量规则", rules.len());
        rules
    }
}
