//! gpui-shell 插件 UI 宿主：加载插件脚本、注册 HostModule，并维持脚本可见的会话/设置快照。

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gpui_kit::{App, Entity, Window};
use gpui_shell::{HostError, HostModule, HostValue, ScriptView, ShellRuntime};
use serde_json::Value;
use zerolaunch_plugin_api::plugin::QueryResponse;
use zerolaunch_plugin_api::Query;
use zerolaunch_rs_lib::core::host::Host as _;
use zerolaunch_rs_lib::runtime;
use zerolaunch_rs_lib::state::app_state::AppState;
use zerolaunch_rs_lib::utils::trace_id::generate_trace_id;

use crate::host::GpuiHost;

/// 脚本宿主注册的模块名。
pub const HOST_MODULE: &str = "zerolaunch";

/// 插件脚本要加载的界面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// 搜索窗插件面板。
    Panel,
    /// 设置窗插件设置页。
    Settings,
}

impl Surface {
    fn entry(self, ui: &zerolaunch_plugin_protocol::manifest::UiSection) -> Option<&str> {
        match self {
            Self::Panel => ui.panel_entry.as_deref(),
            Self::Settings => ui.settings_entry.as_deref(),
        }
    }
}

/// 脚本与宿主共享的会话快照。
#[derive(Default)]
pub struct PluginBridge {
    session: Mutex<Option<Value>>,
    session_plugin: Mutex<Option<String>>,
    settings_plugin: Mutex<Option<String>>,
}

impl PluginBridge {
    /// 更新当前面板会话；`None` 清空。
    pub fn set_session(&self, plugin_id: Option<&str>, session: Option<Value>) {
        *self
            .session_plugin
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = plugin_id.map(str::to_string);
        *self.session.lock().unwrap_or_else(|e| e.into_inner()) = session;
    }

    /// 当前面板会话所属插件。
    fn session_plugin(&self) -> Option<String> {
        self.session_plugin
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 当前会话 JSON。
    fn session(&self) -> Value {
        self.session
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .unwrap_or(Value::Null)
    }

    /// 设置当前设置页对应的插件。
    pub fn set_settings_plugin(&self, plugin_id: Option<&str>) {
        *self
            .settings_plugin
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = plugin_id.map(str::to_string);
    }

    /// 当前设置页对应的插件。
    fn settings_plugin(&self) -> Option<String> {
        self.settings_plugin
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

/// 已加载的插件脚本视图。
pub struct PluginView {
    /// 插件 id。
    pub plugin_id: String,
    /// 脚本视图实体。
    pub view: Entity<ScriptView>,
}

impl PluginView {
    /// 让脚本重新读取宿主快照。
    pub fn refresh(&self, cx: &mut App) {
        self.view.update(cx, |view, cx| view.refresh(cx));
    }
}

/// 插件 UI 宿主：一个隔离运行时承载全部插件脚本。
pub struct PluginUiHost {
    state: Arc<AppState>,
    bridge: Arc<PluginBridge>,
    runtime: Rc<ShellRuntime>,
}

impl PluginUiHost {
    /// 初始化 shell 运行时并注册 HostModule。
    /// 参数：state - 内核状态；host - GPUI 宿主；cx - GPUI App。
    /// 返回：UI 宿主；运行时或模块注册失败时返回错误。
    pub fn new(state: Arc<AppState>, host: Arc<GpuiHost>, cx: &mut App) -> Result<Self, String> {
        let runtime = ShellRuntime::new_isolated().map_err(|error| error.to_string())?;
        gpui_shell::init(cx);
        let bridge = Arc::new(PluginBridge::default());
        let module = build_module(state.clone(), host, bridge.clone());
        gpui_shell::export_module(module).map_err(|error| error.to_string())?;
        Ok(Self {
            state,
            bridge,
            runtime,
        })
    }

    /// 当前会话桥。
    pub fn bridge(&self) -> Arc<PluginBridge> {
        self.bridge.clone()
    }

    /// 加载插件某个界面的脚本视图。
    /// 参数：plugin_id - 插件 id；surface - 面板/设置页；window/cx - GPUI 上下文。
    /// 返回：脚本实体；入口缺失、越界或脚本加载失败时返回可展示错误。
    pub fn load_view(
        &self,
        plugin_id: &str,
        surface: Surface,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<ScriptView>, String> {
        let (root, entry) = self.resolve_entry(plugin_id, surface)?;
        let application = self
            .runtime
            .load_application(&root, &entry)
            .map_err(|error| format!("加载插件脚本失败: {error}"))?;
        self.runtime
            .mount_application(&application, window, cx)
            .map_err(|error| format!("挂载插件脚本失败: {error}"))
    }

    /// 解析并校验插件脚本入口，防止绝对路径或目录逃逸。
    fn resolve_entry(
        &self,
        plugin_id: &str,
        surface: Surface,
    ) -> Result<(PathBuf, String), String> {
        let manager = self.state.get_plugin_manager();
        let manifest = manager
            .get_manifest(plugin_id)
            .ok_or_else(|| format!("插件 {plugin_id} 未安装或清单不可读"))?;
        let ui = manifest
            .ui
            .as_ref()
            .ok_or_else(|| "插件未声明 [ui] 入口".to_string())?;
        let entry = surface
            .entry(ui)
            .filter(|entry| !entry.trim().is_empty())
            .ok_or_else(|| "插件未声明对应界面入口".to_string())?;
        let entry_path = Path::new(entry);
        if entry_path.is_absolute() || entry_path.components().any(|part| part.as_os_str() == "..")
        {
            return Err("插件入口必须是插件目录内的相对路径".to_string());
        }
        let root = manager
            .plugin_dir(plugin_id)
            .ok_or_else(|| format!("插件 {plugin_id} 安装目录不存在"))?;
        let root = root
            .canonicalize()
            .map_err(|error| format!("解析插件目录失败: {error}"))?;
        let asset = root
            .join(entry_path)
            .canonicalize()
            .map_err(|error| format!("插件入口不存在: {error}"))?;
        if !asset.starts_with(&root) {
            return Err("插件入口超出插件目录".to_string());
        }
        let relative = asset
            .strip_prefix(&root)
            .map_err(|_| "插件入口超出插件目录".to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        Ok((root, relative))
    }
}

/// 构造 `zerolaunch` HostModule。
fn build_module(
    state: Arc<AppState>,
    host: Arc<GpuiHost>,
    bridge: Arc<PluginBridge>,
) -> HostModule {
    let module = HostModule::new(HOST_MODULE);
    let bridge_session = bridge.clone();
    let host_confirm = host.clone();
    let bridge_confirm_state = bridge.clone();
    let state_query = state.clone();
    let host_hide = host.clone();
    let bridge_settings_plugin = bridge.clone();
    let state_settings = state.clone();
    let bridge_settings = bridge.clone();
    let bridge_set = bridge.clone();
    let bridge_action = bridge.clone();
    let state_set = state.clone();
    let state_action = state.clone();
    let state_translate = state.clone();
    let state_resource = state.clone();
    let bridge_resource = bridge.clone();
    let bridge_translate = bridge.clone();
    let state_confirm = state.clone();
    module
        .function("session", move |_| {
            Ok(json_to_host(&bridge_session.session()))
        })
        .function("confirmAction", move |arguments| {
            let plugin_id = arguments.string(0)?;
            let action_id = arguments.string(1)?;
            if bridge_confirm_state.session_plugin().as_deref() != Some(plugin_id) {
                return Err(HostError::new("当前会话不属于该插件"));
            }
            if state_confirm
                .get_plugin_manager()
                .get_manifest(plugin_id)
                .is_none()
            {
                return Err(HostError::new("插件未安装"));
            }
            host_confirm.plugin_action(plugin_id, action_id);
            Ok(HostValue::Null)
        })
        .async_function("query", move |arguments| {
            let plugin_id = arguments.string(0)?.to_string();
            let text = arguments.string(1)?.to_string();
            if state_query
                .get_plugin_manager()
                .get_manifest(&plugin_id)
                .is_none()
            {
                return Err(HostError::new("插件未安装"));
            }
            let dispatcher = state_query.get_session_dispatcher().clone();
            let (sender, receiver) = tokio::sync::oneshot::channel();
            runtime::spawn(async move {
                let trace_id = generate_trace_id();
                let query = Query {
                    id: trace_id.clone(),
                    raw_query: text.clone(),
                    search_term: text.to_lowercase(),
                    confirm: false,
                };
                let result = dispatcher
                    .route_query_panel(&trace_id, &query, &plugin_id)
                    .await
                    .map(|routed| routed.response)
                    .map_err(|error| error.to_string());
                let _ = sender.send(result);
            });
            Ok(Box::pin(async move {
                match receiver.await {
                    Ok(Ok(response)) => Ok(query_response_to_host(response)),
                    Ok(Err(error)) => Err(HostError::new(error)),
                    Err(_) => Err(HostError::new("查询任务已终止")),
                }
            }))
        })
        .function("hide", move |_| {
            host_hide.hide_main_window();
            Ok(HostValue::Null)
        })
        .function("settingsPlugin", move |_| {
            Ok(bridge_settings_plugin
                .settings_plugin()
                .map(HostValue::from)
                .unwrap_or(HostValue::Null))
        })
        .function("settings", move |_| {
            let Some(plugin_id) = bridge_settings.settings_plugin() else {
                return Ok(HostValue::Array(Vec::new()));
            };
            Ok(json_to_host(&settings_snapshot(
                &state_settings,
                &plugin_id,
            )))
        })
        .async_function("settingsSet", move |arguments| {
            let component_id = arguments.string(0)?.to_string();
            let settings = arguments.value(1)?.clone();
            let settings = host_to_json(&settings)?;
            let plugin_id = bridge_set
                .settings_plugin()
                .ok_or_else(|| HostError::new("当前没有打开插件设置页"))?;
            if !plugin_component_ids(&state_set, &plugin_id).contains(&component_id) {
                return Err(HostError::new("该组件不属于当前插件"));
            }
            let config = state_set.get_config_manager();
            let (sender, receiver) = tokio::sync::oneshot::channel();
            runtime::spawn(async move {
                let result = config
                    .apply_settings(&component_id, settings)
                    .await
                    .map_err(|error| error.to_string());
                let _ = sender.send(result);
            });
            Ok(Box::pin(async move {
                match receiver.await {
                    Ok(Ok(())) => Ok(HostValue::Null),
                    Ok(Err(error)) => Err(HostError::new(error)),
                    Err(_) => Err(HostError::new("设置保存任务已终止")),
                }
            }))
        })
        .async_function("settingsAction", move |arguments| {
            let component_id = arguments.string(0)?.to_string();
            let action_id = arguments.string(1)?.to_string();
            let params = match arguments.get(2) {
                Some(value) => host_to_json(value)?,
                None => Value::Null,
            };
            let plugin_id = bridge_action
                .settings_plugin()
                .ok_or_else(|| HostError::new("当前没有打开插件设置页"))?;
            if !plugin_component_ids(&state_action, &plugin_id).contains(&component_id) {
                return Err(HostError::new("该组件不属于当前插件"));
            }
            let config = state_action.get_config_manager();
            let (sender, receiver) = tokio::sync::oneshot::channel();
            runtime::spawn(async move {
                let result = config
                    .execute_config_action(&component_id, &action_id, &params)
                    .await
                    .map_err(|error| error.to_string());
                let _ = sender.send(result);
            });
            Ok(Box::pin(async move {
                match receiver.await {
                    Ok(Ok(value)) => Ok(json_to_host(&value)),
                    Ok(Err(error)) => Err(HostError::new(error)),
                    Err(_) => Err(HostError::new("配置动作任务已终止")),
                }
            }))
        })
        .function("translate", move |arguments| {
            let plugin_id = arguments.string(0)?;
            let key = arguments.string(1)?;
            let allowed = bridge_translate.settings_plugin().as_deref() == Some(plugin_id)
                || bridge_translate.session_plugin().as_deref() == Some(plugin_id);
            if !allowed {
                return Err(HostError::new("文案访问不属于当前插件"));
            }
            let i18n = state_translate.get_i18n_manager();
            let full_key = format!("plugin.{plugin_id}.{key}");
            Ok(HostValue::from(i18n.t(&i18n.current_language(), &full_key)))
        })
        .function("readResource", move |arguments| {
            let plugin_id = arguments.string(0)?;
            let path = arguments.string(1)?;
            let allowed = bridge_resource.settings_plugin().as_deref() == Some(plugin_id)
                || bridge_resource.session_plugin().as_deref() == Some(plugin_id);
            if !allowed {
                return Err(HostError::new("资源访问不属于当前插件"));
            }
            let path = Path::new(path);
            if path.is_absolute() || path.components().any(|part| part.as_os_str() == "..") {
                return Err(HostError::new("资源路径必须是插件目录内的相对路径"));
            }
            let root = state_resource
                .get_plugin_manager()
                .plugin_dir(plugin_id)
                .ok_or_else(|| HostError::new("插件未安装"))?
                .canonicalize()
                .map_err(|error| HostError::new(error.to_string()))?;
            let asset = root
                .join(path)
                .canonicalize()
                .map_err(|error| HostError::new(error.to_string()))?;
            if !asset.starts_with(&root) {
                return Err(HostError::new("资源路径超出插件目录"));
            }
            let text = std::fs::read_to_string(asset)
                .map_err(|error| HostError::new(format!("资源不是可读的 UTF-8 文本: {error}")))?;
            Ok(HostValue::from(text))
        })
        .declarations(
            r#"
            export function session(): unknown;
            export function confirmAction(pluginId: string, actionId: string): null;
            export function query(pluginId: string, text: string): Promise<unknown>;
            export function hide(): null;
            export function settingsPlugin(): string | null;
            export function settings(): unknown[];
            export function settingsSet(componentId: string, settings: unknown): Promise<null>;
            export function settingsAction(componentId: string, actionId: string, params?: unknown): Promise<unknown>;
            export function translate(pluginId: string, key: string): string;
            export function readResource(pluginId: string, path: string): string;
            "#,
        )
}

/// 读取插件组件 id。
fn plugin_component_ids(state: &AppState, plugin_id: &str) -> Vec<String> {
    let manager = state.get_plugin_manager();
    let config = state.get_config_manager();
    let dispatcher = state.get_session_dispatcher();
    manager
        .list_plugins(&config, dispatcher)
        .into_iter()
        .find(|plugin| plugin.plugin_id == plugin_id)
        .map(|plugin| plugin.component_ids)
        .unwrap_or_default()
}

/// 组装设置页脚本所需的插件设置快照。
fn settings_snapshot(state: &AppState, plugin_id: &str) -> Value {
    let config = state.get_config_manager();
    let components: Vec<Value> = plugin_component_ids(state, plugin_id)
        .into_iter()
        .map(|component_id| {
            let schema = config.get_component_schema(&component_id);
            let mut object = serde_json::Map::new();
            object.insert("componentId".into(), Value::String(component_id.clone()));
            if let Some(schema) = schema {
                object.insert(
                    "componentName".into(),
                    Value::String(schema.component_name.clone()),
                );
                object.insert(
                    "componentType".into(),
                    Value::String(format!("{:?}", schema.component_type)),
                );
            }
            object.insert(
                "settings".into(),
                config
                    .get_settings(&component_id)
                    .unwrap_or(serde_json::Value::Null),
            );
            if let Ok(actions) = serde_json::to_value(config.get_config_actions(&component_id)) {
                object.insert("configActions".into(), actions);
            }
            Value::Object(object)
        })
        .collect();
    Value::Array(components)
}

/// 查询响应转 HostValue。
fn query_response_to_host(response: QueryResponse) -> HostValue {
    match serde_json::to_value(response) {
        Ok(value) => json_to_host(&value),
        Err(error) => HostValue::Object(vec![("error".into(), HostValue::from(error.to_string()))]),
    }
}

/// serde_json 值转 HostValue。
fn json_to_host(value: &Value) -> HostValue {
    match value {
        Value::Null => HostValue::Null,
        Value::Bool(value) => HostValue::Bool(*value),
        Value::Number(value) => value
            .as_f64()
            .map(HostValue::Number)
            .unwrap_or(HostValue::Null),
        Value::String(value) => HostValue::Str(value.clone()),
        Value::Array(values) => HostValue::Array(values.iter().map(json_to_host).collect()),
        Value::Object(fields) => HostValue::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), json_to_host(value)))
                .collect(),
        ),
    }
}

/// HostValue 转 serde_json 值。
fn host_to_json(value: &HostValue) -> Result<Value, HostError> {
    Ok(match value {
        HostValue::Null => Value::Null,
        HostValue::Bool(value) => Value::Bool(*value),
        HostValue::Number(value) => {
            if !value.is_finite() {
                return Err(HostError::new("数值必须有限"));
            }
            serde_json::Number::from_f64(*value)
                .map(Value::Number)
                .ok_or_else(|| HostError::new("数值无法序列化"))?
        }
        HostValue::Str(value) => Value::String(value.clone()),
        HostValue::Array(values) => Value::Array(
            values
                .iter()
                .map(host_to_json)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        HostValue::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| Ok((key.clone(), host_to_json(value)?)))
                .collect::<Result<serde_json::Map<_, _>, HostError>>()?,
        ),
    })
}

/// 把 JSON 快照包装成脚本可见会话。
pub fn session_snapshot(plugin_id: &str, mode: &str, query: &str, panel: &Value) -> Value {
    let mut object = serde_json::Map::new();
    object.insert("pluginId".into(), Value::String(plugin_id.to_string()));
    object.insert("mode".into(), Value::String(mode.to_string()));
    object.insert("query".into(), Value::String(query.to_string()));
    object.insert("panel".into(), panel.clone());
    Value::Object(object)
}

#[cfg(test)]
mod tests {
    use super::ShellRuntime;

    #[test]
    fn isolated_runtime_starts() {
        assert!(ShellRuntime::new_isolated().is_ok());
    }
}
