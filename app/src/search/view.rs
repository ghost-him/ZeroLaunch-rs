//! 搜索视图的保留状态与异步流程。

use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{
    px, size, App, AppContext as _, Context, Entity, Focusable as _, KeyDownEvent, ScrollHandle,
    Subscription, Task, Window,
};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use zerolaunch_plugin_api::common::ImageUtils;
use zerolaunch_plugin_api::host::CacheLevel;
use zerolaunch_plugin_api::plugin::PluginKind;
use zerolaunch_plugin_api::PluginHost as _;
use zerolaunch_plugin_api::{Query, QueryResponse};
use zerolaunch_rs_lib::core::config::exclusion_settings::ExclusionKind;
use zerolaunch_rs_lib::core::config::ConfigManager;
use zerolaunch_rs_lib::core::host::Host as _;
use zerolaunch_rs_lib::kernel;
use zerolaunch_rs_lib::plugin_framework::{
    ConfirmOutcome, ConfirmRequest, RoutedConfirm, SessionDispatcher,
};
use zerolaunch_rs_lib::runtime;
use zerolaunch_rs_lib::state::app_state::AppState;
use zerolaunch_rs_lib::utils::trace_id::generate_trace_id;

use crate::host::{GpuiHost, HostCommand, HostEvent};
use crate::shell::{session_snapshot, PluginUiHost, PluginView, Surface};

use super::model::{
    available_exclude_kinds, decode_icon, parse_inline_args, ActionInfo, ExcludeState, InlineState,
    Mode, PanelState, ParamField, ParamState, QueryOutcome, SearchItem,
};

/// 外观与窗口尺寸参数。
#[derive(Clone, Copy)]
pub struct Metrics {
    /// 窗口逻辑宽度。
    pub width: f32,
    /// 搜索栏高度。
    pub search_bar_height: f32,
    /// 单条候选高度。
    pub item_height: f32,
    /// 最多可见候选数。
    pub max_visible_results: usize,
    /// 底栏高度。
    pub footer_height: f32,
    /// 候选图标边长。
    pub icon_size: f32,
    /// 窗口圆角。
    pub corner_radius: f32,
    /// 候选标题字号。
    pub title_size: f32,
    /// 候选副标题字号。
    pub subtitle_size: f32,
}

impl Metrics {
    /// 从配置读取界面尺寸，缺失或越界时回落默认值。
    /// 参数：config - 配置管理器。
    /// 返回：视图使用的固定尺寸快照。
    pub fn from_config(config: &ConfigManager) -> Self {
        let width = metric(config, "appearance-config", "window_width", 800.0).clamp(400.0, 1200.0);
        let search_bar_height =
            metric(config, "appearance-config", "search_bar_height", 72.0).clamp(40.0, 120.0);
        let item_height =
            metric(config, "appearance-config", "result_item_height", 54.0).clamp(36.0, 80.0);
        let max_visible = metric(config, "appearance-config", "max_visible_results", 8.0)
            .clamp(3.0, 20.0) as usize;
        let footer_height =
            metric(config, "appearance-config", "footer_height", 48.0).clamp(0.0, 60.0);
        let icon_ratio =
            metric(config, "appearance-config", "result_item_icon_ratio", 0.72).clamp(0.3, 0.9);
        let title_ratio =
            metric(config, "appearance-config", "result_item_font_ratio", 0.30).clamp(0.2, 0.5);
        let subtitle_ratio = metric(
            config,
            "appearance-config",
            "result_item_subtitle_font_ratio",
            0.24,
        )
        .clamp(0.15, 0.4);
        let corner_radius =
            metric(config, "appearance-config", "window_corner_radius", 12.0).clamp(0.0, 24.0);
        Self {
            width: width as f32,
            search_bar_height: search_bar_height as f32,
            item_height: item_height as f32,
            max_visible_results: max_visible,
            footer_height: footer_height as f32,
            icon_size: (item_height * icon_ratio) as f32,
            corner_radius: corner_radius as f32,
            title_size: (item_height * title_ratio) as f32,
            subtitle_size: (item_height * subtitle_ratio) as f32,
        }
    }
}

/// 搜索主视图。
pub struct SearchView {
    /// 内核状态。
    pub(super) state: Arc<AppState>,
    /// 宿主实现。
    pub(super) host: Arc<GpuiHost>,
    /// 主输入框。
    pub(super) input: Entity<InputState>,
    /// 当前查询文本。
    pub(super) query: String,
    /// 当前候选列表。
    pub(super) results: Vec<SearchItem>,
    /// 选中候选下标。
    pub(super) selected: usize,
    /// 选中动作下标。
    pub(super) selected_action: usize,
    /// 当前会话形态。
    pub(super) mode: Mode,
    /// 行内参数状态。
    pub(super) inline: Option<InlineState>,
    /// 参数面板状态。
    pub(super) params: Option<ParamState>,
    /// 插件面板状态。
    pub(super) panel: Option<PanelState>,
    /// 第三方插件脚本视图。
    pub(super) plugin_view: Option<PluginView>,
    /// 插件脚本加载错误。
    pub(super) plugin_view_error: Option<String>,
    /// 插件脚本宿主。
    plugin_ui: Option<Rc<PluginUiHost>>,
    /// 隐藏方式选择面板状态。
    pub(super) exclude: Option<ExcludeState>,
    /// 最近会话代际。
    pub(super) generation: u64,
    /// 最近候选缓存世代。
    pub(super) candidate_generation: u64,
    /// 查询序号（丢弃过期响应）。
    pub(super) query_seq: u64,
    /// 确认请求是否在途。
    pub(super) confirm_in_flight: bool,
    /// 最近错误提示。
    pub(super) status: Option<String>,
    /// 空查询是否加载主页。
    pub(super) home_enabled: bool,
    /// 空格是否等同 Enter。
    pub(super) space_is_enter: bool,
    /// Esc 是否优先隐藏而非清空。
    pub(super) esc_hide_priority: bool,
    /// 向上选择键配置。
    pub(super) move_up_key: String,
    /// 向下选择键配置。
    pub(super) move_down_key: String,
    /// 当前尺寸参数。
    pub(super) metrics: Metrics,
    /// 候选列表滚动状态。
    pub(super) scroll: ScrollHandle,
    /// 窗口是否处于用户可见激活态。
    pub(super) active: bool,
    /// 当前查询任务（新查询取消旧任务）。
    query_task: Option<Task<()>>,
    /// 当前确认任务。
    confirm_task: Option<Task<()>>,
    /// 输入与窗口激活订阅。
    _subscriptions: Vec<Subscription>,
    /// 宿主事件/命令消费任务。
    _task: Task<()>,
}

impl SearchView {
    /// 创建搜索视图并启动会话事件订阅。
    /// 参数：state - 内核状态；host - 宿主；window/cx - GPUI 上下文。
    /// 返回：已聚焦输入框的视图。
    pub fn new(
        state: Arc<AppState>,
        host: Arc<GpuiHost>,
        commands: mpsc::UnboundedReceiver<HostCommand>,
        plugin_ui: Option<Rc<PluginUiHost>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let config = state.get_config_manager();
        let metrics = Metrics::from_config(&config);
        let placeholder = config
            .get_component_setting("appearance-config", "search_bar_placeholder")
            .and_then(|value| value.as_str().map(str::to_string))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| tr(&state, "search.placeholder"));
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));

        let input_subscription =
            cx.subscribe_in(&input, window, |this, input, event, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.on_input_changed(input, window, cx);
                }
            });

        let activation = cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.on_activated(window, cx);
            } else {
                this.on_deactivated(window, cx);
            }
        });

        let event_receiver = host.subscribe();
        let mut command_receiver = commands;
        let task = cx.spawn_in(window, async move |this, cx| {
            let mut receiver = event_receiver;
            loop {
                tokio::select! {
                    event = receiver.recv() => match event {
                        Ok(event) => {
                            let _ = this.update_in(cx, |this, window, cx| {
                                this.on_host_event(event, window, cx);
                            });
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            tracing::warn!("宿主事件积压，跳过 {skipped} 条");
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    },
                    command = command_receiver.recv() => match command {
                        Some(HostCommand::Exit) => {
                            let _ = this.update_in(cx, |_, _, cx| cx.quit());
                            break;
                        }
                        None => break,
                    },
                }
            }
        });

        let state_close = state.clone();
        window.on_window_should_close(cx, {
            let host = host.clone();
            move |_window, _cx| {
                host.hide_main_window();
                state_close.get_session_dispatcher().reset_session(true);
                false
            }
        });

        let mut view = Self {
            state,
            host,
            input,
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            selected_action: 0,
            mode: Mode::Search,
            inline: None,
            params: None,
            panel: None,
            plugin_view: None,
            plugin_view_error: None,
            plugin_ui,
            exclude: None,
            generation: 0,
            candidate_generation: 0,
            query_seq: 0,
            confirm_in_flight: false,
            status: None,
            home_enabled: false,
            space_is_enter: false,
            esc_hide_priority: false,
            move_up_key: String::new(),
            move_down_key: String::new(),
            metrics,
            scroll: ScrollHandle::new(),
            active: false,
            query_task: None,
            confirm_task: None,
            _subscriptions: vec![input_subscription, activation],
            _task: task,
        };
        view.reload_settings();
        view.focus_input(window, cx);
        view.resize(window);
        cx.defer_in(window, |this, window, cx| {
            this.submit_query(String::new(), false, window, cx);
        });
        view
    }

    /// 读取窗口行为与常驻结果框设置。
    fn reload_settings(&mut self) {
        let config = self.state.get_config_manager();
        self.home_enabled = bool_setting(
            &config,
            "window-behavior-config",
            "is_show_home_on_empty_query",
        );
        self.space_is_enter = bool_setting(&config, "window-behavior-config", "space_is_enter");
        self.esc_hide_priority = bool_setting(
            &config,
            "window-behavior-config",
            "is_esc_hide_window_priority",
        );
        self.move_up_key = string_setting(&config, "window-behavior-config", "move_up_key")
            .unwrap_or_else(|| "Ctrl+K".to_string());
        self.move_down_key = string_setting(&config, "window-behavior-config", "move_down_key")
            .unwrap_or_else(|| "Ctrl+J".to_string());
    }

    /// 输入框文本变化后的查询入口。
    fn on_input_changed(
        &mut self,
        input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(self.mode, Mode::InlineParam | Mode::ParamPanel) {
            return;
        }
        let raw = input.read(cx).value().to_string();
        self.submit_query(raw, false, window, cx);
    }

    /// 发起一次 UI 查询并丢弃过期响应。
    fn submit_query(
        &mut self,
        raw: String,
        confirm: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.query = raw.clone();
        self.query_seq = self.query_seq.wrapping_add(1);
        let seq = self.query_seq;
        self.status = None;
        if !confirm {
            self.confirm_in_flight = false;
        }
        if raw.is_empty() && !self.home_enabled {
            self.results.clear();
            self.selected = 0;
            self.selected_action = 0;
        }
        self.resize(window);
        cx.notify();

        let state = self.state.clone();
        let task = cx.spawn_in(window, async move |this, cx| {
            let outcome = run_on_backend(query_backend(state, raw, confirm))
                .await
                .unwrap_or_else(Err);
            let _ = this.update_in(cx, |this, window, cx| {
                this.apply_query_result(seq, outcome, window, cx);
            });
        });
        self.query_task = Some(task);
    }

    /// 应用后台查询结果。
    fn apply_query_result(
        &mut self,
        seq: u64,
        outcome: Result<QueryOutcome, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if seq != self.query_seq {
            return;
        }
        self.confirm_in_flight = false;
        match outcome {
            Ok(outcome) => {
                self.generation = self.generation.max(outcome.generation);
                self.candidate_generation =
                    self.candidate_generation.max(outcome.candidate_generation);
                self.mode = outcome.mode;
                self.results = outcome.results;
                self.inline = outcome.inline;
                self.panel = outcome.panel;
                self.params = None;
                self.exclude = None;
                self.selected = 0;
                self.selected_action = 0;
                self.sync_plugin_view(window, cx);
                if self.mode == Mode::InlineParam {
                    self.query.clear();
                    self.set_input_text("", window, cx);
                    self.focus_input(window, cx);
                }
                for (id, request) in outcome.pending_icons {
                    let host = self.host.clone();
                    let core = self.state.get_core_handle();
                    let generation = self.candidate_generation;
                    runtime::spawn(async move {
                        let data = core.get_icon_or_default(request).await;
                        let icon = ImageUtils::to_data_url(&data);
                        host.emit_icon_updated(generation, id, &icon);
                    });
                }
            }
            Err(error) => {
                tracing::warn!("查询失败: {error}");
                self.status = Some(error);
            }
        }
        self.resize(window);
        cx.notify();
    }

    /// 应用确认结果：进入参数面板则建立字段；执行完成的窗口隐藏由调用方负责。
    fn apply_confirm_result(
        &mut self,
        outcome: Result<RoutedConfirm, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match outcome {
            Ok(routed) => {
                self.generation = self.generation.max(routed.generation);
                self.candidate_generation = self.candidate_generation.max(
                    self.state
                        .get_session_dispatcher()
                        .get_candidates_generation(),
                );
                match routed.outcome {
                    ConfirmOutcome::Executed => {}
                    ConfirmOutcome::EnterParamPanel {
                        candidate_id,
                        user_arg_count,
                    } => {
                        let title = self
                            .results
                            .get(self.selected)
                            .map(|item| item.title.clone())
                            .unwrap_or_default();
                        let mut fields = Vec::with_capacity(user_arg_count);
                        for index in 0..user_arg_count {
                            let label = tr(&self.state, "search.paramLabel")
                                .replace("{n}", &(index + 1).to_string());
                            let placeholder = tr(&self.state, "search.paramPlaceholder")
                                .replace("{n}", &(index + 1).to_string());
                            let input =
                                cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
                            fields.push(ParamField { label, input });
                        }
                        self.mode = Mode::ParamPanel;
                        self.params = Some(ParamState {
                            candidate_id,
                            candidate_title: title,
                            fields,
                            focused: 0,
                            invalid: false,
                        });
                        self.focus_param_field(window, cx);
                        self.resize(window);
                    }
                }
            }
            Err(error) => {
                tracing::warn!("确认失败: {error}");
                self.status = Some(error.clone());
                self.host
                    .notify("ZeroLaunch", &format!("执行失败: {error}"));
            }
        }
        cx.notify();
    }

    /// 触发确认执行。
    pub(super) fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.confirm_in_flight {
            return;
        }
        match self.mode {
            Mode::Search => self.confirm_search(window, cx),
            Mode::InlineParam => self.confirm_inline(window, cx),
            Mode::ParamPanel => self.confirm_param(window, cx),
            Mode::PluginPanel | Mode::PluginImmersive => self.confirm_panel(window, cx),
        }
    }

    /// 普通搜索结果确认。
    fn confirm_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.results.get(self.selected) else {
            return;
        };
        let Some(action_id) = item.action_id(self.selected_action).map(str::to_string) else {
            return;
        };
        let candidate_id = item.id;
        let target_is_plugin = item.target_type == "Plugin";
        let needs_params = item.user_arg_count > 0;
        let payload = ConfirmRequest::Candidate {
            candidate_id,
            action_id,
            query_text: self.query.clone(),
            user_args: Vec::new(),
            generation: self.generation,
            candidate_generation: self.candidate_generation,
        };

        if !needs_params && !target_is_plugin {
            self.reset_local(window, cx);
            self.host.hide_main_window();
        }
        self.confirm_in_flight = true;
        let state = self.state.clone();
        let task = cx.spawn_in(window, async move |this, cx| {
            let outcome = run_on_backend(confirm_backend(state, payload))
                .await
                .unwrap_or_else(Err);
            let _ = this.update_in(cx, |this, window, cx| {
                this.confirm_in_flight = false;
                this.apply_confirm_result(outcome, window, cx);
            });
        });
        self.confirm_task = Some(task);
    }

    /// 行内参数确认。
    fn confirm_inline(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(inline) = self.inline.clone() else {
            return;
        };
        let args = parse_inline_args(self.input.read(cx).value().as_ref());
        if args.len() < inline.user_arg_count {
            self.status = Some(
                tr(&self.state, "search.paramCountHint")
                    .replace("{n}", &inline.user_arg_count.to_string()),
            );
            cx.notify();
            return;
        }
        let payload = ConfirmRequest::Candidate {
            candidate_id: inline.candidate_id,
            action_id: "execute".to_string(),
            query_text: inline.trigger_keyword,
            user_args: args,
            generation: self.generation,
            candidate_generation: self.candidate_generation,
        };
        self.reset_local(window, cx);
        self.host.hide_main_window();
        self.confirm_in_flight = true;
        let state = self.state.clone();
        let task = cx.spawn_in(window, async move |this, cx| {
            let outcome = run_on_backend(confirm_backend(state, payload))
                .await
                .unwrap_or_else(Err);
            let _ = this.update_in(cx, |this, window, cx| {
                this.confirm_in_flight = false;
                this.apply_confirm_result(outcome, window, cx);
            });
        });
        self.confirm_task = Some(task);
    }

    /// 参数面板确认：末字段提交，否则聚焦下一字段。
    fn confirm_param(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(params) = self.params.as_mut() else {
            return;
        };
        if params.focused + 1 < params.fields.len() {
            params.focused += 1;
            self.focus_param_field(window, cx);
            cx.notify();
            return;
        }
        let values: Vec<String> = params
            .fields
            .iter()
            .map(|field| field.input.read(cx).value().to_string())
            .collect();
        let first_empty = values.iter().position(|value| value.trim().is_empty());
        if let Some(index) = first_empty {
            params.focused = index;
            params.invalid = true;
            self.focus_param_field(window, cx);
            cx.notify();
            return;
        }
        let candidate_id = params.candidate_id;
        let payload = ConfirmRequest::Candidate {
            candidate_id,
            action_id: "execute".to_string(),
            query_text: self.query.clone(),
            user_args: values,
            generation: self.generation,
            candidate_generation: self.candidate_generation,
        };
        self.reset_local(window, cx);
        self.host.hide_main_window();
        self.confirm_in_flight = true;
        let state = self.state.clone();
        let task = cx.spawn_in(window, async move |this, cx| {
            let outcome = run_on_backend(confirm_backend(state, payload))
                .await
                .unwrap_or_else(Err);
            let _ = this.update_in(cx, |this, window, cx| {
                this.confirm_in_flight = false;
                this.apply_confirm_result(outcome, window, cx);
            });
        });
        self.confirm_task = Some(task);
    }

    /// 插件面板确认：有动作执行动作，否则发起确认查询。
    fn confirm_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.panel.as_ref() else {
            return;
        };
        let action = panel
            .actions
            .get(
                self.selected_action
                    .min(panel.actions.len().saturating_sub(1)),
            )
            .or_else(|| panel.actions.iter().find(|action| action.is_default))
            .map(|action| action.id.clone());
        if let Some(action_id) = action {
            let payload = ConfirmRequest::Candidate {
                candidate_id: 0,
                action_id,
                query_text: self.query.clone(),
                user_args: Vec::new(),
                generation: self.generation,
                candidate_generation: self.candidate_generation,
            };
            self.reset_local(window, cx);
            self.host.hide_main_window();
            self.confirm_in_flight = true;
            let state = self.state.clone();
            let task = cx.spawn_in(window, async move |this, cx| {
                let outcome = run_on_backend(confirm_backend(state, payload))
                    .await
                    .unwrap_or_else(Err);
                let _ = this.update_in(cx, |this, window, cx| {
                    this.confirm_in_flight = false;
                    this.apply_confirm_result(outcome, window, cx);
                });
            });
            self.confirm_task = Some(task);
        } else {
            self.confirm_in_flight = true;
            self.submit_query(self.query.clone(), true, window, cx);
        }
    }

    /// 键盘入口：搜索/参数/面板/排除面板共用。
    pub(super) fn handle_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.to_lowercase();
        let mods = event.keystroke.modifiers;

        if mods.control && key == "," {
            let host = self.host.clone();
            runtime::spawn(async move {
                host.hide_main_window();
                host.show_settings_window();
            });
            cx.stop_propagation();
            return;
        }

        if self.exclude.is_some() {
            match key.as_str() {
                "up" | "down" => {
                    self.move_exclude_kind(if key == "up" { -1 } else { 1 }, cx);
                    cx.stop_propagation();
                }
                "enter" => {
                    self.apply_exclude(window, cx);
                    cx.stop_propagation();
                }
                "escape" => {
                    self.exclude = None;
                    self.focus_input(window, cx);
                    cx.notify();
                    cx.stop_propagation();
                }
                _ => {}
            }
            return;
        }

        if key == "escape" {
            self.on_escape(window, cx);
            cx.stop_propagation();
            return;
        }
        if key == "enter" && !mods.alt && !mods.shift {
            self.confirm(window, cx);
            cx.stop_propagation();
            return;
        }
        if key == "space" && self.space_is_enter && !mods.modified() && self.mode == Mode::Search {
            self.confirm(window, cx);
            cx.stop_propagation();
            return;
        }
        if key == "backspace"
            && self.mode == Mode::InlineParam
            && self.input.read(cx).value().is_empty()
        {
            self.exit_inline(window, cx);
            cx.stop_propagation();
            return;
        }
        if key == "tab" {
            if self.mode == Mode::ParamPanel {
                self.move_param_focus(if mods.shift { -1 } else { 1 }, window, cx);
            } else {
                self.cycle_action(if mods.shift { -1 } else { 1 }, cx);
            }
            cx.stop_propagation();
            return;
        }

        match self.mode {
            Mode::Search => {
                if (key == "down" && !mods.modified())
                    || self.matches_spec(&self.move_down_key, event)
                {
                    self.select_next(cx);
                    cx.stop_propagation();
                } else if (key == "up" && !mods.modified())
                    || self.matches_spec(&self.move_up_key, event)
                {
                    self.select_prev(cx);
                    cx.stop_propagation();
                } else if let Some(action_id) =
                    self.quick_action(&event.keystroke.key, mods.control || mods.platform)
                {
                    self.confirm_with_action(action_id, window, cx);
                    cx.stop_propagation();
                } else if let Some(action_id) = self.action_shortcut(event) {
                    self.confirm_with_action(action_id, window, cx);
                    cx.stop_propagation();
                }
            }
            Mode::InlineParam | Mode::ParamPanel => {
                if self.matches_spec(&self.move_down_key, event) {
                    self.select_next(cx);
                    cx.stop_propagation();
                } else if self.matches_spec(&self.move_up_key, event) {
                    self.select_prev(cx);
                    cx.stop_propagation();
                }
            }
            Mode::PluginPanel | Mode::PluginImmersive => {}
        }
    }

    /// Escape 语义：面板/参数退出，搜索有输入则清空，否则隐藏窗口。
    fn on_escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.mode {
            Mode::InlineParam => self.exit_inline(window, cx),
            Mode::ParamPanel => self.exit_param(window, cx),
            Mode::PluginPanel | Mode::PluginImmersive => {
                self.mode = Mode::Search;
                self.panel = None;
                self.query.clear();
                self.set_input_text("", window, cx);
                self.submit_query(String::new(), false, window, cx);
                self.focus_input(window, cx);
            }
            Mode::Search => {
                if self.esc_hide_priority || self.query.is_empty() {
                    self.reset_local(window, cx);
                    self.host.hide_main_window();
                } else {
                    self.query.clear();
                    self.set_input_text("", window, cx);
                    self.submit_query(String::new(), false, window, cx);
                    self.focus_input(window, cx);
                }
            }
        }
    }

    /// 退出行内参数模式，恢复到触发词搜索。
    fn exit_inline(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let trigger = self
            .inline
            .take()
            .map(|inline| inline.trigger_keyword)
            .unwrap_or_default();
        self.mode = Mode::Search;
        self.set_input_text(&trigger, window, cx);
        self.submit_query(trigger, false, window, cx);
        self.focus_input(window, cx);
    }

    /// 退出参数面板，回到空查询。
    fn exit_param(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.params = None;
        self.mode = Mode::Search;
        self.set_input_text("", window, cx);
        self.submit_query(String::new(), false, window, cx);
        self.focus_input(window, cx);
    }

    /// 上下选择候选并滚动到可见区域。
    pub(super) fn select_next(&mut self, cx: &mut Context<Self>) {
        if self.results.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.results.len();
        self.selected_action = 0;
        self.scroll.scroll_to_item(self.selected);
        cx.notify();
    }

    /// 向上选择候选。
    pub(super) fn select_prev(&mut self, cx: &mut Context<Self>) {
        if self.results.is_empty() {
            return;
        }
        self.selected = (self.selected + self.results.len() - 1) % self.results.len();
        self.selected_action = 0;
        self.scroll.scroll_to_item(self.selected);
        cx.notify();
    }

    /// 循环切换选中项的动作。
    pub(super) fn cycle_action(&mut self, delta: i32, cx: &mut Context<Self>) {
        let actions = self
            .results
            .get(self.selected)
            .map(|item| item.actions.len())
            .unwrap_or_else(|| {
                self.panel
                    .as_ref()
                    .map(|panel| panel.actions.len())
                    .unwrap_or(0)
            });
        if actions == 0 {
            return;
        }
        let current = self.selected_action as i32;
        self.selected_action = (current + delta).rem_euclid(actions as i32) as usize;
        cx.notify();
    }

    /// 通过动作 ID 执行当前插件面板动作。
    pub(super) fn confirm_panel_action(
        &mut self,
        action_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.panel.is_none() {
            return;
        }
        let payload = ConfirmRequest::Candidate {
            candidate_id: 0,
            action_id,
            query_text: self.query.clone(),
            user_args: Vec::new(),
            generation: self.generation,
            candidate_generation: self.candidate_generation,
        };
        self.reset_local(window, cx);
        self.host.hide_main_window();
        self.confirm_in_flight = true;
        let state = self.state.clone();
        let task = cx.spawn_in(window, async move |this, cx| {
            let outcome = run_on_backend(confirm_backend(state, payload))
                .await
                .unwrap_or_else(Err);
            let _ = this.update_in(cx, |this, window, cx| {
                this.confirm_in_flight = false;
                this.apply_confirm_result(outcome, window, cx);
            });
        });
        self.confirm_task = Some(task);
    }

    /// 通过动作 ID 执行当前候选。
    pub(super) fn confirm_with_action(
        &mut self,
        action_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = self.results.get(self.selected) else {
            return;
        };
        let keep_visible = item.user_arg_count > 0 || item.target_type == "Plugin";
        let payload = ConfirmRequest::Candidate {
            candidate_id: item.id,
            action_id,
            query_text: self.query.clone(),
            user_args: Vec::new(),
            generation: self.generation,
            candidate_generation: self.candidate_generation,
        };
        if !keep_visible {
            self.reset_local(window, cx);
            self.host.hide_main_window();
        }
        self.confirm_in_flight = true;
        let state = self.state.clone();
        let task = cx.spawn_in(window, async move |this, cx| {
            let outcome = run_on_backend(confirm_backend(state, payload))
                .await
                .unwrap_or_else(Err);
            let _ = this.update_in(cx, |this, window, cx| {
                this.confirm_in_flight = false;
                this.apply_confirm_result(outcome, window, cx);
            });
        });
        self.confirm_task = Some(task);
    }

    /// 读取选中项的快捷动作。
    fn action_shortcut(&self, event: &KeyDownEvent) -> Option<String> {
        self.results.get(self.selected).and_then(|item| {
            item.actions
                .iter()
                .find(|action| {
                    !action.shortcut_key.is_empty()
                        && self.matches_spec(&action.shortcut_key, event)
                })
                .map(|action| action.id.clone())
        })
    }

    /// Ctrl/Meta + 1..9 快捷动作。
    fn quick_action(&self, key: &str, secondary: bool) -> Option<String> {
        if !secondary {
            return None;
        }
        let digit = key.parse::<usize>().ok()?;
        if !(1..=9).contains(&digit) {
            return None;
        }
        let item = self.results.get(self.selected)?;
        item.actions.get(digit - 1).map(|action| action.id.clone())
    }

    /// 按键说明符匹配（Ctrl/Shift/Alt/Meta + 主键）。
    fn matches_spec(&self, spec: &str, event: &KeyDownEvent) -> bool {
        let mut parts: Vec<&str> = spec.split('+').collect();
        let Some(main) = parts.pop() else {
            return false;
        };
        let mut ctrl = false;
        let mut shift = false;
        let mut alt = false;
        let mut meta = false;
        for modifier in parts {
            match modifier.to_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "shift" => shift = true,
                "alt" => alt = true,
                "meta" | "super" | "cmd" | "win" => meta = true,
                _ => return false,
            }
        }
        let mods = event.keystroke.modifiers;
        if mods.control != ctrl || mods.shift != shift || mods.alt != alt || mods.platform != meta {
            return false;
        }
        let key = normalize_key(main);
        normalize_key(&event.keystroke.key) == key
            || event
                .keystroke
                .key_char
                .as_deref()
                .map(normalize_key)
                .is_some_and(|typed| typed == key)
    }

    /// 设置主输入框文本。
    fn set_input_text(&mut self, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.input
            .update(cx, |state, cx| state.set_value(value, window, cx));
    }

    /// 聚焦主输入框。
    fn focus_input(&self, window: &mut Window, cx: &mut App) {
        let handle = self.input.read(cx).focus_handle(cx).clone();
        window.focus(&handle, cx);
    }

    /// 聚焦参数面板当前字段。
    fn focus_param_field(&self, window: &mut Window, cx: &mut App) {
        if let Some(params) = &self.params {
            if let Some(field) = params.fields.get(params.focused) {
                let handle = field.input.read(cx).focus_handle(cx).clone();
                window.focus(&handle, cx);
            }
        }
    }

    /// 参数面板上下/前后移动焦点。
    fn move_param_focus(&mut self, delta: i32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(params) = self.params.as_mut() else {
            return;
        };
        if params.fields.is_empty() {
            return;
        }
        let next = (params.focused as i32 + delta).rem_euclid(params.fields.len() as i32) as usize;
        params.focused = next;
        self.focus_param_field(window, cx);
        cx.notify();
    }

    /// 打开隐藏方式选择面板。
    pub(super) fn open_exclude(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.results.get(self.selected) else {
            return;
        };
        let options = match self
            .state
            .get_session_dispatcher()
            .candidate_exclude_options(item.id, self.candidate_generation)
        {
            Ok(options) => options,
            Err(error) => {
                self.status = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        let keyword = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(tr(&self.state, "search.excludeKeywordPlaceholder"))
        });
        self.exclude = Some(ExcludeState {
            candidate_id: item.id,
            title: item.title.clone(),
            options,
            kind: ExclusionKind::Exact,
            keyword,
        });
        self.resize(window);
        cx.notify();
    }

    /// 在可用的隐藏方式间循环。
    fn move_exclude_kind(&mut self, delta: i32, cx: &mut Context<Self>) {
        let Some(exclude) = self.exclude.as_mut() else {
            return;
        };
        let kinds = available_exclude_kinds(&exclude.options);
        let current = kinds
            .iter()
            .position(|kind| *kind == exclude.kind)
            .unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(kinds.len() as i32) as usize;
        exclude.kind = kinds[next];
        cx.notify();
    }

    /// 提交隐藏规则并关闭面板。
    fn apply_exclude(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(exclude) = self.exclude.as_ref() else {
            return;
        };
        let keyword = exclude.keyword.read(cx).value().to_string();
        if exclude.kind == ExclusionKind::Contains && keyword.trim().is_empty() {
            return;
        }
        let candidate_id = exclude.candidate_id;
        let kind = exclude.kind;
        let candidate_generation = self.candidate_generation;
        let dispatcher = self.state.get_session_dispatcher().clone();
        self.exclude = None;
        self.host.hide_main_window();
        self.reset_local(window, cx);
        runtime::spawn(async move {
            if let Err(error) = dispatcher
                .exclude_candidate(candidate_id, candidate_generation, kind, &keyword)
                .await
            {
                tracing::warn!("隐藏候选项失败: {error}");
            }
        });
    }

    /// 窗口失焦：隐藏并复位会话。
    fn on_deactivated(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.active {
            return;
        }
        self.active = false;
        let already_clean = self.query.is_empty()
            && self.mode == Mode::Search
            && self.results.is_empty()
            && self.panel.is_none()
            && self.params.is_none()
            && self.inline.is_none();
        self.host.hide_main_window();
        if already_clean {
            return;
        }
        self.reset_local(window, cx);
        let state = self.state.clone();
        runtime::spawn(async move {
            kernel::on_search_bar_focus_lost(&state).await;
        });
    }

    /// 窗口重新激活：恢复输入焦点并加载常驻结果框。
    fn on_activated(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active {
            return;
        }
        self.active = true;
        self.focus_input(window, cx);
        cx.defer_in(window, |this, window, cx| {
            this.focus_input(window, cx);
        });
        if self.mode == Mode::Search && self.query.is_empty() {
            self.submit_query(String::new(), false, window, cx);
        }
    }

    /// 清空所有本地会话状态与输入。
    fn reset_local(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.query_seq = self.query_seq.wrapping_add(1);
        self.query.clear();
        self.results.clear();
        self.selected = 0;
        self.selected_action = 0;
        self.mode = Mode::Search;
        self.inline = None;
        self.params = None;
        self.panel = None;
        self.plugin_view = None;
        self.plugin_view_error = None;
        if let Some(plugin_ui) = &self.plugin_ui {
            plugin_ui.bridge().set_session(None, None);
        }
        self.exclude = None;
        self.confirm_in_flight = false;
        self.status = None;
        self.set_input_text("", window, cx);
        self.resize(window);
        cx.notify();
    }

    /// 按当前插件面板同步脚本视图与脚本可见会话。
    /// 参数：window/cx - GPUI 上下文。
    /// 返回：无；非第三方面板清空脚本视图。
    fn sync_plugin_view(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(plugin_ui) = self.plugin_ui.clone() else {
            return;
        };
        let Some(panel) = self.panel.as_ref() else {
            self.plugin_view = None;
            self.plugin_view_error = None;
            plugin_ui.bridge().set_session(None, None);
            return;
        };
        let Some(plugin_id) = panel.plugin_id.clone() else {
            self.plugin_view = None;
            self.plugin_view_error = None;
            plugin_ui.bridge().set_session(None, None);
            return;
        };
        if !panel.panel_type.starts_with("third-party:") {
            self.plugin_view = None;
            self.plugin_view_error = None;
            plugin_ui.bridge().set_session(None, None);
            return;
        }
        let mode = match self.mode {
            Mode::PluginImmersive => "plugin_immersive",
            _ => "plugin_panel",
        };
        let actions: Vec<serde_json::Value> = panel
            .actions
            .iter()
            .map(|action| {
                serde_json::json!({
                    "id": action.id,
                    "label": action.label,
                    "isDefault": action.is_default,
                    "shortcutKey": action.shortcut_key,
                })
            })
            .collect();
        let panel_json = serde_json::json!({
            "panelType": panel.panel_type,
            "data": panel.data,
            "actions": actions,
        });
        plugin_ui.bridge().set_session(
            Some(&plugin_id),
            Some(session_snapshot(&plugin_id, mode, &self.query, &panel_json)),
        );
        if self
            .plugin_view
            .as_ref()
            .is_some_and(|view| view.plugin_id == plugin_id)
        {
            if let Some(view) = &self.plugin_view {
                view.refresh(cx);
            }
            return;
        }
        match plugin_ui.load_view(&plugin_id, Surface::Panel, window, cx) {
            Ok(view) => {
                self.plugin_view = Some(PluginView { plugin_id, view });
                self.plugin_view_error = None;
                cx.notify();
            }
            Err(error) => {
                self.plugin_view = None;
                self.plugin_view_error = Some(error);
                cx.notify();
            }
        }
    }

    /// 处理宿主事件。
    fn on_host_event(&mut self, event: HostEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            HostEvent::IconUpdated {
                candidate_generation,
                id,
                icon,
            } => {
                if candidate_generation != self.candidate_generation {
                    return;
                }
                if let Some(item) = self.results.iter_mut().find(|item| item.id == id) {
                    item.icon = decode_icon(&icon);
                    cx.notify();
                }
            }
            HostEvent::CandidatesReady => {
                if self.mode == Mode::Search {
                    self.submit_query(self.query.clone(), false, window, cx);
                }
            }
            HostEvent::MainWindowShown => {
                self.active = true;
                self.focus_input(window, cx);
                cx.defer_in(window, |this, window, cx| {
                    this.focus_input(window, cx);
                });
                if self.mode == Mode::Search && self.query.is_empty() {
                    self.submit_query(String::new(), false, window, cx);
                }
            }
            HostEvent::ConfigChanged {
                component_id,
                component_type,
            } => {
                if component_id == "window-behavior-config" {
                    self.reload_settings();
                }
                if component_id == "appearance-config" {
                    self.metrics = Metrics::from_config(&self.state.get_config_manager());
                    self.resize(window);
                }
                if component_type == "ExclusionRule" && self.mode == Mode::Search {
                    self.submit_query(self.query.clone(), false, window, cx);
                }
            }
            HostEvent::SessionState(event) => {
                if let Some(generation) = event.get("generation").and_then(|value| value.as_u64()) {
                    self.generation = self.generation.max(generation);
                }
                let kind = event.get("kind").and_then(|value| value.as_str());
                let view = event.get("view").and_then(|value| value.as_str());
                if kind == Some("host") && view == Some("none") {
                    self.reset_local(window, cx);
                    return;
                }
                if kind == Some("plugin") {
                    if let Some(content) =
                        event.get("panelContent").filter(|value| !value.is_null())
                    {
                        let panel_type = content
                            .get("panelType")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let data = content
                            .get("data")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                        let mut actions = actions_from_json(content.get("actions"));
                        for action in &mut actions {
                            action.label = tr(&self.state, &action.label);
                        }
                        let plugin_id = event
                            .get("pluginId")
                            .and_then(|value| value.as_str())
                            .map(str::to_string);
                        self.mode = if view == Some("pluginImmersive") {
                            Mode::PluginImmersive
                        } else {
                            Mode::PluginPanel
                        };
                        self.panel = Some(PanelState {
                            panel_type,
                            data,
                            actions,
                            plugin_id,
                        });
                        self.results.clear();
                        self.query.clear();
                        self.set_input_text("", window, cx);
                        self.selected = 0;
                        self.selected_action = 0;
                        self.sync_plugin_view(window, cx);
                        self.resize(window);
                        cx.notify();
                    }
                }
            }
            HostEvent::PluginInstalled {
                plugin_id,
                name,
                version,
            } => {
                tracing::debug!("插件已安装: {plugin_id} {name} {version}");
            }
            HostEvent::PluginUninstalled { plugin_id } => {
                tracing::debug!("插件已卸载: {plugin_id}");
                if self
                    .plugin_view
                    .as_ref()
                    .is_some_and(|view| view.plugin_id == plugin_id)
                {
                    self.plugin_view = None;
                    self.plugin_view_error = Some(tr(&self.state, "search.pluginUninstalled"));
                    cx.notify();
                }
            }
            HostEvent::PluginAction {
                plugin_id,
                action_id,
            } => {
                if self
                    .panel
                    .as_ref()
                    .and_then(|panel| panel.plugin_id.as_deref())
                    == Some(plugin_id.as_str())
                {
                    self.confirm_panel_action(action_id, window, cx);
                }
            }
            HostEvent::SystemThemeChanged(theme) => {
                tracing::debug!("系统主题变化: {theme}");
            }
            HostEvent::InspectorStateUpdated => {}
        }
    }

    /// 按内容计算窗口高度并下发尺寸。
    pub(super) fn resize(&mut self, window: &mut Window) {
        let rows = self.results.len().min(self.metrics.max_visible_results);
        let footer = if self.mode == Mode::Search
            && (!self.query.is_empty() || self.home_enabled)
            && self.exclude.is_none()
        {
            self.metrics.footer_height
        } else {
            0.0
        };
        let height = match self.mode {
            Mode::Search => {
                self.metrics.search_bar_height + rows as f32 * self.metrics.item_height + footer
            }
            Mode::InlineParam => self.metrics.search_bar_height + 110.0,
            Mode::ParamPanel => {
                let fields = self
                    .params
                    .as_ref()
                    .map(|params| params.fields.len())
                    .unwrap_or(1);
                self.metrics.search_bar_height + fields as f32 * 56.0 + 72.0
            }
            Mode::PluginPanel => self.metrics.search_bar_height + 360.0 + footer,
            Mode::PluginImmersive => {
                self.metrics.max_visible_results as f32 * self.metrics.item_height + footer
            }
        }
        .max(self.metrics.search_bar_height);
        window.resize(size(px(self.metrics.width), px(height)));
    }
}

/// 在全局 Tokio runtime 上执行内核异步调用，再把结果送回 GPUI 前台任务。
///
/// 参数：future - 需要 Tokio reactor 的内核 future。
/// 返回：内核结果；后台任务被取消时返回错误字符串。
async fn run_on_backend<F, T>(future: F) -> Result<T, String>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let (sender, receiver) = oneshot::channel();
    runtime::spawn(async move {
        let _ = sender.send(future.await);
    });
    receiver.await.map_err(|_| "后台内核任务已终止".to_string())
}

/// 后台执行一次查询并转换图标。
async fn query_backend(
    state: Arc<AppState>,
    raw_query: String,
    confirm: bool,
) -> Result<QueryOutcome, String> {
    let dispatcher = state.get_session_dispatcher().clone();
    let host_api = state.get_host_api();
    let trace_id = generate_trace_id();
    let query = Query {
        id: trace_id.clone(),
        raw_query: raw_query.clone(),
        search_term: raw_query.to_lowercase(),
        confirm,
    };
    let routed = dispatcher
        .route_query_ui(&trace_id, &query)
        .await
        .map_err(|error| error.to_string())?;
    let generation = routed.generation;
    let candidate_generation = dispatcher.get_candidates_generation();

    match routed.response {
        QueryResponse::List { results } => {
            let mut items = Vec::with_capacity(results.len());
            let mut pending = Vec::new();
            for item in results {
                let icon = match host_api.get_icon(&item.icon, CacheLevel::CacheOnly).await {
                    Ok(data) => decode_icon(&ImageUtils::to_data_url(&data)),
                    Err(_) => {
                        pending.push((item.id, item.icon.clone()));
                        None
                    }
                };
                let mut mapped = SearchItem::from_list_item(item, icon);
                for action in &mut mapped.actions {
                    action.label = tr(&state, &action.label);
                }
                items.push(mapped);
            }
            Ok(QueryOutcome {
                generation,
                candidate_generation,
                mode: Mode::Search,
                results: items,
                inline: None,
                panel: None,
                pending_icons: pending,
            })
        }
        QueryResponse::Empty => Ok(QueryOutcome {
            generation,
            candidate_generation,
            mode: Mode::Search,
            results: Vec::new(),
            inline: None,
            panel: None,
            pending_icons: Vec::new(),
        }),
        QueryResponse::CustomPanel {
            panel_type,
            data,
            actions,
            keep_search_bar,
        } => {
            let kind = routed
                .plugin_id
                .as_deref()
                .and_then(|id| {
                    dispatcher
                        .plugin_registry()
                        .get_metadata(id)
                        .map(|metadata| metadata.kind)
                })
                .unwrap_or(PluginKind::Builtin);
            let panel_type =
                SessionDispatcher::normalize_panel_type(&routed.plugin_id, kind, &panel_type);
            Ok(QueryOutcome {
                generation,
                candidate_generation,
                mode: if keep_search_bar {
                    Mode::PluginPanel
                } else {
                    Mode::PluginImmersive
                },
                results: Vec::new(),
                inline: None,
                panel: Some(PanelState {
                    panel_type,
                    data,
                    actions: actions
                        .into_iter()
                        .map(|action| {
                            let mut info = ActionInfo::from(action);
                            info.label = tr(&state, &info.label);
                            info
                        })
                        .collect(),
                    plugin_id: routed.plugin_id,
                }),
                pending_icons: Vec::new(),
            })
        }
        QueryResponse::InlineParam {
            candidate_id,
            trigger_keyword,
            user_arg_count,
        } => Ok(QueryOutcome {
            generation,
            candidate_generation,
            mode: Mode::InlineParam,
            results: Vec::new(),
            inline: Some(InlineState {
                candidate_id,
                trigger_keyword,
                user_arg_count,
            }),
            panel: None,
            pending_icons: Vec::new(),
        }),
    }
}

/// 后台执行一次确认。
async fn confirm_backend(
    state: Arc<AppState>,
    request: ConfirmRequest,
) -> Result<RoutedConfirm, String> {
    let trace_id = generate_trace_id();
    state
        .get_session_dispatcher()
        .route_confirm(&trace_id, request)
        .await
        .map_err(|error| error.to_string())
}

/// 把 JSON 动作数组解析为展示动作。
fn actions_from_json(value: Option<&serde_json::Value>) -> Vec<ActionInfo> {
    value
        .and_then(|value| value.as_array())
        .map(|actions| {
            actions
                .iter()
                .filter_map(|action| {
                    Some(ActionInfo {
                        id: action.get("id")?.as_str()?.to_string(),
                        label: action.get("label")?.as_str()?.to_string(),
                        is_default: action
                            .get("isDefault")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false),
                        shortcut_key: action
                            .get("shortcutKey")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 读取配置中的布尔项。
fn bool_setting(config: &ConfigManager, component: &str, key: &str) -> bool {
    config
        .get_component_setting(component, key)
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

/// 读取配置中的字符串项。
fn string_setting(config: &ConfigManager, component: &str, key: &str) -> Option<String> {
    config
        .get_component_setting(component, key)
        .and_then(|value| value.as_str().map(str::to_string))
}

/// 读取配置中的数值项。
fn metric(config: &ConfigManager, component: &str, key: &str, fallback: f64) -> f64 {
    config
        .get_component_setting(component, key)
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite())
        .unwrap_or(fallback)
}

/// 按当前界面语言取文案。
pub(super) fn tr(state: &AppState, key: &str) -> String {
    let i18n = state.get_i18n_manager();
    i18n.t(&i18n.current_language(), key)
}

/// 归一化按键名，兼容配置里的 ArrowUp/Space/Esc 写法。
fn normalize_key(key: &str) -> String {
    match key.to_lowercase().as_str() {
        "arrowup" | "up" => "up".to_string(),
        "arrowdown" | "down" => "down".to_string(),
        "arrowleft" | "left" => "left".to_string(),
        "arrowright" | "right" => "right".to_string(),
        "escape" | "esc" => "escape".to_string(),
        "space" | "spacebar" => "space".to_string(),
        "return" | "enter" => "enter".to_string(),
        other => other.to_string(),
    }
}
