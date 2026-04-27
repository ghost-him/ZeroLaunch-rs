use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::{
    ArrayItem, ArrayUiHint, DataSource, ExecutionTarget, FieldDefinition, SearchCandidate,
    SettingType,
};
use crate::plugin_system::{ComponentType, ConfigError, Configurable, SettingDefinition};
use crate::sdk::host_api::PluginHandle;
use parking_lot::RwLock;
use std::path::Path;
use std::sync::Arc;
use tracing::debug;

/// 自定义命令数据源插件，负责从用户配置的命令列表中加载数据源候选项。
pub struct CommandSource {
    settings: RwLock<serde_json::Value>,
    handle: Arc<PluginHandle>,
}

impl CommandSource {
    pub fn new(handle: Arc<PluginHandle>) -> Self {
        CommandSource {
            settings: RwLock::new(serde_json::Value::Null),
            handle,
        }
    }

    /// 从 settings 中解析命令列表配置
    /// 返回 (名称, 命令) 的列表
    fn parse_commands(&self) -> Vec<(String, String)> {
        self.settings
            .read()
            .get("commands")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let name = item.get("name")?.as_str()?.to_string();
                        let command = item.get("command")?.as_str()?.to_string();
                        Some((name, command))
                    })
                    .collect()
            })
            .unwrap_or_default()
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
    fn resolve_icon(&self, command: &str) -> String {
        if let Some(path) = Self::resolve_executable_path(command) {
            debug!("CommandSource: 解析到可执行文件路径用于图标: {}", path);
            return path;
        }
        self.handle
            .get_app_icon_path("terminal")
            .unwrap_or_default()
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
        vec![SettingDefinition {
            field: FieldDefinition {
                key: "commands".to_string(),
                label: "自定义命令".to_string(),
                description: "配置要索引的自定义命令快捷方式".to_string(),
                setting_type: SettingType::Array {
                    item: ArrayItem::Object(vec![
                        FieldDefinition {
                            key: "name".to_string(),
                            label: "名称".to_string(),
                            description: "命令的显示名称".to_string(),
                            setting_type: SettingType::Text,
                            default_value: serde_json::json!(""),
                            visible: true,
                            editable: true,
                        },
                        FieldDefinition {
                            key: "command".to_string(),
                            label: "命令".to_string(),
                            description: "要执行的命令或程序路径".to_string(),
                            setting_type: SettingType::Text,
                            default_value: serde_json::json!(""),
                            visible: true,
                            editable: true,
                        },
                    ]),
                    min_items: None,
                    max_items: None,
                    ui_hint: ArrayUiHint::Table,
                },
                default_value: serde_json::json!([]),
                visible: true,
                editable: true,
            },
            group: Some("命令配置".to_string()),
            order: 1,
            config_action: None,
        }]
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        *self.settings.write() = settings;
        Ok(())
    }
}

impl DataSource for CommandSource {
    fn fetch_candidates(&self) -> CachedCandidateData {
        let mut result = CachedCandidateData::new();
        let commands = self.parse_commands();

        for (name, command) in &commands {
            if name.is_empty() || command.is_empty() {
                continue;
            }

            let icon = self.resolve_icon(command);
            let candidate = SearchCandidate {
                id: 0,
                name: name.clone(),
                icon,
                target: ExecutionTarget::Command(command.clone()),
                keywords: Vec::new(),
                bias: 0.0,
            };

            debug!("CommandSource: 加载命令候选项: {} -> {}", name, command);
            result.add_candidate(candidate);
        }

        result
    }
}
