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
    ActionExecutor, CachedCandidateData, DataSource, ExecutionContext, ExecutionError,
    PanelInteraction, Plugin, PluginContext, PluginError, PluginHandle, PluginMetadata, Query,
    QueryResponse, ResultAction, TargetType,
};

use crate::client::JsonRpcClient;
use zerolaunch_plugin_protocol::messages::*;
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;
use zerolaunch_plugin_protocol::{codes, ProtocolError};

/// 远程插件组件的种类与专属数据。
///
/// 判断字段归属的标准：
/// - 只有某个种类需要 → 放入对应 variant；
/// - 所有种类都需要（如 Configurable 相关的缓存）→ 保留在 `RemoteComponent` struct 层面。
#[derive(Debug)]
pub enum RemoteComponentKind {
    DataSource,
    ActionExecutor {
        target_types: Vec<TargetType>,
        result_actions: Vec<ResultAction>,
    },
    Plugin {
        metadata: PluginMetadata,
        /// 交互策略缓存：discover 拉初始值，查询/设置变更时经 RPC 刷新
        /// （PanelInteraction 是插件级语义——仅 Plugin 组件持有；
        /// 内置插件在每次会话推送时同步求值，远端以此对齐）。
        interaction_policy: RwLock<PanelInteraction>,
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
    /// 默认启用状态：discover 时经 RPC 拉取（与内置 `default_enabled()` 语义一致）。
    cached_default_enabled: RwLock<bool>,

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
        default_enabled: bool,
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
            cached_default_enabled: RwLock::new(default_enabled),
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
#[async_trait]
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

    /// 默认启用状态来自插件自声明（discover 时经 RPC 拉取），与内置组件一致。
    fn default_enabled(&self) -> bool {
        *self.cached_default_enabled.read()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let client = self.client.clone();
        let component_id = self.core.component_id().to_string();
        let settings_clone = settings.clone();
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
            .map_err(to_config_error)?;
        *self.cached_settings.write() = settings;
        // 设置变更可能改变交互策略（如 on-enter 模式开关）：
        // 应用成功后立即刷新策略缓存，避免 reemit_current_session 推送旧策略
        // （内置插件每次推送同步求值，远端以此对齐）。
        self.refresh_interaction_policy().await;
        Ok(())
    }

    async fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
        // 1. 宿主侧 Schema 校验（key 合法性、类型、对象结构、数值/数组约束等）
        //    宿主 Schema 是最终权威，插件校验只能补充不能取代。
        let contribution = self.settings_contribution()?;
        contribution
            .validate_values(settings)
            .map_err(ConfigError::ValidationFailed)?;

        // 2. 插件侧业务校验（通过 RPC 委托远程插件）
        let client = self.client.clone();
        let component_id = self.core.component_id().to_string();
        let settings_clone = settings.clone();
        let result: ValidateSettingsResult = client
            .call(
                plugin_methods::VALIDATE_SETTINGS,
                ValidateSettingsParams {
                    component_id,
                    settings: settings_clone,
                },
                Duration::from_secs(5),
            )
            .await
            .map_err(to_config_error)?;
        if let Some(error) = result.error {
            return Err(ConfigError::ValidationFailed(error));
        }
        Ok(())
    }

    async fn execute_config_action(
        &self,
        action: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let client = self.client.clone();
        let component_id = self.core.component_id().to_string();
        let action = action.to_string();
        let params = params.clone();
        let result: Result<serde_json::Value, ProtocolError> = client
            .call::<_, serde_json::Value>(
                plugin_methods::EXECUTE_CONFIG_ACTION,
                ExecuteConfigActionParams {
                    component_id,
                    action,
                    params,
                },
                Duration::from_secs(10),
            )
            .await;
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

    async fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
        assert!(
            matches!(self.kind, RemoteComponentKind::ActionExecutor { .. }),
            "RemoteComponent {} is not an ActionExecutor but execute() was called",
            self.core.component_id()
        );

        // 完整执行上下文原样透传（与进程内 ActionExecutor 一致），
        // 不再重建伪 PluginContext（旧实现丢弃 target 等字段并伪造 trace_id）。
        let result: Result<ExecutorExecuteResult, _> = self
            .client
            .call(
                plugin_methods::EXECUTOR_EXECUTE,
                ExecutorExecuteParams {
                    component_id: self.core.component_id().to_string(),
                    execution_ctx: ctx.clone(),
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
            RemoteComponentKind::Plugin { metadata, .. } => metadata,
            _ => panic!(
                "RemoteComponent {} is not a Plugin but metadata() was called",
                self.core.component_id()
            ),
        }
    }

    async fn init(
        &self,
        ctx: &PluginContext,
        _handle: Option<Arc<PluginHandle>>,
    ) -> Result<(), PluginError> {
        assert!(
            matches!(self.kind, RemoteComponentKind::Plugin { .. }),
            "RemoteComponent {} is not a Plugin but init() was called",
            self.core.component_id()
        );
        // 远端进程无宿主句柄（跨进程不可序列化），init 语义为通知插件进程完成初始化；
        // 平台能力由插件侧经 host() 的 host/* RPC 访问。
        // METHOD_NOT_FOUND（旧 SDK 插件无 init 方法）= 无初始化需求，静默完成
        // （与引入 init RPC 前的 no-op 行为一致）；其他错误向上传播。
        let metadata = match &self.kind {
            RemoteComponentKind::Plugin { metadata, .. } => metadata,
            _ => unreachable!("kind 已在上面断言为 Plugin"),
        };
        let result: Result<serde_json::Value, _> = self
            .client
            .call(
                plugin_methods::INIT,
                InitParams {
                    plugin_id: metadata.id.clone(),
                    ctx: ctx.clone(),
                },
                Duration::from_secs(10),
            )
            .await;
        match result {
            Ok(_) => Ok(()),
            Err(zerolaunch_plugin_protocol::ProtocolError::Rpc { code, .. })
                if code == codes::METHOD_NOT_FOUND =>
            {
                Ok(())
            }
            Err(e) => Err(PluginError::InitFailed(e.to_string())),
        }
    }

    /// 交互策略：返回查询时刷新的缓存值（与内置每次查询同步求值语义对齐）。
    /// 仅 Plugin 组件有策略（kind 已断言为 Plugin）。
    fn interaction_policy(&self) -> PanelInteraction {
        match &self.kind {
            RemoteComponentKind::Plugin {
                interaction_policy, ..
            } => interaction_policy.read().clone(),
            _ => panic!(
                "RemoteComponent {} is not a Plugin but interaction_policy() was called",
                self.core.component_id()
            ),
        }
    }

    async fn query(
        &self,
        ctx: &PluginContext,
        query: &Query,
    ) -> Result<QueryResponse, PluginError> {
        let metadata = match &self.kind {
            RemoteComponentKind::Plugin { metadata, .. } => metadata,
            _ => panic!(
                "RemoteComponent {} is not a Plugin but query() was called",
                self.core.component_id()
            ),
        };

        // 每次查询先刷新交互策略缓存（内置插件在会话推送时同步求值，远端以此对齐）。
        self.refresh_interaction_policy().await;

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
            RemoteComponentKind::Plugin { metadata, .. } => metadata,
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

impl RemoteComponent {
    /// 经 RPC 刷新交互策略缓存（宿主在会话推送时同步读取）。
    /// 策略为插件级语义：非 Plugin 组件（DataSource/Executor）无策略，直接跳过。
    /// 失败静默：策略保持上次值（新协议方法，老 SDK 插件可能回 METHOD_NOT_FOUND）。
    async fn refresh_interaction_policy(&self) {
        let RemoteComponentKind::Plugin {
            interaction_policy, ..
        } = &self.kind
        else {
            return;
        };
        let result: Result<PanelInteraction, _> = self
            .client
            .call(
                plugin_methods::INTERACTION_POLICY,
                InteractionPolicyParams {
                    component_id: self.core.component_id().to_string(),
                },
                Duration::from_secs(5),
            )
            .await;
        if let Ok(policy) = result {
            *interaction_policy.write() = policy;
        }
    }
}
