//! 读取组件当前设置的工具：敏感字段值先遮蔽再进模型上下文。

use super::AgentTool;
use crate::core::config::ConfigManager;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// 脱敏占位符：敏感字段的值一律替换为该字符串，不进入模型上下文。
///
/// 同时供测试断言与文档引用，避免断言写死另一份文案。
pub const MASK: &str = "[该字段为敏感值，已隐藏]";

/// 读取指定组件的当前设置，敏感字段值遮蔽为 `[该字段为敏感值，已隐藏]`。
///
/// 仅在 `core::agent` 的工具循环内构造与调用，数据来源是 `ConfigManager` 的只读接口，
/// 不写配置、不碰文件系统。
pub struct GetSettingsTool {
    /// 配置管理器（读取 schema 无关的当前设置值）。
    config: Arc<ConfigManager>,
    /// 敏感字段 key 集合（来自说明书，按 key 名递归匹配）。
    sensitive_keys: Vec<String>,
}

impl GetSettingsTool {
    /// 创建工具。
    /// 参数：config - 配置管理器；sensitive_keys - 需要遮蔽的字段 key。
    pub fn new(config: Arc<ConfigManager>, sensitive_keys: Vec<String>) -> Self {
        Self {
            config,
            sensitive_keys,
        }
    }
}

#[async_trait]
impl AgentTool for GetSettingsTool {
    fn name(&self) -> &'static str {
        "get_settings"
    }

    fn description(&self) -> &'static str {
        "读取某个设置组件的当前配置值（只读）。参数 component_id 取说明书中组件标题括号内的 id。"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "component_id": {
                    "type": "string",
                    "description": "组件 id，如 appearance-config"
                }
            },
            "required": ["component_id"]
        })
    }

    async fn execute(&self, arguments: &Value) -> String {
        let Some(component_id) = arguments.get("component_id").and_then(Value::as_str) else {
            return "参数错误：缺少 component_id".to_string();
        };
        let component_id = component_id.trim();
        let Some(settings) = self.config.get_settings(component_id) else {
            return format!("未找到组件 {component_id}；组件 id 见说明书中的组件标题");
        };
        let mut masked = settings;
        mask_sensitive(&mut masked, &self.sensitive_keys);
        match serde_json::to_string(&masked) {
            Ok(text) => format!("组件 {component_id} 的当前设置：{text}"),
            Err(e) => format!("组件 {component_id} 的设置序列化失败: {e}"),
        }
    }
}

/// 递归遮蔽敏感字段值：按字段 key 名匹配，覆盖嵌套对象与数组项。
fn mask_sensitive(value: &mut Value, sensitive_keys: &[String]) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if !child.is_null() && sensitive_keys.iter().any(|sensitive| sensitive == key) {
                    *child = Value::String(MASK.to_string());
                } else {
                    mask_sensitive(child, sensitive_keys);
                }
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                mask_sensitive(item, sensitive_keys);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 敏感字段按 key 名递归遮蔽（对象与数组项都覆盖），null 与非敏感 key 原样保留。
    #[test]
    fn mask_sensitive_covers_nested_values_and_keeps_others() {
        let keys = vec!["api_key".to_string()];
        let mut value = serde_json::json!({
            "api_key": null,
            "base_url": "https://example.com",
            "nested": { "api_key": "sk-nested" },
            "list": [ { "api_key": "sk-item" }, { "other": 1 } ]
        });
        mask_sensitive(&mut value, &keys);
        assert!(value["api_key"].is_null());
        assert_eq!(value["base_url"], "https://example.com");
        assert_eq!(value["nested"]["api_key"], MASK);
        assert_eq!(value["list"][0]["api_key"], MASK);
        assert_eq!(value["list"][1]["other"], 1);
    }
}
