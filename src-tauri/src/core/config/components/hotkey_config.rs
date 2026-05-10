use crate::core::config::setting_builders::{bool_field, text_field};
use crate::core::types::setting_def::SettingDefinition;
use crate::core::types::{ComponentType, ConfigError, Configurable};
use crate::sdk::host_api::HostApi;
use crate::sdk::hotkey::types::{Hotkey, HotkeyConfig, HotkeyRegistration};
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{info, warn};

/// 快捷键配置组件。
/// 管理全局快捷键（打开搜索栏、切换 Everything 等）和双击 Ctrl 开关。
/// 配置变更时异步应用快捷键到 HostApi。
pub struct HotkeyConfigComponent {
    /// HostApi 引用，用于应用快捷键配置
    host_api: Arc<HostApi>,
    /// 当前配置状态
    settings: RwLock<serde_json::Value>,
}

impl HotkeyConfigComponent {
    /// 创建 HotkeyConfigComponent。
    /// 参数：host_api - HostApi 实例，用于应用快捷键配置。
    pub fn new(host_api: Arc<HostApi>) -> Self {
        Self {
            host_api,
            settings: RwLock::new(serde_json::Value::Null),
        }
    }
}

/// 将快捷键字符串（如 "Alt+Space"）解析为 Hotkey 结构体。
/// 支持的修饰键：Ctrl、Alt、Shift、Meta，用 + 连接，最后一个为键名。
/// 参数：hotkey_str - 快捷键字符串。
/// 返回：解析成功的 Hotkey 实例，失败返回 None。
fn parse_hotkey_string(hotkey_str: &str) -> Option<Hotkey> {
    if hotkey_str.is_empty() {
        return None;
    }

    let parts: Vec<&str> = hotkey_str.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() || parts.iter().any(|s| s.is_empty()) {
        return None;
    }

    let mut hotkey = Hotkey::new(parts.last()?.to_string());
    for part in &parts[..parts.len() - 1] {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => hotkey.ctrl = true,
            "alt" => hotkey.alt = true,
            "shift" => hotkey.shift = true,
            "meta" | "win" | "super" => hotkey.meta = true,
            _ => return None,
        }
    }
    Some(hotkey)
}

/// 将 Hotkey 结构体序列化为快捷键字符串（如 "Alt+Space"）。
/// 参数：hotkey - Hotkey 实例。
/// 返回：快捷键字符串。
#[allow(dead_code)]
fn format_hotkey_string(hotkey: &Hotkey) -> String {
    let mut parts = Vec::new();
    if hotkey.ctrl {
        parts.push("Ctrl");
    }
    if hotkey.alt {
        parts.push("Alt");
    }
    if hotkey.shift {
        parts.push("Shift");
    }
    if hotkey.meta {
        parts.push("Meta");
    }
    parts.push(&hotkey.key);
    parts.join("+")
}

/// 将当前配置值转换为 HotkeyConfig。
/// 从语义化的配置项（open_search_bar 等）映射到 SDK 的 HotkeyConfig。
/// 参数：settings - 当前配置 JSON。
/// 返回：HotkeyConfig 实例。
fn settings_to_hotkey_config(settings: &serde_json::Value) -> HotkeyConfig {
    let mut hotkeys = Vec::new();

    // 解析各个语义化快捷键
    let hotkey_fields = ["open_search_bar", "switch_to_everything"];

    for field in &hotkey_fields {
        if let Some(hotkey_str) = settings.get(field).and_then(|v| v.as_str()) {
            if !hotkey_str.is_empty() {
                if let Some(hotkey) = parse_hotkey_string(hotkey_str) {
                    hotkeys.push(HotkeyRegistration { hotkey });
                } else {
                    warn!("快捷键配置解析失败: {} = {}", field, hotkey_str);
                }
            }
        }
    }

    let double_ctrl_enabled = settings
        .get("double_click_ctrl")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    HotkeyConfig {
        hotkeys,
        double_ctrl_enabled,
    }
}

impl Configurable for HotkeyConfigComponent {
    fn component_id(&self) -> &str {
        "hotkey-config"
    }

    fn component_name(&self) -> &str {
        "快捷键配置"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            text_field(
                "open_search_bar",
                "打开搜索栏",
                "全局快捷键，用于显示/隐藏搜索栏",
                "全局快捷键",
                0,
                "Alt+Space",
            ),
            bool_field(
                "double_click_ctrl",
                "双击 Ctrl 打开搜索栏",
                "启用后，快速双击 Ctrl 键可打开搜索栏（此时将忽略打开搜索栏的快捷键）",
                "全局快捷键",
                2,
                false,
            ),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        *self.settings.write() = settings;
        Ok(())
    }

    fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
        let hotkey_fields = ["open_search_bar", "switch_to_everything"];
        for field in &hotkey_fields {
            if let Some(hotkey_str) = settings.get(field).and_then(|v| v.as_str()) {
                if !hotkey_str.is_empty() && parse_hotkey_string(hotkey_str).is_none() {
                    return Err(ConfigError::ValidationFailed(format!(
                        "快捷键格式无效: '{}'",
                        hotkey_str
                    )));
                }
            }
        }
        Ok(())
    }

    fn on_settings_changed(&self) {
        let settings = self.settings.read().clone();
        let hotkey_config = settings_to_hotkey_config(&settings);
        let double_ctrl = hotkey_config.double_ctrl_enabled;

        info!(
            "快捷键配置变更，应用新配置: {} 个快捷键, 双击Ctrl={}",
            hotkey_config.hotkeys.len(),
            double_ctrl
        );

        // 异步应用快捷键配置到 HostApi
        let host_api = self.host_api.clone();
        tokio::spawn(async move {
            if let Err(e) = host_api.apply_hotkey_config(&hotkey_config).await {
                warn!("应用快捷键配置失败: {}", e);
            } else {
                info!("快捷键配置已成功应用");

                // 首次应用时启动监听
                if !host_api.is_hotkey_listening() {
                    if let Err(e) = host_api.init_hotkey_listening().await {
                        warn!("启动快捷键监听失败: {}", e);
                    }
                }
            }
        });
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hotkey_string() {
        let hotkey = parse_hotkey_string("Alt+Space").unwrap();
        assert_eq!(hotkey.key, "Space");
        assert!(hotkey.alt);
        assert!(!hotkey.ctrl);

        let hotkey = parse_hotkey_string("Ctrl+E").unwrap();
        assert_eq!(hotkey.key, "E");
        assert!(hotkey.ctrl);
        assert!(!hotkey.alt);

        let hotkey = parse_hotkey_string("Ctrl+Shift+T").unwrap();
        assert_eq!(hotkey.key, "T");
        assert!(hotkey.ctrl);
        assert!(hotkey.shift);
    }

    #[test]
    fn test_format_hotkey_string() {
        let hotkey = Hotkey::new("Space").with_alt();
        assert_eq!(format_hotkey_string(&hotkey), "Alt+Space");

        let hotkey = Hotkey::new("E").with_ctrl();
        assert_eq!(format_hotkey_string(&hotkey), "Ctrl+E");
    }

    #[test]
    fn test_parse_roundtrip() {
        let original = "Ctrl+Shift+Space";
        let hotkey = parse_hotkey_string(original).unwrap();
        let formatted = format_hotkey_string(&hotkey);
        assert_eq!(formatted, original);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_hotkey_string("").is_none());
        assert!(parse_hotkey_string("Invalid+Foo").is_none());
    }

    #[test]
    fn test_settings_to_hotkey_config() {
        let settings = serde_json::json!({
            "open_search_bar": "Alt+Space",
            "switch_to_everything": "Ctrl+E",
            "double_click_ctrl": true
        });
        let config = settings_to_hotkey_config(&settings);
        assert_eq!(config.hotkeys.len(), 2);
        assert!(config.double_ctrl_enabled);
    }
}
