//! RemoteExecutorAdapter — implements ActionExecutor trait via JSON-RPC.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use zerolaunch_plugin_api::config::{
    ComponentType, ConfigActionDef, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{
    ActionExecutor, ExecutionContext, ExecutionError, PluginContext, ResultAction, TargetType,
};

use crate::adapter::remote_configurable::RemoteConfigurableAdapter;
use crate::client::JsonRpcClient;
use zerolaunch_plugin_protocol::messages::*;
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;

#[derive(Debug, Clone)]
pub struct RemoteExecutorAdapter {
    pub component_id: String,
    pub configurable: Arc<RemoteConfigurableAdapter>,
    pub client: Arc<JsonRpcClient>,
    pub cached_target_types: Vec<TargetType>,
    pub cached_actions: Vec<ResultAction>,
}

impl Configurable for RemoteExecutorAdapter {
    fn component_id(&self) -> &str {
        self.configurable.component_id()
    }
    fn component_name(&self) -> &str {
        self.configurable.component_name()
    }
    fn component_type(&self) -> ComponentType {
        self.configurable.component_type()
    }
    fn priority(&self) -> u32 {
        self.configurable.priority()
    }
    fn setting_schema(&self) -> Vec<SettingDefinition> {
        self.configurable.setting_schema()
    }
    fn get_settings(&self) -> serde_json::Value {
        self.configurable.get_settings()
    }
    fn config_actions(&self) -> Vec<ConfigActionDef> {
        self.configurable.config_actions()
    }
    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        self.configurable.apply_settings(settings)
    }
    fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
        self.configurable.validate_settings(settings)
    }
    fn execute_config_action(
        &self,
        action: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.configurable.execute_config_action(action, params)
    }
}

#[async_trait]
impl ActionExecutor for RemoteExecutorAdapter {
    fn supported_target_types(&self) -> Vec<TargetType> {
        self.cached_target_types.clone()
    }

    fn supported_actions(&self) -> Vec<ResultAction> {
        self.cached_actions.clone()
    }

    async fn execute(
        &self,
        _ctx: &ExecutionContext,
        action_id: &str,
    ) -> Result<(), ExecutionError> {
        let result: Result<ExecutorExecuteResult, _> = self
            .client
            .call(
                plugin_methods::EXECUTOR_EXECUTE,
                ExecutorExecuteParams {
                    component_id: self.component_id.clone(),
                    ctx: PluginContext {
                        trace_id: "exec".into(),
                        query_id: None,
                        plugin_id: Some(self.component_id.clone()),
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
