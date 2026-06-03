use crate::core::config::setting_builders::SchemaBuilder;
use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::{DataSource, ExecutionTarget, SearchCandidate};
use crate::plugin_system::{ComponentType, ConfigError, Configurable, SettingDefinition};
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tracing::debug;
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::IconRequest;

/// 单条自定义命令的配置项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEntry {
    #[serde(rename = "name", default)]
    pub name: String,
    #[serde(rename = "command", default)]
    pub command: String,
    /// 触发关键词，逗号分隔。为空时使用 name 作为默认触发词
    #[serde(rename = "triggerKeywords", default)]
    pub trigger_keywords: String,
}

/// 自定义命令数据源的强类型配置结构。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandSourceSettings {
    #[serde(rename = "commands", default)]
    pub commands: Vec<CommandEntry>,
}

/// 自定义命令数据源插件，负责从用户配置的命令列表中加载数据源候选项。
pub struct CommandSource {
    settings: RwLock<CommandSourceSettings>,
    handle: Arc<PluginHandle>,
}

impl CommandSource {
    pub fn new(handle: Arc<PluginHandle>) -> Self {
        CommandSource {
            settings: RwLock::new(CommandSourceSettings::default()),
            handle,
        }
    }

    /// 尝试从命令字符串中解析出可执行文件路径，用于图标提取。
    /// 处理三种情况：
    /// 1. 命令本身就是绝对路径且存在
    /// 2. 带引号的路径，如 "C:\Program Files\App.exe" --arg
    /// 3. 不带引号但带参数的路径，如 C:\App.exe --arg
    fn resolve_executable_path(command: &str) -> Option<String> {
        // 情况 1：命令本身就是绝对路径且文件存在
        if Path::new(command).is_absolute() && Path::new(command).exists() {
            return Some(command.to_string());
        }

        // 情况 2：处理带引号的路径
        if command.trim().starts_with('"') {
            return command
                .split('"')
                .nth(1)
                .map(|s| s.to_string())
                .filter(|s| Path::new(s).is_absolute() && Path::new(s).exists());
        }

        // 情况 3：处理不带引号但带参数的路径
        command
            .split_whitespace()
            .next()
            .map(|s| s.to_string())
            .filter(|s| Path::new(s).is_absolute() && Path::new(s).exists())
    }

    /// 获取命令候选项的图标路径。
    /// 优先使用解析出的可执行文件路径，无法解析时使用默认终端图标。
    fn resolve_icon(&self, command: &str) -> IconRequest {
        if let Some(path) = Self::resolve_executable_path(command) {
            debug!("CommandSource: 解析到可执行文件路径用于图标: {}", path);
            return IconRequest::Path(path);
        }
        IconRequest::Path(
            self.handle
                .get_app_icon_path("terminal")
                .unwrap_or_default(),
        )
    }
}

impl Configurable for CommandSource {
    fn component_id(&self) -> &str {
        "command-source"
    }

    fn component_name(&self) -> &str {
        "自定义命令数据源"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::DataSource
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::array("commands", "自定义命令", "配置要索引的自定义命令快捷方式")
                .group("命令配置")
                .order(1)
                .object_items(vec![
                    SchemaBuilder::text("name", "名称", "命令的显示名称")
                        .default("")
                        .build_field(),
                    SchemaBuilder::text("command", "命令", "要执行的命令或程序路径")
                        .default("")
                        .build_field(),
                    SchemaBuilder::text(
                        "triggerKeywords",
                        "触发关键词",
                        "逗号分隔的触发词列表。输入触发词+空格进入参数模式。为空时默认使用名称。",
                    )
                    .default("")
                    .build_field(),
                ])
                .table_ui()
                .default(serde_json::json!([]))
                .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: CommandSourceSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
        Ok(())
    }
}

#[async_trait]
impl DataSource for CommandSource {
    async fn fetch_candidates(&self) -> CachedCandidateData {
        let mut result = CachedCandidateData::new();
        let s = self.settings.read();

        for entry in &s.commands {
            if entry.name.is_empty() || entry.command.is_empty() {
                continue;
            }

            let icon = self.resolve_icon(&entry.command);
            // 解析触发关键词：逗号分隔，去空白，过滤空值；为空时默认使用名称
            let trigger_keywords: Vec<String> = if entry.trigger_keywords.is_empty() {
                vec![entry.name.to_lowercase()]
            } else {
                entry
                    .trigger_keywords
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            };
            let candidate = SearchCandidate {
                id: 0,
                name: entry.name.clone(),
                icon,
                target: ExecutionTarget::Command(entry.command.clone()),
                keywords: Vec::new(),
                bias: 0.0,
                trigger_keywords,
            };

            debug!(
                "CommandSource: 加载命令候选项: {} -> {}",
                entry.name, entry.command
            );
            result.add_candidate(candidate);
        }

        result
    }
}

use crate::plugin_system::builtin_registry::{DataSourceEntry, InventoryContext};

pub(crate) fn build_command_source(
    ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn DataSource>) {
    let handle = ctx.get_handle("command-source");
    let source: Arc<dyn DataSource> = Arc::new(CommandSource::new(handle));
    let configurable: Arc<dyn Configurable> = source.clone();
    (configurable, source)
}

::inventory::submit! {
    DataSourceEntry {
        component_id: "command-source",
        handle_key: "command-source",
        priority: 40,
        factory: build_command_source,
    }
}
