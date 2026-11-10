//! 设置表单的 schema 路径与默认值工具。
//!
//! Schema 仍是唯一数据契约：本模块只负责在 JSON 值上定位字段、计算嵌套默认值，
//! 不复制后端的校验与持久化规则。

use std::collections::BTreeMap;

use serde_json::{json, Value};
use zerolaunch_plugin_api::config::{
    FieldAction, FieldUiMetadata, SchemaKind, SchemaNode, SettingsContribution, WidgetHint,
};

/// 设置值中的一级路径：对象键或数组下标。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Segment {
    /// 对象属性。
    Key(String),
    /// 数组元素。
    Index(usize),
}

/// 从根设置对象到某个字段的路径。
pub type FieldPath = Vec<Segment>;

/// 对象字段的 schema、UI 元数据与键。
pub struct FieldRef<'a> {
    /// 属性键。
    pub key: &'a str,
    /// 数据 schema。
    pub schema: &'a SchemaNode,
    /// 展示元数据。
    pub ui: &'a FieldUiMetadata,
}

/// 由 JSON Pointer 取最终键名。
/// 参数：pointer - `/key` 或 `/parent/key` 形式的指针。
/// 返回：对应的对象键；设置键只允许字母、数字、`_`、`-`、`.`，不含指针转义字符。
pub fn key_from_pointer(pointer: &str) -> &str {
    pointer.rsplit('/').next().unwrap_or(pointer)
}

/// 按 UI 顺序枚举对象字段。
/// 参数：properties - 对象属性表；ui - 字段元数据。
/// 返回：按 order、key 排序的字段引用。
pub fn object_fields<'a>(
    properties: &'a BTreeMap<String, SchemaNode>,
    ui: &'a [FieldUiMetadata],
) -> Vec<FieldRef<'a>> {
    let mut fields: Vec<FieldRef<'a>> = ui
        .iter()
        .filter_map(|meta| {
            let key = key_from_pointer(&meta.pointer);
            properties.get(key).map(|schema| FieldRef {
                key,
                schema,
                ui: meta,
            })
        })
        .collect();
    fields.sort_by(|a, b| a.ui.order.cmp(&b.ui.order).then_with(|| a.key.cmp(b.key)));
    fields
}

/// 取设置根对象中的字段值。
/// 参数：root - 根 JSON；path - 字段路径。
/// 返回：命中时返回引用。
pub fn value_at<'a>(root: &'a Value, path: &[Segment]) -> Option<&'a Value> {
    let mut current = root;
    for segment in path {
        current = match segment {
            Segment::Key(key) => current.get(key)?,
            Segment::Index(index) => current.get(*index)?,
        };
    }
    Some(current)
}

/// 写入设置根对象中的字段值，缺失的容器按需创建。
/// 参数：root - 根 JSON；path - 字段路径；value - 新值。
/// 返回：无；路径为空时直接替换根。
pub fn set_value(root: &mut Value, path: &[Segment], value: Value) {
    if path.is_empty() {
        *root = value;
        return;
    }

    let mut current = root;
    for (position, segment) in path.iter().enumerate() {
        let last = position + 1 == path.len();
        match segment {
            Segment::Key(key) => {
                if !current.is_object() {
                    *current = json!({});
                }
                let map = current
                    .as_object_mut()
                    .expect("path parent was normalized to object");
                if last {
                    map.insert(key.clone(), value);
                    return;
                }
                current = map.entry(key.clone()).or_insert(Value::Null);
            }
            Segment::Index(index) => {
                if !current.is_array() {
                    *current = json!([]);
                }
                let array = current
                    .as_array_mut()
                    .expect("path parent was normalized to array");
                while array.len() <= *index {
                    array.push(Value::Null);
                }
                if last {
                    array[*index] = value;
                    return;
                }
                current = &mut array[*index];
            }
        }
    }
}

/// 计算 schema 的默认值。
/// 参数：schema - 字段 schema。
/// 返回：显式 default 优先，否则按类型给出空结构。
pub fn default_value(schema: &SchemaNode) -> Value {
    if let Some(default) = &schema.default {
        return default.clone();
    }
    match &schema.kind {
        SchemaKind::String { .. } => json!(""),
        SchemaKind::Number { .. } => json!(0.0),
        SchemaKind::Integer { .. } => json!(0),
        SchemaKind::Boolean => json!(false),
        SchemaKind::Array { .. } => json!([]),
        SchemaKind::Object { properties, .. } => {
            let values = properties
                .iter()
                .map(|(key, node)| (key.clone(), default_value(node)))
                .collect::<serde_json::Map<_, _>>();
            Value::Object(values)
        }
    }
}

/// 判断字段在当前作用域是否可见。
/// 参数：ui - 字段元数据；root - 根设置；scope - 字段所属对象/数组项的路径。
/// 返回：visible 与 visibleWhen 同时满足时为 true。
pub fn is_visible(ui: &FieldUiMetadata, root: &Value, scope: &[Segment]) -> bool {
    if !ui.visible {
        return false;
    }
    let Some(condition) = &ui.visible_when else {
        return true;
    };
    let mut sibling = scope.to_vec();
    sibling.push(Segment::Key(condition.field.clone()));
    value_at(root, &sibling) == Some(&condition.value)
}

/// 取字段控件提示；无提示时按 schema 类型给出默认控件。
/// 参数：schema - 字段 schema；ui - 字段元数据。
/// 返回：渲染分派使用的控件。
pub fn widget_of(schema: &SchemaNode, ui: &FieldUiMetadata) -> WidgetHint {
    if let Some(widget) = &ui.widget {
        return widget.clone();
    }
    match &schema.kind {
        SchemaKind::Boolean => WidgetHint::Toggle,
        SchemaKind::Number { .. } | SchemaKind::Integer { .. } => WidgetHint::Number,
        SchemaKind::Array { .. } => WidgetHint::List,
        SchemaKind::String { enum_values, .. } if !enum_values.is_empty() => WidgetHint::Select,
        SchemaKind::String { .. } => WidgetHint::Text,
        SchemaKind::Object { .. } => WidgetHint::Cards,
    }
}

/// 取 contribution 的提交策略。
/// 参数：contribution - 组件 schema。
/// 返回：是否为即时保存。
pub fn is_immediate(contribution: &SettingsContribution) -> bool {
    matches!(
        contribution.commit_policy,
        zerolaunch_plugin_api::config::CommitPolicy::ImmediateAllowed
    )
}

/// 去掉 schema 声明为 transient 的 effect 字段，递归处理数组与嵌套对象。
/// 参数：contribution - 组件 schema；value - 表单值。
/// 返回：可直接提交的设置值。
pub fn strip_transient(contribution: &SettingsContribution, value: Value) -> Value {
    strip_object(&contribution.ui, &contribution.properties, value)
}

/// 递归剥离对象作用域中的 transient 字段。
/// 参数：ui - 字段元数据；properties - 对象属性；value - 对象值。
/// 返回：剥离后的对象。
fn strip_object(
    ui: &[FieldUiMetadata],
    properties: &BTreeMap<String, SchemaNode>,
    value: Value,
) -> Value {
    let Value::Object(mut map) = value else {
        return value;
    };
    for meta in ui {
        let key = key_from_pointer(&meta.pointer);
        if meta.action.as_ref().is_some_and(
            |action| matches!(action, FieldAction::Effect(binding) if binding.transient),
        ) {
            map.remove(key);
            continue;
        }
        if let (Some(schema), Some(field_value)) = (properties.get(key), map.get_mut(key)) {
            strip_node(schema, field_value);
        }
    }
    Value::Object(map)
}

/// 递归剥离 schema 子节点中的 transient 字段。
/// 参数：schema - 节点 schema；value - 节点值。
/// 返回：无；原地修改对象/数组。
fn strip_node(schema: &SchemaNode, value: &mut Value) {
    match &schema.kind {
        SchemaKind::Object { properties, ui, .. } => {
            *value = strip_object(ui, properties, std::mem::take(value));
        }
        SchemaKind::Array { items, .. } => {
            if let Value::Array(entries) = value {
                for entry in entries {
                    strip_node(items, entry);
                }
            }
        }
        _ => {}
    }
}
