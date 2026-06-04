use serde::Deserialize;

/// Top-level plugin manifest deserialized from `manifest.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub plugin: PluginSection,
    #[serde(default)]
    pub runtime: RuntimeSection,
    #[serde(default)]
    pub components: ComponentsSection,
    #[serde(default)]
    pub ui: Option<UiSection>,
    #[serde(default)]
    pub icon: Option<IconSection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginSection {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    #[serde(default, rename = "homepage")]
    pub homepage: Option<String>,
    #[serde(default, rename = "license")]
    pub license: Option<String>,
    #[serde(rename = "minHostVersion")]
    pub min_host_version: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RuntimeSection {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_startup_timeout")]
    #[serde(rename = "startupTimeout")]
    pub startup_timeout: u64,
    #[serde(default = "default_true")]
    #[serde(rename = "autoRestart")]
    pub auto_restart: bool,
    #[serde(default = "default_max_restart")]
    #[serde(rename = "maxRestart")]
    pub max_restart: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ComponentsSection {
    #[serde(default = "default_provides")]
    pub provides: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UiSection {
    #[serde(rename = "panelEntry")]
    pub panel_entry: Option<String>,
    #[serde(rename = "settingsEntry")]
    pub settings_entry: Option<String>,
    #[serde(rename = "resultItemEntry")]
    pub result_item_entry: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IconSection {
    pub path: String,
}

fn default_startup_timeout() -> u64 {
    10
}
fn default_true() -> bool {
    true
}
fn default_max_restart() -> u32 {
    3
}
fn default_provides() -> Vec<String> {
    vec![]
}

/// Required manifest fields that must be present and valid.
pub const REQUIRED_PROVIDES_VALUES: &[&str] = &["plugin", "data_source", "action_executor"];

/// Regex pattern for reverse-domain plugin IDs.
pub const PLUGIN_ID_RE: &str = r"^[a-z][a-z0-9]*(\.[a-z][a-z0-9_-]*)+$";
