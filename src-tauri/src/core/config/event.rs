use crate::core::types::ComponentType;

/// 配置变更事件
#[derive(Debug, Clone)]
pub enum ConfigEvent {
    /// 组件配置变更
    SettingsChanged {
        component_id: String,
        component_type: ComponentType,
    },
    /// 组件启用状态变更
    EnabledChanged {
        component_id: String,
        component_type: ComponentType,
        enabled: bool,
    },
    /// 组件注册
    Registered {
        component_id: String,
        component_type: ComponentType,
    },
    /// 组件注销
    Unregistered { component_id: String },
}

/// 事件广播通道类型
pub type ConfigEventSender = tokio::sync::broadcast::Sender<ConfigEvent>;
pub type ConfigEventReceiver = tokio::sync::broadcast::Receiver<ConfigEvent>;

/// 创建事件广播通道
/// 参数：capacity - 通道容量
pub fn create_event_bus(capacity: usize) -> (ConfigEventSender, ConfigEventReceiver) {
    tokio::sync::broadcast::channel(capacity)
}
