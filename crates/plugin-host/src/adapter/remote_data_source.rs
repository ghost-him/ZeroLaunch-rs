//! RemoteDataSourceAdapter — implements DataSource trait via JSON-RPC.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use zerolaunch_plugin_api::config::{
    ComponentType, ConfigActionDef, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{CachedCandidateData, DataSource};

use crate::adapter::remote_configurable::RemoteConfigurableAdapter;
use crate::client::JsonRpcClient;
use zerolaunch_plugin_protocol::messages::*;
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;

#[derive(Debug, Clone)]
pub struct RemoteDataSourceAdapter {
    pub component_id: String,
    pub configurable: Arc<RemoteConfigurableAdapter>,
    pub client: Arc<JsonRpcClient>,
}

impl Configurable for RemoteDataSourceAdapter {
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
impl DataSource for RemoteDataSourceAdapter {
    async fn fetch_candidates(&self) -> CachedCandidateData {
        let result: Result<FetchCandidatesResult, _> = self
            .client
            .call(
                plugin_methods::FETCH_CANDIDATES,
                FetchCandidatesParams {
                    component_id: self.component_id.clone(),
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
                    self.component_id,
                    e
                );
                CachedCandidateData::new()
            }
        }
    }
}
