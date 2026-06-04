//! RemotePluginAdapter — implements Plugin trait via JSON-RPC.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use zerolaunch_plugin_api::config::{
    ComponentType, ConfigActionDef, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{
    Plugin, PluginContext, PluginError, PluginHandle, PluginMetadata, Query, QueryResponse,
};

use crate::adapter::remote_configurable::RemoteConfigurableAdapter;
use crate::client::JsonRpcClient;
use zerolaunch_plugin_protocol::messages::*;
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;

pub struct RemotePluginAdapter {
    pub metadata: PluginMetadata,
    pub client: Arc<JsonRpcClient>,
    pub configurable: Arc<RemoteConfigurableAdapter>,
}

impl Configurable for RemotePluginAdapter {
    fn component_id(&self) -> &str {
        self.configurable.component_id()
    }
    fn component_name(&self) -> &str {
        self.configurable.component_name()
    }
    fn component_type(&self) -> ComponentType {
        self.configurable.component_type()
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
impl Plugin for RemotePluginAdapter {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(
        &self,
        _ctx: &PluginContext,
        _handle: Arc<PluginHandle>,
    ) -> Result<(), PluginError> {
        Ok(())
    }

    async fn query(
        &self,
        ctx: &PluginContext,
        query: &Query,
    ) -> Result<QueryResponse, PluginError> {
        self.client
            .call::<_, QueryResponse>(
                plugin_methods::QUERY,
                QueryParams {
                    plugin_id: self.metadata.id.clone(),
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
        self.client
            .call::<_, serde_json::Value>(
                plugin_methods::EXECUTE_ACTION,
                ExecuteActionParams {
                    plugin_id: self.metadata.id.clone(),
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
