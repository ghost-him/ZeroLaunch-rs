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
    #[serde(rename = "is_esc_hide_window_priority", default)]
    pub is_esc_hide_window_priority: bool,
    #[serde(rename = "space_is_enter", default)]
    pub space_is_enter: bool,
    #[serde(rename = "is_wake_on_fullscreen", default)]
    pub is_wake_on_fullscreen: bool,
    #[serde(rename = "launch_new_on_failure", default = "default_true")]
    pub launch_new_on_failure: bool,
    #[serde(rename = "is_enable_drag_window", default)]
    pub is_enable_drag_window: bool,
    #[serde(rename = "show_pos_follow_mouse", default = "default_true")]
    pub show_pos_follow_mouse: bool,
    #[serde(rename = "window_position_x", default)]
    pub window_position_x: i32,
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
            window_position_x: 0,
            window_position_y: 0,
        }
    }
}

fn default_true() -> bool {
    true
}

/// 窗口交互行为配置组件。
/// 管理 ESC 键行为、空格确认、全屏唤醒、窗口激活失败降级策略、
/// 拖动窗口记忆、鼠标跟随定位以及窗口位置持久化。
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
                "窗口行为".to_string(),
                "定制搜索窗口的打开、隐藏和焦点行为".to_string(),
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
                "ESC 优先隐藏",
                "启用后，按下 ESC 键直接隐藏窗口，不再先清空输入内容",
            )
            .group("窗口行为")
            .order(0)
            .default(false)
            .build(),
            SchemaBuilder::boolean(
                "space_is_enter",
                "空格键确认",
                "启用后，按下空格键等同于回车键，直接启动选中项",
            )
            .group("窗口行为")
            .order(1)
            .default(false)
            .build(),
            SchemaBuilder::boolean(
                "is_wake_on_fullscreen",
                "全屏时允许弹出",
                "启用后，前台程序全屏时仍可弹出搜索栏（会打断全屏体验）",
            )
            .group("窗口行为")
            .order(2)
            .default(false)
            .build(),
            SchemaBuilder::boolean(
                "launch_new_on_failure",
                "激活失败时启动新实例",
                "启用后，窗口激活失败时自动启动程序新实例作为降级方案",
            )
            .group("窗口行为")
            .order(3)
            .default(true)
            .build(),
            SchemaBuilder::boolean(
                "is_enable_drag_window",
                "启用窗口拖动",
                "启用后，可拖动搜索栏窗口并记住位置。下次唤醒时恢复到上次拖动的位置",
            )
            .group("窗口行为")
            .order(10)
            .default(false)
            .build(),
            SchemaBuilder::boolean(
                "show_pos_follow_mouse",
                "跟随鼠标显示器",
                "启用后，唤醒搜索栏时自动定位到鼠标所在的显示器（多显示器环境有效）。优先级低于「启用窗口拖动」",
            )
            .group("窗口行为")
            .order(11)
            .default(false)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
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
