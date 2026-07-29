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
    /// 最近一次从持久化存储加载的配置快照。
    /// 用于第三方插件延迟注册（在 load_from_storage 之后）时恢复其已保存配置。
    /// None 表示尚未执行 load_from_storage（首次运行或启动初期）。
    loaded_config: RwLock<Option<PersistentConfig>>,
}

impl ConfigManager {
    pub fn new(config_dir: PathBuf) -> Self {
        let (event_sender, _receiver) = create_event_bus(256);
        Self {
            registry: ConfigurableRegistry::new(),
            enabled_map: RwLock::new(HashMap::new()),
            store: ConfigStore::new(config_dir),
            event_sender,
            loaded_config: RwLock::new(None),
        }
    }

    /// 获取事件发送端的引用，用于订阅配置变更事件
    pub fn event_sender(&self) -> &ConfigEventSender {
        &self.event_sender
    }

    // region: 组件注册

    /// 注册一个可配置组件。
    /// 同时将其信息写入类型索引，并发布 Registered 事件。
    ///
    /// 如果已通过 `load_from_storage()` 加载了持久化配置且该组件有已保存状态，
    /// 则优先恢复已保存配置（用于第三方插件在启动后延迟注册的场景）。
    /// 否则应用 schema 默认值。
    /// 应用初始值后执行校验，校验失败则拒绝注册。
    pub fn register(&self, component: Arc<dyn Configurable>) {
        let id = component.component_id().to_string();
        let component_type = component.component_type();

        if let Err(error) = component.settings_contribution() {
            error!("拒绝注册（配置 schema 无效）: {} - {}", id, error);
            return;
        }

        // 检查是否有已加载的持久化配置适用于此组件
        // 用于 load_from_storage 之后注册的第三方插件恢复其已保存配置
        let saved_state = self
            .loaded_config
            .read()
            .as_ref()
            .and_then(|config| config.components.get(&id).cloned());

        let initialized = if let Some(state) = &saved_state {
            // 存在已保存配置：验证通过后应用，失败则回退默认值
            if component.validate_settings(&state.settings).is_err() {
                warn!("组件 {} 的已保存配置校验失败，回退默认值", id);
                false
            } else if let Err(e) = component.apply_settings(state.settings.clone()) {
                error!("组件 {} 的已保存配置应用失败: {}, 回退默认值", id, e);
                false
            } else {
                info!("注册（恢复已保存配置）: {} ({:?})", id, component_type);
                true
            }
        } else {
            // 无已保存配置，需应用 schema 默认值
            false
        };

        if !initialized {
            // 应用 defaults 作为回退或初始值
            let defaults = component.get_default_settings();
            if let Err(e) = component.apply_settings(defaults) {
                error!("拒绝注册（应用 schema 默认值失败）: {} - {}", id, e);
                return;
            }
            if let Err(error) = component.validate_settings(&component.get_settings()) {
                error!("拒绝注册（默认配置值无效）: {} - {}", id, error);
                return;
            }
            info!("注册可配置组件: {} ({:?})", id, component_type);
        } else {
            info!(
                "恢复已保存配置并注册可配置组件: {} ({:?})",
                id, component_type
            );
        }

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
    /// 流程：验证 → 剔除 transient 字段 → 应用 → 持久化（成功后才发事件）
    ///
    /// 持久化失败时回滚内存状态，保证运行时状态与持久化状态一致。
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

        // 备份旧配置，以便持久化失败时回滚
        let old_settings = component.get_settings();

        component.apply_settings(cleaned)?;

        // 先持久化，成功后才发布事件
        if let Err(e) = self.save_to_storage() {
            // 持久化失败，回滚内存状态
            let _ = component.apply_settings(old_settings);
            return Err(e);
        }

        // 持久化成功后，触发回调和事件
        component.on_settings_changed();
        self.event_sender
            .send(ConfigEvent::SettingsChanged {
                component_id: component_id.to_string(),
                component_type: component.component_type(),
            })
            .ok();

        Ok(())
    }

    pub fn reset_to_default(&self, component_id: &str) -> Result<(), ConfigError> {
        let component = self
            .registry
            .get(component_id)
            .ok_or_else(|| ConfigError::NotFound(component_id.to_string()))?;

        let old_settings = component.get_settings();
        let default_settings = component.get_default_settings();
        component.apply_settings(default_settings.clone())?;

        // 先持久化，成功后才发布事件
        if let Err(e) = self.save_to_storage() {
            // 持久化失败，回滚内存状态
            let _ = component.apply_settings(old_settings);
            return Err(e);
        }

        component.on_settings_changed();
        self.event_sender
            .send(ConfigEvent::SettingsChanged {
                component_id: component_id.to_string(),
                component_type: component.component_type(),
            })
            .ok();

        Ok(())
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

    /// 设置组件启用状态。
    /// 先持久化，成功后才发布事件。
    pub fn set_enabled(&self, component_id: &str, enabled: bool) -> Result<(), ConfigError> {
        let component = self
            .registry
            .get(component_id)
            .ok_or_else(|| ConfigError::NotFound(component_id.to_string()))?;

        let old_enabled = self.is_enabled(component_id);
        self.enabled_map
            .write()
            .insert(component_id.to_string(), enabled);

        // 先持久化，成功后才发布事件
        if let Err(e) = self.save_to_storage() {
            // 持久化失败，回滚内存状态
            self.enabled_map
                .write()
                .insert(component_id.to_string(), old_enabled);
            return Err(e);
        }

        self.event_sender
            .send(ConfigEvent::EnabledChanged {
                component_id: component_id.to_string(),
                component_type: component.component_type(),
                enabled,
            })
            .ok();

        Ok(())
    }

    /// 从本地持久化文件加载配置，应用到所有已注册组件。
    ///
    /// 加载前先校验每个组件的已保存配置是否符合当前 schema，校验失败时回退到默认值。
    /// 配置文件损坏时自动备份并继续使用空配置。
    /// 加载完成后保存配置快照供后续延迟注册的组件（如第三方插件）恢复。
    pub fn load_from_storage(&self) -> Result<(), ConfigError> {
        let config = match self.store.load() {
            Ok(config) => config,
            Err(e) => {
                warn!("加载持久化配置失败: {}，将使用默认配置", e);
                // 备份损坏的配置文件，保留现场便于排查
                if let Err(backup_err) = self.store.backup_corrupted() {
                    warn!("备份损坏配置文件失败: {}", backup_err);
                }
                PersistentConfig::default()
            }
        };

        for (component_id, state) in &config.components {
            self.enabled_map
                .write()
                .insert(component_id.clone(), state.enabled);

            if let Some(component) = self.registry.get(component_id) {
                // 先校验已保存配置是否符合当前 schema
                if let Err(e) = component.validate_settings(&state.settings) {
                    warn!(
                        "组件 {} 的已保存配置校验失败，跳过加载: {}",
                        component_id, e
                    );
                    continue;
                }

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

        // 保存配置快照，供后续 register() 恢复延迟注册组件的配置
        *self.loaded_config.write() = Some(config.clone());

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
    /// 保存成功后更新内存中的配置快照，供后续 register() 恢复延迟注册组件使用。
    pub fn save_to_storage(&self) -> Result<(), ConfigError> {
        let config = self.build_persistent_config();
        self.store.save(&config)?;
        // 保存成功后更新内存快照
        *self.loaded_config.write() = Some(config);
        Ok(())
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
