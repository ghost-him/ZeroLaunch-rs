use crate::core::config::setting_builders::SchemaBuilder;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::warn;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};

/// 窗口行为设置的强类型配置结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBehaviorSettings {
    /// ESC 键行为：启用后直接隐藏窗口，不再先清空输入内容（read-at-use，前端 matchesKey 消费）。
    #[serde(rename = "is_esc_hide_window_priority", default)]
    pub is_esc_hide_window_priority: bool,
    /// 空格键确认：启用后空格等同回车直接启动选中项（read-at-use，前端 matchesKey 消费）。
    #[serde(rename = "space_is_enter", default)]
    pub space_is_enter: bool,
    /// 全屏时允许弹出：启用后前台程序全屏时仍可弹出搜索栏（read-at-use）。
    #[serde(rename = "is_wake_on_fullscreen", default)]
    pub is_wake_on_fullscreen: bool,
    /// 激活失败降级：启用后窗口激活失败时自动启动程序新实例（read-at-use）。
    #[serde(rename = "launch_new_on_failure", default = "default_true")]
    pub launch_new_on_failure: bool,
    /// 启用窗口拖动并记忆位置（read-at-use）。
    #[serde(rename = "is_enable_drag_window", default)]
    pub is_enable_drag_window: bool,
    /// 跟随鼠标显示器：唤醒时定位到鼠标所在显示器；优先级低于「启用窗口拖动」（read-at-use）。
    #[serde(rename = "show_pos_follow_mouse", default = "default_true")]
    pub show_pos_follow_mouse: bool,
    /// 向上选择键（Hotkey 字符串如 "Ctrl+K"，空串 = 未设置仅保留方向键；前端 matchesKey 消费）。
    #[serde(rename = "move_up_key", default = "default_move_up_key")]
    pub move_up_key: String,
    /// 向下选择键（Hotkey 字符串如 "Ctrl+J"，空串 = 未设置仅保留方向键；前端 matchesKey 消费）。
    #[serde(rename = "move_down_key", default = "default_move_down_key")]
    pub move_down_key: String,
    /// 窗口水平位置（上次拖动后的 X 坐标，仅启用拖动时更新；read-at-use）。
    #[serde(rename = "window_position_x", default)]
    pub window_position_x: i32,
    /// 窗口垂直位置（上次拖动后的 Y 坐标，仅启用拖动时更新；read-at-use）。
    #[serde(rename = "window_position_y", default)]
    pub window_position_y: i32,
}

impl Default for WindowBehaviorSettings {
    fn default() -> Self {
        Self {
            is_esc_hide_window_priority: false,
            space_is_enter: false,
            is_wake_on_fullscreen: false,
            launch_new_on_failure: true,
            is_enable_drag_window: false,
            show_pos_follow_mouse: true,
            move_up_key: default_move_up_key(),
            move_down_key: default_move_down_key(),
            window_position_x: 0,
            window_position_y: 0,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_move_up_key() -> String {
    "Ctrl+K".to_string()
}

fn default_move_down_key() -> String {
    "Ctrl+J".to_string()
}

/// 校验上下选择键配置：空串 = 未设置（放行）；非空必须为「修饰键+主键」组合，
/// 修饰键仅允许 Ctrl/Alt/Shift/Meta 且至少一个（防单键配置拦截打字），
/// 主键不得为宿主静态绑定保留键（Enter/Escape/Tab/Space/Backspace/Home/End/方向键
/// 及数字 0-9——数字与 Ctrl+Digit/Meta+Digit 快捷动作冲突）。
/// 返回 Err 时设置页展示校验失败，配置不落盘。
fn validate_move_key(field: &str, value: &str) -> Result<(), ConfigError> {
    if value.is_empty() {
        return Ok(());
    }
    let parts: Vec<&str> = value.split('+').map(str::trim).collect();
    let (mods, main) = parts.split_at(parts.len().saturating_sub(1));
    let main = main.first().copied().unwrap_or("");
    if mods.is_empty() {
        return Err(ConfigError::ValidationFailed(format!(
            "{} 必须包含至少一个修饰键（Ctrl/Alt/Shift/Meta）",
            field
        )));
    }
    if mods
        .iter()
        .any(|m| !matches!(m.to_lowercase().as_str(), "ctrl" | "alt" | "shift" | "meta"))
    {
        return Err(ConfigError::ValidationFailed(format!(
            "{} 含不支持的修饰键: {}",
            field,
            mods.join("+")
        )));
    }
    let lower_main = main.to_lowercase();
    if lower_main.is_empty()
        || matches!(
            lower_main.as_str(),
            "enter"
                | "escape"
                | "tab"
                | "space"
                | "backspace"
                | "home"
                | "end"
                | "arrowup"
                | "arrowdown"
                | "arrowleft"
                | "arrowright"
                | "0"
                | "1"
                | "2"
                | "3"
                | "4"
                | "5"
                | "6"
                | "7"
                | "8"
                | "9"
        )
    {
        return Err(ConfigError::ValidationFailed(format!(
            "{} 主键 '{}' 与内置快捷键冲突，请换用其它键",
            field, main
        )));
    }
    Ok(())
}

/// 窗口交互行为配置组件。
/// 管理 ESC 键行为、空格确认、全屏唤醒、窗口激活失败降级策略、
/// 拖动窗口记忆、鼠标跟随定位、上下选择键（move_up_key/move_down_key）
/// 以及窗口位置持久化。
/// 所有配置项均为被动设置（read-at-use），无 on_settings_changed 副作用。
pub struct WindowBehaviorConfigComponent {
    core: ComponentCore,
    settings: RwLock<WindowBehaviorSettings>,
}

impl Default for WindowBehaviorConfigComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowBehaviorConfigComponent {
    /// 创建 WindowBehaviorConfigComponent。
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "window-behavior-config".to_string(),
                t_key!("window-behavior-config", "name").to_string(),
                t_key!("window-behavior-config", "description").to_string(),
                ComponentType::Core,
                20,
            ),
            settings: RwLock::new(WindowBehaviorSettings::default()),
        }
    }
}

#[async_trait]
impl Configurable for WindowBehaviorConfigComponent {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::boolean(
                "is_esc_hide_window_priority",
                t_key!(
                    "window-behavior-config",
                    "fields.is_esc_hide_window_priority.label"
                ),
                t_key!(
                    "window-behavior-config",
                    "fields.is_esc_hide_window_priority.desc"
                ),
            )
            .group(t_key!("window-behavior-config", "groups.windowBehavior"))
            .order(0)
            .default(false)
            .build(),
            SchemaBuilder::boolean(
                "space_is_enter",
                t_key!("window-behavior-config", "fields.space_is_enter.label"),
                t_key!("window-behavior-config", "fields.space_is_enter.desc"),
            )
            .group(t_key!("window-behavior-config", "groups.windowBehavior"))
            .order(1)
            .default(false)
            .build(),
            SchemaBuilder::boolean(
                "is_wake_on_fullscreen",
                t_key!(
                    "window-behavior-config",
                    "fields.is_wake_on_fullscreen.label"
                ),
                t_key!(
                    "window-behavior-config",
                    "fields.is_wake_on_fullscreen.desc"
                ),
            )
            .group(t_key!("window-behavior-config", "groups.windowBehavior"))
            .order(2)
            .default(false)
            .build(),
            SchemaBuilder::boolean(
                "launch_new_on_failure",
                t_key!(
                    "window-behavior-config",
                    "fields.launch_new_on_failure.label"
                ),
                t_key!(
                    "window-behavior-config",
                    "fields.launch_new_on_failure.desc"
                ),
            )
            .group(t_key!("window-behavior-config", "groups.windowBehavior"))
            .order(3)
            .default(true)
            .build(),
            SchemaBuilder::boolean(
                "is_enable_drag_window",
                t_key!(
                    "window-behavior-config",
                    "fields.is_enable_drag_window.label"
                ),
                t_key!(
                    "window-behavior-config",
                    "fields.is_enable_drag_window.desc"
                ),
            )
            .group(t_key!("window-behavior-config", "groups.windowBehavior"))
            .order(10)
            .default(false)
            .build(),
            SchemaBuilder::boolean(
                "show_pos_follow_mouse",
                t_key!(
                    "window-behavior-config",
                    "fields.show_pos_follow_mouse.label"
                ),
                t_key!(
                    "window-behavior-config",
                    "fields.show_pos_follow_mouse.desc"
                ),
            )
            .group(t_key!("window-behavior-config", "groups.windowBehavior"))
            .order(11)
            .default(true)
            .build(),
            SchemaBuilder::hotkey(
                "move_up_key",
                t_key!("window-behavior-config", "fields.move_up_key.label"),
                t_key!("window-behavior-config", "fields.move_up_key.desc"),
            )
            .group(t_key!("window-behavior-config", "groups.windowBehavior"))
            .order(12)
            .default(default_move_up_key())
            .build(),
            SchemaBuilder::hotkey(
                "move_down_key",
                t_key!("window-behavior-config", "fields.move_down_key.label"),
                t_key!("window-behavior-config", "fields.move_down_key.desc"),
            )
            .group(t_key!("window-behavior-config", "groups.windowBehavior"))
            .order(13)
            .default(default_move_down_key())
            .build(),
            SchemaBuilder::integer(
                "window_position_x",
                t_key!("window-behavior-config", "fields.window_position_x.label"),
                t_key!("window-behavior-config", "fields.window_position_x.desc"),
            )
            .group(t_key!("window-behavior-config", "groups.windowPosition"))
            .order(99)
            .default(0)
            .editable(false)
            .build(),
            SchemaBuilder::integer(
                "window_position_y",
                t_key!("window-behavior-config", "fields.window_position_y.label"),
                t_key!("window-behavior-config", "fields.window_position_y.desc"),
            )
            .group(t_key!("window-behavior-config", "groups.windowPosition"))
            .order(100)
            .default(0)
            .editable(false)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: WindowBehaviorSettings = serde_json::from_value(settings).unwrap_or_else(|e| {
            warn!(
                "failed to parse settings for {}, using defaults: {e}",
                self.component_id()
            );
            Default::default()
        });
        *self.settings.write() = parsed;
        Ok(())
    }

    async fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
        for field in ["move_up_key", "move_down_key"] {
            if let Some(value) = settings.get(field).and_then(|v| v.as_str()) {
                validate_move_key(field, value)?;
            }
        }
        // 两个选择键配置为同一组合会互相遮蔽（先匹配者恒生效），拒绝
        let up = settings
            .get("move_up_key")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let down = settings
            .get("move_down_key")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !up.is_empty() && up == down {
            return Err(ConfigError::ValidationFailed(
                "向上与向下选择键不能配置为同一组合".to_string(),
            ));
        }
        Ok(())
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

use crate::plugin_framework::builtin_registry::{ConfigEntry, InventoryContext};

fn build_window_behavior_config(_ctx: &InventoryContext) -> std::sync::Arc<dyn Configurable> {
    std::sync::Arc::new(WindowBehaviorConfigComponent::new())
}

::inventory::submit! {
    ConfigEntry {
        component_id: "window-behavior-config",
        priority: 40,
        factory: build_window_behavior_config,
    }
}
