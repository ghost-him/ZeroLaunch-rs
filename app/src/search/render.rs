//! 搜索主窗口的 GPUI 渲染。

use gpui_kit::base::{Selectable as _, StyledExt as _};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Icon, IconName, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    div, img, px, AnyElement, FontWeight, ImageSource, InteractiveElement as _, IntoElement,
    MouseButton, ObjectFit, ParentElement, Render, StatefulInteractiveElement as _, Styled,
    StyledImage as _, Window,
};

use zerolaunch_rs_lib::core::config::exclusion_settings::ExclusionKind;
use zerolaunch_rs_lib::state::app_state::AppState;

use super::model::{ActionInfo, ParamState, SearchItem};
use super::view::{tr, SearchView};
use super::Mode;

impl Render for SearchView {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> impl IntoElement {
        let immersive = self.mode == Mode::PluginImmersive;
        div()
            .id("search-root")
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(self.metrics.corner_radius))
            .capture_key_down(cx.listener(Self::handle_key))
            .when(immersive, |this| this.child(self.render_immersive(cx)))
            .when(!immersive, |this| {
                this.child(self.render_search_bar(cx))
                    .child(self.render_body(cx))
                    .child(self.render_footer(cx))
            })
    }
}

impl SearchView {
    /// 搜索栏：前缀、输入框与状态提示。
    fn render_search_bar(&self, cx: &mut gpui_kit::Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let prefix = match self.mode {
            Mode::InlineParam => self
                .inline
                .as_ref()
                .map(|inline| inline.trigger_keyword.clone()),
            Mode::PluginPanel => self.panel.as_ref().map(|panel| panel.panel_type.clone()),
            _ => None,
        };
        h_flex()
            .h(px(self.metrics.search_bar_height))
            .flex_shrink_0()
            .px_4()
            .gap_3()
            .border_b_1()
            .border_color(theme.border)
            .child(
                Icon::new(IconName::Search)
                    .small()
                    .text_color(theme.muted_foreground),
            )
            .when_some(prefix, |this, prefix| {
                this.child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded(px(4.0))
                        .bg(theme.accent)
                        .text_color(theme.accent_foreground)
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(prefix),
                )
            })
            .child(
                div().flex_1().min_w_0().child(
                    Input::new(&self.input)
                        .appearance(false)
                        .cleanable(false)
                        .w_full(),
                ),
            )
            .when_some(self.status.clone(), |this, status| {
                this.child(
                    div()
                        .max_w(px(260.0))
                        .truncate()
                        .text_size(px(12.0))
                        .text_color(theme.danger)
                        .child(status),
                )
            })
            .into_any_element()
    }

    /// 主内容区：排除选择器 / 参数面板 / 插件面板 / 结果列表。
    fn render_body(&self, cx: &mut gpui_kit::Context<Self>) -> AnyElement {
        if let Some(exclude) = &self.exclude {
            return self.render_exclude(exclude, cx);
        }
        if let Some(params) = &self.params {
            return self.render_params(params, cx);
        }
        if let Some(panel) = &self.panel {
            return self.render_panel(panel, cx);
        }
        self.render_results(cx)
    }

    /// 渲染候选列表。
    fn render_results(&self, cx: &mut gpui_kit::Context<Self>) -> AnyElement {
        if self.results.is_empty() {
            let message = if self.query.is_empty() {
                tr(&self.state, "search.typeToSearch")
            } else {
                tr(&self.state, "search.noResults")
            };
            return div()
                .flex_1()
                .min_h_0()
                .p_4()
                .text_size(px(13.0))
                .text_color(cx.theme().muted_foreground)
                .child(message)
                .into_any_element();
        }

        let rows: Vec<AnyElement> = self
            .results
            .iter()
            .enumerate()
            .map(|(index, item)| self.render_item(index, item, cx))
            .collect();
        div()
            .id("results-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .v_flex()
            .children(rows)
            .into_any_element()
    }

    /// 单条候选：图标、标题/副标题、当前动作提示。
    fn render_item(
        &self,
        index: usize,
        item: &SearchItem,
        cx: &mut gpui_kit::Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let selected = index == self.selected;
        let icon_size = px(self.metrics.icon_size);
        let icon = item
            .icon
            .clone()
            .map(|image| {
                img(ImageSource::Image(image))
                    .size(icon_size)
                    .object_fit(ObjectFit::Contain)
                    .into_any_element()
            })
            .unwrap_or_else(|| {
                let letter = item.title.chars().next().unwrap_or('?').to_string();
                div()
                    .size(icon_size)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.0))
                    .bg(theme.muted)
                    .text_color(theme.muted_foreground)
                    .text_size(px(14.0))
                    .child(letter)
                    .into_any_element()
            });
        let action = item
            .actions
            .get(
                self.selected_action
                    .min(item.actions.len().saturating_sub(1)),
            )
            .or_else(|| item.actions.iter().find(|action| action.is_default));
        let action_text = action
            .map(|action| {
                if action.shortcut_key.is_empty() {
                    action.label.clone()
                } else {
                    format!("{}  {}", action.label, action.shortcut_key)
                }
            })
            .unwrap_or_default();

        h_flex()
            .id(("result", item.id))
            .h(px(self.metrics.item_height))
            .mx_2()
            .px_3()
            .gap_3()
            .rounded(px(7.0))
            .when(selected, |this| {
                this.bg(theme.accent).text_color(theme.accent_foreground)
            })
            .when(!selected, |this| {
                this.hover(|style| style.bg(theme.button_hover))
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    this.selected = index;
                    this.selected_action = 0;
                    this.confirm(window, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, _, window, cx| {
                    this.selected = index;
                    this.open_exclude(window, cx);
                }),
            )
            .child(icon)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .truncate()
                            .text_size(px(self.metrics.title_size))
                            .font_weight(FontWeight::MEDIUM)
                            .child(item.title.clone()),
                    )
                    .when(!item.subtitle.is_empty(), |this| {
                        this.child(
                            div()
                                .truncate()
                                .text_size(px(self.metrics.subtitle_size))
                                .text_color(if selected {
                                    theme.accent_foreground
                                } else {
                                    theme.muted_foreground
                                })
                                .child(item.subtitle.clone()),
                        )
                    }),
            )
            .when(!action_text.is_empty(), |this| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(11.0))
                        .text_color(if selected {
                            theme.accent_foreground
                        } else {
                            theme.muted_foreground
                        })
                        .child(action_text),
                )
            })
            .into_any_element()
    }

    /// 参数面板：逐字段输入与提交提示。
    fn render_params(&self, params: &ParamState, cx: &mut gpui_kit::Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let hint = if params.invalid {
            tr(&self.state, "search.paramIncomplete")
        } else if params.fields.len() > 1 {
            tr(&self.state, "search.paramHintMulti")
        } else {
            tr(&self.state, "search.paramHint")
        };
        div()
            .id("params-scroll")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .p_4()
            .gap_3()
            .overflow_y_scroll()
            .when(!params.candidate_title.is_empty(), |this| {
                this.child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.muted_foreground)
                        .child(params.candidate_title.clone()),
                )
            })
            .children(params.fields.iter().enumerate().map(|(index, field)| {
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.muted_foreground)
                            .child(field.label.clone()),
                    )
                    .child(div().h(px(40.0)).child(Input::new(&field.input).w_full()))
                    .when(index == params.focused && params.invalid, |this| {
                        this.border_1().border_color(theme.danger).rounded(px(6.0))
                    })
            }))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(if params.invalid {
                        theme.danger
                    } else {
                        theme.muted_foreground
                    })
                    .child(hint),
            )
            .into_any_element()
    }

    /// 隐藏方式选择面板。
    fn render_exclude(
        &self,
        exclude: &super::model::ExcludeState,
        cx: &mut gpui_kit::Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let kinds = super::model::available_exclude_kinds(&exclude.options);
        v_flex()
            .flex_1()
            .min_h_0()
            .p_4()
            .gap_3()
            .child(
                div()
                    .text_size(px(14.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!(
                        "{}: {}",
                        tr(&self.state, "search.hideResult"),
                        exclude.title
                    )),
            )
            .children(kinds.into_iter().map(|kind| {
                let selected = kind == exclude.kind;
                let label = exclude_kind_label(kind, &self.state);
                Button::new(format!("exclude-kind-{}", exclusion_key(&kind)))
                    .ghost()
                    .small()
                    .selected(selected)
                    .label(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(exclude) = this.exclude.as_mut() {
                            exclude.kind = kind;
                        }
                        cx.notify();
                    }))
            }))
            .when(exclude.kind == ExclusionKind::Contains, |this| {
                this.child(
                    v_flex().gap_1().child(
                        div()
                            .h(px(40.0))
                            .child(Input::new(&exclude.keyword).w_full()),
                    ),
                )
            })
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.muted_foreground)
                    .child(tr(&self.state, "search.excludeAdvancedHint")),
            )
            .into_any_element()
    }

    /// 插件面板容器：内置 calculator/translator 做轻量渲染，其他显示数据摘要。
    fn render_panel(
        &self,
        panel: &super::model::PanelState,
        cx: &mut gpui_kit::Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        v_flex()
            .flex_1()
            .min_h_0()
            .p_4()
            .gap_3()
            .when(self.mode == Mode::PluginImmersive, |this| {
                this.child(
                    h_flex()
                        .gap_2()
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded(px(4.0))
                                .bg(theme.accent)
                                .text_color(theme.accent_foreground)
                                .text_size(px(12.0))
                                .child(panel.panel_type.clone()),
                        )
                        .when_some(panel.plugin_id.clone(), |this, plugin_id| {
                            this.child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.muted_foreground)
                                    .child(plugin_id),
                            )
                        }),
                )
            })
            .child(
                div()
                    .id("panel-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(self.render_panel_data(panel)),
            )
            .when(!panel.actions.is_empty(), |this| {
                this.child(
                    h_flex()
                        .gap_2()
                        .children(panel.actions.iter().enumerate().map(|(index, action)| {
                            Button::new(format!("panel-action-{}", action.id))
                                .ghost()
                                .small()
                                .selected(index == self.selected_action)
                                .label(action_label(action))
                                .on_click(cx.listener({
                                    let action_id = action.id.clone();
                                    move |this, _, window, cx| {
                                        this.confirm_panel_action(action_id.clone(), window, cx);
                                    }
                                }))
                        })),
                )
            })
            .into_any_element()
    }

    /// 插件面板正文。
    fn render_panel_data(&self, panel: &super::model::PanelState) -> AnyElement {
        if panel.panel_type == "calculator" {
            let expression = panel
                .data
                .get("expression")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let result = panel
                .data
                .get("result")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let error = panel
                .data
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let text = if !error.is_empty() {
                error.to_string()
            } else if !result.is_empty() {
                format!("{expression} = {result}")
            } else {
                expression.to_string()
            };
            return div()
                .text_size(px(24.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(text)
                .into_any_element();
        }
        if panel.panel_type == "translator" {
            let status = panel
                .data
                .get("status")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let primary = panel
                .data
                .get("primary")
                .and_then(|value| value.get("text"))
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let message = panel
                .data
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            return v_flex()
                .gap_2()
                .child(
                    div()
                        .text_size(px(20.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(if primary.is_empty() {
                            status.to_string()
                        } else {
                            primary.to_string()
                        }),
                )
                .when(!message.is_empty(), |this| {
                    this.child(div().text_size(px(13.0)).child(message.to_string()))
                })
                .into_any_element();
        }
        let text =
            serde_json::to_string_pretty(&panel.data).unwrap_or_else(|_| panel.data.to_string());
        let text = text.chars().take(2000).collect::<String>();
        div()
            .font_family("Consolas")
            .text_size(px(12.0))
            .child(text)
            .into_any_element()
    }

    /// 沉浸式插件容器：无搜索栏，占满窗口。
    fn render_immersive(&self, cx: &mut gpui_kit::Context<Self>) -> AnyElement {
        if let Some(panel) = &self.panel {
            return self.render_panel(panel, cx);
        }
        div()
            .flex_1()
            .p_4()
            .text_color(cx.theme().muted_foreground)
            .child(tr(&self.state, "search.ready"))
            .into_any_element()
    }

    /// 底栏：结果数、当前候选的动作与快捷键提示。
    fn render_footer(&self, cx: &mut gpui_kit::Context<Self>) -> AnyElement {
        if self.mode != Mode::Search
            || self.exclude.is_some()
            || (self.query.is_empty() && !self.home_enabled)
        {
            return div().into_any_element();
        }
        let theme = cx.theme();
        let actions: Vec<ActionInfo> = self
            .results
            .get(self.selected)
            .map(|item| item.actions.clone())
            .unwrap_or_default();
        let count = tr(&self.state, "search.candidates")
            .replace("{count}", &self.results.len().to_string());
        h_flex()
            .h(px(self.metrics.footer_height))
            .flex_shrink_0()
            .px_4()
            .gap_3()
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.muted_foreground)
                    .child(count),
            )
            .child(
                h_flex()
                    .flex_1()
                    .gap_2()
                    .children(actions.iter().enumerate().map(|(index, action)| {
                        Button::new(format!("footer-action-{}", action.id))
                            .ghost()
                            .xsmall()
                            .selected(index == self.selected_action)
                            .label(action_label(action))
                            .on_click(cx.listener({
                                let action_id = action.id.clone();
                                move |this, _, window, cx| {
                                    this.confirm_with_action(action_id.clone(), window, cx);
                                }
                            }))
                    })),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.muted_foreground)
                    .child("↑↓ · Enter · Esc"),
            )
            .into_any_element()
    }
}

/// 动作标签 + 可选快捷键。
fn action_label(action: &ActionInfo) -> String {
    if action.shortcut_key.is_empty() {
        action.label.clone()
    } else {
        format!("{}  {}", action.label, action.shortcut_key)
    }
}

/// 隐藏方式的可读标签。
fn exclude_kind_label(kind: ExclusionKind, state: &AppState) -> String {
    let key = match kind {
        ExclusionKind::Exact => "search.excludeExactLabel",
        ExclusionKind::PathPrefix => "search.excludeFolderLabel",
        ExclusionKind::Contains => "search.excludeKeywordLabel",
    };
    tr(state, key)
}

/// 隐藏方式用于稳定元素 ID 的键。
fn exclusion_key(kind: &ExclusionKind) -> &'static str {
    match kind {
        ExclusionKind::Exact => "exact",
        ExclusionKind::PathPrefix => "path-prefix",
        ExclusionKind::Contains => "contains",
    }
}
