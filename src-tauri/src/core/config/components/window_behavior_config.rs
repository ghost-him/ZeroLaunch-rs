use crate::core::config::setting_builders::SchemaBuilder;
use crate::core::types::setting_def::SettingDefinition;
use crate::core::types::{ComponentType, ConfigError, Configurable};
use parking_lot::RwLock;

/// 窗口交互行为配置组件。
/// 管理 ESC 键行为、空格确认、全屏唤醒和窗口激活失败降级策略。
/// 所有配置项均为被动设置（read-at-use），无 on_settings_changed 副作用。
pub struct WindowBehaviorConfigComponent {
    settings: RwLock<serde_json::Value>,
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
            settings: RwLock::new(serde_json::Value::Null),
        }
    }
}

impl Configurable for WindowBehaviorConfigComponent {
    fn component_id(&self) -> &str {
        "window-behavior"
    }

    fn component_name(&self) -> &str {
        "窗口行为"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Core
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
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        *self.settings.write() = settings;
        Ok(())
    }

    fn default_enabled(&self) -> bool {
        true
    }
}
