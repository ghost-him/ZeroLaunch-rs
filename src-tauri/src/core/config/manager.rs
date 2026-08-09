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
    /// async：远端插件组件的 apply_settings/validate_settings 需经 RPC 下发。
    pub async fn register(&self, component: Arc<dyn Configurable>) {
        let id = component.component_id().to_string();
        let component_type = component.component_type();

        // 查重：同一 component_id 只允许注册一次。跨插件组件 id 碰撞时，
        // 直接 insert 会静默覆盖先注册者的配置（settings/enabled 键同 id），
        // 这里拒绝并报错；第三方插件加载路径在 handle_plugin_event 中整包预检，
        // 此处为兜底防线（内置注册、自声明重复 id 等所有路径统一生效）。
        if self.registry.get(&id).is_some() {
            error!(
                "拒绝注册重复组件 id: {}（已存在同名组件，跳过本次注册）",
                id
            );
            return;
        }

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
            if component.validate_settings(&state.settings).await.is_err() {
                warn!("组件 {} 的已保存配置校验失败，回退默认值", id);
                false
            } else if let Err(e) = component.apply_settings(state.settings.clone()).await {
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
            if let Err(e) = component.apply_settings(defaults).await {
                error!("拒绝注册（应用 schema 默认值失败）: {} - {}", id, e);
                return;
            }
            if let Err(error) = component.validate_settings(&component.get_settings()).await {
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
    /// async：远端插件组件的 validate/apply 需经 RPC 下发。
    pub async fn apply_settings(
        &self,
        component_id: &str,
        settings: serde_json::Value,
    ) -> Result<(), ConfigError> {
        let component = self
            .registry
            .get(component_id)
            .ok_or_else(|| ConfigError::NotFound(component_id.to_string()))?;

        component.validate_settings(&settings).await?;

        // 剔除 transient effect 字段，防止其被持久化。
        // transient 字段仅用于 UI 动作参数传递，不应写入 settings。
        let cleaned = strip_transient_fields(&*component, settings);

        // 备份旧配置，以便持久化失败时回滚
        let old_settings = component.get_settings();

        component.apply_settings(cleaned).await?;

        // 先持久化，成功后才发布事件
        if let Err(e) = self.save_to_storage() {
            // 持久化失败，回滚内存状态
            let _ = component.apply_settings(old_settings).await;
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

    pub async fn reset_to_default(&self, component_id: &str) -> Result<(), ConfigError> {
        let component = self
            .registry
            .get(component_id)
            .ok_or_else(|| ConfigError::NotFound(component_id.to_string()))?;

        let old_settings = component.get_settings();
        let default_settings = component.get_default_settings();
        component.apply_settings(default_settings.clone()).await?;

        // 先持久化，成功后才发布事件
        if let Err(e) = self.save_to_storage() {
            // 持久化失败，回滚内存状态
            let _ = component.apply_settings(old_settings).await;
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
    /// 从本地持久化文件加载配置，应用到所有已注册组件。
    /// async：远端插件组件的 validate/apply 需经 RPC 下发（不得同步 block_on）。
    pub async fn load_from_storage(&self) -> Result<(), ConfigError> {
        let config = match self.store.load() {
            Ok(c) => c,
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
                if let Err(e) = component.validate_settings(&state.settings).await {
                    warn!(
                        "组件 {} 的已保存配置校验失败，跳过加载: {}",
                        component_id, e
                    );
                    continue;
                }

                if let Err(e) = component.apply_settings(state.settings.clone()).await {
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
                if let Err(e) = component.apply_settings(defaults).await {
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
    /// async：注册远端插件组件时 apply/validate 需经 RPC 下发（不得同步 block_on）。
    pub async fn handle_plugin_event(&self, event: &PluginRuntimeEvent) {
        match event {
            PluginRuntimeEvent::PluginLoaded(adapters) => {
                // 兜底防线：主冲突预检已下沉到 plugin-host 的 load/crash_loop
                // （组件清单到手后、登记前拒绝并关进程）。此处防的是未来新增的
                // 加载路径漏检，或插件间竞态（预检与注册之间插入其他插件）。
                // 任一组件与已注册组件撞 id 则拒绝整个插件，避免
                // 「部分组件注册成功、路由已建但配置缺失」的半提交状态。
                //
                // 注意：命中此处**不必然**表示预检有缺陷——预检是同步快照、
                // 注册是异步事件，两者之间并发加载同 id 插件会正常触发本分支；
                // 排查时应先排除「并发安装/重载同 id 插件」再怀疑预检漏检。
                for c in &adapters.components {
                    if let Some(existing) = self.registry.get(c.component_id()) {
                        error!(
                            "拒绝注册插件 {}：组件 id '{}' 与已注册组件（{}）冲突，插件整体跳过\
                             （兜底命中：竞态窗口或预检漏检，见 handle_plugin_event 注释）",
                            adapters.plugin_id,
                            c.component_id(),
                            existing.component_name()
                        );
                        return;
                    }
                }
                for c in &adapters.components {
                    self.register(c.clone()).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use zerolaunch_plugin_api::config::{ComponentCore, SettingDefinition};
    use zerolaunch_plugin_api::plugin::PluginMetadata;
    use zerolaunch_plugin_api::PanelInteraction;
    use zerolaunch_plugin_host::adapter::remote_component::{RemoteComponent, RemoteComponentKind};
    use zerolaunch_plugin_host::client::JsonRpcClient;
    use zerolaunch_plugin_host::manager::PluginRegistration;
    use zerolaunch_plugin_protocol::manifest::{Manifest, PluginSection};

    /// 测试用最小 Configurable —— 仅承载身份元数据，空 schema 与空设置。
    ///
    /// 仅限本文件（manager.rs 测试模块）内使用，
    /// 用于验证注册查重与第三方插件整包拒绝逻辑。
    struct StubComponent {
        /// 组件 ID、名称、类型等元数据。
        core: ComponentCore,
    }
    impl Configurable for StubComponent {
        fn core(&self) -> &ComponentCore {
            &self.core
        }
        fn setting_schema(&self) -> Vec<SettingDefinition> {
            vec![]
        }
        fn get_settings(&self) -> serde_json::Value {
            json!({})
        }
    }

    /// 注册查重契约：同一 component_id 第二次注册被拒绝，先注册者不被覆盖。
    #[tokio::test]
    async fn register_rejects_duplicate_component_id() {
        let cm = ConfigManager::new(std::env::temp_dir().join("zl-duplicate-register-test"));
        let first: Arc<dyn Configurable> = Arc::new(StubComponent {
            core: ComponentCore::new(
                "dup".into(),
                "先注册".into(),
                "第一个".into(),
                ComponentType::Plugin,
                0,
            ),
        });
        let second: Arc<dyn Configurable> = Arc::new(StubComponent {
            core: ComponentCore::new(
                "dup".into(),
                "后注册".into(),
                "第二个".into(),
                ComponentType::Plugin,
                0,
            ),
        });

        cm.register(first).await;
        cm.register(second).await;

        let registered = cm.find_configurable("dup").expect("先注册组件应保留");
        assert_eq!(
            registered.component_name(),
            "先注册",
            "后注册者不得覆盖先注册者"
        );
    }

    /// 第三方插件整包拒绝契约：任一组件 id 与已注册组件冲突时，
    /// 整个插件不注册任何组件（含无冲突组件），避免「路由已建但配置缺失」的半提交状态。
    #[tokio::test]
    async fn handle_plugin_event_rejects_plugin_with_colliding_component() {
        let cm = ConfigManager::new(std::env::temp_dir().join("zl-plugin-collision-test"));
        cm.register(Arc::new(StubComponent {
            core: ComponentCore::new(
                "shared".into(),
                "内置组件".into(),
                "宿主占位".into(),
                ComponentType::Plugin,
                0,
            ),
        }))
        .await;

        // 构造第三方插件注册包：组件 shared（与宿主冲突）+ unique（无冲突）。
        let (req_tx, _req_rx) = tokio::sync::mpsc::channel(16);
        let (notif_tx, _notif_rx) = tokio::sync::mpsc::channel(16);
        let (reader, writer) = tokio::io::duplex(64);
        let client =
            JsonRpcClient::new(tokio::io::BufReader::new(reader), writer, req_tx, notif_tx);
        let metadata = PluginMetadata {
            id: "com.example.collide".into(),
            name: "碰撞测试".into(),
            version: "1.0.0".into(),
            description: String::new(),
            author: String::new(),
            trigger_keywords: vec![],
            supported_os: vec![],
            priority: 0,
        };
        let make_component = |component_id: &str, name: &str| {
            Arc::new(RemoteComponent::new(
                component_id.into(),
                name.into(),
                String::new(),
                ComponentType::Plugin,
                50,
                client.clone(),
                vec![],
                json!({}),
                vec![],
                true,
                RemoteComponentKind::Plugin {
                    metadata: metadata.clone(),
                    interaction_policy: parking_lot::RwLock::new(PanelInteraction::default()),
                },
            ))
        };
        let registration = PluginRegistration {
            plugin_id: "com.example.collide".into(),
            manifest: Manifest {
                plugin: PluginSection {
                    id: "com.example.collide".into(),
                    name: "碰撞测试".into(),
                    version: "1.0.0".into(),
                    description: String::new(),
                    author: String::new(),
                    homepage: None,
                    license: None,
                    min_host_version: "0.0.0".into(),
                },
                runtime: Default::default(),
                components: Default::default(),
                ui: None,
                icon: None,
            },
            components: vec![
                make_component("shared", "冲突组件"),
                make_component("unique", "独立组件"),
            ],
        };

        cm.handle_plugin_event(&PluginRuntimeEvent::PluginLoaded(registration))
            .await;

        assert!(
            cm.find_configurable("unique").is_none(),
            "冲突插件应整包拒绝，无冲突组件也不得注册"
        );
        let shared = cm.find_configurable("shared").expect("宿主组件应保留");
        assert_eq!(
            shared.component_name(),
            "内置组件",
            "插件组件不得覆盖宿主组件"
        );
    }
}
