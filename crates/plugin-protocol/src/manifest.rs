use serde::{Deserialize, Serialize};

/// 顶层插件 manifest，从 `manifest.toml` 反序列化，也可序列化为 JSON 返回给前端。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(rename = "plugin")]
    pub plugin: PluginSection,
    #[serde(default, rename = "runtime")]
    pub runtime: RuntimeSection,
    #[serde(default, rename = "components")]
    pub components: ComponentsSection,
    #[serde(default, rename = "ui")]
    pub ui: Option<UiSection>,
    #[serde(default, rename = "icon")]
    pub icon: Option<IconSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSection {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "version")]
    pub version: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "author")]
    pub author: String,
    #[serde(default, rename = "homepage")]
    pub homepage: Option<String>,
    #[serde(default, rename = "license")]
    pub license: Option<String>,
    #[serde(rename = "minHostVersion")]
    pub min_host_version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeSection {
    #[serde(rename = "command")]
    pub command: String,
    #[serde(default, rename = "args")]
    pub args: Vec<String>,
    #[serde(default = "default_startup_timeout", rename = "startupTimeout")]
    pub startup_timeout: u64,
    #[serde(default = "default_true", rename = "autoRestart")]
    pub auto_restart: bool,
    #[serde(default = "default_max_restart", rename = "maxRestart")]
    pub max_restart: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentsSection {
    #[serde(default = "default_provides", rename = "provides")]
    pub provides: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSection {
    #[serde(rename = "panelEntry")]
    pub panel_entry: Option<String>,
    #[serde(rename = "settingsEntry")]
    pub settings_entry: Option<String>,
    #[serde(rename = "resultItemEntry")]
    pub result_item_entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconSection {
    #[serde(rename = "path")]
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

/// 必须存在的 manifest 字段。
pub const REQUIRED_PROVIDES_VALUES: &[&str] = &["plugin", "data_source", "action_executor"];

/// 反向域名格式插件 ID 的正则表达式。
pub const PLUGIN_ID_RE: &str = r"^[a-z][a-z0-9]*(\.[a-z][a-z0-9_-]*)+$";
