//! Schema 构建器 — 链式调用 API。
//!
//! 提供统一的 `SchemaBuilder` 来构建 `SettingDefinition`，
//! 覆盖所有 SchemaKind + WidgetHint 组合。
//!
//! # 使用示例
//!
//! ```ignore
//! // 简单文本字段
//! SchemaBuilder::text("key", "Label", "Description")
//!     .group("Group").order(0).default("value").build()
//!
//! // 带约束的数值字段
//! SchemaBuilder::number("height", "Height", "Height in px")
//!     .group("Layout").order(1).default(72.0)
//!     .min(40.0).max(120.0).step(1.0).build()
//!
//! // Array + Object + action
//! SchemaBuilder::array("sources", "Sources", "Browser sources")
//!     .group("Sources").order(2)
//!     .data_action(DataActionBinding { ... })
//!     .object_items(vec![
//!         SchemaBuilder::text("name", "Name", "Name").default("").build(),
//!     ])
//!     .table_ui().default(serde_json::json!([])).build()
//! ```

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use zerolaunch_plugin_api::config::{
    DataActionBinding, EffectActionBinding, FieldAction, FieldUiMetadata, PathMode, PrimitiveType,
    SchemaKind, SchemaNode, SettingDefinition, WidgetHint,
};

/// Schema 构建器 —— 链式构建 `SettingDefinition`。
///
/// 每个构造器方法（`text()`、`number()`、`boolean()` 等）创建对应的 schema 类型，
/// 后续可通过链式方法添加约束、UI 提示和 action 绑定。
/// 最终通过 `build()` 输出 `SettingDefinition`。
pub struct SchemaBuilder {
    key: String,
    schema: SchemaNode,
    ui: FieldUiMetadata,
}

impl SchemaBuilder {
    // ── constructors ──────────────────────────────────────────────

    /// 创建字符串类型字段。
    pub fn text(key: &str, label: &str, desc: &str) -> Self {
        Self::new(key, label, desc, SchemaNode::string(), WidgetHint::Text)
    }

    /// 创建浮点数类型字段。
    pub fn number(key: &str, label: &str, desc: &str) -> Self {
        Self::new(key, label, desc, SchemaNode::number(), WidgetHint::Number)
    }

    /// 创建整数类型字段。
    pub fn integer(key: &str, label: &str, desc: &str) -> Self {
        Self::new(key, label, desc, SchemaNode::integer(), WidgetHint::Number)
    }

    /// 创建布尔类型字段。
    pub fn boolean(key: &str, label: &str, desc: &str) -> Self {
        Self::new(key, label, desc, SchemaNode::boolean(), WidgetHint::Toggle)
    }

    /// 创建下拉选择字段（string + Select widget）。
    pub fn select(key: &str, label: &str, desc: &str) -> Self {
        Self::new(key, label, desc, SchemaNode::string(), WidgetHint::Select)
    }

    /// 创建颜色选择字段（string + Color widget）。
    pub fn color(key: &str, label: &str, desc: &str) -> Self {
        Self::new(key, label, desc, SchemaNode::string(), WidgetHint::Color)
    }

    /// 创建路径选择字段（string + Path widget）。
    pub fn path(key: &str, label: &str, desc: &str) -> Self {
        Self::new(
            key,
            label,
            desc,
            SchemaNode::string(),
            WidgetHint::Path {
                mode: PathMode::File,
            },
        )
    }

    /// 创建数组类型字段。
    pub fn array(key: &str, label: &str, desc: &str) -> Self {
        Self::new(
            key,
            label,
            desc,
            SchemaNode {
                kind: SchemaKind::Array {
                    items: Box::new(SchemaNode::string()),
                    item_widget: None,
                    min_items: None,
                    max_items: None,
                },
                default: None,
            },
            WidgetHint::List,
        )
    }

    /// 创建图片选择字段（string + Image widget）。
    pub fn image(key: &str, label: &str, desc: &str) -> Self {
        Self::new(
            key,
            label,
            desc,
            SchemaNode::string(),
            WidgetHint::Image {
                accept: vec![
                    "png".into(),
                    "jpg".into(),
                    "jpeg".into(),
                    "webp".into(),
                    "ico".into(),
                ],
                max_size: Some(2 * 1024 * 1024),
            },
        )
    }

    /// 创建字体选择字段（string + Font widget）。
    ///
    /// 前端通过声明的 config action（默认 `list_fonts`）拉取系统字体列表，
    /// 用户从列表中直接选择。字段值持久化为字体族名称，空串表示跟随系统。
    pub fn font(key: &str, label: &str, desc: &str) -> Self {
        Self::new(
            key,
            label,
            desc,
            SchemaNode::string(),
            WidgetHint::Font {
                action: "list_fonts".to_string(),
                component: None,
            },
        )
    }

    /// 创建快捷键录制字段（string + Hotkey widget）。
    ///
    /// 前端聚焦后进入录制态，用户按下组合键（修饰键 + 主键，如 "Alt+Space"）
    /// 即完成录入；空串表示未设置快捷键。
    pub fn hotkey(key: &str, label: &str, desc: &str) -> Self {
        Self::new(key, label, desc, SchemaNode::string(), WidgetHint::Hotkey)
    }

    fn new(key: &str, label: &str, desc: &str, schema: SchemaNode, widget: WidgetHint) -> Self {
        Self {
            key: key.to_string(),
            schema,
            ui: FieldUiMetadata {
                pointer: format!("/{}", key.replace('~', "~0").replace('/', "~1")),
                label: label.to_string(),
                description: desc.to_string(),
                group: None,
                order: 0,
                visible: true,
                read_only: false,
                widget: Some(widget),
                action: None,
                detail_action: None,
            },
        }
    }

    // ── universal methods ─────────────────────────────────────────

    /// 设置分组名称。相同 group 的字段在前端渲染在一起。
    pub fn group(mut self, group: &str) -> Self {
        self.ui.group = Some(group.to_string());
        self
    }

    /// 设置组内排序序号（越小越靠前）。
    pub fn order(mut self, order: u32) -> Self {
        self.ui.order = order;
        self
    }

    /// 设置默认值。
    pub fn default(mut self, value: impl Into<Value>) -> Self {
        self.schema.default = Some(value.into());
        self
    }

    /// 设置字段可见性。
    pub fn visible(mut self, visible: bool) -> Self {
        self.ui.visible = visible;
        self
    }

    /// 设置字段可否编辑。
    pub fn editable(mut self, editable: bool) -> Self {
        self.ui.read_only = !editable;
        self
    }

    /// 绑定数据注入 action，前端据此渲染搜索/检测按钮并填充字段。
    pub fn data_action(mut self, action: DataActionBinding) -> Self {
        self.ui.action = Some(FieldAction::Data(action));
        self
    }

    /// 绑定用户显式触发的副作用 action，不参与配置保存。
    pub fn effect_action(mut self, action: EffectActionBinding) -> Self {
        self.ui.action = Some(FieldAction::Effect(action));
        self
    }

    // ── Number / Integer ──────────────────────────────────────────

    /// 设置数值最小值。
    pub fn min(mut self, value: f64) -> Self {
        match &mut self.schema.kind {
            SchemaKind::Number { minimum, .. } => *minimum = Some(value),
            SchemaKind::Integer { minimum, .. } => *minimum = Some(value as i64),
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::min() 只能在数字或整数类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数字类型",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::min() 只能在数字或整数类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数字类型",
                    self.key
                );
                return self;
            }
        }
        self
    }

    /// 设置数值最大值。
    pub fn max(mut self, value: f64) -> Self {
        match &mut self.schema.kind {
            SchemaKind::Number { maximum, .. } => *maximum = Some(value),
            SchemaKind::Integer { maximum, .. } => *maximum = Some(value as i64),
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::max() 只能在数字或整数类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数字类型",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::max() 只能在数字或整数类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数字类型",
                    self.key
                );
                return self;
            }
        }
        self
    }

    /// 设置数值步长。
    pub fn step(mut self, value: f64) -> Self {
        match &mut self.schema.kind {
            SchemaKind::Number { multiple_of, .. } => *multiple_of = Some(value),
            SchemaKind::Integer { multiple_of, .. } => *multiple_of = Some(value as i64),
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::step() 只能在数字或整数类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数字类型",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::step() 只能在数字或整数类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数字类型",
                    self.key
                );
                return self;
            }
        }
        self
    }

    // ── Select ────────────────────────────────────────────────────
    /// 设置下拉选项（值 + 标签）。
    /// 前端展示时优先使用标签，value 用于设置持久化值。
    pub fn options_with_labels(mut self, items: &[(&str, &str)]) -> Self {
        match &mut self.schema.kind {
            SchemaKind::String {
                enum_values,
                enum_labels,
                ..
            } => {
                enum_values.clear();
                enum_labels.clear();
                for (value, label) in items {
                    enum_values.push(value.to_string());
                    enum_labels.push(label.to_string());
                }
            }
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::options_with_labels() 只能在字符串类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是字符串类型",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::options_with_labels() 只能在字符串类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是字符串类型",
                    self.key
                );
                return self;
            }
        }
        self
    }

    /// 设置下拉选项（写入 schema 的 enum 约束）。
    pub fn options(mut self, options: &[&str]) -> Self {
        match &mut self.schema.kind {
            SchemaKind::String {
                enum_values,
                enum_labels,
                ..
            } => {
                *enum_values = options.iter().map(|s| s.to_string()).collect();
                enum_labels.clear();
            }
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::options() 只能在字符串类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是字符串类型",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::options() 只能在字符串类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是字符串类型",
                    self.key
                );
                return self;
            }
        }
        self
    }
    // ── Path ──────────────────────────────────────────────────────

    /// 切换为文件选择模式。
    pub fn file(mut self) -> Self {
        match &mut self.ui.widget {
            Some(WidgetHint::Path { mode }) => *mode = PathMode::File,
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::file() 只能在路径选择器上调用。\
                     字段 '{}' 的控件为 {other:?}，请使用 path() 构造器或先调用 path_ui()",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::file() 只能在路径选择器上调用。\
                     字段 '{}' 的控件为 {other:?}，请使用 path() 构造器或先调用 path_ui()",
                    self.key
                );
                return self;
            }
        }
        self
    }

    /// 切换为目录选择模式。
    pub fn directory(mut self) -> Self {
        match &mut self.ui.widget {
            Some(WidgetHint::Path { mode }) => *mode = PathMode::Directory,
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::directory() 只能在路径选择器上调用。\
                     字段 '{}' 的控件为 {other:?}，请使用 path() 构造器或先调用 path_ui()",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::directory() 只能在路径选择器上调用。\
                     字段 '{}' 的控件为 {other:?}，请使用 path() 构造器或先调用 path_ui()",
                    self.key
                );
                return self;
            }
        }
        self
    }

    // ── Font ─────────────────────────────────────────────────────

    /// 指定列出系统字体的 config action 名称（默认 `list_fonts`）。
    pub fn font_action(mut self, action: &str) -> Self {
        match &mut self.ui.widget {
            Some(WidgetHint::Font { action: target, .. }) => *target = action.to_string(),
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::font_action() 只能在字体选择器上调用。\
                     字段 '{}' 的控件为 {other:?}，请使用 font() 构造器",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::font_action() 只能在字体选择器上调用。\
                     字段 '{}' 的控件为 {other:?}，请使用 font() 构造器",
                    self.key
                );
                return self;
            }
        }
        self
    }

    /// 指定提供字体的 config 组件 id（默认使用拥有该字段的组件自身）。
    pub fn font_component(mut self, component: &str) -> Self {
        match &mut self.ui.widget {
            Some(WidgetHint::Font {
                component: target, ..
            }) => {
                *target = Some(component.to_string());
            }
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::font_component() 只能在字体选择器上调用。\
                     字段 '{}' 的控件为 {other:?}，请使用 font() 构造器",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::font_component() 只能在字体选择器上调用。\
                     字段 '{}' 的控件为 {other:?}，请使用 font() 构造器",
                    self.key
                );
                return self;
            }
        }
        self
    }

    // ── Array ─────────────────────────────────────────────────────

    /// 设置数组元素为原始类型（如 `PrimitiveType::Text`）。
    pub fn primitive_item(mut self, item: PrimitiveType) -> Self {
        match &mut self.schema.kind {
            SchemaKind::Array {
                items, item_widget, ..
            } => {
                let (schema, widget) = primitive_schema(item);
                **items = schema;
                *item_widget = Some(widget);
            }
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::primitive_item() 只能在数组类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数组类型",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::primitive_item() 只能在数组类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数组类型",
                    self.key
                );
                return self;
            }
        }
        self
    }

    /// 设置数组元素为对象类型，由 `SettingDefinition` 列表定义字段。
    pub fn object_items(mut self, fields: Vec<SettingDefinition>) -> Self {
        match &mut self.schema.kind {
            SchemaKind::Array {
                items, item_widget, ..
            } => {
                let mut properties = BTreeMap::new();
                let mut ui = Vec::with_capacity(fields.len());
                for field in fields {
                    ui.push(field.ui);
                    properties.insert(field.key, field.schema);
                }
                **items = SchemaNode {
                    kind: SchemaKind::Object {
                        properties,
                        ui,
                        required: BTreeSet::new(),
                    },
                    default: None,
                };
                *item_widget = None;
            }
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::object_items() 只能在数组类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数组类型",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::object_items() 只能在数组类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数组类型",
                    self.key
                );
                return self;
            }
        }
        self
    }

    /// 设置最小元素数量。
    pub fn min_items(mut self, n: usize) -> Self {
        match &mut self.schema.kind {
            SchemaKind::Array { min_items, .. } => *min_items = Some(n),
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::min_items() 只能在数组类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数组类型",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::min_items() 只能在数组类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数组类型",
                    self.key
                );
                return self;
            }
        }
        self
    }

    /// 设置最大元素数量。
    pub fn max_items(mut self, n: usize) -> Self {
        match &mut self.schema.kind {
            SchemaKind::Array { max_items, .. } => *max_items = Some(n),
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::max_items() 只能在数组类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数组类型",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::max_items() 只能在数组类型的字段上调用。\
                     字段 '{}' 的类型为 {other:?}，不是数组类型",
                    self.key
                );
                return self;
            }
        }
        self
    }

    // ── Widget hints ──────────────────────────────────────────────

    /// 使用默认列表 UI。
    pub fn default_ui(mut self) -> Self {
        self.ui.widget = Some(WidgetHint::List);
        self
    }

    /// 使用表格 UI。
    pub fn table_ui(mut self) -> Self {
        self.ui.widget = Some(WidgetHint::Table);
        self
    }

    /// 使用标签式 UI。
    pub fn tags_ui(mut self) -> Self {
        self.ui.widget = Some(WidgetHint::Tags);
        self
    }

    /// 使用卡片式 UI。
    pub fn cards_ui(mut self) -> Self {
        self.ui.widget = Some(WidgetHint::Cards);
        self
    }

    /// 使用搜索弹窗表格 UI。
    pub fn search_table_ui(mut self) -> Self {
        self.ui.widget = Some(WidgetHint::SearchTable);
        self
    }

    /// 使用主从详情面板 UI。
    pub fn master_detail_ui(mut self) -> Self {
        self.ui.widget = Some(WidgetHint::MasterDetail);
        self
    }

    /// 设置 MasterDetail 详情面板联动动作。
    /// 选中列表项时，前端调用指定的 config_action 获取预览数据，
    /// 用户编辑结果写入 `detail_action.targetField` 指定的兄弟设置字段。
    pub fn detail_action(mut self, def: zerolaunch_plugin_api::config::DetailActionDef) -> Self {
        self.ui.detail_action = Some(def);
        self
    }

    // ── Image ─────────────────────────────────────────────────────

    /// 设置允许的文件格式。
    pub fn accept(mut self, formats: &[&str]) -> Self {
        match &mut self.ui.widget {
            Some(WidgetHint::Image { accept, .. }) => {
                *accept = formats.iter().map(|s| s.to_string()).collect();
            }
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::accept() 只能在图片选择器上调用。\
                     字段 '{}' 的控件为 {other:?}，不是图片选择器",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::accept() 只能在图片选择器上调用。\
                     字段 '{}' 的控件为 {other:?}，不是图片选择器",
                    self.key
                );
                return self;
            }
        }
        self
    }

    /// 设置最大文件大小（字节）。
    pub fn max_image_size(mut self, bytes: u64) -> Self {
        match &mut self.ui.widget {
            Some(WidgetHint::Image { max_size, .. }) => *max_size = Some(bytes),
            other => {
                debug_assert!(
                    false,
                    "SchemaBuilder::max_image_size() 只能在图片选择器上调用。\
                     字段 '{}' 的控件为 {other:?}，不是图片选择器",
                    self.key
                );
                tracing::warn!(
                    "SchemaBuilder::max_image_size() 只能在图片选择器上调用。\
                     字段 '{}' 的控件为 {other:?}，不是图片选择器",
                    self.key
                );
                return self;
            }
        }
        self
    }
    // ── build ─────────────────────────────────────────────────────

    /// 构建为 `SettingDefinition`。构建前校验字段完整性，不匹配时 panic。
    pub fn build(self) -> SettingDefinition {
        // detail_action 只能在 masterDetail widget 上使用
        if self.ui.detail_action.is_some() {
            let is_master_detail = matches!(self.ui.widget, Some(WidgetHint::MasterDetail));
            if !is_master_detail {
                debug_assert!(
                    false,
                    "字段 '{}' 设置了 detail_action，但控件不是 masterDetail（当前控件为 {:?}）。\
                     detail_action 仅适用于 masterDetail 数组字段",
                    self.key, self.ui.widget
                );
                tracing::warn!(
                    "字段 '{}' 设置了 detail_action，但控件不是 masterDetail（当前控件为 {:?}）。\
                     detail_action 仅适用于 masterDetail 数组字段",
                    self.key,
                    self.ui.widget
                );
            }
        }
        SettingDefinition {
            key: self.key,
            schema: self.schema,
            ui: self.ui,
        }
    }

    /// 构建为 `SettingDefinition`（仅用于 object_items 内部，与 build() 行为一致）。
    pub fn build_field(self) -> SettingDefinition {
        self.build()
    }
}
/// 将 PrimitiveType 转换为数组 item schema 及对应 itemWidget。
fn primitive_schema(item: PrimitiveType) -> (SchemaNode, WidgetHint) {
    match item {
        PrimitiveType::Text => (SchemaNode::string(), WidgetHint::Text),
        PrimitiveType::Path { mode } => (SchemaNode::string(), WidgetHint::Path { mode }),
        PrimitiveType::Color => (SchemaNode::string(), WidgetHint::Color),
        PrimitiveType::Boolean => (SchemaNode::boolean(), WidgetHint::Toggle),
        PrimitiveType::Integer { min, max, step } => (
            SchemaNode {
                kind: SchemaKind::Integer {
                    minimum: min,
                    maximum: max,
                    multiple_of: step,
                },
                default: None,
            },
            WidgetHint::Number,
        ),
        PrimitiveType::Select { options } => (
            SchemaNode {
                kind: SchemaKind::String {
                    enum_values: options,
                    enum_labels: Vec::new(),
                    min_length: None,
                    max_length: None,
                    pattern: None,
                },
                default: None,
            },
            WidgetHint::Select,
        ),
        PrimitiveType::Number { min, max, step } => (
            SchemaNode {
                kind: SchemaKind::Number {
                    minimum: min,
                    maximum: max,
                    multiple_of: step,
                },
                default: None,
            },
            WidgetHint::Number,
        ),
    }
}
