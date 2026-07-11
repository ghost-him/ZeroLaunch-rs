use crate::config::component_type::ComponentType;

/// 所有组件共享的身份信息核心。
///
/// 内置插件与远程插件采用同一身份模型：
/// 一个逻辑组件无论 kind 是什么，其 `component_id`、`component_name`、
/// `component_description`、`component_type`、`priority` 都统一存放在
/// `ComponentCore` 中，避免在多个 struct 或 trait 实现里重复硬编码。
///
/// ComponentCore 不包含 settings —— settings 的存储模式因组件而异：
/// 内置插件使用强类型 struct（如 `AppearanceSettings`），远程插件使用
/// `serde_json::Value`。这里强制统一反而导致死存储，见 `decisions.md 第7条`。
pub struct ComponentCore {
    pub(crate) component_id: String,
    pub(crate) component_name: String,
    pub(crate) component_description: String,
    pub(crate) component_type: ComponentType,
    pub(crate) priority: u32,
}

impl ComponentCore {
    pub fn new(
        component_id: String,
        component_name: String,
        component_description: String,
        component_type: ComponentType,
        priority: u32,
    ) -> Self {
        Self {
            component_id,
            component_name,
            component_description,
            component_type,
            priority,
        }
    }

    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn component_name(&self) -> &str {
        &self.component_name
    }

    pub fn component_description(&self) -> &str {
        &self.component_description
    }

    pub fn component_type(&self) -> ComponentType {
        self.component_type
    }

    pub fn priority(&self) -> u32 {
        self.priority
    }
}

impl std::fmt::Debug for ComponentCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentCore")
            .field("component_id", &self.component_id)
            .field("component_name", &self.component_name)
            .field("component_description", &self.component_description)
            .field("component_type", &self.component_type)
            .field("priority", &self.priority)
            .finish()
    }
}
