use crate::config::action::ConfigActionDef;
use crate::config::component_core::ComponentCore;
use crate::config::component_type::ComponentType;
use crate::config::error::ConfigError;
use crate::config::setting_def::SettingDefinition;

/// 所有可配置组件都需实现的核心契约。
/// 提供组件标识、配置定义、配置读写和配置变更回调能力。
pub trait Configurable: Send + Sync {
    /// 返回组件身份核心。
    ///
    /// 实现者只需提供对 `ComponentCore` 的引用，identity 相关方法即可使用默认实现。
    fn core(&self) -> &ComponentCore;

    fn component_id(&self) -> &str {
        self.core().component_id()
    }

    fn component_name(&self) -> &str {
        self.core().component_name()
    }

    fn component_type(&self) -> ComponentType {
        self.core().component_type()
    }

    /// 组件显示排序优先级，数值越小越靠前。
    fn priority(&self) -> u32 {
        self.core().priority()
    }

    /// 组件的功能描述文本，用于设置面板中向用户解释该组件的用途。
    fn component_description(&self) -> &str {
        self.core().component_description()
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::Value::Object(serde_json::Map::new())
    }

    /// 应用配置到组件。
    /// 使用 &self 签名，组件内部通过 RwLock 等实现可变性。
    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let _ = settings;
        Ok(())
    }

    fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
        let _ = settings;
        Ok(())
    }

    fn get_default_settings(&self) -> serde_json::Value {
        let schema = self.setting_schema();
        let mut map = serde_json::Map::new();
        for def in schema {
            if !def.field.default_value.is_null() {
                map.insert(def.field.key.clone(), def.field.default_value.clone());
            }
        }
        serde_json::Value::Object(map)
    }

    fn on_settings_changed(&self) {}

    /// 返回该组件支持的配置动作定义列表。
    /// 配置动作用于在设置面板中提供一键式辅助操作（如自动检测浏览器），
    /// 前端据此渲染操作按钮，用户点击后通过 execute_config_action 执行。
    fn config_actions(&self) -> Vec<ConfigActionDef> {
        vec![]
    }

    /// 执行配置动作。
    /// 参数：action - 动作标识符，对应 ConfigActionDef.action。
    ///       params - 前端传递的附加参数（如书签文件路径）。
    /// 返回：动作执行结果（JSON 格式），由前端根据配置项类型解析并填充。
    fn execute_config_action(
        &self,
        action: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let _ = params;
        Err(format!("Unknown config action: {}", action))
    }

    /// 返回组件的默认启用状态。
    /// 某些组件可能默认禁用（如实验性功能）。
    /// 实际启用状态由 ConfigManager 管理，用户设置会覆盖此默认值。
    fn default_enabled(&self) -> bool {
        true
    }
}
