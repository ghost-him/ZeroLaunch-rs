use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{CachedCandidateData, DataSource, ExecutionTarget, SearchCandidate};

/// 内置命令数据源的强类型配置结构（当前无用户可配置项，仅用于占位）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuiltinCommandSourceSettings;

/// 单条内置命令的定义。
struct BuiltinCommandDef {
    name: &'static str,
    command: &'static str,
    trigger_keywords: &'static [&'static str],
}

/// 5 条内置命令的静态定义。
const BUILTIN_COMMANDS: &[BuiltinCommandDef] = &[
    BuiltinCommandDef {
        name: "打开设置",
        command: "ShowSettings",
        trigger_keywords: &["设置", "settings"],
    },
    BuiltinCommandDef {
        name: "刷新数据库",
        command: "RefreshDatabase",
        trigger_keywords: &["刷新", "refresh"],
    },
    BuiltinCommandDef {
        name: "重新注册快捷键",
        command: "ReregisterHotkeys",
        trigger_keywords: &["注册", "reshortcut"],
    },
    BuiltinCommandDef {
        name: "切换游戏模式",
        command: "ToggleGameMode",
        trigger_keywords: &["游戏", "gamemode"],
    },
    BuiltinCommandDef {
        name: "退出程序",
        command: "ExitProgram",
        trigger_keywords: &["退出", "exit"],
    },
];

/// 内置命令数据源插件，产出 5 个系统级操作候选项。
pub struct BuiltinCommandSource {
    core: ComponentCore,
    settings: RwLock<BuiltinCommandSourceSettings>,
}

impl BuiltinCommandSource {
    pub fn new(_handle: Arc<PluginHandle>) -> Self {
        BuiltinCommandSource {
            core: ComponentCore::new(
                "builtin-command-source".to_string(),
                "内置命令数据源".to_string(),
                "提供 ZeroLaunch 内置命令，如打开设置、清空缓存".to_string(),
                ComponentType::DataSource,
                10,
            ),
            settings: RwLock::new(BuiltinCommandSourceSettings),
        }
    }
}

#[async_trait]
impl Configurable for BuiltinCommandSource {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        Vec::new() // 无用户可配置项
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let _parsed: BuiltinCommandSourceSettings =
            serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = _parsed;
        Ok(())
    }
}

#[async_trait]
impl DataSource for BuiltinCommandSource {
    async fn fetch_candidates(&self) -> CachedCandidateData {
        let mut result = CachedCandidateData::new();

        for cmd in BUILTIN_COMMANDS {
            let candidate = SearchCandidate {
                id: 0,
                name: cmd.name.to_string(),
                icon: IconRequest::Path(String::new()),
                target: ExecutionTarget::BuiltinCommand(cmd.command.to_string()),
                keywords: vec![],
                bias: 0.0, // 较高的固定权重，确保内置命令容易搜到
                trigger_keywords: cmd
                    .trigger_keywords
                    .iter()
                    .map(|s| s.to_lowercase())
                    .collect(),
            };

            tracing::debug!(
                "BuiltinCommandSource: 加载内置命令候选项: {} -> {}",
                cmd.name,
                cmd.command
            );
            result.add_candidate(candidate);
        }

        result
    }
}

use crate::plugin_framework::builtin_registry::{DataSourceEntry, InventoryContext};

pub(crate) fn build_builtin_command_source(
    ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn DataSource>) {
    let handle = ctx.get_handle("builtin-command-source");
    let source: Arc<dyn DataSource> = Arc::new(BuiltinCommandSource::new(handle));
    let configurable: Arc<dyn Configurable> = source.clone();
    (configurable, source)
}

::inventory::submit! {
    DataSourceEntry {
        component_id: "builtin-command-source",
        handle_key: "builtin-command-source",
        priority: 10,
        factory: build_builtin_command_source,
    }
}
