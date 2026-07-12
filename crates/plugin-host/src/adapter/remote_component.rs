//! RemoteComponent — 远程插件组件的统一承载结构。
//!
//! 一个逻辑组件对应一个 `RemoteComponent` 实例，同时实现 `Configurable`、
//! `DataSource`、`ActionExecutor`、`Plugin` 四个 trait，消除原先多 adapter
//! 之间对 `RemoteConfigurableAdapter` 的机械委托。

use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigActionDef, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{
    ActionExecutor, CachedCandidateData, DataSource, ExecutionContext, ExecutionError, Plugin,
    PluginContext, PluginError, PluginHandle, PluginMetadata, Query, QueryResponse, ResultAction,
    TargetType,
};

use crate::client::JsonRpcClient;
use zerolaunch_plugin_protocol::messages::*;
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;
use zerolaunch_plugin_protocol::ProtocolError;

/// 远程插件组件的种类与专属数据。
///
/// 判断字段归属的标准：
/// - 只有某个种类需要 → 放入对应 variant；
/// - 所有种类都需要（如 Configurable 相关的缓存）→ 保留在 `RemoteComponent` struct 层面。
#[derive(Debug, Clone)]
pub enum RemoteComponentKind {
    DataSource,
    ActionExecutor {
        target_types: Vec<TargetType>,
        result_actions: Vec<ResultAction>,
    },
    Plugin {
        metadata: PluginMetadata,
    },
}

pub struct RemoteComponent {
    /// 身份核心。内置插件与远程插件共享同一身份模型。
    pub core: ComponentCore,

    // ── 通信 ──
    pub client: Arc<JsonRpcClient>,

    // ── 私有缓存 ──
    cached_settings: RwLock<serde_json::Value>,
    cached_schema: RwLock<Vec<SettingDefinition>>,
    /// 配置动作缓存。`config_actions()` 是 `Configurable` trait 的通用方法，
    /// 所有组件类型均可能使用，因此放在 struct 层面而非 kind variant 内部。
    cached_actions: RwLock<Vec<ConfigActionDef>>,

    // ── 种类与专属数据 ──
    pub kind: RemoteComponentKind,
}

/// Helper: convert a ProtocolError to a ConfigError.
fn to_config_error(e: ProtocolError) -> ConfigError {
    ConfigError::ApplyFailed(e.to_string())
}
impl std::fmt::Debug for RemoteComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteComponent")
            .field("core", &self.core)
            .field("cached_settings", &self.cached_settings.read())
            .field("cached_schema", &self.cached_schema.read())
            .field("cached_actions", &self.cached_actions.read())
            .field("kind", &self.kind)
            .finish()
    }
}

impl RemoteComponent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        component_id: String,
        component_name: String,
        component_description: String,
        component_type: ComponentType,
        priority: u32,
        client: Arc<JsonRpcClient>,
        schema: Vec<SettingDefinition>,
        settings: serde_json::Value,
        actions: Vec<ConfigActionDef>,
        kind: RemoteComponentKind,
    ) -> Self {
        Self {
            core: ComponentCore::new(
                component_id,
                component_name,
                component_description,
                component_type,
                priority,
            ),
            client,
            cached_settings: RwLock::new(settings),
            cached_schema: RwLock::new(schema),
            cached_actions: RwLock::new(actions),
            kind,
        }
    }

    /// 将自身转换为 `DataSource` trait object，仅在 kind 为 DataSource 时成功。
    pub fn as_data_source(self: Arc<Self>) -> Option<Arc<dyn DataSource>> {
        matches!(self.kind, RemoteComponentKind::DataSource).then(|| self as Arc<dyn DataSource>)
    }

    /// 将自身转换为 `ActionExecutor` trait object，仅在 kind 为 ActionExecutor 时成功。
    pub fn as_action_executor(self: Arc<Self>) -> Option<Arc<dyn ActionExecutor>> {
        matches!(self.kind, RemoteComponentKind::ActionExecutor { .. })
            .then(|| self as Arc<dyn ActionExecutor>)
    }

    /// 将自身转换为 `Plugin` trait object，仅在 kind 为 Plugin 时成功。
    pub fn as_plugin(self: Arc<Self>) -> Option<Arc<dyn Plugin>> {
        matches!(self.kind, RemoteComponentKind::Plugin { .. }).then(|| self as Arc<dyn Plugin>)
    }

    pub fn is_data_source(&self) -> bool {
        matches!(self.kind, RemoteComponentKind::DataSource)
    }

    pub fn is_action_executor(&self) -> bool {
        matches!(self.kind, RemoteComponentKind::ActionExecutor { .. })
    }

    pub fn is_plugin(&self) -> bool {
        matches!(self.kind, RemoteComponentKind::Plugin { .. })
    }
}
impl Configurable for RemoteComponent {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        self.cached_schema.read().clone()
    }

    fn get_settings(&self) -> serde_json::Value {
        self.cached_settings.read().clone()
    }

    fn config_actions(&self) -> Vec<ConfigActionDef> {
        self.cached_actions.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let client = self.client.clone();
        let component_id = self.core.component_id().to_string();
        let settings_clone = settings.clone();
        tokio::runtime::Handle::current().block_on(async move {
            client
                .call::<_, serde_json::Value>(
                    plugin_methods::APPLY_SETTINGS,
                    ApplySettingsParams {
                        component_id,
                        settings: settings_clone,
                    },
                    Duration::from_secs(5),
                )
                .await
                .map_err(to_config_error)
        })?;
        *self.cached_settings.write() = settings;
        Ok(())
    }

    fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
        let client = self.client.clone();
        let component_id = self.core.component_id().to_string();
        let settings_clone = settings.clone();
        let result: ValidateSettingsResult =
            tokio::runtime::Handle::current().block_on(async move {
                client
                    .call(
                        plugin_methods::VALIDATE_SETTINGS,
                        ValidateSettingsParams {
                            component_id,
                            settings: settings_clone,
                        },
                        Duration::from_secs(5),
                    )
                    .await
                    .map_err(to_config_error)
            })?;
        if let Some(error) = result.error {
            Err(ConfigError::ValidationFailed(error))
        } else {
            Ok(())
        }
    }

    fn execute_config_action(
        &self,
        action: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let client = self.client.clone();
        let component_id = self.core.component_id().to_string();
        let action = action.to_string();
        let params = params.clone();
        let result: Result<serde_json::Value, ProtocolError> = tokio::runtime::Handle::current()
            .block_on(async move {
                client
                    .call::<_, serde_json::Value>(
                        plugin_methods::EXECUTE_CONFIG_ACTION,
                        ExecuteConfigActionParams {
                            component_id,
                            action,
                            params,
                        },
                        Duration::from_secs(10),
                    )
                    .await
            });
        result.map_err(|e| e.to_string())
    }
}

#[async_trait]
impl DataSource for RemoteComponent {
    async fn fetch_candidates(&self) -> CachedCandidateData {
        assert!(
            matches!(self.kind, RemoteComponentKind::DataSource),
            "RemoteComponent {} is not a DataSource but fetch_candidates() was called",
            self.core.component_id()
        );

        let result: Result<FetchCandidatesResult, _> = self
            .client
            .call(
                plugin_methods::FETCH_CANDIDATES,
                FetchCandidatesParams {
                    component_id: self.core.component_id().to_string(),
                },
                Duration::from_secs(30),
            )
            .await;

        match result {
            Ok(data) => {
                let mut cache = CachedCandidateData::new();
                for candidate in data.candidates {
                    cache.add_candidate(candidate);
                }
                cache
            }
            Err(e) => {
                tracing::warn!(
                    "DataSource {} fetch_candidates failed: {}",
                    self.core.component_id(),
                    e
                );
                CachedCandidateData::new()
            }
        }
    }
}

#[async_trait]
impl ActionExecutor for RemoteComponent {
    fn supported_target_types(&self) -> Vec<TargetType> {
        match &self.kind {
            RemoteComponentKind::ActionExecutor { target_types, .. } => target_types.clone(),
            _ => panic!(
                "RemoteComponent {} is not an ActionExecutor but supported_target_types() was called",
                self.core.component_id()
            ),
        }
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        match &self.kind {
            RemoteComponentKind::ActionExecutor { result_actions, .. } => result_actions.clone(),
            _ => panic!(
                "RemoteComponent {} is not an ActionExecutor but supported_actions() was called",
                self.core.component_id()
            ),
        }
    }

    async fn execute(
        &self,
        _ctx: &ExecutionContext,
        action_id: &str,
    ) -> Result<(), ExecutionError> {
        assert!(
            matches!(self.kind, RemoteComponentKind::ActionExecutor { .. }),
            "RemoteComponent {} is not an ActionExecutor but execute() was called",
            self.core.component_id()
        );

        let result: Result<ExecutorExecuteResult, _> = self
            .client
            .call(
                plugin_methods::EXECUTOR_EXECUTE,
                ExecutorExecuteParams {
                    component_id: self.core.component_id().to_string(),
                    ctx: PluginContext {
                        trace_id: "exec".into(),
                        query_id: None,
                        plugin_id: Some(self.core.component_id().to_string()),
                    },
                    action_id: action_id.to_string(),
                },
                Duration::from_secs(30),
            )
            .await;

        match result {
            Ok(r) => {
                if let Some(error) = r.error {
                    Err(ExecutionError::Failed(error))
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(ExecutionError::Failed(e.to_string())),
        }
    }
}

#[async_trait]
impl Plugin for RemoteComponent {
    fn metadata(&self) -> &PluginMetadata {
        match &self.kind {
            RemoteComponentKind::Plugin { metadata } => metadata,
            _ => panic!(
                "RemoteComponent {} is not a Plugin but metadata() was called",
                self.core.component_id()
            ),
        }
    }

    async fn init(
        &self,
        _ctx: &PluginContext,
        _handle: Arc<PluginHandle>,
    ) -> Result<(), PluginError> {
        assert!(
            matches!(self.kind, RemoteComponentKind::Plugin { .. }),
            "RemoteComponent {} is not a Plugin but init() was called",
            self.core.component_id()
        );
        Ok(())
    }

    async fn query(
        &self,
        ctx: &PluginContext,
        query: &Query,
    ) -> Result<QueryResponse, PluginError> {
        let metadata = match &self.kind {
            RemoteComponentKind::Plugin { metadata } => metadata,
            _ => panic!(
                "RemoteComponent {} is not a Plugin but query() was called",
                self.core.component_id()
            ),
        };

        self.client
            .call::<_, QueryResponse>(
                plugin_methods::QUERY,
                QueryParams {
                    plugin_id: metadata.id.clone(),
                    ctx: ctx.clone(),
                    query: query.clone(),
                },
                Duration::from_secs(30),
            )
            .await
            .map_err(|e| PluginError::QueryFailed(e.to_string()))
    }

    async fn execute_action(
        &self,
        ctx: &PluginContext,
        action_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), PluginError> {
        let metadata = match &self.kind {
            RemoteComponentKind::Plugin { metadata } => metadata,
            _ => panic!(
                "RemoteComponent {} is not a Plugin but execute_action() was called",
                self.core.component_id()
            ),
        };

        self.client
            .call::<_, serde_json::Value>(
                plugin_methods::EXECUTE_ACTION,
                ExecuteActionParams {
                    plugin_id: metadata.id.clone(),
                    ctx: ctx.clone(),
                    action_id: action_id.to_string(),
                    payload,
                },
                Duration::from_secs(30),
            )
            .await
            .map_err(|e| PluginError::ActionFailed(e.to_string()))?;
        Ok(())
    }
}
