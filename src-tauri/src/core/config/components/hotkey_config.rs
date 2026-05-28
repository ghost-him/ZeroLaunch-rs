use crate::core::config::setting_builders::SchemaBuilder;
use crate::core::types::setting_def::SettingDefinition;
use crate::core::types::{ComponentType, ConfigError, Configurable};
use crate::sdk::host_api::HostApi;
use crate::sdk::hotkey::types::{Hotkey, HotkeyConfig, HotkeyRegistration};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// 快捷键设置的强类型配置结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeySettings {
    #[serde(rename = "open_search_bar", default = "default_open_search_bar")]
    pub open_search_bar: String,
    #[serde(rename = "double_click_ctrl", default)]
    pub double_click_ctrl: bool,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            open_search_bar: default_open_search_bar(),
            double_click_ctrl: false,
        }
    }
}

fn default_open_search_bar() -> String {
    "Alt+Space".to_string()
}

/// 快捷键配置组件。
/// 管理全局快捷键（打开搜索栏、切换 Everything 等）和双击 Ctrl 开关。
/// 配置变更时异步应用快捷键到 HostApi。
pub struct HotkeyConfigComponent {
    /// HostApi 引用，用于应用快捷键配置
    host_api: Arc<HostApi>,
    /// 当前配置状态
    settings: RwLock<HotkeySettings>,
}

impl HotkeyConfigComponent {
    /// 创建 HotkeyConfigComponent。
    /// 参数：host_api - HostApi 实例，用于应用快捷键配置。
    pub fn new(host_api: Arc<HostApi>) -> Self {
        Self {
            host_api,
            settings: RwLock::new(HotkeySettings::default()),
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
/// 参数：settings - 当前热键配置。
/// 返回：HotkeyConfig 实例。
fn settings_to_hotkey_config(settings: &HotkeySettings) -> HotkeyConfig {
    let mut hotkeys = Vec::new();

    let hotkey_strs = [&settings.open_search_bar];

    for hotkey_str in &hotkey_strs {
        if !hotkey_str.is_empty() {
            if let Some(hotkey) = parse_hotkey_string(hotkey_str) {
                hotkeys.push(HotkeyRegistration { hotkey });
            } else {
                warn!("快捷键配置解析失败: {}", hotkey_str);
            }
        }
    }

    HotkeyConfig {
        hotkeys,
        double_ctrl_enabled: settings.double_click_ctrl,
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
            SchemaBuilder::text(
                "open_search_bar",
                "打开搜索栏",
                "全局快捷键，用于显示/隐藏搜索栏",
            )
            .group("全局快捷键")
            .order(0)
            .default("Alt+Space")
            .build(),
            SchemaBuilder::boolean(
                "double_click_ctrl",
                "双击 Ctrl 打开搜索栏",
                "启用后，快速双击 Ctrl 键可打开搜索栏（此时将忽略打开搜索栏的快捷键）",
            )
            .group("全局快捷键")
            .order(2)
            .default(false)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: HotkeySettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
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
        let hotkey_config = settings_to_hotkey_config(&self.settings.read());
        let double_ctrl = hotkey_config.double_ctrl_enabled;

        info!(
            "快捷键配置变更，应用新配置: {} 个快捷键, 双击Ctrl={}",
            hotkey_config.hotkeys.len(),
            double_ctrl
        );

        // 异步应用快捷键配置到 HostApi
        let host_api = self.host_api.clone();
        tauri::async_runtime::spawn(async move {
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
        let settings = HotkeySettings {
            open_search_bar: "Alt+Space".to_string(),
            double_click_ctrl: true,
        };
        let config = settings_to_hotkey_config(&settings);
        assert_eq!(config.hotkeys.len(), 1);
        assert!(config.double_ctrl_enabled);
    }
}
