use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zerolaunch_plugin_api::config::SettingDefinition;
use zerolaunch_plugin_api::config::{ComponentCore, ComponentType, ConfigError, Configurable};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::keyword_trigger_match;
use zerolaunch_plugin_api::services::shell::CommandRunMode;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    ListItem, Plugin, PluginContext, PluginError, PluginKind, PluginMetadata, PluginMode, Query,
    QueryResponse, ResultAction, TargetType,
};

use crate::core::config::setting_builders::SchemaBuilder;
use crate::plugin_framework::builtin_registry::PluginEntry;

/// 触发词：声明（metadata.trigger_keywords）与命令剥离（execute_action）共用同一常量，
/// 避免两处字面量漂移。
const TRIGGER: &str = ">";

/// 确认动作标识：结果行的默认动作，Enter 触发。
const ACTION_RUN: &str = "run";

/// cmd 运行插件的配置结构。
///
/// 数据来源：`ConfigManager` 中组件 `cmd-run` 的 settings 段（JSON 键名 snake_case）。
/// 仅由 `apply_settings` 写入，`query` 与 `execute_action` 读取。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CmdRunSettings {
    /// 命令执行结束后是否关闭控制台窗口：true 关闭（cmd /C），false 保留窗口（cmd /K，默认）。
    #[serde(rename = "close_after_run", default)]
    pub close_after_run: bool,
}

/// cmd 运行插件 —— 输入 `触发词 + 空格 + 命令行` 时接管查询，确认后在可见 cmd 窗口中执行该命令。
///
/// 使用范围：内置行内插件（经 `PluginEntry` 由 builtin_registry 注册），查询接管走框架默认的
/// 关键词判定（不覆盖 `match_query`）；执行经 `PluginHandle::shell_execute_command`
/// 以可见控制台方式提交，不直接调用平台 API。返回 `QueryResponse::List` 单行结果（标准结果行渲染，无自定义面板）。
pub struct CmdRunPlugin {
    /// 组件元数据（id / 名称 / 描述 / 类型 / 注册优先级）。
    core: ComponentCore,
    /// 插件级元数据（触发词、模式、路由优先级等），注册时由 builtin_registry 读取。
    metadata: PluginMetadata,
    /// 插件配置（内部可变性：`apply_settings` 写入、`query`/`execute_action` 读取）。
    settings: RwLock<CmdRunSettings>,
    /// 服务句柄（`init` 时发放）；`None` 表示尚未初始化，此时不展示也不执行。
    handle: RwLock<Option<Arc<PluginHandle>>>,
}

impl Default for CmdRunPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CmdRunPlugin {
    /// 创建插件实例（元数据名称/描述使用 i18n key，前端按 key-or-literal 渲染）。
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "cmd-run".to_string(),
                t_key!("cmd-run", "name").to_string(),
                t_key!("cmd-run", "description").to_string(),
                ComponentType::Plugin,
                30,
            ),
            metadata: PluginMetadata {
                id: "cmd-run".to_string(),
                name: t_key!("cmd-run", "name").to_string(),
                version: String::new(),
                description: t_key!("cmd-run", "description").to_string(),
                author: String::new(),
                trigger_keywords: vec![TRIGGER.to_string()],
                trigger_description: None,
                supported_os: vec!["windows".to_string()],
                priority: 50,
                kind: PluginKind::Builtin,
                hotkey: None,
                icon: None,
                mode: PluginMode::Inline,
            },
            settings: RwLock::new(CmdRunSettings::default()),
            handle: RwLock::new(None),
        }
    }
}

#[async_trait]
impl Configurable for CmdRunPlugin {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![SchemaBuilder::boolean(
            "close_after_run",
            t_key!("cmd-run", "fields.close_after_run.label"),
            t_key!("cmd-run", "fields.close_after_run.desc"),
        )
        .order(0)
        .default(false)
        .build()]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(&*self.settings.read()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: CmdRunSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
        Ok(())
    }

    /// 默认启用：该插件是开箱即用的行内命令入口。
    fn default_enabled(&self) -> bool {
        true
    }
}

#[async_trait]
impl Plugin for CmdRunPlugin {
    /// 保存服务句柄（`execute_action` 与图标解析经句柄访问宿主能力）。
    async fn init(
        &self,
        _ctx: &PluginContext,
        handle: Option<Arc<PluginHandle>>,
    ) -> Result<(), PluginError> {
        *self.handle.write() = handle;
        Ok(())
    }

    /// 渲染单行结果：标题为命令原文，副标题说明执行方式与窗口结束行为。
    ///
    /// 命令取 `search_term`——路由已按同一份关键词判定（`keyword_trigger_match`）剥离触发词，
    /// 故展示与执行共用同一条派生规则。剩余为空时返回空结果：命中触发词即视为已处理，
    /// 不回落默认搜索。
    async fn query(
        &self,
        _ctx: &PluginContext,
        query: &Query,
    ) -> Result<QueryResponse, PluginError> {
        let command = query.search_term.trim().to_string();
        if command.is_empty() {
            return Ok(QueryResponse::Empty);
        }
        // 展示内容即确认时实际执行的内容：结束策略与图标在查询时点读一次
        let subtitle = if self.settings.read().close_after_run {
            t_key!("cmd-run", "row.subtitleCloseAfterRun")
        } else {
            t_key!("cmd-run", "row.subtitleKeepOpen")
        };
        // 图标用内置终端图标（与命令数据源一致），资源缺失时回退默认图标
        let icon_path = self
            .handle
            .read()
            .clone()
            .and_then(|handle| handle.get_app_icon_path("terminal"))
            .unwrap_or_default();

        Ok(QueryResponse::List {
            results: vec![ListItem {
                // 插件自有会话的展示行不指向宿主候选缓存，候选 id 使用虚拟值 0
                id: 0,
                title: command,
                subtitle: subtitle.to_string(),
                icon: IconRequest::Path(icon_path),
                score: 100.0,
                actions: vec![ResultAction {
                    id: ACTION_RUN.to_string(),
                    label: t_key!("cmd-run", "row.actionRun").to_string(),
                    icon: IconRequest::Path(String::new()),
                    is_default: true,
                    shortcut_key: "Enter".to_string(),
                }],
                // 展示元数据：确认由插件会话归属路由到本插件的 execute_action，不经执行器
                target_type: TargetType::Command.as_str().to_string(),
                user_arg_count: 0,
                has_system_params: false,
                trigger_keywords: vec![TRIGGER.to_string()],
            }],
        })
    }

    /// 执行结果行的默认动作：在可见 cmd 窗口中运行命令。
    ///
    /// 命令从载荷 `query_text`（用户原始输入）经同一份关键词判定剥离触发词得到，
    /// 与 `query` 展示所用的规则同源；窗口结束行为由配置决定。
    async fn execute_action(
        &self,
        _ctx: &PluginContext,
        action_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), PluginError> {
        if action_id != ACTION_RUN {
            return Err(PluginError::ActionFailed(format!(
                "Unknown action: {}",
                action_id
            )));
        }

        let query_text = payload
            .get("query_text")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                PluginError::ActionFailed("缺少查询文本，无法确定要执行的命令".to_string())
            })?;
        // 一次取齐执行所需的两个状态，避免分离加锁读到不同版本
        let (handle, close_after_run) = {
            let handle = self.handle.read().clone();
            let close_after_run = self.settings.read().close_after_run;
            (handle, close_after_run)
        };
        let handle =
            handle.ok_or_else(|| PluginError::ActionFailed("插件服务句柄不可用".to_string()))?;

        let command = keyword_trigger_match(&[TRIGGER.to_string()], query_text)
            .map(|rest| rest.trim().to_string())
            .filter(|command| !command.is_empty())
            .ok_or_else(|| PluginError::ActionFailed("命令为空".to_string()))?;

        handle
            .shell_execute_command(
                &command,
                CommandRunMode::Console {
                    keep_open: !close_after_run,
                },
            )
            .await
            .map_err(|e| PluginError::ActionFailed(format!("命令执行失败: {}", e)))
    }
}

fn build_cmd_run_plugin() -> (Arc<dyn Configurable>, Arc<dyn Plugin>, PluginMetadata) {
    let plugin = Arc::new(CmdRunPlugin::new());
    let metadata = plugin.metadata.clone();
    let configurable: Arc<dyn Configurable> = plugin.clone();
    let plugin: Arc<dyn Plugin> = plugin;
    (configurable, plugin, metadata)
}

::inventory::submit! {
    PluginEntry {
        component_id: "cmd-run",
        priority: 30,
        factory: build_cmd_run_plugin
    }
}
