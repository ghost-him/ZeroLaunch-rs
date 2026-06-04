//! PluginHostManager — top-level orchestration for third-party plugins.

use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info, warn};

use zerolaunch_plugin_protocol::manifest::Manifest;
use zerolaunch_plugin_protocol::messages::ComponentKind;
use zerolaunch_plugin_protocol::ProtocolError;

use crate::adapter::remote_configurable::RemoteConfigurableAdapter;
use crate::adapter::remote_data_source::RemoteDataSourceAdapter;
use crate::adapter::remote_executor::RemoteExecutorAdapter;
use crate::adapter::remote_plugin::RemotePluginAdapter;
use crate::host_dispatch::HostCallHandler;
use crate::process::PluginProcess;

/// All adapters registered for a single plugin instance.
pub struct RegisteredAdapters {
    pub plugin_id: String,
    pub manifest: Manifest,
    pub plugin: Option<Arc<RemotePluginAdapter>>,
    pub data_sources: Vec<Arc<RemoteDataSourceAdapter>>,
    pub executors: Vec<Arc<RemoteExecutorAdapter>>,
    pub configurables: Vec<Arc<RemoteConfigurableAdapter>>,
}

/// Top-level manager for all third-party plugin processes.
pub struct PluginHostManager {
    pub processes: DashMap<String, Arc<PluginProcess>>,
    pub adapters: DashMap<String, RegisteredAdapters>,
    pub data_dir_root: PathBuf,
    pub log_dir_root: PathBuf,
}

/// Error type for plugin loading operations.
#[derive(Debug, thiserror::Error)]
pub enum PluginLoadError {
    #[error("manifest error: {0}")]
    Manifest(String),
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("plugin already loaded: {0}")]
    AlreadyLoaded(String),
    #[error("plugin not found: {0}")]
    NotFound(String),
}

impl PluginHostManager {
    pub fn new(data_dir_root: PathBuf, log_dir_root: PathBuf) -> Self {
        Self {
            processes: DashMap::new(),
            adapters: DashMap::new(),
            data_dir_root,
            log_dir_root,
        }
    }

    /// Load a plugin from a directory containing manifest.toml.
    pub async fn load(
        &self,
        plugin_dir: &Path,
        host_call_handler: Arc<dyn HostCallHandler>,
    ) -> Result<RegisteredAdapters, PluginLoadError> {
        let manifest_path = plugin_dir.join("manifest.toml");
        let manifest_bytes = std::fs::read_to_string(&manifest_path)
            .map_err(|e| PluginLoadError::Manifest(format!("cannot read manifest.toml: {}", e)))?;

        let manifest: Manifest = toml::from_str(&manifest_bytes)
            .map_err(|e| PluginLoadError::Manifest(format!("invalid manifest: {}", e)))?;

        // Validate manifest
        validate_manifest(&manifest, plugin_dir)?;

        let plugin_id = manifest.plugin.id.clone();

        // Check for duplicate
        if self.processes.contains_key(&plugin_id) {
            return Err(PluginLoadError::AlreadyLoaded(plugin_id));
        }

        let data_dir = self.data_dir_root.join(&plugin_id);
        let log_dir = self.log_dir_root.clone();

        // Ensure data directory exists
        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            warn!(
                "Failed to create plugin data dir {}: {}",
                data_dir.display(),
                e
            );
        }
        if let Err(e) = std::fs::create_dir_all(&log_dir) {
            warn!(
                "Failed to create plugin log dir {}: {}",
                log_dir.display(),
                e
            );
        }

        info!("Loading plugin {} from {}", plugin_id, plugin_dir.display());

        // Spawn process and run handshake
        let process = PluginProcess::spawn(
            &manifest,
            plugin_dir,
            &data_dir,
            &log_dir,
            host_call_handler,
        )
        .await?;

        // Discover components
        let init_result = process.discover_components().await?;

        // Build adapters from discovered components
        let adapters = build_adapters(&plugin_id, &manifest, process.client.clone(), &init_result);

        self.processes.insert(plugin_id.clone(), Arc::new(process));
        self.adapters.insert(plugin_id.clone(), adapters);

        let registered = self.adapters.get(&plugin_id).unwrap();
        Ok(RegisteredAdapters {
            plugin_id: registered.plugin_id.clone(),
            manifest: registered.manifest.clone(),
            plugin: registered.plugin.clone(),
            data_sources: registered.data_sources.clone(),
            executors: registered.executors.clone(),
            configurables: registered.configurables.clone(),
        })
    }

    /// Returns the plugins directory (parent of data_dir_root).
    pub fn plugins_dir(&self) -> PathBuf {
        self.data_dir_root
            .parent()
            .map(|p| p.join("plugins"))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// Unload a plugin: shutdown process and remove from registries.
    pub async fn unload(&self, plugin_id: &str) -> Result<(), PluginLoadError> {
        info!("Unloading plugin {}", plugin_id);

        if let Some((_, proc)) = self.processes.remove(plugin_id) {
            if let Ok(process) = Arc::try_unwrap(proc) {
                process.shutdown(std::time::Duration::from_secs(5)).await;
            }
        }
        self.adapters.remove(plugin_id);

        // Remove log file
        let log_file = self.log_dir_root.join(format!("{}.log", plugin_id));
        let _ = std::fs::remove_file(&log_file);

        Ok(())
    }

    /// Reload a plugin (unload + load).
    pub async fn reload(
        &self,
        plugin_id: &str,
        plugin_dir: &Path,
        host_call_handler: Arc<dyn HostCallHandler>,
    ) -> Result<RegisteredAdapters, PluginLoadError> {
        self.unload(plugin_id).await?;
        self.load(plugin_dir, host_call_handler).await
    }
}

/// Information about an installed plugin for the management UI / CLI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstalledPluginInfo {
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "version")]
    pub version: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "author")]
    pub author: String,
    #[serde(rename = "state")]
    pub state: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
}

// ─── Helpers ──────────────────────────────────────────────────────

fn validate_manifest(manifest: &Manifest, plugin_dir: &Path) -> Result<(), PluginLoadError> {
    let id = &manifest.plugin.id;

    // Validate plugin ID format
    let re = regex::Regex::new(zerolaunch_plugin_protocol::manifest::PLUGIN_ID_RE).unwrap();
    if !re.is_match(id) {
        return Err(PluginLoadError::Manifest(format!(
            "invalid plugin id '{}': must match reverse domain",
            id
        )));
    }

    // Validate version
    if semver::Version::parse(&manifest.plugin.version).is_err() {
        return Err(PluginLoadError::Manifest(format!(
            "invalid plugin version '{}'",
            manifest.plugin.version
        )));
    }

    // Validate required provides
    if manifest.components.provides.is_empty() {
        return Err(PluginLoadError::Manifest(
            "components.provides must have at least one entry".into(),
        ));
    }

    for p in &manifest.components.provides {
        if !zerolaunch_plugin_protocol::manifest::REQUIRED_PROVIDES_VALUES.contains(&p.as_str()) {
            return Err(PluginLoadError::Manifest(format!(
                "unknown component type '{}'",
                p
            )));
        }
    }

    // Validate command exists
    let cmd_path = plugin_dir.join(&manifest.runtime.command);
    if !cmd_path.exists() {
        return Err(PluginLoadError::Manifest(format!(
            "command not found: {}",
            cmd_path.display()
        )));
    }

    Ok(())
}

fn build_adapters(
    plugin_id: &str,
    manifest: &Manifest,
    client: Arc<crate::client::JsonRpcClient>,
    init_result: &crate::process::InitResult,
) -> RegisteredAdapters {
    let mut plugin_adapter: Option<Arc<RemotePluginAdapter>> = None;
    let mut data_sources = Vec::new();
    let mut executors = Vec::new();
    let mut configurables = Vec::new();

    for comp in &init_result.components {
        let schema = init_result
            .settings_schemas
            .iter()
            .find(|(id, _)| id == &comp.component_id)
            .map(|(_, s)| s.clone())
            .unwrap_or_default();

        let settings = init_result
            .settings_values
            .iter()
            .find(|(id, _)| id == &comp.component_id)
            .map(|(_, s)| s.clone())
            .unwrap_or(serde_json::Value::Null);

        let actions = init_result
            .config_actions_map
            .iter()
            .find(|(id, _)| id == &comp.component_id)
            .map(|(_, a)| a.clone())
            .unwrap_or_default();

        let config = Arc::new(RemoteConfigurableAdapter::new(
            comp.component_id.clone(),
            comp.component_name.clone(),
            comp.component_type,
            client.clone(),
            schema,
            settings,
            actions,
        ));
        configurables.push(config.clone());

        match &comp.kind {
            ComponentKind::Plugin { trigger_keywords } => {
                // Create a RemotePluginAdapter
                let metadata = zerolaunch_plugin_api::PluginMetadata {
                    id: plugin_id.to_string(),
                    name: comp.component_name.clone(),
                    version: manifest.plugin.version.clone(),
                    description: manifest.plugin.description.clone(),
                    author: manifest.plugin.author.clone(),
                    trigger_keywords: trigger_keywords.clone(),
                    supported_os: vec!["windows".to_string()],
                    priority: comp.priority,
                };

                plugin_adapter = Some(Arc::new(RemotePluginAdapter {
                    metadata,
                    client: client.clone(),
                    configurable: config.clone(),
                }));
            }
            ComponentKind::DataSource => {
                data_sources.push(Arc::new(RemoteDataSourceAdapter {
                    component_id: comp.component_id.clone(),
                    configurable: config.clone(),
                    client: client.clone(),
                }));
            }
            ComponentKind::ActionExecutor { target_types } => {
                let cached_actions = init_result
                    .config_actions_map
                    .iter()
                    .find(|(id, _)| id == &comp.component_id)
                    .map(|(_, a)| {
                        a.iter()
                            .map(|ca| zerolaunch_plugin_api::ResultAction {
                                id: ca.action.clone(),
                                label: ca.label.clone(),
                                icon: zerolaunch_plugin_api::services::icon_request::IconRequest::Path(String::new()),
                                is_default: false,
                                shortcut_key: String::new(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                executors.push(Arc::new(RemoteExecutorAdapter {
                    component_id: comp.component_id.clone(),
                    configurable: config.clone(),
                    client: client.clone(),
                    cached_target_types: target_types.clone(),
                    cached_actions,
                }));
            }
        }
    }

    RegisteredAdapters {
        plugin_id: plugin_id.to_string(),
        manifest: manifest.clone(),
        plugin: plugin_adapter,
        data_sources,
        executors,
        configurables,
    }
}
