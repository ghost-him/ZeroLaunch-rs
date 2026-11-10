//! 设置窗口渲染：侧栏、schema 表单、插件/市场/关于/检查器。

use gpui_kit::base::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::select::Select;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    div, img, px, rgb, AnyElement, Context, ImageSource, InteractiveElement as _, IntoElement,
    ParentElement, Render, StatefulInteractiveElement as _, Styled, Window,
};
use serde_json::Value;
use zerolaunch_plugin_api::config::{
    DataActionBinding, FieldAction, FieldUiMetadata, SchemaKind, SchemaNode, WidgetHint,
};

use super::schema::{self, Segment};
use super::view::{path_id, ActionMode, Category, Page, SearchTarget, SettingsView};

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
            .bg(cx.theme().background)
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

    /// 左侧分类/搜索栏。
    fn render_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w(px(236.0))
            .flex_shrink_0()
            .h_full()
            .border_r_1()
            .border_color(cx.theme().border)
            .p_3()
            .gap_2()
            .child(
                div()
                    .h(px(32.0))
                    .child(Input::new(self.search_input()).w_full()),
            )
            .child(
                div()
                    .id("settings-sidebar-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(if self.search_query().trim().is_empty() {
                        self.render_categories(cx)
                    } else {
                        self.render_search_hits(cx)
                    }),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("v{}", self.version())),
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
            Category::About,
        ] {
            list = list.child(self.render_category_item(category, cx));
        }
        if self.debug_enabled() {
            list = list.child(self.render_category_item(Category::Debug, cx));
        }
        list.into_any_element()
    }

    /// 单个分类按钮。
    fn render_category_item(&self, category: Category, cx: &mut Context<Self>) -> AnyElement {
        let selected = self.category() == category;
        let label = self.category_label(category);
        div()
            .id(("settings-category", category as u8 as usize))
            .h(px(32.0))
            .px_2()
            .rounded(px(6.0))
            .flex()
            .items_center()
            .text_size(px(13.0))
            .when(selected, |this| {
                this.bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
            .when(!selected, |this| {
                this.text_color(cx.theme().foreground)
                    .hover(|style| style.bg(cx.theme().button_hover))
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                this.select_category(category, window, cx);
            }))
            .child(label)
            .into_any_element()
    }

    /// 设置项搜索结果。
    fn render_search_hits(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.search_hits().is_empty() {
            return div()
                .p_2()
                .text_size(px(12.0))
                .text_color(cx.theme().muted_foreground)
                .child(self.tr("settings.searchNoResults"))
                .into_any_element();
        }
        let mut list = v_flex().gap_1();
        for hit in self.search_hits() {
            let hit = hit.clone();
            let click_hit = hit.clone();
            list = list.child(
                div()
                    .id(format!("settings-hit-{}-{:?}", hit.component_id, hit.field))
                    .px_2()
                    .py_1()
                    .rounded(px(5.0))
                    .hover(|style| style.bg(cx.theme().button_hover))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select_search_hit(&click_hit, window, cx);
                    }))
                    .child(div().text_size(px(12.0)).child(hit.label.clone()))
                    .when(!hit.detail.is_empty(), |this| {
                        this.child(
                            div()
                                .text_size(px(11.0))
                                .text_color(cx.theme().muted_foreground)
                                .truncate()
                                .child(hit.detail.clone()),
                        )
                    }),
            );
        }
        list.into_any_element()
    }

    /// 右侧内容区。
    fn render_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .id("settings-page-scroll")
            .flex_1()
            .min_w_0()
            .min_h_0()
            .h_full()
            .overflow_y_scrollbar()
            .child(match self.page() {
                Page::Component => self.render_component_page(window, cx),
                Page::Plugins => self.render_plugins(cx),
                Page::Market => self.render_market(cx),
                Page::About => self.render_about(cx),
                Page::Debug => self.render_debug(cx),
            })
            .into_any_element()
    }

    /// 组件设置页。
    fn render_component_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
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
            .p_4()
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
                    .child(
                        Switch::new("settings-component-enabled")
                            .checked(component.enabled)
                            .on_click(cx.listener(|this, checked, _, cx| {
                                if *checked
                                    != this.component_info().is_some_and(|info| info.enabled)
                                {
                                    this.toggle_enabled(cx);
                                }
                            })),
                    ),
            )
            .when(!tabs.is_empty(), |this| {
                this.child(
                    h_flex()
                        .flex_wrap()
                        .gap_1()
                        .children(tabs.into_iter().map(|tab| {
                            let selected =
                                self.selected_component() == Some(tab.component_id.as_str());
                            let id = tab.component_id.clone();
                            let tab_label = self.tr(&tab.component_name);
                            div()
                                .id(format!("settings-tab-{}", tab.component_id))
                                .px_2()
                                .py_1()
                                .rounded(px(5.0))
                                .text_size(px(12.0))
                                .when(selected, |this| {
                                    this.bg(cx.theme().accent)
                                        .text_color(cx.theme().accent_foreground)
                                })
                                .when(!selected, |this| {
                                    this.hover(|style| style.bg(cx.theme().button_hover))
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
                    .text_size(px(12.0))
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

        let mut form = v_flex().gap_3();
        if groups.is_empty() {
            return div()
                .p_4()
                .text_color(cx.theme().muted_foreground)
                .child(self.tr("settings.noConfigurableFields"))
                .into_any_element();
        }
        for (group, fields) in groups {
            if !group.is_empty() {
                form = form.child(
                    div()
                        .pt_2()
                        .text_size(px(13.0))
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .child(self.tr(&group)),
                );
            }
            form = form.child(self.render_object_fields(fields, &[], window, cx));
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
        let mut list = v_flex().gap_3();
        for field in fields {
            if !self.field_visible(&field.ui, scope) {
                continue;
            }
            let mut path = scope.to_vec();
            path.push(Segment::Key(field.key.clone()));
            list = list.child(self.render_field(&field, &path, scope, window, cx));
        }
        list.into_any_element()
    }

    /// 渲染单个字段。
    fn render_field(
        &mut self,
        field: &OwnedField,
        path: &[Segment],
        scope: &[Segment],
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
        body = body.child(
            h_flex().gap_2().items_center().child(
                div()
                    .text_size(px(13.0))
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .child(label),
            ),
        );

        if !description.is_empty() {
            body = body.child(
                div()
                    .text_size(px(11.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(description),
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
        if let Some(note) = note {
            body = body.child(
                div()
                    .text_size(px(11.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(note),
            );
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
        let search_table = matches!(
            schema::widget_of(&field.schema, &field.ui),
            WidgetHint::SearchTable
        );
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
        let mut page = v_flex().p_4().gap_3().child(
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
                        .small()
                        .on_click(cx.listener(|this, _, window, cx| this.pick_plugin(window, cx))),
                )
                .child(
                    Button::new("settings-plugin-refresh")
                        .label(self.tr("settings.retry"))
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

        for plugin in self.plugins().to_vec() {
            let id = plugin.plugin_id.clone();
            let is_third_party =
                plugin.kind == zerolaunch_plugin_api::plugin::PluginKind::ThirdParty;
            let pending_uninstall = self.pending_uninstall() == Some(id.as_str());
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
                                    .child(plugin.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("v{} · {}", plugin.version, plugin.author)),
                            )
                            .child(div().flex_1())
                            .child(
                                Switch::new(format!("plugin-enabled-{id}"))
                                    .checked(plugin.enabled)
                                    .on_click(cx.listener({
                                        let plugin = plugin.clone();
                                        move |this, checked, window, cx| {
                                            this.toggle_plugin(
                                                plugin.clone(),
                                                *checked,
                                                window,
                                                cx,
                                            );
                                        }
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(plugin.description.clone()),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .text_size(px(11.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(plugin_state_label(&plugin.state, self))
                            .child(format!(
                                "{}: {}",
                                self.tr("settings.thirdPartyPlugins.colEnabled"),
                                if plugin.enabled {
                                    self.tr("settings.enabled")
                                } else {
                                    self.tr("settings.disabled")
                                }
                            ))
                            .child(div().flex_1())
                            .when(is_third_party, |this| {
                                this.child(
                                    Button::new(format!("plugin-logs-{id}"))
                                        .label(self.tr("settings.thirdPartyPlugins.logs"))
                                        .small()
                                        .on_click(cx.listener({
                                            let id = id.clone();
                                            move |this, _, _, cx| this.show_plugin_logs(&id, cx)
                                        })),
                                )
                                .child(
                                    Button::new(format!("plugin-reload-{id}"))
                                        .label(self.tr("settings.thirdPartyPlugins.reload"))
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
                                        .small()
                                        .on_click(cx.listener({
                                            let id = id.clone();
                                            move |this, _, window, cx| {
                                                this.uninstall_plugin(id.clone(), window, cx)
                                            }
                                        })),
                                )
                            }),
                    ),
            );
        }
        page.into_any_element()
    }

    /// 插件市场页。
    fn render_market(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut page = v_flex().p_4().gap_3().child(
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
                        .small()
                        .disabled(self.market_loading())
                        .on_click(cx.listener(|this, _, window, cx| this.load_market(window, cx))),
                ),
        );
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
            let repo_for_overwrite = repo.clone();
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
                                        this.install_market(
                                            repo_for_install.clone(),
                                            false,
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new(format!("market-overwrite-{}", repo.full_name))
                                    .label(self.tr("settings.pluginMarket.update"))
                                    .small()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.install_market(
                                            repo_for_overwrite.clone(),
                                            true,
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new(format!("market-open-{}", repo.full_name))
                                    .label(self.tr("settings.pluginMarket.viewOnGithub"))
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

    /// 调试检查器页。
    fn render_debug(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut page = v_flex().p_4().gap_3().child(
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
            .child(
                v_flex()
                    .gap_1()
                    .children(inspector.registered_plugins.iter().map(|plugin| {
                        h_flex()
                            .gap_2()
                            .text_size(px(11.0))
                            .child(plugin.component_id.clone())
                            .child(plugin.component_type.clone())
                            .child(if plugin.enabled {
                                self.tr("inspector.enabled")
                            } else {
                                self.tr("inspector.disabled")
                            })
                            .into_any_element()
                    })),
            )
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
                    .text_color(cx.theme().muted_foreground)
                    .child(self.tr("inspector.emptyQueries")),
            );
        }
        for query in inspector.recent_queries.iter().rev().take(80) {
            page = page.child(
                h_flex()
                    .gap_2()
                    .text_size(px(11.0))
                    .child(query.timestamp.clone())
                    .child(query.raw_query.clone())
                    .child(format!("{} ms", query.duration_ms))
                    .child(format!("{}", query.result_count)),
            );
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
