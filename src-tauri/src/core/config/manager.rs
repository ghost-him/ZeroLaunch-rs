use crate::core::config::event::{create_event_bus, ConfigEvent, ConfigEventSender};
use crate::core::config::models::{
    ComponentInfo, ComponentPersistentState, ComponentSchema, PersistentConfig,
};
use crate::core::config::registry::ConfigurableRegistry;
use crate::core::config::store::ConfigStore;
use crate::core::types::{ComponentType, ConfigError, Configurable};
use crate::sdk::host_api::HostApi;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// 配置管理中枢。
/// 负责所有可配置组件的注册、配置 CRUD、持久化和事件发布。
pub struct ConfigManager {
    /// 组件注册中心
    registry: ConfigurableRegistry,
    /// 配置持久化层（始终使用本地存储）
    store: ConfigStore,
    /// 可选的 HostApi 引用（设置后启用远程同步）
    host_api: RwLock<Option<Arc<HostApi>>>,
    /// enabled 状态持久化
    enabled_map: RwLock<HashMap<String, bool>>,
    /// 配置变更事件发送端
    event_sender: ConfigEventSender,
}

impl ConfigManager {
    /// 创建 ConfigManager 实例。
    /// 参数：config_dir - 配置文件目录
    pub fn new(config_dir: PathBuf) -> Self {
        let store = ConfigStore::new(config_dir);
        let (event_sender, _) = create_event_bus(256);

        Self {
            registry: ConfigurableRegistry::new(),
            store,
            host_api: RwLock::new(None),
            enabled_map: RwLock::new(HashMap::new()),
            event_sender,
        }
    }

    /// 设置 HostApi 引用，启用远程同步。
    /// 参数：host_api - HostApi 实例。
    /// 返回：无。
    /// 特性：设置后，配置保存时会同步到远程存储后端。
    pub fn set_host_api(&self, host_api: Arc<HostApi>) {
        *self.host_api.write() = Some(host_api);
    }

    /// 获取事件发送端的引用，用于订阅配置变更事件
    pub fn event_sender(&self) -> &ConfigEventSender {
        &self.event_sender
    }

    // region: 组件注册

    /// 注册一个可配置组件。
    /// 同时将其信息写入类型索引，并发布 Registered 事件。
    pub fn register(&self, component: Arc<dyn Configurable>) {
        let id = component.component_id().to_string();
        let component_type = component.component_type();

        info!("注册可配置组件: {} ({:?})", id, component_type);
        self.registry.register(component);

        self.event_sender
            .send(ConfigEvent::Registered {
                component_id: id,
                component_type,
            })
            .ok();
    }

    /// 注销一个可配置组件
    pub fn unregister(&self, component_id: &str) {
        info!("注销可配置组件: {}", component_id);
        self.registry.unregister(component_id);

        self.event_sender
            .send(ConfigEvent::Unregistered {
                component_id: component_id.to_string(),
            })
            .ok();
    }

    // endregion

    // region: 配置读取

    /// 获取所有可配置组件的概览信息
    pub fn get_all_components(&self) -> Vec<ComponentInfo> {
        self.registry
            .get_all()
            .iter()
            .map(|c| ComponentInfo {
                component_id: c.component_id().to_string(),
                component_name: c.component_name().to_string(),
                component_type: c.component_type(),
                enabled: self.is_enabled(c.component_id()),
                default_enabled: c.default_enabled(),
            })
            .collect()
    }

    /// 获取指定组件的配置 Schema
    pub fn get_component_schema(&self, component_id: &str) -> Option<ComponentSchema> {
        self.registry.get(component_id).map(|c| ComponentSchema {
            component_id: c.component_id().to_string(),
            component_name: c.component_name().to_string(),
            component_type: c.component_type(),
            settings: c.setting_schema(),
        })
    }

    /// 获取指定组件的当前配置值
    pub fn get_settings(&self, component_id: &str) -> Option<serde_json::Value> {
        self.registry.get(component_id).map(|c| c.get_settings())
    }

    /// 按 component_id 查找已注册的 Configurable 组件
    pub fn find_configurable(&self, component_id: &str) -> Option<Arc<dyn Configurable>> {
        self.registry.get(component_id)
    }

    /// 按 ComponentType 查找所有组件
    pub fn get_by_type(&self, component_type: ComponentType) -> Vec<Arc<dyn Configurable>> {
        self.registry.get_by_type(component_type)
    }

    // endregion

    // region: 配置写入

    /// 应用配置到指定组件。
    /// 流程：验证 → 应用 → 回调 → 事件 → 持久化
    pub fn apply_settings(
        &self,
        component_id: &str,
        settings: serde_json::Value,
    ) -> Result<(), ConfigError> {
        let component = self
            .registry
            .get(component_id)
            .ok_or_else(|| ConfigError::NotFound(component_id.to_string()))?;

        // 1. 验证
        component.validate_settings(&settings)?;

        // 2. 应用
        component.apply_settings(settings.clone())?;

        // 3. 回调
        component.on_settings_changed();

        // 4. 事件
        self.event_sender
            .send(ConfigEvent::SettingsChanged {
                component_id: component_id.to_string(),
                component_type: component.component_type(),
            })
            .ok();

        // 5. 持久化
        self.save_to_storage();

        Ok(())
    }

    /// 重置组件配置为默认值
    pub fn reset_to_default(&self, component_id: &str) -> Result<(), ConfigError> {
        let component = self
            .registry
            .get(component_id)
            .ok_or_else(|| ConfigError::NotFound(component_id.to_string()))?;

        let default_settings = component.get_default_settings();
        component.apply_settings(default_settings.clone())?;
        component.on_settings_changed();

        self.event_sender
            .send(ConfigEvent::SettingsChanged {
                component_id: component_id.to_string(),
                component_type: component.component_type(),
            })
            .ok();

        self.save_to_storage();

        Ok(())
    }

    // endregion

    // region: 启用/禁用

    /// 查询组件是否启用。
    /// 优先查 enabled_map（持久化的用户选择），未记录则查组件的 default_enabled() 默认值。
    pub fn is_enabled(&self, component_id: &str) -> bool {
        self.enabled_map
            .read()
            .get(component_id)
            .copied()
            .unwrap_or_else(|| {
                self.registry
                    .get(component_id)
                    .map(|c| c.default_enabled())
                    .unwrap_or(true)
            })
    }

    /// 设置组件启用状态
    pub fn set_enabled(&self, component_id: &str, enabled: bool) -> Result<(), ConfigError> {
        let component = self
            .registry
            .get(component_id)
            .ok_or_else(|| ConfigError::NotFound(component_id.to_string()))?;

        // 1. 更新内存中的 enabled 状态
        self.enabled_map
            .write()
            .insert(component_id.to_string(), enabled);

        // 2. 发布事件
        self.event_sender
            .send(ConfigEvent::EnabledChanged {
                component_id: component_id.to_string(),
                component_type: component.component_type(),
                enabled,
            })
            .ok();

        // 3. 持久化
        self.save_to_storage();

        Ok(())
    }

    // endregion

    // region: 持久化

    /// 从持久化文件加载配置，应用到所有已注册组件。
    /// 参数：local_only - true 时仅从本地文件加载，跳过远程存储。
    ///         初始化阶段应传 true，因为远程存储可能尚未配置。
    pub fn load_from_storage(&self, local_only: bool) -> Result<(), ConfigError> {
        let config = if local_only {
            self.store.load().unwrap_or_default()
        } else if let Some(host_api) = self.host_api.read().as_ref() {
            match tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async { host_api.storage().download("zerolaunch_config.json").await })
            }) {
                Ok(Some(data)) => match serde_json::from_slice::<PersistentConfig>(&data) {
                    Ok(remote_config) => {
                        info!("从远程存储加载配置成功");
                        if let Err(e) = self.store.save(&remote_config) {
                            warn!("远程配置本地备份失败: {}", e);
                        }
                        remote_config
                    }
                    Err(e) => {
                        warn!("远程配置解析失败: {}, 回退到本地", e);
                        self.store.load().unwrap_or_default()
                    }
                },
                Ok(None) => {
                    debug!("远程存储无配置文件，使用本地配置");
                    self.store.load().unwrap_or_default()
                }
                Err(e) => {
                    warn!("远程配置加载失败: {}, 回退到本地", e);
                    self.store.load().unwrap_or_default()
                }
            }
        } else {
            self.store.load().unwrap_or_default()
        };

        for (component_id, state) in &config.components {
            self.enabled_map
                .write()
                .insert(component_id.clone(), state.enabled);

            if let Some(component) = self.registry.get(component_id) {
                if let Err(e) = component.apply_settings(state.settings.clone()) {
                    warn!("加载组件配置失败: {}, 错误: {}", component_id, e);
                } else {
                    component.on_settings_changed();
                    debug!("已从持久化加载组件配置: {}", component_id);
                }
            }
        }

        // 初始化在持久化配置中不存在的新组件，应用其默认配置
        for component in self.registry.get_all() {
            let component_id = component.component_id().to_string();
            if !config.components.contains_key(&component_id) {
                let defaults = component.get_default_settings();
                if defaults.is_null() || defaults.as_object().map(|o| o.is_empty()).unwrap_or(false)
                {
                    continue;
                }
                if let Err(e) = component.apply_settings(defaults) {
                    warn!("应用默认配置失败: {}, 错误: {}", component_id, e);
                } else {
                    component.on_settings_changed();
                    info!("首次初始化组件默认配置: {}", component_id);
                }
            }
        }

        info!(
            "配置加载完成，已加载 {} 个持久化配置，共 {} 个已注册组件",
            config.components.len(),
            self.registry.len()
        );
        Ok(())
    }

    /// 将当前所有组件的配置保存到持久化文件。
    /// 如果设置了 HostApi，同时异步同步到远程存储后端。
    pub fn save_to_storage(&self) {
        let mut config = PersistentConfig::default();

        for component in self.registry.get_all() {
            let component_id = component.component_id().to_string();
            let enabled = self.is_enabled(&component_id);
            let settings = component.get_settings();

            config
                .components
                .insert(component_id, ComponentPersistentState { enabled, settings });
        }

        // 始终保存到本地
        if let Err(e) = self.store.save(&config) {
            warn!("配置本地持久化失败: {}", e);
        }

        // 如果设置了 HostApi，异步同步到远程（仅在 tokio 上下文中执行）
        if let Some(host_api) = self.host_api.read().as_ref() {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let json_bytes = match serde_json::to_vec(&config) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        warn!("配置序列化失败，跳过远程同步: {}", e);
                        return;
                    }
                };
                let storage = host_api.storage();
                handle.spawn(async move {
                    if let Err(e) = storage.upload("zerolaunch_config.json", &json_bytes).await {
                        warn!("配置远程同步失败: {}", e);
                    }
                });
            }
        }
    }

    // endregion
}
