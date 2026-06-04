//! RemoteConfigurableAdapter — implements Configurable trait via JSON-RPC.

use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use zerolaunch_plugin_api::config::{
    ComponentType, ConfigActionDef, ConfigError, Configurable, SettingDefinition,
};

use crate::client::JsonRpcClient;
use zerolaunch_plugin_protocol::messages::*;
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;
use zerolaunch_plugin_protocol::ProtocolError;

pub struct RemoteConfigurableAdapter {
    pub component_id: String,
    pub component_name: String,
    pub component_type: ComponentType,
    pub client: Arc<JsonRpcClient>,
    pub cached_schema: RwLock<Vec<SettingDefinition>>,
    pub cached_settings: RwLock<serde_json::Value>,
    pub cached_actions: RwLock<Vec<ConfigActionDef>>,
}

/// Helper: convert a ProtocolError to a ConfigError.
fn to_config_error(e: ProtocolError) -> ConfigError {
    ConfigError::ApplyFailed(e.to_string())
}

impl RemoteConfigurableAdapter {
    pub fn new(
        component_id: String,
        component_name: String,
        component_type: ComponentType,
        client: Arc<JsonRpcClient>,
        schema: Vec<SettingDefinition>,
        settings: serde_json::Value,
        actions: Vec<ConfigActionDef>,
    ) -> Self {
        Self {
            component_id,
            component_name,
            component_type,
            client,
            cached_schema: RwLock::new(schema),
            cached_settings: RwLock::new(settings),
            cached_actions: RwLock::new(actions),
        }
    }

    /// Block on an async RPC call from a synchronous context.
    fn block_on_rpc<F, T, E>(f: F) -> Result<T, E>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(f))
    }
}

impl Configurable for RemoteConfigurableAdapter {
    fn component_id(&self) -> &str {
        &self.component_id
    }

    fn component_name(&self) -> &str {
        &self.component_name
    }

    fn component_type(&self) -> ComponentType {
        self.component_type
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
        let component_id = self.component_id.clone();
        let settings_clone = settings.clone();
        Self::block_on_rpc(async move {
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
        let component_id = self.component_id.clone();
        let settings_clone = settings.clone();
        let result: ValidateSettingsResult = Self::block_on_rpc(async move {
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
        let component_id = self.component_id.clone();
        let action = action.to_string();
        let params = params.clone();
        let result: Result<serde_json::Value, ProtocolError> = Self::block_on_rpc(async move {
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
