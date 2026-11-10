//! 设置窗口：分类导航、组件设置、插件管理、市场、关于与检查器。

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures::channel::oneshot;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{AppContext as _, Context, Entity, PathPromptOptions, Subscription, Task, Window};
use serde_json::{json, Value};
use zerolaunch_plugin_api::config::{
    ComponentType, FieldUiMetadata, SchemaKind, SchemaNode, WidgetHint,
};
use zerolaunch_plugin_api::host::OpenTarget;
use zerolaunch_plugin_api::plugin::PluginKind;
use zerolaunch_plugin_host::PluginInfo;
use zerolaunch_rs_lib::core::config::models::{ComponentInfo, ComponentSchema};
use zerolaunch_rs_lib::core::host::Host as _;
use zerolaunch_rs_lib::plugin_framework::inspector::InspectorState;
use zerolaunch_rs_lib::plugin_market::{GitHubMarketClient, MarketRepo};
use zerolaunch_rs_lib::runtime;
use zerolaunch_rs_lib::state::app_state::AppState;

use crate::host::{GpuiHost, HostEvent};
use crate::settings::schema::{self, FieldPath, Segment};

/// 当前设置页。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Page {
    /// 组件设置。
    Component,
    /// 插件管理。
    Plugins,
    /// 插件市场。
    Market,
    /// 关于。
    About,
    /// 调试检查器。
    Debug,
}

/// 侧栏分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Category {
    /// 常规设置。
    Core,
    /// 外观设置。
    Appearance,
    /// 模型管理。
    Models,
    /// 搜索管道。
    Pipeline,
    /// 插件管理页。
    Plugins,
    /// 插件市场页。
    Market,
    /// 关于页。
    About,
    /// 调试页。
    Debug,
}

/// 设置项搜索结果。
#[derive(Debug, Clone)]
pub(super) struct SearchHit {
    /// 目标组件。
    pub component_id: String,
    /// 字段 key；None 表示组件本身。
    pub field: Option<String>,
    /// 结果主标题。
    pub label: String,
    /// 结果附加说明。
    pub detail: String,
}

/// 设置窗口视图。
pub struct SettingsView {
    /// 内核状态。
    state: Arc<AppState>,
    /// 宿主窗口出口。
    host: Arc<GpuiHost>,
    /// 设置项搜索输入。
    search: Entity<InputState>,
    /// 搜索词。
    search_query: String,
    /// 搜索结果。
    search_hits: Vec<SearchHit>,
    /// 全部可配置组件。
    components: Vec<ComponentInfo>,
    /// 当前分类。
    category: Category,
    /// 当前页面。
    page: Page,
    /// 当前组件。
    selected_component: Option<String>,
    /// 当前组件 schema。
    schema: Option<ComponentSchema>,
    /// 本地编辑值。
    draft: Value,
    /// 最近一次保存值。
    saved: Value,
    /// 字段 key 过滤（来自设置项搜索）。
    field_filter: Option<String>,
    /// 字段输入框缓存。
    inputs: HashMap<String, Entity<InputState>>,
    /// 字段输入框订阅。
    field_subscriptions: Vec<Subscription>,
    /// 视图级订阅与任务。
    subscriptions: Vec<Subscription>,
    /// 宿主事件任务。
    _event_task: Task<()>,
    /// 是否有未提交修改。
    dirty: bool,
    /// 重置按钮是否进入确认态。
    reset_armed: bool,
    /// 状态提示。
    status: Option<String>,
    /// 即时保存防抖序号。
    immediate_revision: u64,
    /// 插件列表。
    plugins: Vec<PluginInfo>,
    /// 插件日志。
    plugin_logs: Option<(String, String)>,
    /// 待确认卸载的插件。
    pending_uninstall: Option<String>,
    /// 市场仓库。
    market_repos: Vec<MarketRepo>,
    /// 市场加载中。
    market_loading: bool,
    /// 市场错误。
    market_error: Option<String>,
    /// 检查器快照。
    inspector: Option<InspectorState>,
}

impl SettingsView {
    /// 创建设置视图并加载第一个组件。
    /// 参数：state - 内核状态；host - 宿主；window/cx - GPUI 上下文。
    /// 返回：已订阅宿主事件与搜索输入的视图。
    pub fn new(
        state: Arc<AppState>,
        host: Arc<GpuiHost>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let placeholder = tr(&state, "settings.searchPlaceholder");
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let components = state.get_config_manager().get_all_components();
        let selected_component = default_component_id(&components);

        let event_receiver = host.subscribe();
        let event_task = cx.spawn_in(window, async move |this, cx| {
            let mut receiver = event_receiver;
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        let _ = this.update_in(cx, |this, window, cx| {
                            this.on_host_event(event, window, cx);
                        });
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!("设置窗口宿主事件积压，跳过 {skipped} 条");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        let mut view = Self {
            state,
            host,
            search,
            search_query: String::new(),
            search_hits: Vec::new(),
            components,
            category: Category::Core,
            page: Page::Component,
            selected_component: None,
            schema: None,
            draft: json!({}),
            saved: json!({}),
            field_filter: None,
            inputs: HashMap::new(),
            field_subscriptions: Vec::new(),
            subscriptions: Vec::new(),
            _event_task: event_task,
            dirty: false,
            reset_armed: false,
            status: None,
            immediate_revision: 0,
            plugins: Vec::new(),
            plugin_logs: None,
            pending_uninstall: None,
            market_repos: Vec::new(),
            market_loading: false,
            market_error: None,
            inspector: None,
        };

        let search_subscription =
            cx.subscribe_in(&view.search, window, |this, input, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.search_query = input.read(cx).value().to_string();
                    this.rebuild_search_hits(cx);
                }
            });
        view.subscriptions.push(search_subscription);

        if let Some(component_id) = selected_component {
            let category = view.category_for_component(&component_id);
            view.selected_component = Some(component_id);
            view.category = category;
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(80))
                    .await;
                let _ = this.update_in(cx, |this, window, cx| {
                    this.load_component(window, cx);
                });
            })
            .detach();
        }

        window.on_window_should_close(cx, {
            let host = view.host.clone();
            move |_window, _cx| {
                host.hide_settings_window();
                false
            }
        });

        view
    }

    /// 读取当前语言译文。
    /// 参数：key - i18n key。
    /// 返回：译文；缺失时回退原文。
    pub(super) fn tr(&self, key: &str) -> String {
        tr(&self.state, key)
    }

    /// 当前页面。
    pub(super) fn page(&self) -> Page {
        self.page
    }

    /// 当前分类。
    pub(super) fn category(&self) -> Category {
        self.category
    }

    /// 当前组件 id。
    pub(super) fn selected_component(&self) -> Option<&str> {
        self.selected_component.as_deref()
    }

    /// 当前组件 schema。
    pub(super) fn component_schema(&self) -> Option<&ComponentSchema> {
        self.schema.as_ref()
    }

    /// 当前状态提示。
    pub(super) fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    /// 是否有未保存修改。
    pub(super) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 重置按钮是否处于确认态。
    pub(super) fn reset_armed(&self) -> bool {
        self.reset_armed
    }

    /// 当前分类中的组件。
    pub(super) fn category_components(&self, category: Category) -> Vec<ComponentInfo> {
        self.components
            .iter()
            .filter(|component| self.category_for_component(&component.component_id) == category)
            .cloned()
            .collect()
    }

    /// 分类显示名。
    pub(super) fn category_label(&self, category: Category) -> String {
        self.tr(match category {
            Category::Core => "settings.sidebar.general",
            Category::Appearance => "settings.sidebar.appearance",
            Category::Models => "settings.sidebar.models",
            Category::Pipeline => "settings.sidebar.pipeline",
            Category::Plugins => "settings.sidebar.plugins",
            Category::Market => "settings.sidebar.pluginMarket",
            Category::About => "settings.sidebar.about",
            Category::Debug => "settings.sidebar.debug",
        })
    }

    /// 当前搜索输入框。
    pub(super) fn search_input(&self) -> &Entity<InputState> {
        &self.search
    }

    /// 搜索结果。
    pub(super) fn search_hits(&self) -> &[SearchHit] {
        &self.search_hits
    }

    /// 搜索词。
    pub(super) fn search_query(&self) -> &str {
        &self.search_query
    }

    /// 插件列表。
    pub(super) fn plugins(&self) -> &[PluginInfo] {
        &self.plugins
    }

    /// 插件日志。
    pub(super) fn plugin_logs(&self) -> Option<&(String, String)> {
        self.plugin_logs.as_ref()
    }

    /// 待确认卸载插件 id。
    pub(super) fn pending_uninstall(&self) -> Option<&str> {
        self.pending_uninstall.as_deref()
    }

    /// 市场仓库。
    pub(super) fn market_repos(&self) -> &[MarketRepo] {
        &self.market_repos
    }

    /// 市场是否加载中。
    pub(super) fn market_loading(&self) -> bool {
        self.market_loading
    }

    /// 市场错误。
    pub(super) fn market_error(&self) -> Option<&str> {
        self.market_error.as_deref()
    }

    /// 检查器快照。
    pub(super) fn inspector(&self) -> Option<&InspectorState> {
        self.inspector.as_ref()
    }

    /// 当前组件信息。
    pub(super) fn component_info(&self) -> Option<&ComponentInfo> {
        let id = self.selected_component.as_deref()?;
        self.components
            .iter()
            .find(|component| component.component_id == id)
    }

    /// 字段是否应显示。
    pub(super) fn field_visible(
        &self,
        ui: &zerolaunch_plugin_api::config::FieldUiMetadata,
        scope: &[Segment],
    ) -> bool {
        if let Some(filter) = &self.field_filter {
            if !scope.is_empty() {
                return true;
            }
            if ui.pointer.rsplit('/').next() != Some(filter.as_str()) {
                return false;
            }
        }
        schema::is_visible(ui, &self.draft, scope)
    }

    /// 取字段当前值。
    pub(super) fn value(&self, path: &[Segment]) -> Option<&Value> {
        schema::value_at(&self.draft, path)
    }

    /// 切换分类。
    pub(super) fn select_category(
        &mut self,
        category: Category,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if category == self.category
            && category != Category::Plugins
            && category != Category::Market
        {
            return;
        }
        if !self.confirm_navigation(cx) {
            return;
        }
        self.category = category;
        self.field_filter = None;
        match category {
            Category::Plugins => {
                self.page = Page::Plugins;
                self.reload_plugins(cx);
            }
            Category::Market => {
                self.page = Page::Market;
                self.reload_plugins(cx);
                if self.market_repos.is_empty() && !self.market_loading {
                    self.load_market(window, cx);
                }
            }
            Category::About => self.page = Page::About,
            Category::Debug => {
                self.page = Page::Debug;
                self.refresh_inspector(cx);
            }
            _ => {
                self.page = Page::Component;
                if let Some(component) = self.category_components(category).first() {
                    let id = component.component_id.clone();
                    self.selected_component = Some(id);
                    self.load_component(window, cx);
                }
            }
        }
        cx.notify();
    }

    /// 选择组件。
    pub(super) fn select_component(
        &mut self,
        component_id: &str,
        field: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_component.as_deref() == Some(component_id) && self.page == Page::Component
        {
            self.field_filter = field;
            cx.notify();
            return;
        }
        if !self.confirm_navigation(cx) {
            return;
        }
        self.selected_component = Some(component_id.to_string());
        self.category = self.category_for_component(component_id);
        self.page = Page::Component;
        self.load_component(window, cx);
        self.field_filter = field;
        cx.notify();
    }

    /// 搜索命中项跳转。
    pub(super) fn select_search_hit(
        &mut self,
        hit: &SearchHit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let hit = hit.clone();
        self.select_component(&hit.component_id, hit.field, window, cx);
    }

    /// 切换组件启用状态。
    pub(super) fn toggle_enabled(&mut self, cx: &mut Context<Self>) {
        let Some(component) = self.component_info().cloned() else {
            return;
        };
        let enabled = !component.enabled;
        let config = self.state.get_config_manager();
        if let Err(error) = config.set_enabled(&component.component_id, enabled) {
            self.status = Some(format!("{}: {error}", self.tr("settings.toggleFailed")));
        } else if let Some(item) = self
            .components
            .iter_mut()
            .find(|item| item.component_id == component.component_id)
        {
            item.enabled = enabled;
            self.status = Some(self.tr(if enabled {
                "settings.enabled"
            } else {
                "settings.disabled"
            }));
        }
        cx.notify();
    }

    /// 保存当前组件设置。
    pub(super) fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(schema) = self.schema.as_ref() else {
            return;
        };
        let component_id = schema.component_id.clone();
        let id_for_view = component_id.clone();
        let config = self.state.get_config_manager();
        let contribution = schema.contribution.clone();
        let values = schema::strip_transient(&contribution, self.draft.clone());
        let saved_values = values.clone();
        let contribution_for_done = contribution.clone();
        let (sender, receiver) = oneshot::channel();
        runtime::spawn(async move {
            let _ = sender.send(config.apply_settings(&component_id, values).await);
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver.await;
            let _ = this.update_in(cx, |this, _window, cx| {
                if this.selected_component.as_deref() != Some(id_for_view.as_str()) {
                    return;
                }
                match result {
                    Ok(Ok(())) => {
                        this.saved = saved_values;
                        this.dirty =
                            schema::strip_transient(&contribution_for_done, this.draft.clone())
                                != this.saved;
                        this.status = Some(this.tr("settings.saveSuccess"));
                    }
                    Ok(Err(error)) => {
                        this.status = Some(format!("{}: {error}", this.tr("settings.saveFailed")));
                    }
                    Err(_) => {
                        this.status = Some(this.tr("settings.saveFailed"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 重置当前组件设置；首次点击进入确认态。
    pub(super) fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.reset_armed {
            self.reset_armed = true;
            self.status = Some(self.tr("settings.resetConfirmContent"));
            cx.notify();
            return;
        }
        self.reset_armed = false;
        self.immediate_revision = self.immediate_revision.wrapping_add(1);
        let Some(schema) = self.schema.as_ref() else {
            return;
        };
        let component_id = schema.component_id.clone();
        let id_for_view = component_id.clone();
        let config = self.state.get_config_manager();
        let (sender, receiver) = oneshot::channel();
        runtime::spawn(async move {
            let _ = sender.send(config.reset_to_default(&component_id).await);
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.selected_component.as_deref() != Some(id_for_view.as_str()) {
                    return;
                }
                match result {
                    Ok(Ok(())) => {
                        this.selected_component = Some(id_for_view);
                        this.load_component(window, cx);
                        this.status = Some(this.tr("settings.resetSuccess"));
                    }
                    Ok(Err(error)) => {
                        this.status = Some(format!("{}: {error}", this.tr("settings.resetFailed")));
                    }
                    Err(_) => {
                        this.status = Some(this.tr("settings.resetFailed"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 丢弃未保存修改。
    pub(super) fn discard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.load_component(window, cx);
        self.status = Some(self.tr("settings.unsavedDiscard"));
        cx.notify();
    }

    /// 切换布尔值。
    pub(super) fn set_bool(
        &mut self,
        path: &[Segment],
        value: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_value(path, Value::Bool(value), window, cx);
    }

    /// 切换多选项。
    pub(super) fn toggle_multi(
        &mut self,
        path: &[Segment],
        option: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut values = self
            .value(path)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if let Some(index) = values.iter().position(|value| value == option) {
            values.remove(index);
        } else {
            values.push(option.to_string());
        }
        let value = Value::Array(values.into_iter().map(Value::String).collect());
        self.set_value(path, value, window, cx);
    }

    /// 字符串输入框变更。
    pub(super) fn apply_text(
        &mut self,
        path: FieldPath,
        text: String,
        kind: TextKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = match kind {
            TextKind::Text => Value::String(text),
            TextKind::Number => match text.trim().parse::<f64>() {
                Ok(number) if number.is_finite() => json!(number),
                _ => {
                    self.status = Some(self.tr("settings.invalidNumber"));
                    cx.notify();
                    return;
                }
            },
            TextKind::Integer => match text.trim().parse::<i64>() {
                Ok(number) => json!(number),
                _ => {
                    self.status = Some(self.tr("settings.invalidInteger"));
                    cx.notify();
                    return;
                }
            },
        };
        self.set_value(&path, value, window, cx);
    }

    /// 添加数组元素。
    pub(super) fn add_array_item(
        &mut self,
        path: &[Segment],
        item_schema: &SchemaNode,
        max_items: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut values = self
            .value(path)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if max_items.is_some_and(|max| values.len() >= max) {
            return;
        }
        values.push(schema::default_value(item_schema));
        self.inputs.clear();
        self.field_subscriptions.clear();
        self.set_value(path, Value::Array(values), window, cx);
        self.prepare_inputs(window, cx);
    }

    /// 删除数组元素。
    pub(super) fn remove_array_item(
        &mut self,
        path: &[Segment],
        index: usize,
        min_items: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut values = self
            .value(path)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if min_items.is_some_and(|min| values.len() <= min) || index >= values.len() {
            return;
        }
        values.remove(index);
        self.inputs.clear();
        self.field_subscriptions.clear();
        self.set_value(path, Value::Array(values), window, cx);
        self.prepare_inputs(window, cx);
    }

    /// 取已创建的字段输入框。
    /// 参数：path - 字段路径。
    /// 返回：加载阶段已创建的输入框；不存在时 None。
    pub(super) fn input(&self, path: &[Segment]) -> Option<Entity<InputState>> {
        self.inputs.get(&path_id(path)).cloned()
    }

    /// 确保字段输入框存在；只在组件加载与数组结构变化时调用。
    /// 参数：path - 字段路径；value - 初始值；kind - 文本/数字类型；window/cx - GPUI 上下文。
    /// 返回：无。
    fn ensure_input(
        &mut self,
        path: &[Segment],
        value: &str,
        kind: TextKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = path_id(path);
        if self.inputs.contains_key(&key) {
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).default_value(value));
        let path = path.to_vec();
        let subscription =
            cx.subscribe_in(&input, window, move |this, input, event, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let text = input.read(cx).value().to_string();
                    this.apply_text(path.clone(), text, kind, window, cx);
                }
            });
        self.field_subscriptions.push(subscription);
        self.inputs.insert(key, input);
    }

    /// 选择路径并写入字段。
    pub(super) fn pick_path(
        &mut self,
        path: FieldPath,
        directory: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let prompt = if directory {
            self.tr("settings.pathSelectDirectory")
        } else {
            self.tr("settings.selectPath")
        };
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: !directory,
            directories: directory,
            multiple: false,
            prompt: Some(prompt.into()),
        });
        let field_path = path;
        let input_key = path_id(&field_path);
        let component_id = self.selected_component.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver.await;
            let selected = match result {
                Ok(Ok(Some(mut paths))) => paths.pop(),
                _ => None,
            };
            if let Some(selected_path) = selected {
                let _ = this.update_in(cx, |this, window, cx| {
                    if this.selected_component != component_id {
                        return;
                    }
                    let text = selected_path.to_string_lossy().into_owned();
                    if let Some(input) = this.inputs.get(&input_key) {
                        input.update(cx, |state, cx| {
                            state.set_value(text.clone(), window, cx);
                        });
                    }
                    this.set_value(&field_path, Value::String(text), window, cx);
                });
            }
        })
        .detach();
    }

    /// 选择图片并上传为资源 id。
    pub(super) fn pick_image(
        &mut self,
        path: FieldPath,
        resource_id: String,
        max_size: Option<u64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(self.tr("settings.imageSelect").into()),
        });
        let handle = self.state.get_core_handle();
        let component_id = self.selected_component.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver.await;
            let selected = match result {
                Ok(Ok(Some(mut paths))) => paths.pop(),
                _ => None,
            };
            let Some(selected) = selected else {
                return;
            };
            let (sender, uploaded) = oneshot::channel();
            runtime::spawn(async move {
                let file = selected.to_string_lossy().to_string();
                let _ = sender.send(
                    handle
                        .resource_upload(&resource_id, &file, max_size)
                        .await
                        .map_err(|error| error.to_string()),
                );
            });
            let result = uploaded.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.selected_component != component_id {
                    return;
                }
                match result {
                    Ok(Ok(id)) => {
                        this.set_value(&path, Value::String(id), window, cx);
                        this.status = Some(this.tr("settings.imageUploadSuccess"));
                    }
                    Ok(Err(error)) => {
                        this.status = Some(format!(
                            "{}: {error}",
                            this.tr("settings.imageUploadFailed")
                        ));
                    }
                    Err(_) => this.status = Some(this.tr("settings.imageUploadFailed")),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 刷新插件列表。
    pub(super) fn reload_plugins(&mut self, cx: &mut Context<Self>) {
        let manager = self.state.get_plugin_manager();
        let config = self.state.get_config_manager();
        let dispatcher = self.state.get_session_dispatcher();
        self.plugins = manager.list_plugins(&config, dispatcher);
        self.pending_uninstall = None;
        cx.notify();
    }

    /// 安装本地插件（文件或目录）。
    pub(super) fn pick_plugin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: false,
            prompt: Some(self.tr("settings.thirdPartyPlugins.chooseFile").into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver.await;
            let selected = match result {
                Ok(Ok(Some(mut paths))) => paths.pop(),
                _ => None,
            };
            let Some(selected) = selected else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.install_plugin(selected, window, cx);
            });
        })
        .detach();
    }

    /// 执行本地插件安装。
    fn install_plugin(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let manager = self.state.get_plugin_manager();
        let host: Arc<dyn zerolaunch_rs_lib::core::host::Host> = self.host.clone();
        let (sender, receiver) = oneshot::channel();
        runtime::spawn(async move {
            let _ = sender.send(
                manager
                    .install(&path, false, host)
                    .await
                    .map_err(|error| error.to_string()),
            );
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver.await;
            let _ = this.update_in(cx, |this, _window, cx| {
                match result {
                    Ok(Ok(info)) => {
                        this.status = Some(format!(
                            "{}: {} {}",
                            this.tr("settings.thirdPartyPlugins.installSuccess"),
                            info.name,
                            info.version
                        ));
                    }
                    Ok(Err(error)) => {
                        this.status = Some(format!(
                            "{}: {error}",
                            this.tr("settings.thirdPartyPlugins.installFailed")
                        ));
                    }
                    Err(_) => {
                        this.status = Some(this.tr("settings.thirdPartyPlugins.installFailed"))
                    }
                }
                this.reload_plugins(cx);
            });
        })
        .detach();
    }

    /// 切换插件启用状态。
    pub(super) fn toggle_plugin(
        &mut self,
        plugin: PluginInfo,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let config = self.state.get_config_manager();
        let dispatcher = self.state.get_session_dispatcher().clone();
        let plugin_id = plugin.plugin_id.clone();
        let component_ids = plugin.component_ids.clone();
        let is_third_party = plugin.kind == PluginKind::ThirdParty;
        let (sender, receiver) = oneshot::channel();
        runtime::spawn(async move {
            let result = (|| {
                if is_third_party {
                    for component_id in component_ids {
                        config
                            .set_enabled(&component_id, enabled)
                            .map_err(|error| error.to_string())?;
                    }
                } else {
                    config
                        .set_enabled(&plugin_id, enabled)
                        .map_err(|error| error.to_string())?;
                }
                dispatcher.set_plugin_enabled(&plugin_id, enabled);
                Ok::<(), String>(())
            })();
            if result.is_ok() {
                dispatcher.refresh_candidates().await;
            }
            let _ = sender.send(result);
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver.await;
            let _ = this.update_in(cx, |this, _window, cx| {
                match result {
                    Ok(Ok(())) => {
                        this.status = Some(this.tr(if enabled {
                            "settings.thirdPartyPlugins.toggleEnabledSuccess"
                        } else {
                            "settings.thirdPartyPlugins.toggleDisabledSuccess"
                        }));
                    }
                    Ok(Err(error)) => {
                        this.status = Some(format!(
                            "{}: {error}",
                            this.tr("settings.thirdPartyPlugins.toggleFailed")
                        ));
                    }
                    Err(_) => {
                        this.status = Some(this.tr("settings.thirdPartyPlugins.toggleFailed"))
                    }
                }
                this.reload_plugins(cx);
            });
        })
        .detach();
    }

    /// 重载第三方插件。
    pub(super) fn reload_plugin(
        &mut self,
        plugin_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let manager = self.state.get_plugin_manager();
        let host: Arc<dyn zerolaunch_rs_lib::core::host::Host> = self.host.clone();
        let (sender, receiver) = oneshot::channel();
        runtime::spawn(async move {
            let _ = sender.send(
                manager
                    .reload(&plugin_id, host)
                    .await
                    .map_err(|error| error.to_string()),
            );
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver.await;
            let _ = this.update_in(cx, |this, _window, cx| {
                match result {
                    Ok(Ok(())) => {
                        this.status = Some(this.tr("settings.thirdPartyPlugins.reloadSuccess"))
                    }
                    Ok(Err(error)) => {
                        this.status = Some(format!(
                            "{}: {error}",
                            this.tr("settings.thirdPartyPlugins.reloadFailed")
                        ));
                    }
                    Err(_) => {
                        this.status = Some(this.tr("settings.thirdPartyPlugins.reloadFailed"))
                    }
                }
                this.reload_plugins(cx);
            });
        })
        .detach();
    }

    /// 卸载插件；首次点击进入确认态。
    pub(super) fn uninstall_plugin(
        &mut self,
        plugin_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_uninstall.as_deref() != Some(plugin_id.as_str()) {
            self.pending_uninstall = Some(plugin_id);
            self.status = Some(self.tr("settings.thirdPartyPlugins.uninstallConfirmTitle"));
            cx.notify();
            return;
        }
        self.pending_uninstall = None;
        let manager = self.state.get_plugin_manager();
        let host: Arc<dyn zerolaunch_rs_lib::core::host::Host> = self.host.clone();
        let (sender, receiver) = oneshot::channel();
        runtime::spawn(async move {
            let _ = sender.send(
                manager
                    .uninstall(&plugin_id, host)
                    .await
                    .map_err(|error| error.to_string()),
            );
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver.await;
            let _ = this.update_in(cx, |this, _window, cx| {
                match result {
                    Ok(Ok(())) => {
                        this.status = Some(this.tr("settings.thirdPartyPlugins.uninstallSuccess"))
                    }
                    Ok(Err(error)) => {
                        this.status = Some(format!(
                            "{}: {error}",
                            this.tr("settings.thirdPartyPlugins.uninstallFailed")
                        ));
                    }
                    Err(_) => {
                        this.status = Some(this.tr("settings.thirdPartyPlugins.uninstallFailed"))
                    }
                }
                this.reload_plugins(cx);
            });
        })
        .detach();
    }

    /// 读取插件日志。
    pub(super) fn show_plugin_logs(&mut self, plugin_id: &str, cx: &mut Context<Self>) {
        let manager = self.state.get_plugin_manager();
        let text = match manager.get_logs(plugin_id, 80) {
            Ok(lines) => lines.join("\n"),
            Err(error) => error.to_string(),
        };
        self.plugin_logs = Some((plugin_id.to_string(), text));
        cx.notify();
    }

    /// 关闭日志面板。
    pub(super) fn close_plugin_logs(&mut self, cx: &mut Context<Self>) {
        self.plugin_logs = None;
        cx.notify();
    }

    /// 加载插件市场列表。
    pub(super) fn load_market(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.market_loading = true;
        self.market_error = None;
        cx.notify();
        let (sender, receiver) = oneshot::channel();
        runtime::spawn(async move {
            let _ = sender.send(GitHubMarketClient::new().list_repos().await);
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver.await;
            let _ = this.update_in(cx, |this, _window, cx| {
                match result {
                    Ok(Ok(repos)) => {
                        this.market_repos = repos;
                        this.market_loading = false;
                        this.market_error = None;
                    }
                    Ok(Err(error)) => {
                        this.market_loading = false;
                        this.market_error = Some(error.to_string());
                    }
                    Err(_) => {
                        this.market_loading = false;
                        this.market_error = Some(
                            this.tr("settings.pluginMarket.loadFailed")
                                .replace("{reason}", &this.tr("settings.cancel")),
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 安装市场仓库最新发布。
    pub(super) fn install_market(
        &mut self,
        repo: MarketRepo,
        overwrite: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let manager = self.state.get_plugin_manager();
        let handle = self.state.get_core_handle();
        let host: Arc<dyn zerolaunch_rs_lib::core::host::Host> = self.host.clone();
        let (sender, receiver) = oneshot::channel();
        runtime::spawn(async move {
            let result = async {
                let client = GitHubMarketClient::new();
                let release = client
                    .get_release(&repo.full_name)
                    .await
                    .map_err(|error| error.to_string())?;
                let cache_dir = handle
                    .resolve_path(zerolaunch_plugin_api::services::path::KnownPath::AppCacheDir)
                    .map_err(|error| error.to_string())?;
                let dest = Path::new(&cache_dir)
                    .join(handle.plugin_id())
                    .join("market")
                    .join(&release.asset_name);
                if !dest.exists() {
                    let bytes = client
                        .download(&release.download_url)
                        .await
                        .map_err(|error| error.to_string())?;
                    handle
                        .cache_put("market", &release.asset_name, &bytes)
                        .await
                        .map_err(|error| error.to_string())?;
                }
                let installed = manager
                    .install(&dest, overwrite, host)
                    .await
                    .map_err(|error| error.to_string());
                let _ = handle.cache_delete("market", &release.asset_name).await;
                installed
            }
            .await;
            let _ = sender.send(result);
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver.await;
            let _ = this.update_in(cx, |this, _window, cx| {
                match result {
                    Ok(Ok(info)) => {
                        this.status = Some(format!(
                            "{}: {} {}",
                            this.tr("settings.pluginMarket.installSuccess"),
                            info.name,
                            info.version
                        ));
                    }
                    Ok(Err(error)) => {
                        this.status = Some(format!(
                            "{}: {error}",
                            this.tr("settings.pluginMarket.installFailed")
                        ));
                    }
                    Err(_) => this.status = Some(this.tr("settings.pluginMarket.installFailed")),
                }
                this.reload_plugins(cx);
            });
        })
        .detach();
    }

    /// 刷新检查器快照。
    pub(super) fn refresh_inspector(&mut self, cx: &mut Context<Self>) {
        if !self.state.is_debug_mode() {
            self.inspector = None;
            cx.notify();
            return;
        }
        let Some(inspector) = self.state.get_inspector() else {
            self.inspector = None;
            cx.notify();
            return;
        };
        let config = self.state.get_config_manager();
        self.inspector = Some(inspector.snapshot(&config));
        cx.notify();
    }

    /// 用系统默认程序打开链接。
    pub(super) fn open_url(&self, url: &str) {
        let handle = self.state.get_core_handle();
        let url = url.to_string();
        runtime::spawn(async move {
            if let Err(error) = handle.shell_open(OpenTarget::Url(url)).await {
                tracing::warn!("打开链接失败: {error}");
            }
        });
    }

    /// 应用版本。
    pub(super) fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// 判断调试页面是否显示。
    pub(super) fn debug_enabled(&self) -> bool {
        self.state.is_debug_mode()
    }

    /// 按 schema 预先创建所有输入框，避免渲染期间创建实体。
    /// 参数：window/cx - GPUI 上下文。
    /// 返回：无。
    fn prepare_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(schema) = self.schema.clone() else {
            return;
        };
        self.prepare_object_inputs(
            &schema.contribution.properties,
            &schema.contribution.ui,
            &[],
            window,
            cx,
        );
    }

    /// 为一个对象作用域预先创建输入框。
    /// 参数：properties/ui - 对象 schema；scope - 对象路径；window/cx - GPUI 上下文。
    /// 返回：无。
    fn prepare_object_inputs(
        &mut self,
        properties: &BTreeMap<String, SchemaNode>,
        ui: &[FieldUiMetadata],
        scope: &[Segment],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let fields = schema::object_fields(properties, ui)
            .into_iter()
            .map(|field| {
                (
                    field.key.to_string(),
                    field.schema.clone(),
                    field.ui.clone(),
                )
            })
            .collect::<Vec<_>>();
        for (key, node, meta) in fields {
            let mut path = scope.to_vec();
            path.push(Segment::Key(key));
            let widget = schema::widget_of(&node, &meta);
            match &widget {
                WidgetHint::Toggle
                | WidgetHint::Select
                | WidgetHint::MultiSelect
                | WidgetHint::Image { .. } => {}
                WidgetHint::List
                | WidgetHint::Tags
                | WidgetHint::Table
                | WidgetHint::Cards
                | WidgetHint::MasterDetail
                | WidgetHint::SearchTable => {
                    let SchemaKind::Array { items, .. } = &node.kind else {
                        continue;
                    };
                    let values = self
                        .value(&path)
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    for (index, item) in values.iter().enumerate() {
                        let mut item_path = path.clone();
                        item_path.push(Segment::Index(index));
                        match &items.kind {
                            SchemaKind::Object { properties, ui, .. } => {
                                self.prepare_object_inputs(properties, ui, &item_path, window, cx)
                            }
                            _ => self.ensure_input(
                                &item_path,
                                &value_text(item),
                                text_kind(items),
                                window,
                                cx,
                            ),
                        }
                    }
                }
                _ => {
                    let current = self.value(&path).cloned().unwrap_or(Value::Null);
                    self.ensure_input(&path, &value_text(&current), text_kind(&node), window, cx);
                }
            }
        }
    }

    /// 选择组件后加载 schema 与设置。
    fn load_component(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.immediate_revision = self.immediate_revision.wrapping_add(1);
        let Some(component_id) = self.selected_component.clone() else {
            return;
        };
        let config = self.state.get_config_manager();
        self.schema = config.get_component_schema(&component_id);
        self.draft = config
            .get_settings(&component_id)
            .unwrap_or_else(|| json!({}));
        self.saved = self.draft.clone();
        self.inputs.clear();
        self.field_subscriptions.clear();
        self.dirty = false;
        self.reset_armed = false;
        self.status = None;
        self.field_filter = None;
        self.prepare_inputs(window, cx);
        cx.notify();
    }

    /// 写入字段值并处理脏状态与即时保存。
    pub(super) fn set_value(
        &mut self,
        path: &[Segment],
        value: Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        schema::set_value(&mut self.draft, path, value);
        self.dirty = self.draft != self.saved;
        self.reset_armed = false;
        self.schedule_immediate_save(window, cx);
        cx.notify();
    }

    /// 即时保存策略下，防抖提交当前 draft。
    fn schedule_immediate_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(schema) = self.schema.as_ref() else {
            return;
        };
        if !schema::is_immediate(&schema.contribution) {
            return;
        }
        self.immediate_revision = self.immediate_revision.wrapping_add(1);
        let revision = self.immediate_revision;
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(180))
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.immediate_revision == revision {
                    this.save(window, cx);
                }
            });
        })
        .detach();
    }

    /// 导航前检查未保存修改。
    fn confirm_navigation(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.dirty {
            return true;
        }
        self.status = Some(self.tr("settings.unsavedContent"));
        cx.notify();
        false
    }

    /// 按组件 id 归类。
    fn category_for_component(&self, component_id: &str) -> Category {
        if matches!(component_id, "appearance-config" | "icon-override-config") {
            return Category::Appearance;
        }
        if matches!(component_id, "model-openai-config" | "model-ollama-config") {
            return Category::Models;
        }
        let Some(component) = self
            .components
            .iter()
            .find(|component| component.component_id == component_id)
        else {
            return Category::Core;
        };
        match component.component_type {
            ComponentType::DataSource
            | ComponentType::KeywordOptimizer
            | ComponentType::KeywordInjector
            | ComponentType::SearchEngine
            | ComponentType::ScoreBooster
            | ComponentType::ActionExecutor
            | ComponentType::BiasRule
            | ComponentType::ExclusionRule => Category::Pipeline,
            ComponentType::Core => Category::Core,
            ComponentType::Plugin => Category::Plugins,
        }
    }

    /// 重建设置项搜索结果。
    fn rebuild_search_hits(&mut self, cx: &mut Context<Self>) {
        let query = self.search_query.trim().to_lowercase();
        self.search_hits.clear();
        if query.is_empty() {
            cx.notify();
            return;
        }
        let config = self.state.get_config_manager();
        for component in &self.components {
            let name = self.tr(&component.component_name);
            let description = self.tr(&component.component_description);
            if name.to_lowercase().contains(&query) || description.to_lowercase().contains(&query) {
                self.search_hits.push(SearchHit {
                    component_id: component.component_id.clone(),
                    field: None,
                    label: name.clone(),
                    detail: description.clone(),
                });
            }
            let Some(schema) = config.get_component_schema(&component.component_id) else {
                continue;
            };
            for field in
                schema::object_fields(&schema.contribution.properties, &schema.contribution.ui)
            {
                let label = self.tr(&field.ui.label);
                let detail = self.tr(&field.ui.description);
                if label.to_lowercase().contains(&query) || detail.to_lowercase().contains(&query) {
                    self.search_hits.push(SearchHit {
                        component_id: component.component_id.clone(),
                        field: Some(field.key.to_string()),
                        label: format!("{} · {}", name, label),
                        detail,
                    });
                }
                if self.search_hits.len() >= 80 {
                    cx.notify();
                    return;
                }
            }
        }
        cx.notify();
    }

    /// 消费宿主事件。
    fn on_host_event(&mut self, event: HostEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            HostEvent::ConfigChanged { component_id, .. } => {
                if self.selected_component.as_deref() == Some(component_id.as_str()) && !self.dirty
                {
                    self.load_component(window, cx);
                }
                self.components = self.state.get_config_manager().get_all_components();
            }
            HostEvent::PluginInstalled { .. } | HostEvent::PluginUninstalled { .. } => {
                self.components = self.state.get_config_manager().get_all_components();
                if self.page == Page::Plugins {
                    self.reload_plugins(cx);
                }
            }
            HostEvent::InspectorStateUpdated if self.page == Page::Debug => {
                self.refresh_inspector(cx);
            }
            HostEvent::InspectorStateUpdated => {}
            _ => {}
        }
    }
}

/// 字段输入类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TextKind {
    /// 普通文本。
    Text,
    /// 浮点数。
    Number,
    /// 整数。
    Integer,
}

/// 路径转稳定字符串。
/// 参数：path - 字段路径。
/// 返回：以 `/` 分隔的缓存键。
pub(super) fn path_id(path: &[Segment]) -> String {
    let mut id = String::new();
    for segment in path {
        match segment {
            Segment::Key(key) => {
                if !id.is_empty() {
                    id.push('/');
                }
                id.push_str(key);
            }
            Segment::Index(index) => {
                id.push('/');
                id.push_str(&index.to_string());
            }
        }
    }
    id
}

/// 取默认组件。
/// 参数：components - 全部组件。
/// 返回：优先 general-config，其次第一个核心组件，最后第一个组件。
fn default_component_id(components: &[ComponentInfo]) -> Option<String> {
    components
        .iter()
        .find(|component| component.component_id == "general-config")
        .or_else(|| {
            components
                .iter()
                .find(|component| component.component_type == ComponentType::Core)
        })
        .or_else(|| components.first())
        .map(|component| component.component_id.clone())
}

/// 读取后端译文。
/// 参数：state - 应用状态；key - i18n key。
/// 返回：当前语言译文。
fn tr(state: &AppState, key: &str) -> String {
    let i18n = state.get_i18n_manager();
    i18n.t(&i18n.current_language(), key)
}

/// 字段值转文本。
/// 参数：value - JSON 值。
/// 返回：字符串；非字符串按 JSON 文本展示。
fn value_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// 按 schema 类型选择文本输入种类。
/// 参数：schema - 字段 schema。
/// 返回：普通文本、浮点数或整数。
fn text_kind(schema: &SchemaNode) -> TextKind {
    match &schema.kind {
        SchemaKind::Number { .. } => TextKind::Number,
        SchemaKind::Integer { .. } => TextKind::Integer,
        _ => TextKind::Text,
    }
}
