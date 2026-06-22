use zerolaunch_plugin_host::manager::RegisteredAdapters;

use crate::core::types::ComponentType;

/// 配置变更事件。也承载第三方插件的运行时组件注册/解注册事件，
/// 通过 broadcast 通道传递给 SessionRouter。
#[derive(Clone, Debug)]
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
    /// 第三方插件运行时组件已注册（携带完整 RegisteredAdapters）
    PluginRegistered(RegisteredAdapters),
    /// 第三方插件运行时组件已解注册（携带被解注册的完整 RegisteredAdapters）
    PluginUnregistered(RegisteredAdapters),
}

// ── ConfigEvent 通道 ─────────────────────────────────────────────────

/// ConfigEvent 广播通道发送端
pub type ConfigEventSender = tokio::sync::broadcast::Sender<ConfigEvent>;
/// ConfigEvent 广播通道接收端
pub type ConfigEventReceiver = tokio::sync::broadcast::Receiver<ConfigEvent>;

/// 创建 ConfigEvent 广播通道
pub fn create_event_bus(capacity: usize) -> (ConfigEventSender, ConfigEventReceiver) {
    tokio::sync::broadcast::channel(capacity)
}

// ── PluginRuntimeEvent 通道（PluginManager → ConfigManager 解耦管道）──

/// PluginManager 发给 ConfigManager 的运行时事件。
///
/// CM 监听此通道：注册/解注册 Configurable，并转发为 ConfigEvent 通知 SessionRouter。
/// 这使得 PluginManager 不再直接依赖 ConfigManager。
#[derive(Clone, Debug)]
pub enum PluginRuntimeEvent {
    /// 插件组件已加载：CM 应注册所有 Configurable，然后转发 PluginRegistered 到 SR
    PluginLoaded(RegisteredAdapters),
    /// 插件组件已卸载：CM 应解注册所有 Configurable，然后转发 PluginUnregistered 到 SR
    PluginUnloaded(RegisteredAdapters),
}

/// PluginRuntimeEvent 广播通道发送端
pub type PluginEventSender = tokio::sync::broadcast::Sender<PluginRuntimeEvent>;
/// PluginRuntimeEvent 广播通道接收端
pub type PluginEventReceiver = tokio::sync::broadcast::Receiver<PluginRuntimeEvent>;

/// 创建 PluginRuntimeEvent 广播通道
pub fn create_plugin_event_bus(capacity: usize) -> (PluginEventSender, PluginEventReceiver) {
    tokio::sync::broadcast::channel(capacity)
}
