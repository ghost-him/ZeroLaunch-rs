use crate::core::config::event::{
    create_event_bus, ConfigEvent, ConfigEventSender, PluginRuntimeEvent,
};
use crate::core::config::models::{
    ComponentInfoSnapshot, ComponentPersistentState, ComponentSchemaSnapshot, PersistentConfig,
};
use crate::core::config::registry::ConfigurableRegistry;
use crate::core::config::store::ConfigStore;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use zerolaunch_plugin_api::config::{ComponentType, ConfigError, Configurable};

/// 配置管理中枢。
/// 负责所有可配置组件的注册、配置 CRUD、持久化和事件发布。
pub struct ConfigManager {
    /// 组件注册中心
    registry: ConfigurableRegistry,
    /// 配置持久化层（始终使用本地存储）
    store: ConfigStore,
    /// enabled 状态持久化
    enabled_map: RwLock<HashMap<String, bool>>,
    /// 配置变更事件发送端
    event_sender: ConfigEventSender,
}

impl ConfigManager {
    pub fn new(config_dir: PathBuf) -> Self {
        let (event_sender, _receiver) = create_event_bus(256);
        Self {
            registry: ConfigurableRegistry::new(),
            enabled_map: RwLock::new(HashMap::new()),
            store: ConfigStore::new(config_dir),
            event_sender,
        }
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

        if let Err(error) = component.settings_contribution() {
            error!("拒绝注册（配置 schema 无效）: {} - {}", id, error);
            return;
        }
        if let Err(error) = component.validate_settings(&component.get_settings()) {
            error!("拒绝注册（当前配置值无效）: {} - {}", id, error);
            return;
        }

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
    pub fn get_all_components(&self) -> Vec<ComponentInfoSnapshot> {
        let mut components: Vec<ComponentInfoSnapshot> = self
            .registry
            .get_all()
            .iter()
            .map(|c| ComponentInfoSnapshot {
                component_id: c.component_id().to_string(),
                component_name: c.component_name().to_string(),
                component_description: c.component_description().to_string(),
                component_type: c.component_type(),
                priority: c.priority(),
                enabled: self.is_enabled(c.component_id()),
                default_enabled: c.default_enabled(),
            })
            .collect();
        components.sort_by_key(|c| (c.priority, c.component_id.clone()));
        components
    }

    pub fn get_component_schema(&self, component_id: &str) -> Option<ComponentSchemaSnapshot> {
        self.registry.get(component_id).and_then(|c| {
            let contribution = match c.settings_contribution() {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("组件 '{}' 的 schema 校验失败: {}", component_id, e);
                    return None;
                }
            };
            Some(ComponentSchemaSnapshot {
                component_id: c.component_id().to_string(),
                component_name: c.component_name().to_string(),
                component_description: c.component_description().to_string(),
                component_type: c.component_type(),
                contribution,
            })
        })
    }

    /// 获取指定组件的当前配置值
    pub fn get_settings(&self, component_id: &str) -> Option<serde_json::Value> {
        self.registry.get(component_id).map(|c| c.get_settings())
    }

    /// 获取指定组件中单个配置项的值。
    /// 用于运行时读取被动配置（如窗口行为设置）。
    pub fn get_component_setting(
        &self,
        component_id: &str,
        key: &str,
    ) -> Option<serde_json::Value> {
        self.get_settings(component_id)
            .and_then(|settings| settings.get(key).cloned())
    }

    /// 按 component_id 查找已注册的 Configurable 组件
    pub fn find_configurable(&self, component_id: &str) -> Option<Arc<dyn Configurable>> {
        self.registry.get(component_id)
    }

    /// 获取指定组件的配置动作列表。
    pub fn get_config_actions(
        &self,
        component_id: &str,
    ) -> Vec<zerolaunch_plugin_api::config::ConfigActionDef> {
        self.registry
            .get(component_id)
            .map(|c| c.config_actions())
            .unwrap_or_default()
    }

    /// 执行指定组件的配置动作。
    pub async fn execute_config_action(
        &self,
        component_id: &str,
        action: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, ConfigError> {
        let component = self
            .registry
            .get(component_id)
            .ok_or_else(|| ConfigError::NotFound(component_id.to_string()))?;
        component
            .execute_config_action(action, params)
            .await
            .map_err(ConfigError::ApplyFailed)
    }

    /// 按 ComponentType 查找所有组件
    pub fn get_by_type(&self, component_type: ComponentType) -> Vec<Arc<dyn Configurable>> {
        self.registry.get_by_type(component_type)
    }

    /// 应用配置到指定组件。
    /// 流程：验证 → 剔除 transient 字段 → 应用 → 回调 → 事件 → 持久化
    pub fn apply_settings(
        &self,
        component_id: &str,
        settings: serde_json::Value,
    ) -> Result<(), ConfigError> {
        let component = self
            .registry
            .get(component_id)
            .ok_or_else(|| ConfigError::NotFound(component_id.to_string()))?;

        component.validate_settings(&settings)?;

        // 剔除 transient effect 字段，防止其被持久化。
        // transient 字段仅用于 UI 动作参数传递，不应写入 settings。
        let cleaned = strip_transient_fields(&*component, settings);

        component.apply_settings(cleaned)?;
        component.on_settings_changed();

        self.event_sender
            .send(ConfigEvent::SettingsChanged {
                component_id: component_id.to_string(),
                component_type: component.component_type(),
            })
            .ok();

        self.save_to_storage()
    }

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

        self.save_to_storage()
    }

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

        self.enabled_map
            .write()
            .insert(component_id.to_string(), enabled);

        self.event_sender
            .send(ConfigEvent::EnabledChanged {
                component_id: component_id.to_string(),
                component_type: component.component_type(),
                enabled,
            })
            .ok();

        self.save_to_storage()
    }

    /// 从本地持久化文件加载配置，应用到所有已注册组件。
    pub fn load_from_storage(&self) -> Result<(), ConfigError> {
        let config = self.store.load().unwrap_or_default();

        for (component_id, state) in &config.components {
            self.enabled_map
                .write()
                .insert(component_id.clone(), state.enabled);

            if let Some(component) = self.registry.get(component_id) {
                if let Err(e) = component.apply_settings(state.settings.clone()) {
                    warn!("加载组件配置失败: {}, 错误: {}", component_id, e);
                } else {
                    component.on_settings_changed();
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

    /// 构建包含所有已注册组件当前配置的 PersistentConfig 对象。
    ///
    /// 此方法仅读取状态、构建数据结构，不执行任何 I/O。
    /// 返回的 PersistentConfig 可供本地持久化或远程同步使用。
    pub fn build_persistent_config(&self) -> PersistentConfig {
        let mut config = PersistentConfig::default();

        for component in self.registry.get_all() {
            let component_id = component.component_id().to_string();
            let enabled = self.is_enabled(&component_id);
            let settings = component.get_settings();

            config
                .components
                .insert(component_id, ComponentPersistentState { enabled, settings });
        }

        config
    }
    /// 将当前所有组件的配置保存到本地持久化文件。
    /// 返回：保存成功返回 Ok，失败返回 Err。
    /// 远程同步已提取到 bootstrap.rs 中，由 ConfigEvent 监听器负责触发。
    pub fn save_to_storage(&self) -> Result<(), ConfigError> {
        let config = self.build_persistent_config();
        self.store.save(&config)
    }

    /// 处理 PluginManager 发来的 PluginRuntimeEvent。
    ///
    /// 纯业务逻辑：注册/解注册 Configurable，转发 ConfigEvent 通知 SessionRouter。
    /// 事件循环由 lib.rs 负责（与 SR 的 ConfigEvent 监听模式一致）。
    pub fn handle_plugin_event(&self, event: &PluginRuntimeEvent) {
        match event {
            PluginRuntimeEvent::PluginLoaded(adapters) => {
                for c in &adapters.components {
                    self.register(c.clone());
                }
                self.event_sender
                    .send(ConfigEvent::PluginRegistered(adapters.clone()))
                    .ok();
            }
            PluginRuntimeEvent::PluginUnloaded(adapters) => {
                for c in &adapters.components {
                    self.unregister(c.component_id());
                }
                self.event_sender
                    .send(ConfigEvent::PluginUnregistered(adapters.clone()))
                    .ok();
            }
        }
    }

    // endregion
}

/// 从 settings 中剔除当前组件声明为 transient 的 effect 字段。
///
/// transient 字段仅作为动作参数传递给 `config_execute_action`，不应写入持久化 settings。
/// 前端 `stripTransientSettings()` 在 IPC 调用前已做一次过滤，后端此处做二次保障，
/// 确保即使绕过前端（如通过 CLI HTTP API），transient 字段也不会被持久化。
fn strip_transient_fields(
    component: &dyn Configurable,
    settings: serde_json::Value,
) -> serde_json::Value {
    let Some(mut object) = settings.as_object().cloned() else {
        // 非 object 类型的 settings（如 null）没有字段需要剔除。
        return settings;
    };

    let contribution = match component.settings_contribution() {
        Ok(c) => c,
        Err(e) => {
            warn!("获取组件 schema 失败，跳过 transient 过滤: {}", e);
            return serde_json::Value::Object(object);
        }
    };

    for ui in &contribution.ui {
        let Some(action) = &ui.action else { continue };
        let is_transient = matches!(
            action,
            zerolaunch_plugin_api::config::FieldAction::Effect(b)
                if b.transient
        );
        if !is_transient {
            continue;
        }
        // pointer 格式如 "/custom_icon_path"，去掉前导 / 即为 settings key。
        let key = ui.pointer.trim_start_matches('/');
        if object.contains_key(key) {
            debug!(
                "剔除 transient 字段: {} (组件 {})",
                key,
                component.component_id()
            );
            object.remove(key);
        }
    }

    serde_json::Value::Object(object)
}
