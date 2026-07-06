use serde::{Deserialize, Serialize};

/// 顶层插件 manifest，从 `manifest.toml` 反序列化，也可序列化为 JSON 返回给前端。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// 插件元信息（ID、名称、版本、作者等）
    #[serde(rename = "plugin")]
    pub plugin: PluginSection,
    /// 运行时配置（启动命令、超时、自动重启策略）
    #[serde(default, rename = "runtime")]
    pub runtime: RuntimeSection,
    /// 组件声明（插件对外提供哪些能力）
    #[serde(default, rename = "components")]
    pub components: ComponentsSection,
    /// 前端 UI 入口（第三方插件可选的 Vue 面板）
    #[serde(default, rename = "ui")]
    pub ui: Option<UiSection>,
    /// 插件图标文件路径
    #[serde(default, rename = "icon")]
    pub icon: Option<IconSection>,
}

/// 插件元信息段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSection {
    /// 插件唯一标识，反向域名格式，如 `com.example.my-plugin`
    #[serde(rename = "id")]
    pub id: String,
    /// 显示名称
    #[serde(rename = "name")]
    pub name: String,
    /// 语义版本号
    #[serde(rename = "version")]
    pub version: String,
    /// 简短描述
    #[serde(rename = "description")]
    pub description: String,
    /// 作者名
    #[serde(rename = "author")]
    pub author: String,
    /// 项目主页 URL（可选）
    #[serde(default, rename = "homepage")]
    pub homepage: Option<String>,
    /// 开源许可证标识（可选），如 `MIT`、`GPL-3.0`
    #[serde(default, rename = "license")]
    pub license: Option<String>,
    /// 宿主最低兼容版本，如 `"1.2.0"`
    #[serde(rename = "minHostVersion")]
    pub min_host_version: String,
}

/// 运行时配置段。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeSection {
    /// 可执行文件路径（相对于插件目录），如 `./bin/my-plugin`
    #[serde(rename = "command")]
    pub command: String,
    /// 启动参数列表
    #[serde(default, rename = "args")]
    pub args: Vec<String>,
    /// 启动超时秒数，默认 10 秒
    #[serde(default = "default_startup_timeout", rename = "startupTimeout")]
    pub startup_timeout: u64,
    /// 崩溃后是否自动重启，默认 true
    #[serde(default = "default_true", rename = "autoRestart")]
    pub auto_restart: bool,
    /// 最大重启次数，超过后不再自动拉起，默认 3
    #[serde(default = "default_max_restart", rename = "maxRestart")]
    pub max_restart: u32,
}

/// 组件声明段。
/// 插件在此声明对外提供哪些能力。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentsSection {
    /// 能力列表，可选值见 `REQUIRED_PROVIDES_VALUES`。
    /// 插件可以声明多个能力（如同时提供 `data_source` 和 `action_executor`），
    /// 每个能力在构建期对应一个 `RemoteConfigurableAdapter`。
    #[serde(default = "default_provides", rename = "provides")]
    pub provides: Vec<String>,
}

/// 前端 UI 入口段（可选）。
/// 第三方插件可以注册自定义 Vue 面板，嵌入宿主设置页。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSection {
    /// 插件信息面板入口组件路径
    #[serde(rename = "panelEntry")]
    pub panel_entry: Option<String>,
    /// 插件配置面板入口组件路径
    #[serde(rename = "settingsEntry")]
    pub settings_entry: Option<String>,
    /// 搜索结果项自定义渲染组件路径
    #[serde(rename = "resultItemEntry")]
    pub result_item_entry: Option<String>,
}

/// 图标配置段（可选）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconSection {
    /// 图标文件路径（相对于插件目录），建议使用 PNG 或 SVG
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
