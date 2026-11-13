//! 设置窗口渲染：侧栏、schema 表单、插件/市场/关于/检查器。

use gpui_kit::base::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::select::Select;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Icon, IconName, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    div, img, px, rgb, AnyElement, Context, ImageSource, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, Render, StatefulInteractiveElement as _, Styled, Window,
};
use serde_json::Value;
use zerolaunch_plugin_api::config::{
    DataActionBinding, FieldAction, FieldUiMetadata, SchemaKind, SchemaNode, WidgetHint,
};
use zerolaunch_plugin_api::plugin::PluginKind;

use super::schema::{self, Segment};
use super::view::{
    path_id, pipeline_type_key, ActionMode, Category, Page, SearchTarget, SettingsView,
};

/// 渲染时使用的字段副本，避免在可变借用视图时继续借用 schema。
#[derive(Clone)]
struct OwnedField {
    key: String,
    schema: SchemaNode,
    ui: FieldUiMetadata,
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("settings-root")
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .capture_key_down(cx.listener(Self::handle_hotkey_key))
            .child(self.render_header(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_sidebar(cx))
                    .child(self.render_content(window, cx)),
            )
    }
}

impl SettingsView {
    /// 顶部标题与状态。
    fn render_header(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .h(px(48.0))
            .flex_shrink_0()
            .px_4()
            .gap_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().secondary)
            .child(
                div()
                    .text_size(px(16.0))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(self.tr("settings.title")),
            )
            .when_some(self.status().map(str::to_string), |this, status| {
                this.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(12.0))
                        .text_color(cx.theme().muted_foreground)
                        .child(status),
                )
            })
            .into_any_element()
    }

    /// 左侧分类栏。
    fn render_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w(px(200.0))
            .flex_shrink_0()
            .h_full()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .py(px(8.0))
            .child(
                div()
                    .id("settings-sidebar-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(self.render_categories(cx)),
            )
            .into_any_element()
    }

    /// 分类列表。
    fn render_categories(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut list = v_flex().gap_1();
        for category in [
            Category::Core,
            Category::Appearance,
            Category::Models,
            Category::Pipeline,
            Category::Plugins,
            Category::Market,
        ] {
            list = list.child(self.render_category_item(category, cx));
        }
        if self.debug_enabled() {
            list = list.child(self.render_category_item(Category::Debug, cx));
        }
        for category in [Category::About, Category::Assistant] {
            list = list.child(self.render_category_item(category, cx));
        }
        list.into_any_element()
    }

    /// 单个分类按钮。
    fn render_category_item(&self, category: Category, cx: &mut Context<Self>) -> AnyElement {
        let selected = self.category() == category;
        let label = self.category_label(category);
        let icon = category_icon(category);
        h_flex()
            .id(("settings-category", category as u8 as usize))
            .h(px(42.0))
            .mx(px(8.0))
            .px(px(12.0))
            .gap(px(12.0))
            .rounded(px(6.0))
            .text_size(px(14.0))
            .when(selected, |this| {
                this.bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .when(!selected, |this| {
                this.text_color(cx.theme().sidebar_foreground)
                    .hover(|style| style.bg(cx.theme().button_hover))
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                this.select_category(category, window, cx);
            }))
            .child(Icon::new(icon).text_size(px(18.0)))
            .child(label)
            .into_any_element()
    }

    /// 右侧内容区。
    fn render_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let pipeline = self.page() == Page::Component && self.category() == Category::Pipeline;
        v_flex()
            .id("settings-page-scroll")
            .flex_1()
            .min_w_0()
            .min_h_0()
            .h_full()
            .when(pipeline, |this| this.overflow_hidden())
            .when(!pipeline, |this| this.overflow_y_scroll())
            .child(match self.page() {
                Page::Component if pipeline => self.render_pipeline(window, cx),
                Page::Component => self.render_component_page(window, cx),
                Page::Plugins => self.render_plugins(cx),
                Page::PluginSettings => self.render_plugin_settings(cx),
                Page::Market => self.render_market(cx),
                Page::About => self.render_about(cx),
                Page::Assistant => self.render_assistant(cx),
                Page::Debug => self.render_debug(cx),
            })
            .into_any_element()
    }

    /// 搜索管道：流程说明 + 分组页签 + 组件主从面板。
    fn render_pipeline(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        const GROUPS: [(&str, &str); 8] = [
            ("DataSource", "settings.dataSource"),
            ("KeywordOptimizer", "settings.contentProcessor"),
            ("KeywordInjector", "settings.keywordInjector"),
            ("BiasRule", "settings.biasRule"),
            ("ExclusionRule", "settings.exclusionRule"),
            ("SearchEngine", "settings.searchEngine"),
            ("ScoreBooster", "settings.scoreBooster"),
            ("ActionExecutor", "settings.actionExecutor"),
        ];
        const STEPS: [(&str, &str); 8] = [
            (
                "settings.dataSource",
                "settings.pipelineFlow.steps.dataSource",
            ),
            (
                "settings.contentProcessor",
                "settings.pipelineFlow.steps.contentProcessor",
            ),
            (
                "settings.keywordInjector",
                "settings.pipelineFlow.steps.keywordInjector",
            ),
            ("settings.biasRule", "settings.pipelineFlow.steps.biasRule"),
            (
                "settings.exclusionRule",
                "settings.pipelineFlow.steps.exclusionRule",
            ),
            (
                "settings.searchEngine",
                "settings.pipelineFlow.steps.searchEngine",
            ),
            (
                "settings.scoreBooster",
                "settings.pipelineFlow.steps.scoreBooster",
            ),
            (
                "settings.actionExecutor",
                "settings.pipelineFlow.steps.actionExecutor",
            ),
        ];

        let active = self.pipeline_group().to_string();
        let items = self
            .category_components(Category::Pipeline)
            .into_iter()
            .filter(|item| pipeline_type_key(&item.component_type) == active)
            .collect::<Vec<_>>();
        let selected_in_group = self
            .selected_component()
            .is_some_and(|id| items.iter().any(|item| item.component_id == id));
        let expanded = self.pipeline_flow_expanded();
        let groups = GROUPS
            .into_iter()
            .map(|(key, label)| (key, self.tr(label)))
            .collect::<Vec<_>>();
        let steps = STEPS
            .into_iter()
            .map(|(label, desc)| (self.tr(label), self.tr(desc)))
            .collect::<Vec<_>>();
        let title = self.tr("settings.sidebar.pipeline");
        let flow_title = self.tr("settings.pipelineFlow.title");
        let empty_text = self.tr("settings.selectComponent");
        let group_label = groups
            .iter()
            .find(|(key, _)| *key == active)
            .map(|(_, label)| label.clone())
            .unwrap_or_default();

        let detail = if selected_in_group {
            self.render_component_page_with_chrome(window, cx, false)
        } else {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_2()
                .child(
                    Icon::new(IconName::File)
                        .text_size(px(32.0))
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(cx.theme().muted_foreground)
                        .child(empty_text),
                )
                .into_any_element()
        };

        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        let primary = cx.theme().primary;
        let flow = v_flex()
            .border_1()
            .border_color(border)
            .rounded(px(8.0))
            .overflow_hidden()
            .bg(cx.theme().secondary)
            .child(
                h_flex()
                    .id("pipeline-flow-toggle")
                    .px(px(16.0))
                    .py(px(14.0))
                    .gap_2()
                    .items_center()
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_pipeline_flow(cx)))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(muted)
                            .child(if expanded { "▼" } else { "▶" }),
                    )
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(flow_title),
                    ),
            )
            .when(expanded, |this| {
                this.child(
                    h_flex()
                        .flex_wrap()
                        .gap_2()
                        .px(px(16.0))
                        .pb(px(16.0))
                        .children(steps.into_iter().enumerate().map(|(index, (label, desc))| {
                            v_flex()
                                .gap(px(4.0))
                                .w(px(190.0))
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(
                                            div()
                                                .size(px(24.0))
                                                .rounded_full()
                                                .bg(cx.theme().sidebar_accent)
                                                .text_color(primary)
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_size(px(12.0))
                                                .child((index + 1).to_string()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(13.0))
                                                .font_weight(gpui_kit::FontWeight::MEDIUM)
                                                .child(label),
                                        ),
                                )
                                .child(div().text_size(px(11.0)).text_color(muted).child(desc))
                                .into_any_element()
                        })),
                )
            });

        let mut tabs = h_flex().gap(px(32.0)).border_b_1().border_color(border);
        for (key, label) in groups {
            let selected = key == active;
            let key_owned = key.to_string();
            tabs = tabs.child(
                div()
                    .id(format!("pipeline-group-{key}"))
                    .pb(px(8.0))
                    .border_b_2()
                    .text_size(px(14.0))
                    .when(selected, |this| {
                        this.text_color(primary).border_color(primary)
                    })
                    .when(!selected, |this| {
                        this.text_color(cx.theme().foreground)
                            .border_color(cx.theme().transparent)
                            .hover(|style| style.text_color(primary))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_pipeline_group(&key_owned, cx);
                    }))
                    .child(label),
            );
        }

        let mut list = v_flex().gap(px(2.0)).p(px(4.0));
        for item in &items {
            let id = item.component_id.clone();
            let name = self.tr(&item.component_name);
            let enabled = item.enabled;
            let selected = self.selected_component() == Some(id.as_str());
            let switch_id = id.clone();
            list = list.child(
                h_flex()
                    .id(format!("pipeline-item-{id}"))
                    .h(px(56.0))
                    .px(px(12.0))
                    .gap(px(10.0))
                    .rounded(px(6.0))
                    .when(selected, |this| this.bg(cx.theme().sidebar_accent))
                    .when(!enabled, |this| this.opacity(0.5))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            this.select_component(&id, None, window, cx);
                        }),
                    )
                    .child(
                        div()
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.0))
                            .bg(cx.theme().sidebar_accent)
                            .text_color(primary)
                            .child(Icon::new(IconName::LayoutDashboard).text_size(px(16.0))),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(13.0))
                                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                                    .child(name),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(11.0))
                                    .text_color(muted)
                                    .child(item.component_id.clone()),
                            ),
                    )
                    .child(
                        Switch::new(format!("pipeline-switch-{switch_id}"))
                            .checked(enabled)
                            .small()
                            .on_click(cx.listener(move |this, checked, _, cx| {
                                this.set_component_enabled(&switch_id, *checked, cx);
                            })),
                    ),
            );
        }

        v_flex()
            .size_full()
            .min_h_0()
            .px(px(24.0))
            .pt(px(16.0))
            .pb(px(16.0))
            .gap_3()
            .child(
                div()
                    .text_size(px(20.0))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(flow)
            .child(tabs)
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .border_1()
                    .border_color(border)
                    .rounded(px(8.0))
                    .overflow_hidden()
                    .child(
                        v_flex()
                            .w(px(240.0))
                            .h_full()
                            .min_h_0()
                            .border_r_1()
                            .border_color(border)
                            .child(
                                h_flex()
                                    .h(px(48.0))
                                    .px(px(16.0))
                                    .items_center()
                                    .border_b_1()
                                    .border_color(border)
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                            .child(group_label),
                                    )
                                    .child(div().flex_1())
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(muted)
                                            .child(items.len().to_string()),
                                    ),
                            )
                            .child(
                                div()
                                    .id("pipeline-list")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .child(list),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .h_full()
                            .overflow_y_scrollbar()
                            .child(detail),
                    ),
            )
            .into_any_element()
    }

    /// 组件设置页。
    fn render_component_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.render_component_page_with_chrome(window, cx, true)
    }

    /// 组件设置页；`with_chrome` 为 false 时用于管道详情，不渲染页签与启用开关。
    fn render_component_page_with_chrome(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        with_chrome: bool,
    ) -> AnyElement {
        let Some(schema) = self.component_schema() else {
            return div()
                .p_4()
                .text_color(cx.theme().muted_foreground)
                .child(self.tr("settings.selectComponent"))
                .into_any_element();
        };
        let Some(component) = self.component_info().cloned() else {
            return div()
                .p_4()
                .child(self.tr("settings.selectComponent"))
                .into_any_element();
        };

        let tabs = self.category_components(self.category());
        let immediate = schema::is_immediate(&schema.contribution);
        let title = self.tr(&component.component_name);
        let description = self.tr(&component.component_description);

        v_flex()
            .w_full()
            .min_h_0()
            .px(px(24.0))
            .pt(px(16.0))
            .pb(px(16.0))
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(20.0))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(div().flex_1())
                    .when(with_chrome, |this| {
                        this.child(
                            Switch::new("settings-component-enabled")
                                .checked(component.enabled)
                                .on_click(cx.listener(|this, checked, _, cx| {
                                    if *checked
                                        != this.component_info().is_some_and(|info| info.enabled)
                                    {
                                        this.toggle_enabled(cx);
                                    }
                                })),
                        )
                    }),
            )
            .when(with_chrome && !tabs.is_empty(), |this| {
                this.child(
                    h_flex()
                        .flex_wrap()
                        .gap(px(32.0))
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .children(tabs.into_iter().map(|tab| {
                            let selected =
                                self.selected_component() == Some(tab.component_id.as_str());
                            let id = tab.component_id.clone();
                            let tab_label = self.tr(&tab.component_name);
                            div()
                                .id(format!("settings-tab-{}", tab.component_id))
                                .pb(px(8.0))
                                .border_b_2()
                                .text_size(px(14.0))
                                .when(selected, |this| {
                                    this.text_color(cx.theme().primary)
                                        .border_color(cx.theme().primary)
                                })
                                .when(!selected, |this| {
                                    this.text_color(cx.theme().foreground)
                                        .border_color(cx.theme().transparent)
                                        .hover(|style| style.text_color(cx.theme().primary))
                                })
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.select_component(&id, None, window, cx);
                                }))
                                .child(tab_label)
                                .into_any_element()
                        })),
                )
            })
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(description),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("settings-save")
                            .label(self.tr("settings.save"))
                            .primary()
                            .small()
                            .disabled(!self.is_dirty() || immediate)
                            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                    )
                    .child(
                        Button::new("settings-discard")
                            .label(self.tr("settings.discard"))
                            .outline()
                            .small()
                            .disabled(!self.is_dirty())
                            .on_click(cx.listener(|this, _, window, cx| this.discard(window, cx))),
                    )
                    .child(
                        Button::new("settings-reset")
                            .label(if self.reset_armed() {
                                self.tr("settings.resetConfirmPositive")
                            } else {
                                self.tr("settings.reset")
                            })
                            .outline()
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| this.reset(window, cx))),
                    )
                    .when(immediate, |this| {
                        this.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(cx.theme().muted_foreground)
                                .child(self.tr("settings.autoSave")),
                        )
                    }),
            )
            .child(self.render_form(window, cx))
            .into_any_element()
    }

    /// schema 表单主体。
    fn render_form(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(schema) = self.component_schema() else {
            return div().into_any_element();
        };
        let fields: Vec<OwnedField> =
            schema::object_fields(&schema.contribution.properties, &schema.contribution.ui)
                .into_iter()
                .map(|field| OwnedField {
                    key: field.key.to_string(),
                    schema: field.schema.clone(),
                    ui: field.ui.clone(),
                })
                .collect();

        let mut groups: Vec<(String, Vec<OwnedField>)> = Vec::new();
        for field in fields {
            let group = field.ui.group.clone().unwrap_or_default();
            if let Some((_, items)) = groups.iter_mut().find(|(name, _)| *name == group) {
                items.push(field);
            } else {
                groups.push((group, vec![field]));
            }
        }

        let border = cx.theme().border;
        let mut form = v_flex().gap_3();
        if groups.is_empty() {
            return div()
                .p_4()
                .text_color(cx.theme().muted_foreground)
                .child(self.tr("settings.noConfigurableFields"))
                .into_any_element();
        }
        for (group, fields) in groups {
            let group_label = (!group.is_empty()).then(|| self.tr(&group));
            let fields = self.render_object_fields(fields, &[], window, cx);
            if let Some(group_label) = group_label {
                form = form.child(
                    v_flex()
                        .border_1()
                        .border_color(border)
                        .rounded(px(6.0))
                        .p_3()
                        .gap_3()
                        .child(
                            div()
                                .text_size(px(14.0))
                                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                .child(group_label),
                        )
                        .child(fields),
                );
            } else {
                form = form.child(fields);
            }
        }
        form.into_any_element()
    }

    /// 渲染一个对象作用域的字段列表。
    fn render_object_fields(
        &mut self,
        fields: Vec<OwnedField>,
        scope: &[Segment],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut list = v_flex().gap(px(14.0));
        for field in fields {
            if !self.field_visible(&field.ui, scope) {
                continue;
            }
            let mut path = scope.to_vec();
            path.push(Segment::Key(field.key.clone()));
            list = list.child(self.render_field(&field, &path, scope, true, window, cx));
        }
        list.into_any_element()
    }

    /// 渲染单个字段。
    fn render_field(
        &mut self,
        field: &OwnedField,
        path: &[Segment],
        scope: &[Segment],
        show_label: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let widget = schema::widget_of(&field.schema, &field.ui);
        let current = self.value(path).cloned().unwrap_or(Value::Null);
        let label = self.tr(&field.ui.label);
        let description = self.tr(&field.ui.description);
        let note = field
            .ui
            .note
            .as_deref()
            .map(|note| self.tr(note))
            .filter(|note| !note.is_empty());
        let read_only = field.ui.read_only;

        let mut body = v_flex().gap_1().py_1();
        if show_label {
            body = body.child(
                h_flex()
                    .gap(px(4.0))
                    .items_center()
                    .min_w_0()
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(14.0))
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .child(label),
                    )
                    .when(!description.is_empty(), |this| {
                        this.child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(px(13.0))
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("— {description}")),
                        )
                    }),
            );
        }

        let is_color = matches!(&widget, WidgetHint::Color);
        let widget_kind = widget.clone();
        let array_widget = matches!(
            widget_kind,
            WidgetHint::List
                | WidgetHint::Tags
                | WidgetHint::Table
                | WidgetHint::Cards
                | WidgetHint::MasterDetail
                | WidgetHint::SearchTable
        );
        let mut action_button = field.ui.action.clone().and_then(|action| {
            let data_select =
                matches!(widget_kind, WidgetHint::Select) && matches!(action, FieldAction::Data(_));
            let search_table = matches!(widget_kind, WidgetHint::SearchTable);
            if data_select || search_table {
                return None;
            }
            let label = self.action_label(&action);
            Some(self.render_action_button(path, scope, read_only, action, label, cx))
        });
        if array_widget {
            if let Some(button) = action_button.take() {
                body = body.child(button);
            }
        }
        let control = match widget {
            WidgetHint::Toggle => {
                let checked = current.as_bool().unwrap_or(false);
                let path = path.to_vec();
                Switch::new(format!("settings-switch-{}", path_id(path.as_slice())))
                    .checked(checked)
                    .disabled(read_only)
                    .on_click(cx.listener(move |this, checked, window, cx| {
                        this.set_bool(&path, *checked, window, cx);
                    }))
                    .into_any_element()
            }
            WidgetHint::Select => self.render_select(field, path, &current, read_only, window, cx),
            WidgetHint::MultiSelect => self.render_multi(field, path, &current, read_only, cx),
            WidgetHint::Path { mode } => {
                let path = path.to_vec();
                let value = text_value(&current);
                let input = self
                    .input(&path)
                    .map(|input| {
                        Input::new(&input)
                            .w_full()
                            .disabled(read_only)
                            .into_any_element()
                    })
                    .unwrap_or_else(|| {
                        div()
                            .text_size(px(12.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(value.clone())
                            .into_any_element()
                    });
                let directory = matches!(mode, zerolaunch_plugin_api::config::PathMode::Directory);
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(input))
                    .child(
                        Button::new(format!("settings-path-{}", path_id(&path)))
                            .label(self.tr("settings.selectPath"))
                            .small()
                            .disabled(read_only)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.pick_path(path.clone(), directory, window, cx);
                            })),
                    )
                    .into_any_element()
            }
            WidgetHint::Image { max_size, .. } => {
                let path = path.to_vec();
                let value_text = text_value(&current);
                let has_value = !value_text.is_empty();
                let resource_id = field.key.clone();
                if has_value {
                    self.ensure_image(&path, &value_text, window, cx);
                }
                h_flex()
                    .gap_2()
                    .items_center()
                    .when_some(self.image(&path), |this, image| {
                        this.child(
                            img(ImageSource::Image(image))
                                .size(px(40.0))
                                .rounded(px(4.0)),
                        )
                    })
                    .when(has_value, |this| {
                        this.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(cx.theme().muted_foreground)
                                .child(self.tr("settings.imageSet")),
                        )
                    })
                    .child(
                        Button::new(format!("settings-image-{}", path_id(&path)))
                            .label(if has_value {
                                self.tr("settings.imageReselect")
                            } else {
                                self.tr("settings.imageSelect")
                            })
                            .small()
                            .disabled(read_only)
                            .on_click(cx.listener({
                                let path = path.clone();
                                move |this, _, window, cx| {
                                    this.pick_image(
                                        path.clone(),
                                        resource_id.clone(),
                                        max_size,
                                        window,
                                        cx,
                                    );
                                }
                            })),
                    )
                    .when(has_value, |this| {
                        let path = path.clone();
                        this.child(
                            Button::new(format!("settings-image-clear-{}", path_id(&path)))
                                .label(self.tr("settings.imageClear"))
                                .small()
                                .disabled(read_only)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.clear_image(&path);
                                    this.set_value(&path, Value::String(String::new()), window, cx);
                                })),
                        )
                    })
                    .into_any_element()
            }
            WidgetHint::Font { action, component } => {
                self.render_font(path, &action, &component, read_only, window, cx)
            }
            WidgetHint::Hotkey { require_modifier } => {
                self.render_hotkey(path, &current, read_only, require_modifier, cx)
            }
            WidgetHint::Text | WidgetHint::Textarea | WidgetHint::Number | WidgetHint::Color => {
                let path = path.to_vec();
                let value = text_value(&current);
                let input = self
                    .input(&path)
                    .map(|input| {
                        Input::new(&input)
                            .w_full()
                            .disabled(read_only)
                            .into_any_element()
                    })
                    .unwrap_or_else(|| {
                        div()
                            .text_size(px(12.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(value.clone())
                            .into_any_element()
                    });
                let mut row = h_flex().gap_2().items_center();
                row = row.child(div().flex_1().child(input));
                if is_color {
                    if let Some(color) = parse_hex_color(&value) {
                        row = row.child(div().size(px(20.0)).rounded(px(4.0)).bg(rgb(color)));
                    }
                }
                row.into_any_element()
            }
            WidgetHint::List
            | WidgetHint::Tags
            | WidgetHint::Table
            | WidgetHint::Cards
            | WidgetHint::MasterDetail
            | WidgetHint::SearchTable => {
                self.render_array(field, path, scope, read_only, window, cx)
            }
        };

        body = body.child(control);
        if let Some(button) = action_button {
            body = body.child(button);
        }
        if show_label {
            if let Some(note) = note {
                body = body.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(cx.theme().muted_foreground)
                        .child(note),
                );
            }
        }
        body.into_any_element()
    }

    /// 渲染字段动作按钮。
    /// 参数：path/scope - 字段路径与作用域；read_only - 是否只读；action - 动作；
    ///       label - 动作标签；cx - 视图上下文。
    /// 返回：动作按钮。
    fn render_action_button(
        &self,
        path: &[Segment],
        scope: &[Segment],
        read_only: bool,
        action: FieldAction,
        label: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let loading = self.action_loading(path);
        let path = path.to_vec();
        let scope = scope.to_vec();
        Button::new(format!("settings-action-{}", path_id(&path)))
            .label(if loading { "...".to_string() } else { label })
            .w(px(140.0))
            .small()
            .disabled(read_only || loading)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.run_action(
                    path.clone(),
                    scope.clone(),
                    action.clone(),
                    ActionMode::Apply,
                    window,
                    cx,
                );
            }))
            .into_any_element()
    }

    /// 渲染系统字体选择器。
    /// 参数：path - 字段路径；action/component - 字体来源动作与组件；current - 当前值；
    ///       read_only - 是否只读；window/cx - GPUI 上下文。
    /// 返回：带跟随系统与重试的字体控件。
    fn render_font(
        &mut self,
        path: &[Segment],
        action: &str,
        component: &Option<String>,
        read_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.ensure_fonts(path, action, component, window, cx);
        let path = path.to_vec();
        let value = self.value(&path).map(text_value).unwrap_or_default();
        let mut row = h_flex().gap_2().items_center();
        if self.fonts_loaded(&path) {
            let select = self.font_select(&path, &value, read_only, window, cx);
            row = row.child(
                div()
                    .flex_1()
                    .child(Select::new(&select).w_full().disabled(read_only)),
            );
        } else if self.font_failed(&path) {
            row = row.child(
                div()
                    .flex_1()
                    .text_size(px(12.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.tr("settings.fontLoadFailed")),
            );
            let action = action.to_string();
            let component = component.clone();
            let path_for_retry = path.clone();
            row = row.child(
                Button::new(format!("settings-font-retry-{}", path_id(&path)))
                    .label(self.tr("settings.retry"))
                    .small()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.retry_fonts(&path_for_retry, cx);
                        this.ensure_fonts(&path_for_retry, &action, &component, window, cx);
                    })),
            );
        } else {
            row = row.child(
                div()
                    .flex_1()
                    .text_size(px(12.0))
                    .text_color(cx.theme().muted_foreground)
                    .child("..."),
            );
        }
        if !value.is_empty() {
            let path_for_clear = path.clone();
            row = row.child(
                Button::new(format!("settings-font-clear-{}", path_id(&path)))
                    .label(self.tr("settings.fontFollowSystem"))
                    .small()
                    .disabled(read_only)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.clear_font(&path_for_clear, window, cx);
                    })),
            );
        }
        row.into_any_element()
    }

    /// 渲染快捷键录制控件。
    /// 参数：path - 字段路径；current - 当前值；read_only - 是否只读；
    ///       require_modifier - 是否要求修饰键；cx - 视图上下文。
    /// 返回：可点击进入录制态的控件。
    fn render_hotkey(
        &mut self,
        path: &[Segment],
        current: &Value,
        read_only: bool,
        require_modifier: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let path = path.to_vec();
        let value = text_value(current);
        let recording = self.hotkey_recording(&path);
        let display = if recording {
            self.tr("settings.hotkeyPressToRecord")
        } else if value.is_empty() {
            self.tr("settings.hotkeyPlaceholder")
        } else {
            value.clone()
        };
        let path_for_click = path.clone();
        let mut row = h_flex().gap_2().items_center();
        row = row.child(
            div().flex_1().child(
                Button::new(format!("settings-hotkey-{}", path_id(&path)))
                    .label(display)
                    .w_full()
                    .disabled(read_only)
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.start_hotkey_recording(path_for_click.clone(), require_modifier, cx);
                    })),
            ),
        );
        if !value.is_empty() && !read_only {
            let path_for_clear = path.clone();
            row = row.child(
                Button::new(format!("settings-hotkey-clear-{}", path_id(&path)))
                    .label(self.tr("settings.clear"))
                    .small()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.set_value(&path_for_clear, Value::String(String::new()), window, cx);
                    })),
            );
        }
        row.into_any_element()
    }

    /// 下拉/枚举字段用选项芯片呈现。
    fn render_select(
        &mut self,
        field: &OwnedField,
        path: &[Segment],
        current: &Value,
        read_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let data_binding = field.ui.action.as_ref().and_then(|action| match action {
            FieldAction::Data(binding) => Some(binding.clone()),
            FieldAction::Effect(_) => None,
        });
        let options = if let Some(binding) = &data_binding {
            self.ensure_select_options(path, binding, window, cx);
            self.data_options(path)
                .map(|options| options.to_vec())
                .unwrap_or_default()
        } else {
            string_options(&field.schema)
        };
        if options.is_empty() && data_binding.is_none() {
            let path = path.to_vec();
            let value = text_value(current);
            return self
                .input(&path)
                .map(|input| {
                    Input::new(&input)
                        .w_full()
                        .disabled(read_only)
                        .into_any_element()
                })
                .unwrap_or_else(|| {
                    div()
                        .text_size(px(12.0))
                        .text_color(cx.theme().muted_foreground)
                        .child(value.clone())
                        .into_any_element()
                });
        }
        let selected = current.as_str().unwrap_or_default().to_string();
        let path = path.to_vec();
        let mut row = h_flex().flex_wrap().gap_1();
        if options.is_empty() && data_binding.is_some() {
            row = row.child(
                div()
                    .text_size(px(12.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(if self.action_loading(&path) {
                        "...".to_string()
                    } else {
                        self.tr("settings.noOptions")
                    }),
            );
        }
        for (value, label) in options {
            let active = value == selected;
            let label = if data_binding.is_some() && !label.is_empty() {
                label
            } else {
                self.tr(&label)
            };
            let path = path.clone();
            row = row.child(
                div()
                    .id(format!("settings-select-{}-{}", value, path_id(&path)))
                    .px_2()
                    .py_1()
                    .rounded(px(4.0))
                    .text_size(px(12.0))
                    .when(active, |this| {
                        this.bg(cx.theme().accent)
                            .text_color(cx.theme().accent_foreground)
                    })
                    .when(!active, |this| {
                        this.border_1()
                            .border_color(cx.theme().border)
                            .hover(|style| style.bg(cx.theme().button_hover))
                    })
                    .when(read_only, |this| this.opacity(0.5))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if !read_only {
                            this.set_value(&path, Value::String(value.clone()), window, cx);
                        }
                    }))
                    .child(label)
                    .into_any_element(),
            );
        }
        if let Some(binding) = data_binding {
            let label = field
                .ui
                .action
                .as_ref()
                .map(|action| self.action_label(action))
                .unwrap_or_default();
            let path_for_reload = path.clone();
            row = row.child(
                Button::new(format!("settings-select-refresh-{}", path_id(&path)))
                    .label(if self.action_loading(&path) {
                        "...".to_string()
                    } else {
                        label
                    })
                    .small()
                    .disabled(read_only || self.action_loading(&path))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.reload_select_options(&path_for_reload, &binding, window, cx);
                    })),
            );
        }
        row.into_any_element()
    }

    /// 多选字段用可点击选项芯片呈现。
    fn render_multi(
        &self,
        field: &OwnedField,
        path: &[Segment],
        current: &Value,
        read_only: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let options = string_options(&field.schema);
        let selected: Vec<String> = current
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let path = path.to_vec();
        let mut row = h_flex().flex_wrap().gap_1();
        for (value, label) in options {
            let active = selected.iter().any(|item| item == &value);
            let label = self.tr(&label);
            let path = path.clone();
            let option = value.clone();
            row = row.child(
                div()
                    .id(format!("settings-multi-{}-{}", value, path_id(&path)))
                    .px_2()
                    .py_1()
                    .rounded(px(4.0))
                    .text_size(px(12.0))
                    .when(active, |this| {
                        this.bg(cx.theme().accent)
                            .text_color(cx.theme().accent_foreground)
                    })
                    .when(!active, |this| {
                        this.border_1()
                            .border_color(cx.theme().border)
                            .hover(|style| style.bg(cx.theme().button_hover))
                    })
                    .when(read_only, |this| this.opacity(0.5))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if !read_only {
                            this.toggle_multi(&path, &option, window, cx);
                        }
                    }))
                    .child(label),
            );
        }
        row.into_any_element()
    }

    /// 数组字段：对象条目卡片 + 增删。
    fn render_array(
        &mut self,
        field: &OwnedField,
        path: &[Segment],
        _scope: &[Segment],
        read_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (items_schema, min_items, max_items) = match &field.schema.kind {
            SchemaKind::Array {
                items,
                min_items,
                max_items,
                ..
            } => (items.as_ref(), *min_items, *max_items),
            _ => return div().into_any_element(),
        };
        let widget = schema::widget_of(&field.schema, &field.ui);
        let item_object = matches!(&items_schema.kind, SchemaKind::Object { .. });
        if matches!(widget, WidgetHint::Tags) {
            return self.render_array_tags(field, path, read_only, window, cx);
        }
        if item_object && matches!(widget, WidgetHint::Table) {
            return self.render_array_table(field, path, items_schema, read_only, window, cx);
        }
        if item_object && matches!(widget, WidgetHint::MasterDetail) {
            return self.render_array_master_detail(
                field,
                path,
                items_schema,
                read_only,
                window,
                cx,
            );
        }
        if !item_object && matches!(widget, WidgetHint::List) {
            return self.render_array_primitive_rows(
                field,
                path,
                items_schema,
                read_only,
                window,
                cx,
            );
        }
        let search_table = matches!(widget, WidgetHint::SearchTable);
        let search_binding = if search_table {
            field.ui.action.as_ref().and_then(|action| match action {
                FieldAction::Data(binding) => Some(binding.clone()),
                FieldAction::Effect(_) => None,
            })
        } else {
            None
        };
        let values = self
            .value(path)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let can_add = !search_table && !read_only && max_items.is_none_or(|max| values.len() < max);

        let mut section = v_flex().gap_2().child(
            h_flex()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(cx.theme().muted_foreground)
                        .child(
                            self.tr("settings.itemCount")
                                .replace("{count}", &values.len().to_string()),
                        ),
                )
                .child(div().flex_1())
                .child(
                    Button::new(format!("settings-add-{}", path_id(path)))
                        .label(self.tr("settings.add"))
                        .w(px(80.0))
                        .small()
                        .disabled(!can_add)
                        .on_click(cx.listener({
                            let path = path.to_vec();
                            let item_schema = items_schema.clone();
                            move |this, _, window, cx| {
                                this.add_array_item(&path, &item_schema, max_items, window, cx);
                            }
                        })),
                ),
        );

        if let Some(binding) = &search_binding {
            section = section.child(self.render_search_action(
                path,
                binding,
                items_schema,
                max_items,
                read_only,
                cx,
            ));
        }

        for (index, item) in values.iter().enumerate() {
            let item_path = {
                let mut path = path.to_vec();
                path.push(Segment::Index(index));
                path
            };
            let item_fields = match &items_schema.kind {
                SchemaKind::Object { properties, ui, .. } => schema::object_fields(properties, ui)
                    .into_iter()
                    .map(|field| OwnedField {
                        key: field.key.to_string(),
                        schema: field.schema.clone(),
                        ui: field.ui.clone(),
                    })
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            };
            let mut card = v_flex()
                .gap_1()
                .p_2()
                .rounded(px(6.0))
                .border_1()
                .border_color(cx.theme().border)
                .child(
                    h_flex()
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("#{}", index + 1)),
                        )
                        .child(div().flex_1())
                        .child(
                            Button::new(format!("settings-remove-{}-{index}", path_id(path)))
                                .label(self.tr("settings.delete"))
                                .small()
                                .disabled(
                                    read_only || min_items.is_some_and(|min| values.len() <= min),
                                )
                                .on_click(cx.listener({
                                    let path = path.to_vec();
                                    move |this, _, window, cx| {
                                        this.remove_array_item(&path, index, min_items, window, cx);
                                    }
                                })),
                        ),
                );
            if item_fields.is_empty() {
                let current = text_value(item);
                let path = item_path.clone();
                let input = self
                    .input(&path)
                    .map(|input| {
                        Input::new(&input)
                            .w_full()
                            .disabled(read_only)
                            .into_any_element()
                    })
                    .unwrap_or_else(|| {
                        div()
                            .text_size(px(12.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(current.clone())
                            .into_any_element()
                    });
                card = card.child(input);
            } else {
                card = card.child(self.render_object_fields(item_fields, &item_path, window, cx));
            }
            section = section.child(card);
        }

        if values.is_empty() {
            section = section.child(
                div()
                    .text_size(px(12.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.tr("settings.noEntries")),
            );
        }
        section.into_any_element()
    }

    /// 标签数组：标签块 + 新增输入框。
    fn render_array_tags(
        &mut self,
        _field: &OwnedField,
        path: &[Segment],
        read_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let max_items = match &_field.schema.kind {
            SchemaKind::Array { max_items, .. } => *max_items,
            _ => None,
        };
        let values = self
            .value(path)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let key = path_id(path);
        let input = self.tag_input(&key, window, cx);
        let can_add = !read_only && max_items.is_none_or(|max| values.len() < max);
        let mut chips = h_flex().flex_wrap().gap_1();
        for (index, item) in values.iter().enumerate() {
            let text = item.as_str().unwrap_or_default().to_string();
            let path_for_remove = path.to_vec();
            let key_for_remove = key.clone();
            chips = chips.child(
                h_flex()
                    .id(format!("tag-{key}-{index}"))
                    .gap_2()
                    .items_center()
                    .px_2()
                    .py_1()
                    .rounded(px(4.0))
                    .bg(cx.theme().secondary)
                    .text_size(px(12.0))
                    .child(text)
                    .child(
                        div()
                            .id(format!("tag-remove-{key}-{index}"))
                            .cursor_pointer()
                            .text_color(cx.theme().muted_foreground)
                            .child("×")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let mut next = this
                                    .value(&path_for_remove)
                                    .and_then(Value::as_array)
                                    .cloned()
                                    .unwrap_or_default();
                                if index < next.len() {
                                    next.remove(index);
                                    this.set_value(
                                        &path_for_remove,
                                        Value::Array(next),
                                        window,
                                        cx,
                                    );
                                }
                                this.remove_tag_input(&key_for_remove);
                            })),
                    ),
            );
        }
        let add_path = path.to_vec();
        let key_for_add = key.clone();
        v_flex()
            .gap_2()
            .child(chips)
            .child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&input).w_full()))
                    .child(
                        Button::new(format!("tag-add-{key}"))
                            .label(self.tr("settings.add"))
                            .w(px(80.0))
                            .small()
                            .disabled(!can_add)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let text = this.tag_input_value(&key_for_add, cx);
                                let text = text.trim();
                                if text.is_empty() {
                                    return;
                                }
                                let mut next = this
                                    .value(&add_path)
                                    .and_then(Value::as_array)
                                    .cloned()
                                    .unwrap_or_default();
                                next.push(Value::String(text.to_string()));
                                this.set_value(&add_path, Value::Array(next), window, cx);
                                this.clear_tag_input(&key_for_add, window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    /// 原语数组：逐行编辑器 + 增删。
    fn render_array_primitive_rows(
        &mut self,
        field: &OwnedField,
        path: &[Segment],
        items_schema: &SchemaNode,
        read_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (min_items, max_items) = match &field.schema.kind {
            SchemaKind::Array {
                min_items,
                max_items,
                ..
            } => (*min_items, *max_items),
            _ => (None, None),
        };
        let values = self
            .value(path)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let can_add = !read_only && max_items.is_none_or(|max| values.len() < max);
        let mut rows = v_flex().gap_1();
        for (index, item) in values.iter().enumerate() {
            let mut item_path = path.to_vec();
            item_path.push(Segment::Index(index));
            let is_boolean = matches!(&items_schema.kind, SchemaKind::Boolean);
            let control = if is_boolean {
                let checked = item.as_bool().unwrap_or(false);
                let path_bool = item_path.clone();
                Switch::new(format!("array-bool-{}", path_id(&item_path)))
                    .checked(checked)
                    .disabled(read_only)
                    .on_click(cx.listener(move |this, checked, window, cx| {
                        this.set_value(&path_bool, Value::Bool(*checked), window, cx);
                    }))
                    .into_any_element()
            } else {
                let text = match item {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                };
                let kind = match &items_schema.kind {
                    SchemaKind::Integer { .. } => super::view::TextKind::Integer,
                    SchemaKind::Number { .. } => super::view::TextKind::Number,
                    _ => super::view::TextKind::Text,
                };
                let input = self.ensure_array_input(&item_path, &text, kind, window, cx);
                Input::new(&input)
                    .w_full()
                    .disabled(read_only)
                    .into_any_element()
            };
            let path_for_remove = path.to_vec();
            rows = rows.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().child(control))
                    .child(
                        Button::new(format!("array-remove-{}", path_id(&item_path)))
                            .label(self.tr("settings.delete"))
                            .small()
                            .disabled(read_only || min_items.is_some_and(|min| values.len() <= min))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.remove_array_item(
                                    &path_for_remove,
                                    index,
                                    min_items,
                                    window,
                                    cx,
                                );
                            })),
                    ),
            );
        }
        let add_path = path.to_vec();
        let item_schema = items_schema.clone();
        v_flex()
            .gap_2()
            .child(rows)
            .child(
                Button::new(format!("array-add-{}", path_id(path)))
                    .label(self.tr("settings.add"))
                    .w(px(80.0))
                    .small()
                    .disabled(!can_add)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.add_array_item(&add_path, &item_schema, max_items, window, cx);
                    })),
            )
            .into_any_element()
    }

    /// 对象数组表格：表头 + 行内字段编辑。
    fn render_array_table(
        &mut self,
        field: &OwnedField,
        path: &[Segment],
        items_schema: &SchemaNode,
        read_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (min_items, max_items) = match &field.schema.kind {
            SchemaKind::Array {
                min_items,
                max_items,
                ..
            } => (*min_items, *max_items),
            _ => (None, None),
        };
        let values = self
            .value(path)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let fields = array_object_fields(items_schema);
        let can_add = !read_only && max_items.is_none_or(|max| values.len() < max);
        let mut header = h_flex()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().secondary);
        for field in &fields {
            header = header.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .px_2()
                    .py_1()
                    .text_size(px(12.0))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(self.tr(&field.ui.label)),
            );
        }
        header = header.child(div().w(px(64.0)));
        let mut rows = v_flex();
        for (index, _) in values.iter().enumerate() {
            let mut item_path = path.to_vec();
            item_path.push(Segment::Index(index));
            let mut row = h_flex()
                .border_b_1()
                .border_color(cx.theme().border)
                .items_center();
            for field in &fields {
                let mut field_path = item_path.clone();
                field_path.push(Segment::Key(field.key.clone()));
                row = row.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .px_2()
                        .py_1()
                        .child(self.render_field(
                            field,
                            &field_path,
                            &item_path,
                            false,
                            window,
                            cx,
                        )),
                );
            }
            let path_for_remove = path.to_vec();
            row = row.child(
                div().w(px(64.0)).px_2().child(
                    Button::new(format!("array-row-remove-{}", path_id(&item_path)))
                        .label(self.tr("settings.delete"))
                        .small()
                        .disabled(read_only || min_items.is_some_and(|min| values.len() <= min))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.remove_array_item(&path_for_remove, index, min_items, window, cx);
                        })),
                ),
            );
            rows = rows.child(row);
        }
        let add_path = path.to_vec();
        let item_schema = items_schema.clone();
        v_flex()
            .gap_1()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(6.0))
            .overflow_hidden()
            .child(header)
            .child(rows)
            .child(
                div().p_2().child(
                    Button::new(format!("array-table-add-{}", path_id(path)))
                        .label(self.tr("settings.add"))
                        .w(px(80.0))
                        .small()
                        .disabled(!can_add)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.add_array_item(&add_path, &item_schema, max_items, window, cx);
                        })),
                ),
            )
            .into_any_element()
    }

    /// 对象数组主从：左侧条目列表，右侧选中项字段。
    fn render_array_master_detail(
        &mut self,
        field: &OwnedField,
        path: &[Segment],
        items_schema: &SchemaNode,
        read_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (min_items, max_items) = match &field.schema.kind {
            SchemaKind::Array {
                min_items,
                max_items,
                ..
            } => (*min_items, *max_items),
            _ => (None, None),
        };
        let values = self
            .value(path)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let fields = array_object_fields(items_schema);
        let key = path_id(path);
        let selected = self
            .array_selected(&key)
            .min(values.len().saturating_sub(1));
        let can_add = !read_only && max_items.is_none_or(|max| values.len() < max);
        let mut list = v_flex().gap_1();
        for (index, item) in values.iter().enumerate() {
            let summary = fields
                .first()
                .and_then(|field| item.get(&field.key))
                .and_then(Value::as_str)
                .map(str::to_string)
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| format!("#{}", index + 1));
            let key_for_select = key.clone();
            let path_for_remove = path.to_vec();
            list = list.child(
                h_flex()
                    .id(format!("md-item-{key}-{index}"))
                    .gap_2()
                    .items_center()
                    .px_2()
                    .py_2()
                    .rounded(px(5.0))
                    .when(index == selected, |this| this.bg(cx.theme().sidebar_accent))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.0))
                            .child(summary),
                    )
                    .child(
                        Button::new(format!("md-remove-{key}-{index}"))
                            .label(self.tr("settings.delete"))
                            .small()
                            .disabled(read_only || min_items.is_some_and(|min| values.len() <= min))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.remove_array_item(
                                    &path_for_remove,
                                    index,
                                    min_items,
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_array_selected(&key_for_select, index, cx);
                    })),
            );
        }
        let add_path = path.to_vec();
        let item_schema = items_schema.clone();
        let key_for_add = key.clone();
        let detail = if values.is_empty() {
            div()
                .p_4()
                .text_size(px(12.0))
                .text_color(cx.theme().muted_foreground)
                .child(self.tr("settings.noEntries"))
                .into_any_element()
        } else {
            let mut item_path = path.to_vec();
            item_path.push(Segment::Index(selected));
            self.render_object_fields(fields, &item_path, window, cx)
        };
        h_flex()
            .gap_0()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(8.0))
            .overflow_hidden()
            .child(
                v_flex()
                    .w(px(220.0))
                    .flex_shrink_0()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .p_1()
                    .child(list)
                    .child(
                        Button::new(format!("md-add-{key}"))
                            .label(self.tr("settings.add"))
                            .w(px(80.0))
                            .small()
                            .disabled(!can_add)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let next_index = this
                                    .value(&add_path)
                                    .and_then(Value::as_array)
                                    .map(Vec::len)
                                    .unwrap_or(0);
                                this.add_array_item(&add_path, &item_schema, max_items, window, cx);
                                this.set_array_selected(&key_for_add, next_index, cx);
                            })),
                    ),
            )
            .child(v_flex().flex_1().min_w_0().p_3().child(detail))
            .into_any_element()
    }

    /// 渲染搜索表格的查询框、结果与添加按钮。
    /// 参数：path - 数组路径；binding - data action；item_schema - 条目 schema；
    ///       max_items - 最大条目数；read_only - 是否只读；window/cx - GPUI 上下文。
    /// 返回：搜索表格区块。
    fn render_search_action(
        &mut self,
        path: &[Segment],
        binding: &DataActionBinding,
        item_schema: &SchemaNode,
        max_items: Option<usize>,
        read_only: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let path = path.to_vec();
        let input = self.field_search_input(&path);
        let loading = self.action_loading(&path);
        let mut search_row = h_flex().gap_2().items_center();
        search_row = match input {
            Some(input) => search_row.child(
                div()
                    .flex_1()
                    .child(Input::new(&input).w_full().disabled(read_only)),
            ),
            None => search_row.child(div().flex_1()),
        };
        let binding_for_search = binding.clone();
        let path_for_search = path.clone();
        search_row = search_row.child(
            Button::new(format!("settings-search-{}", path_id(&path)))
                .label(if loading {
                    "...".to_string()
                } else {
                    self.tr("common.search")
                })
                .small()
                .disabled(read_only || loading)
                .on_click(cx.listener(move |this, _, window, cx| {
                    let query = this
                        .field_search_input(&path_for_search)
                        .map(|input| input.read(cx).value().to_string())
                        .unwrap_or_default();
                    this.run_action(
                        path_for_search.clone(),
                        Vec::new(),
                        FieldAction::Data(binding_for_search.clone()),
                        ActionMode::Search(query),
                        window,
                        cx,
                    );
                })),
        );
        let mut section = v_flex().gap_1().child(search_row);
        let results = self.search_results(&path).to_vec();
        for (index, row) in results.iter().enumerate() {
            let label = row
                .get(&binding.label_field)
                .and_then(Value::as_str)
                .filter(|label| !label.is_empty())
                .or_else(|| row.get(&binding.value_field).and_then(Value::as_str))
                .unwrap_or_default()
                .to_string();
            let value = row
                .get(&binding.value_field)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let exists = self
                .value(&path)
                .and_then(Value::as_array)
                .is_some_and(|items| {
                    items.iter().any(|item| {
                        item.get(&binding.value_field).and_then(Value::as_str)
                            == Some(value.as_str())
                    })
                });
            let binding_for_add = binding.clone();
            let item_schema = item_schema.clone();
            let path_for_add = path.clone();
            let row = row.clone();
            let target = SearchTarget {
                binding: binding_for_add,
                item_schema,
                max_items,
            };
            section = section.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .px_2()
                    .py_1()
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(div().flex_1().text_size(px(12.0)).child(label))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(value.clone()),
                    )
                    .child(
                        Button::new(format!("settings-search-add-{}-{index}", path_id(&path)))
                            .label(self.tr("settings.add"))
                            .w(px(80.0))
                            .small()
                            .disabled(read_only || exists || loading)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.add_search_result(
                                    &path_for_add,
                                    target.clone(),
                                    &row,
                                    window,
                                    cx,
                                );
                            })),
                    ),
            );
        }
        section.into_any_element()
    }

    /// 插件管理页。
    fn render_plugins(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut page = v_flex()
            .px(px(24.0))
            .pt(px(16.0))
            .pb(px(16.0))
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(20.0))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(self.tr("settings.thirdPartyPlugins.title")),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("settings-plugin-install")
                            .label(self.tr("settings.thirdPartyPlugins.installFromFile"))
                            .outline()
                            .w(px(132.0))
                            .small()
                            .on_click(
                                cx.listener(|this, _, window, cx| this.pick_plugin(window, cx)),
                            ),
                    )
                    .child(
                        Button::new("settings-plugin-refresh")
                            .label(self.tr("settings.retry"))
                            .outline()
                            .w(px(64.0))
                            .small()
                            .on_click(cx.listener(|this, _, _, cx| this.reload_plugins(cx))),
                    ),
            );

        if let Some((plugin_id, text)) = self.plugin_logs() {
            page = page.child(
                v_flex()
                    .gap_2()
                    .p_2()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().text_size(px(12.0)).child(format!(
                                "{}: {plugin_id}",
                                self.tr("settings.thirdPartyPlugins.logsTitle")
                            )))
                            .child(div().flex_1())
                            .child(
                                Button::new("settings-close-logs")
                                    .label(self.tr("settings.cancel"))
                                    .small()
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.close_plugin_logs(cx)),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .max_h(px(180.0))
                            .overflow_y_scrollbar()
                            .text_size(px(11.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(text.clone()),
                    ),
            );
        }

        if self.plugins().is_empty() {
            page = page.child(
                div()
                    .text_size(px(12.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.tr("settings.thirdPartyPlugins.empty")),
            );
            return page.into_any_element();
        }

        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        let primary = cx.theme().primary;
        let mut table = v_flex()
            .border_1()
            .border_color(border)
            .rounded(px(6.0))
            .overflow_hidden()
            .child(
                h_flex()
                    .h(px(40.0))
                    .px(px(12.0))
                    .gap(px(8.0))
                    .bg(cx.theme().secondary)
                    .border_b_1()
                    .border_color(border)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.0))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(self.tr("settings.thirdPartyPlugins.colName")),
                    )
                    .child(
                        div()
                            .w(px(90.0))
                            .text_size(px(12.0))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(self.tr("settings.thirdPartyPlugins.colVersion")),
                    )
                    .child(
                        div()
                            .w(px(110.0))
                            .text_size(px(12.0))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(self.tr("settings.thirdPartyPlugins.colAuthor")),
                    )
                    .child(
                        div()
                            .w(px(90.0))
                            .text_size(px(12.0))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(self.tr("settings.thirdPartyPlugins.colState")),
                    )
                    .child(
                        div()
                            .w(px(70.0))
                            .text_size(px(12.0))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(self.tr("settings.thirdPartyPlugins.colEnabled")),
                    )
                    .child(div().w(px(300.0))),
            );

        for plugin in self.plugins().to_vec() {
            let id = plugin.plugin_id.clone();
            let is_third_party = plugin.kind == PluginKind::ThirdParty;
            let pending_uninstall = self.pending_uninstall() == Some(id.as_str());
            let name = self.tr(&plugin.name);
            let description = self.tr(&plugin.description);
            let version = if plugin.version.is_empty() {
                String::new()
            } else {
                format!("v{}", plugin.version)
            };
            let author = if is_third_party {
                plugin.author.clone()
            } else {
                self.tr("settings.thirdPartyPlugins.builtin")
            };
            let state_text = if is_third_party {
                plugin_state_label(&plugin.state, self)
            } else {
                self.tr(if plugin.enabled {
                    "settings.enabled"
                } else {
                    "settings.disabled"
                })
            };
            let state_bg = if plugin.enabled {
                cx.theme().sidebar_accent
            } else {
                cx.theme().secondary
            };
            let state_fg = if plugin.enabled { primary } else { muted };
            let mut actions = h_flex().w(px(300.0)).gap(px(4.0));
            if is_third_party {
                actions = actions
                    .child(
                        Button::new(format!("plugin-settings-{id}"))
                            .label(self.tr("settings.thirdPartyPlugins.settings"))
                            .outline()
                            .w(px(68.0))
                            .small()
                            .on_click(cx.listener({
                                let plugin = plugin.clone();
                                move |this, _, window, cx| {
                                    this.open_plugin_settings(plugin.clone(), window, cx)
                                }
                            })),
                    )
                    .child(
                        Button::new(format!("plugin-logs-{id}"))
                            .label(self.tr("settings.thirdPartyPlugins.logs"))
                            .outline()
                            .w(px(60.0))
                            .small()
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, _, cx| this.show_plugin_logs(&id, cx)
                            })),
                    )
                    .child(
                        Button::new(format!("plugin-reload-{id}"))
                            .label(self.tr("settings.thirdPartyPlugins.reload"))
                            .outline()
                            .w(px(60.0))
                            .small()
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, window, cx| {
                                    this.reload_plugin(id.clone(), window, cx)
                                }
                            })),
                    )
                    .child(
                        Button::new(format!("plugin-uninstall-{id}"))
                            .label(if pending_uninstall {
                                self.tr("settings.resetConfirmPositive")
                            } else {
                                self.tr("settings.thirdPartyPlugins.uninstall")
                            })
                            .outline()
                            .w(px(68.0))
                            .small()
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, window, cx| {
                                    this.uninstall_plugin(id.clone(), window, cx)
                                }
                            })),
                    );
            }
            table = table.child(
                h_flex()
                    .min_h(px(56.0))
                    .px(px(12.0))
                    .gap(px(8.0))
                    .border_b_1()
                    .border_color(border)
                    .items_center()
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(13.0))
                                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                                    .child(name),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(11.0))
                                    .text_color(muted)
                                    .child(if description.is_empty() {
                                        id.clone()
                                    } else {
                                        description
                                    }),
                            ),
                    )
                    .child(div().w(px(90.0)).text_size(px(12.0)).child(version))
                    .child(
                        div()
                            .w(px(110.0))
                            .truncate()
                            .text_size(px(12.0))
                            .text_color(muted)
                            .child(author),
                    )
                    .child(
                        div().w(px(90.0)).child(
                            div()
                                .px(px(8.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(state_bg)
                                .text_color(state_fg)
                                .text_size(px(11.0))
                                .child(state_text),
                        ),
                    )
                    .child(
                        div().w(px(70.0)).child(
                            Switch::new(format!("plugin-enabled-{id}"))
                                .checked(plugin.enabled)
                                .on_click(cx.listener({
                                    let plugin = plugin.clone();
                                    move |this, checked, window, cx| {
                                        this.toggle_plugin(plugin.clone(), *checked, window, cx);
                                    }
                                })),
                        ),
                    )
                    .child(actions),
            );
        }
        page = page.child(table);
        page.into_any_element()
    }

    /// 第三方插件脚本设置页。
    fn render_plugin_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(state) = self.plugin_settings() else {
            return div()
                .p_4()
                .text_size(px(12.0))
                .child(self.tr("settings.thirdPartyPlugins.noSettings"))
                .into_any_element();
        };
        let mut header = h_flex()
            .gap_2()
            .items_center()
            .child(
                Button::new("plugin-settings-back")
                    .label(self.tr("settings.cancel"))
                    .small()
                    .on_click(cx.listener(|this, _, _, cx| this.close_plugin_settings(cx))),
            )
            .child(
                div()
                    .text_size(px(20.0))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(state.name.clone()),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(state.plugin_id.clone()),
            );
        if let Some(view) = &state.view {
            return v_flex()
                .size_full()
                .p_4()
                .gap_3()
                .child(header)
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scrollbar()
                        .child(view.view.clone()),
                )
                .into_any_element();
        }
        header =
            header.child(
                div()
                    .text_size(px(12.0))
                    .text_color(cx.theme().danger)
                    .child(state.error.clone().unwrap_or_else(|| {
                        self.tr("settings.thirdPartyPlugins.settingsLoadFailed")
                    })),
            );
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(header)
            .into_any_element()
    }

    /// 插件市场页。
    fn render_market(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut page = v_flex()
            .px(px(24.0))
            .pt(px(16.0))
            .pb(px(16.0))
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(20.0))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(self.tr("settings.pluginMarket.title")),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("settings-market-refresh")
                            .label(self.tr("settings.pluginMarket.refresh"))
                            .outline()
                            .w(px(64.0))
                            .small()
                            .disabled(self.market_loading())
                            .on_click(
                                cx.listener(|this, _, window, cx| this.load_market(window, cx)),
                            ),
                    ),
            );
        if let Some(preview) = self.market_preview() {
            let installed = preview.installed_version.as_deref();
            let content = match installed {
                Some(version) => self
                    .tr("settings.pluginMarket.overwriteConfirmContent")
                    .replace("{name}", &preview.name)
                    .replace("{version}", version),
                None => self.tr("settings.pluginMarket.installRiskContent"),
            };
            let confirm_label = if preview.overwrite {
                self.tr("settings.pluginMarket.overwriteConfirmPositive")
            } else {
                self.tr("settings.pluginMarket.installConfirmPositive")
            };
            page =
                page.child(
                    v_flex()
                        .gap_2()
                        .p_3()
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(cx.theme().warning)
                        .child(
                            div()
                                .text_size(px(13.0))
                                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                .child(self.tr("settings.pluginMarket.installRiskTitle")),
                        )
                        .child(div().text_size(px(12.0)).child(content))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(cx.theme().muted_foreground)
                                .child(format!(
                                    "{} · {} v{} · {}",
                                    preview.repo.full_name,
                                    preview.plugin_id,
                                    preview.version,
                                    match installed {
                                        Some(version) => self
                                            .tr("settings.pluginMarket.installedVersion")
                                            .replace("{version}", version),
                                        None => self.tr("settings.pluginMarket.install"),
                                    }
                                )),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Button::new("market-preview-confirm")
                                        .label(confirm_label)
                                        .small()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.confirm_market_install(window, cx)
                                        })),
                                )
                                .child(
                                    Button::new("market-preview-cancel")
                                        .label(self.tr("settings.cancel"))
                                        .small()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.cancel_market_preview(cx)
                                        })),
                                ),
                        ),
                );
        }
        if self.market_preview_loading() {
            page = page.child(
                div()
                    .text_size(px(12.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.tr("settings.loading")),
            );
        }
        if self.market_loading() {
            page = page.child(
                div()
                    .text_size(px(12.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.tr("settings.loading")),
            );
        }
        if let Some(error) = self.market_error() {
            page = page.child(
                div()
                    .text_size(px(12.0))
                    .text_color(cx.theme().danger)
                    .child(error.to_string()),
            );
        }
        if self.market_repos().is_empty() && !self.market_loading() {
            page = page.child(
                div()
                    .text_size(px(12.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.tr("settings.pluginMarket.empty")),
            );
            return page.into_any_element();
        }
        for repo in self.market_repos().to_vec() {
            let repo_for_install = repo.clone();
            page = page.child(
                v_flex()
                    .gap_1()
                    .p_2()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                                    .child(repo.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(repo.full_name.clone()),
                            )
                            .child(div().flex_1())
                            .child(
                                Button::new(format!("market-install-{}", repo.full_name))
                                    .label(self.tr("settings.pluginMarket.install"))
                                    .small()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.preview_market(
                                            repo_for_install.clone(),
                                            false,
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new(format!("market-open-{}", repo.full_name))
                                    .label(self.tr("settings.pluginMarket.viewOnGithub"))
                                    .outline()
                                    .small()
                                    .on_click(cx.listener({
                                        let url = repo.html_url.clone();
                                        move |this, _, _, _| this.open_url(&url)
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(repo.description.clone()),
                    ),
            );
        }
        page.into_any_element()
    }

    /// 关于页。
    fn render_about(&self, cx: &mut Context<Self>) -> AnyElement {
        let links = [
            (
                "settings.aboutPage.website",
                "https://zerolaunch.ghost-him.com",
            ),
            (
                "settings.aboutPage.github",
                "https://github.com/ghost-him/ZeroLaunch-rs",
            ),
            (
                "settings.aboutPage.gitee",
                "https://gitee.com/ghost-him/ZeroLaunch-rs",
            ),
            (
                "settings.aboutPage.wiki",
                "https://github.com/ghost-him/ZeroLaunch-rs/wiki",
            ),
            (
                "settings.aboutPage.issues",
                "https://github.com/ghost-him/ZeroLaunch-rs/issues",
            ),
        ];
        let mut link_row = h_flex().flex_wrap().gap_2();
        for (label, url) in links {
            link_row = link_row.child(
                Button::new(format!("about-link-{url}"))
                    .label(self.tr(label))
                    .outline()
                    .small()
                    .on_click(cx.listener(move |this, _, _, _| this.open_url(url))),
            );
        }
        v_flex()
            .p_6()
            .gap_3()
            .max_w(px(760.0))
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_size(px(24.0))
                            .font_weight(gpui_kit::FontWeight::BOLD)
                            .child("ZeroLaunch-rs"),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("v{}", self.version())),
                    ),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.tr("settings.aboutPage.tagline")),
            )
            .child(self.render_update_card(cx))
            .child(self.render_log_card(cx))
            .child(
                h_flex()
                    .gap_6()
                    .text_size(px(12.0))
                    .child(format!(
                        "{}: {}",
                        self.tr("settings.aboutPage.stackLabel"),
                        self.tr("settings.aboutPage.stackValue")
                    ))
                    .child(format!(
                        "{}: {}",
                        self.tr("settings.aboutPage.licenseLabel"),
                        self.tr("settings.aboutPage.licenseValue")
                    ))
                    .child(format!(
                        "{}: {}",
                        self.tr("settings.aboutPage.authorLabel"),
                        self.tr("settings.aboutPage.authorValue")
                    )),
            )
            .child(link_row)
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.tr("settings.aboutPage.copyright")),
            )
            .into_any_element()
    }

    /// 关于页的版本更新卡片。
    fn render_update_card(&self, cx: &mut Context<Self>) -> AnyElement {
        let checking = self.update_checking();
        let status = self.update_status();
        let hint = if checking {
            self.tr("settings.aboutPage.checking")
        } else {
            match status {
                Some(status) if status.has_update => {
                    self.tr("settings.aboutPage.updateAvailable").replace(
                        "{version}",
                        status.latest_version.as_deref().unwrap_or_default(),
                    )
                }
                Some(status) if status.error.is_some() => self.tr("settings.aboutPage.checkFailed"),
                Some(status) if status.checked_at.is_some() => self
                    .tr("settings.aboutPage.upToDate")
                    .replace("{version}", &status.current_version),
                _ => String::new(),
            }
        };
        let release_url = status
            .filter(|status| status.has_update)
            .and_then(|status| status.release_url.clone());
        h_flex()
            .p_3()
            .gap_3()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(self.tr("settings.aboutPage.updateTitle")),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(hint),
                    ),
            )
            .child(
                Button::new("about-check-update")
                    .label(self.tr("settings.aboutPage.checkUpdate"))
                    .outline()
                    .w(px(96.0))
                    .small()
                    .disabled(checking)
                    .on_click(cx.listener(|this, _, window, cx| this.check_update(window, cx))),
            )
            .when_some(release_url, |this, url| {
                this.child(
                    Button::new("about-view-release")
                        .label(self.tr("settings.aboutPage.viewRelease"))
                        .outline()
                        .w(px(96.0))
                        .small()
                        .on_click(cx.listener(move |this, _, _, _| this.open_url(&url))),
                )
            })
            .into_any_element()
    }

    /// 关于页的日志导出卡片。
    fn render_log_card(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .p_3()
            .gap_3()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(self.tr("settings.aboutPage.logsTitle")),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(self.tr("settings.aboutPage.exportLogsHint")),
                    ),
            )
            .child(
                Button::new("about-export-logs")
                    .label(self.tr("settings.aboutPage.exportLogs"))
                    .outline()
                    .w(px(120.0))
                    .small()
                    .disabled(self.log_exporting())
                    .on_click(cx.listener(|this, _, window, cx| this.export_logs(window, cx))),
            )
            .into_any_element()
    }

    /// AI 助手页。
    fn render_assistant(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut messages = v_flex()
            .id("assistant-messages")
            .flex_1()
            .min_h_0()
            .gap_2()
            .overflow_y_scroll();
        if self.assistant_messages().is_empty() {
            messages = messages.child(
                div()
                    .text_size(px(13.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.tr("settings.assistant.empty")),
            );
        }
        for (message_index, message) in self.assistant_messages().iter().enumerate() {
            let mut bubble = v_flex()
                .gap_2()
                .p_3()
                .rounded_md()
                .max_w(px(620.0))
                .child(div().text_size(px(13.0)).child(message.text.clone()));
            for (index, reference) in message.references.iter().enumerate() {
                let label = if reference.field_label.is_empty() {
                    reference.component_name.clone()
                } else {
                    format!("{} · {}", reference.component_name, reference.field_label)
                };
                let reference = reference.clone();
                bubble = bubble.child(
                    Button::new(format!(
                        "assistant-ref-{message_index}-{index}-{}",
                        reference.component_id
                    ))
                    .label(label)
                    .small()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.jump_to_reference(&reference, window, cx)
                    })),
                );
            }
            let bubble = if message.user {
                bubble.bg(cx.theme().secondary)
            } else {
                bubble.bg(cx.theme().background)
            };
            messages = messages.child(
                h_flex()
                    .w_full()
                    .when(message.user, |this| this.justify_end())
                    .child(bubble),
            );
        }
        if self.assistant_asking() {
            messages = messages.child(
                div()
                    .text_size(px(12.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.tr("settings.assistant.thinking")),
            );
        }

        v_flex()
            .p_6()
            .gap_3()
            .max_w(px(760.0))
            .child(
                div()
                    .text_size(px(20.0))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(self.tr("settings.assistant.title")),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.tr("settings.assistant.subtitle")),
            )
            .child(messages)
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(self.assistant_input()).w_full()),
                    )
                    .child(
                        Button::new("assistant-send")
                            .label(self.tr("settings.assistant.send"))
                            .primary()
                            .small()
                            .disabled(self.assistant_asking())
                            .on_click(
                                cx.listener(|this, _, window, cx| this.ask_assistant(window, cx)),
                            ),
                    ),
            )
            .into_any_element()
    }

    /// 调试检查器页。
    fn render_debug(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut page = v_flex()
            .px(px(24.0))
            .pt(px(16.0))
            .pb(px(16.0))
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(20.0))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(self.tr("inspector.title")),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("inspector-refresh")
                            .label(self.tr("inspector.refresh"))
                            .outline()
                            .w(px(64.0))
                            .small()
                            .on_click(cx.listener(|this, _, _, cx| this.refresh_inspector(cx))),
                    ),
            );
        if !self.debug_enabled() {
            return page
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(cx.theme().muted_foreground)
                        .child(self.tr("inspector.disabledTag")),
                )
                .into_any_element();
        }
        let Some(inspector) = self.inspector() else {
            return page
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(cx.theme().muted_foreground)
                        .child(self.tr("settings.loading")),
                )
                .into_any_element();
        };
        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        let primary = cx.theme().primary;
        let mut plugins = inspector.registered_plugins.clone();
        plugins.sort_by(|a, b| a.component_id.cmp(&b.component_id));
        let mut plugin_table = v_flex()
            .border_1()
            .border_color(border)
            .rounded(px(6.0))
            .overflow_hidden()
            .child(
                h_flex()
                    .h(px(36.0))
                    .px(px(12.0))
                    .gap(px(8.0))
                    .bg(cx.theme().secondary)
                    .border_b_1()
                    .border_color(border)
                    .text_size(px(12.0))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(div().flex_1().child(self.tr("inspector.colId")))
                    .child(div().w(px(140.0)).child(self.tr("inspector.colType")))
                    .child(div().w(px(80.0)).child(self.tr("inspector.colStatus"))),
            );
        for plugin in &plugins {
            plugin_table = plugin_table.child(
                h_flex()
                    .min_h(px(32.0))
                    .px(px(12.0))
                    .gap(px(8.0))
                    .items_center()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_family("Consolas")
                            .text_size(px(12.0))
                            .child(plugin.component_id.clone()),
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .truncate()
                            .text_size(px(12.0))
                            .text_color(muted)
                            .child(plugin.component_type.clone()),
                    )
                    .child(
                        div().w(px(80.0)).child(
                            div()
                                .px(px(8.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(if plugin.enabled {
                                    cx.theme().sidebar_accent
                                } else {
                                    cx.theme().secondary
                                })
                                .text_color(if plugin.enabled { primary } else { muted })
                                .text_size(px(11.0))
                                .child(if plugin.enabled {
                                    self.tr("inspector.enabled")
                                } else {
                                    self.tr("inspector.disabled")
                                }),
                        ),
                    ),
            );
        }
        page = page
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(
                        self.tr("inspector.pluginsTitle")
                            .replace("{count}", &inspector.registered_plugins.len().to_string()),
                    ),
            )
            .child(plugin_table)
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(
                        self.tr("inspector.queriesTitle")
                            .replace("{count}", &inspector.recent_queries.len().to_string())
                            .replace("{total}", &inspector.total_queries_logged.to_string()),
                    ),
            );
        if inspector.recent_queries.is_empty() {
            page = page.child(
                div()
                    .text_size(px(12.0))
                    .text_color(muted)
                    .child(self.tr("inspector.emptyQueries")),
            );
        } else {
            let mut query_table = v_flex()
                .border_1()
                .border_color(border)
                .rounded(px(6.0))
                .overflow_hidden()
                .child(
                    h_flex()
                        .h(px(36.0))
                        .px(px(12.0))
                        .gap(px(8.0))
                        .bg(cx.theme().secondary)
                        .border_b_1()
                        .border_color(border)
                        .text_size(px(12.0))
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .child(div().w(px(170.0)).child(self.tr("inspector.time")))
                        .child(div().flex_1().child(self.tr("inspector.query")))
                        .child(div().w(px(80.0)).child(self.tr("inspector.durationMs")))
                        .child(div().w(px(70.0)).child(self.tr("inspector.resultCount"))),
                );
            for query in inspector.recent_queries.iter().rev().take(80) {
                query_table = query_table.child(
                    h_flex()
                        .min_h(px(30.0))
                        .px(px(12.0))
                        .gap(px(8.0))
                        .items_center()
                        .border_b_1()
                        .border_color(border)
                        .text_size(px(11.0))
                        .child(
                            div()
                                .w(px(170.0))
                                .text_color(muted)
                                .child(query.timestamp.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .child(query.raw_query.clone()),
                        )
                        .child(
                            div()
                                .w(px(80.0))
                                .text_color(muted)
                                .child(format!("{} ms", query.duration_ms)),
                        )
                        .child(
                            div()
                                .w(px(70.0))
                                .text_color(muted)
                                .child(query.result_count.to_string()),
                        ),
                );
            }
            page = page.child(query_table);
        }
        page.into_any_element()
    }
}

/// 取字符串 schema 的选项。
/// 参数：schema - 字段 schema。
/// 返回：(值, 展示文案) 列表。
fn string_options(schema: &SchemaNode) -> Vec<(String, String)> {
    match &schema.kind {
        SchemaKind::String {
            enum_values,
            enum_labels,
            ..
        } => enum_values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let label = enum_labels
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| value.clone());
                (value.clone(), label)
            })
            .collect(),
        SchemaKind::Array { items, .. } => match &items.kind {
            SchemaKind::String {
                enum_values,
                enum_labels,
                ..
            } => enum_values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let label = enum_labels
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| value.clone());
                    (value.clone(), label)
                })
                .collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// 解析对象数组条目的字段副本。
/// 参数：items_schema - 数组条目 schema。
/// 返回：按 UI 顺序排列的字段。
fn array_object_fields(items_schema: &SchemaNode) -> Vec<OwnedField> {
    let SchemaKind::Object { properties, ui, .. } = &items_schema.kind else {
        return Vec::new();
    };
    schema::object_fields(properties, ui)
        .into_iter()
        .map(|field| OwnedField {
            key: field.key.to_string(),
            schema: field.schema.clone(),
            ui: field.ui.clone(),
        })
        .collect()
}

/// 字段值转文本。
/// 参数：value - JSON 值。
/// 返回：字符串；非字符串按 JSON 文本展示。
fn text_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// 解析 `#RRGGBB` / `RRGGBB` 颜色。
/// 参数：text - 颜色文本。
/// 返回：RGB 数值；非法返回 None。
fn parse_hex_color(text: &str) -> Option<u32> {
    let text = text.trim().trim_start_matches('#');
    if text.len() != 6 {
        return None;
    }
    u32::from_str_radix(text, 16).ok()
}

/// 侧栏分类图标。
/// 参数：category - 分类。
/// 返回：与旧版菜单一致的图标。
fn category_icon(category: Category) -> IconName {
    match category {
        Category::Core => IconName::Settings,
        Category::Appearance => IconName::Palette,
        Category::Models => IconName::Cpu,
        Category::Pipeline => IconName::Search,
        Category::Plugins => IconName::LayoutDashboard,
        Category::Market => IconName::Globe,
        Category::About => IconName::Info,
        Category::Assistant => IconName::Bot,
        Category::Debug => IconName::Inspector,
    }
}

/// 插件运行状态的本地化文案。
/// 参数：state - 运行状态；view - 设置视图。
/// 返回：状态译文。
fn plugin_state_label(
    state: &zerolaunch_plugin_host::manager::PluginRuntimeState,
    view: &SettingsView,
) -> String {
    use zerolaunch_plugin_host::manager::PluginRuntimeState;
    view.tr(match state {
        PluginRuntimeState::Starting => "settings.thirdPartyPlugins.stateStarting",
        PluginRuntimeState::Running => "settings.thirdPartyPlugins.stateRunning",
        PluginRuntimeState::Stopped => "settings.thirdPartyPlugins.stateStopped",
        PluginRuntimeState::Crashed => "settings.thirdPartyPlugins.stateCrashed",
        PluginRuntimeState::Error => "settings.thirdPartyPlugins.stateError",
        PluginRuntimeState::Unknown => "settings.thirdPartyPlugins.stateUnknown",
    })
}
