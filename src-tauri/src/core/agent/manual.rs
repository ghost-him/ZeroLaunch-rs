//! 设置说明书：把配置系统的 schema 与译文整理成可读、可检索的条目。
//!
//! 数据来源：`ConfigManager`（组件与字段 schema）与 `I18nManager`（译文）；另有两条渲染期注入——
//! 前端下发的 `navigation`（完整点击链与组件内字段分组形态）与命令层从插件元数据投影的
//! `triggers`（触发方式）——都只参与渲染，不进条目。
//! 不另维护第二份文案 —— schema 或文案变了，说明书随之变化。
//! 内嵌对象字段（数组项、对象字段的子字段）同样收进条目，key 用 `父key.子key`，
//! 与 `get_settings` 返回的 JSON 结构一致；界面状态（只读、设置页不直接显示、
//! 显示条件）按 schema 的 UI 元数据标注、并由子字段继承父级，免得模型把用户看不见
//! 或改不了的字段当普通可配项。
//! 条目同时服务两个消费方：拼装模型上下文、确定性检索（UI 搜索框与 agent 共用）。

use crate::core::config::ConfigManager;
use crate::core::i18n::I18nManager;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use zerolaunch_plugin_api::config::{FieldUiMetadata, SchemaKind, SchemaNode, VisibleWhen};

/// 组件级说明条目。
pub struct ManualComponent {
    /// 组件 id；模型在「相关设置」行中引用它。
    pub id: String,
    /// 组件显示名（已按当前语言翻译）。
    pub name: String,
    /// 组件说明（已按当前语言翻译）。
    pub description: String,
}

/// 组件在设置页中的完整点击链，以及组件内字段分组的渲染形态。
#[derive(Debug, Clone)]
pub struct ComponentNavigation {
    /// 完整点击路径（含分类、组件类型标签页与组件名），由前端按当前界面语言拼好。
    pub path: String,
    /// 组件内字段分组是否渲染为标签页；false 表示页内分组标题。
    pub group_tabs: bool,
}

/// 字段级说明条目。
pub struct ManualField {
    /// 所属组件 id。
    pub component_id: String,
    /// 所属组件显示名。
    pub component_name: String,
    /// 字段 key（settings JSON 中的键；内嵌对象字段为 `父key.子key`）。
    pub key: String,
    /// 字段标签（已翻译）。
    pub label: String,
    /// 字段说明（已翻译）。
    pub description: String,
    /// 字段补充说明（已翻译；空表示无补充）。承载作用范围、生效时机与特殊限制。
    pub note: String,
    /// 字段所在分组（已翻译；空表示未分组）。界面上是组件内的标签页或分组标题，
    /// 由组件导航的 `group_tabs` 决定。
    pub group: String,
    /// 是否只读：界面上不可编辑。
    pub read_only: bool,
    /// 是否在设置页不直接显示（schema 的 `visible = false`）。
    pub hidden_in_settings: bool,
    /// 可见性条件（已翻译为「同级字段标签=值」；空表示无条件显示）。
    pub condition: String,
    /// 取值形态摘要（可选值 / 范围 / 默认值）。
    pub value_hint: String,
    /// 是否敏感字段：其**值**不进入模型上下文。
    pub sensitive: bool,
}

/// 检索命中：字段条目 + 匹配得分。
pub struct ManualHit {
    /// 组件 id。
    pub component_id: String,
    /// 字段 key。
    pub field_key: String,
    /// 字段标签。
    pub label: String,
    /// 匹配得分（越大越靠前）。
    pub score: u32,
}

/// 设置说明书：组件与字段的扁平清单。
pub struct SettingsManual {
    /// 组件条目（保持注册顺序）。
    components: Vec<ManualComponent>,
    /// 字段条目（保持组件内 schema 顺序）。
    fields: Vec<ManualField>,
}

impl SettingsManual {
    /// 构建说明书。
    ///
    /// 参数：config - 配置管理器（schema 来源）；i18n - 后端翻译服务（文案来源）。
    /// 返回：说明书；某组件 schema 校验失败时只保留其组件级条目（不中断构建）。
    pub fn build(config: &ConfigManager, i18n: &I18nManager) -> Self {
        let lang = i18n.current_language();
        let mut components = Vec::new();
        let mut fields = Vec::new();
        for info in config.get_all_components() {
            let name = i18n.t(&lang, &info.component_name);
            let description = i18n.t(&lang, &info.component_description);
            if let Some(schema) = config.get_component_schema(&info.component_id) {
                let contribution = schema.contribution;
                let mut component_fields = Vec::new();
                collect_fields(
                    &contribution.ui,
                    &contribution.properties,
                    &FieldScope {
                        component_id: &info.component_id,
                        component_name: &name,
                        i18n,
                        lang: &lang,
                        parent_key: "",
                        parent_hidden: false,
                        parent_read_only: false,
                    },
                    &mut component_fields,
                );
                // 组件内 key 必须两两不同，否则引用校验与标签查找会张冠李戴（见 `has_unique_keys`）。
                debug_assert!(
                    has_unique_keys(&component_fields),
                    "组件 {} 的说明书字段 key 重复",
                    info.component_id
                );
                fields.extend(component_fields);
            }
            components.push(ManualComponent {
                id: info.component_id,
                name,
                description,
            });
        }
        Self { components, fields }
    }

    /// 渲染为模型上下文文本：组件（id + 设置页路径 + 触发方式 + 说明）后跟该组件的字段清单。
    ///
    /// 参数：navigation - 组件 id → 设置页完整点击链与字段分组形态（由前端按当前界面语言拼好）；
    /// triggers - 组件 id → 触发段文本（自含标签，如「触发词：fy / tr」，由命令层从插件元数据组装）。
    /// 缺路径或缺触发的组件只渲染已有部分，模型不会看到空占位；
    /// 两者都缺时模型无法给出逐步导航，会退回组件名。
    /// 字段行带上所在分组（`位置：标签页「…」` 或 `位置：分组「…」`）：
    /// 标签页是组件页内的一级导航，分组只是页内分区标题。
    /// 前端只在组件有多个顶层分组时才渲染分组 tab，这里按同一前提统计，单组仍标为分组。
    pub fn render(
        &self,
        navigation: &HashMap<String, ComponentNavigation>,
        triggers: &HashMap<String, String>,
    ) -> String {
        let mut out = String::new();
        for component in &self.components {
            let fields: Vec<&ManualField> = self
                .fields
                .iter()
                .filter(|field| field.component_id == component.id)
                .collect();
            // 前端 showGroupTabs 只在顶层非隐藏字段分成多组时才渲染 tab；单组会平铺，
            // 因此这里按同样的静态前提统计，避免把单组组件说成「标签页」。
            let group_count = fields
                .iter()
                .filter(|field| {
                    !field.hidden_in_settings && !field.key.contains('.') && !field.group.is_empty()
                })
                .map(|field| field.group.as_str())
                .collect::<BTreeSet<_>>()
                .len();
            out.push_str(&format!("## {}({})", component.name, component.id));
            let entry = navigation.get(&component.id);
            let group_tabs = entry.is_some_and(|nav| nav.group_tabs) && group_count > 1;
            if let Some(entry) = entry {
                out.push_str(&format!("｜设置页路径：{}", entry.path));
            }
            if let Some(trigger) = triggers.get(&component.id) {
                out.push_str(&format!("｜{trigger}"));
            }
            out.push('\n');
            if !component.description.is_empty() {
                out.push_str(&format!("{}\n", component.description));
            }
            for field in fields {
                out.push_str("- ");
                out.push_str(&field.render_line(group_tabs));
                out.push('\n');
            }
        }
        out
    }

    /// 按关键词检索字段：key 命中优先于标签、标签优先于说明。
    ///
    /// 参数：query - 用户输入；limit - 最大返回条数。
    /// 返回：按得分降序的命中列表（空查询返回空列表）。
    pub fn search(&self, query: &str, limit: usize) -> Vec<ManualHit> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<ManualHit> = self
            .fields
            .iter()
            .filter_map(|field| {
                let score = field_score(field, &needle)?;
                Some(ManualHit {
                    component_id: field.component_id.clone(),
                    field_key: field.key.clone(),
                    label: field.label.clone(),
                    score,
                })
            })
            .collect();
        hits.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.component_id.cmp(&b.component_id))
                .then_with(|| a.field_key.cmp(&b.field_key))
        });
        hits.truncate(limit);
        hits
    }

    /// 组件是否存在（校验模型给出的引用，剔除臆造项）。
    pub fn has_component(&self, component_id: &str) -> bool {
        self.components.iter().any(|c| c.id == component_id)
    }

    /// 字段是否存在（同上）。
    pub fn has_field(&self, component_id: &str, field_key: &str) -> bool {
        self.fields
            .iter()
            .any(|field| field.component_id == component_id && field.key == field_key)
    }

    /// 组件显示名；未知组件返回 None。
    pub fn component_name(&self, component_id: &str) -> Option<&str> {
        self.components
            .iter()
            .find(|c| c.id == component_id)
            .map(|c| c.name.as_str())
    }

    /// 字段标签；未知字段返回 None。
    pub fn field_label(&self, component_id: &str, field_key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|field| field.component_id == component_id && field.key == field_key)
            .map(|field| field.label.as_str())
    }

    /// 敏感字段 key 集合（供取值脱敏用）。
    ///
    /// 内嵌字段给出的是 `父key.子key` 全路径，而消费方 `mask_sensitive` 按 JSON 叶子 key 名比对，
    /// 两者只在顶层字段上等价——嵌套敏感字段要在那条链路改成按路径段匹配才生效。
    pub fn sensitive_keys(&self) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|field| field.sensitive)
            .map(|field| field.key.as_str())
            .collect()
    }
}

/// 递归收集字段的作用域上下文：固定上下文（组件身份与译文来源）与父级继承状态。
///
/// 仅限本文件使用：`collect_fields` 在顶层构造一次，递归时按子字段继承父级状态派生。
/// 内部引用都不超过一次 `SettingsManual::build` 调用的生命周期。
#[derive(Clone, Copy)]
struct FieldScope<'a> {
    /// 所属组件 id，写入每个字段条目（引用校验按它比对）。
    component_id: &'a str,
    /// 所属组件显示名，写入每个字段条目（引用回显用）。
    component_name: &'a str,
    /// 译文来源（标签、说明、补充说明、分组都经它翻译）。
    i18n: &'a I18nManager,
    /// 当前语言码。
    lang: &'a str,
    /// 父字段 key；顶层为空串，内嵌字段的 key 按 `父key.子key` 拼接。
    parent_key: &'a str,
    /// 父级是否在设置页不直接显示（子字段继承：父级不可见时子字段也无处可改）。
    parent_hidden: bool,
    /// 父级是否只读（子字段继承，与前端 `fieldDefToConfig` 的父级只读语义一致）。
    parent_read_only: bool,
}

/// 收集一个作用域的字段条目，并递归展开内嵌对象字段（key 用 `父key.子key`）。
///
/// 参数：ui/properties - 当前作用域的 UI 元数据与 schema 节点；scope - 组件身份、译文来源与父级继承状态；
/// out - 收集结果（保持 schema 顺序：父字段先于其子字段）。
fn collect_fields(
    ui: &[FieldUiMetadata],
    properties: &BTreeMap<String, SchemaNode>,
    scope: &FieldScope<'_>,
    out: &mut Vec<ManualField>,
) {
    for entry in ui {
        // pointer 为 "/<key>"；字段 key 允许的字符集不含需要转义的字符
        let key = entry.pointer.trim_start_matches('/').to_string();
        let full_key = if scope.parent_key.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", scope.parent_key, key)
        };
        let hidden = scope.parent_hidden || !entry.visible;
        let read_only = scope.parent_read_only || entry.read_only;
        let node = properties.get(&key);
        out.push(ManualField {
            component_id: scope.component_id.to_string(),
            component_name: scope.component_name.to_string(),
            key: full_key.clone(),
            label: scope.i18n.t(scope.lang, &entry.label),
            description: scope.i18n.t(scope.lang, &entry.description),
            note: entry
                .note
                .as_deref()
                .map(|note| scope.i18n.t(scope.lang, note))
                .unwrap_or_default(),
            group: entry
                .group
                .as_deref()
                .map(|group| scope.i18n.t(scope.lang, group))
                .unwrap_or_default(),
            read_only,
            hidden_in_settings: hidden,
            condition: entry
                .visible_when
                .as_ref()
                .map(|condition| condition_text(condition, ui, properties, scope.i18n, scope.lang))
                .unwrap_or_default(),
            value_hint: node
                .map(|node| value_hint(node, scope.i18n, scope.lang))
                .unwrap_or_default(),
            sensitive: entry.sensitive,
        });
        if let Some((child_ui, child_properties)) = node.and_then(object_children) {
            collect_fields(
                child_ui,
                child_properties,
                &FieldScope {
                    parent_key: &full_key,
                    parent_hidden: hidden,
                    parent_read_only: read_only,
                    ..*scope
                },
                out,
            );
        }
    }
}

/// 字段 key 是否两两不同：说明书按扁平 key 做引用校验与标签查找，
/// 内嵌字段的 `父key.子key` 与「本身含 `.` 的字段名」可能拼出同一串。
fn has_unique_keys(fields: &[ManualField]) -> bool {
    let mut seen = HashSet::new();
    fields.iter().all(|field| seen.insert(field.key.as_str()))
}

/// 取出字段节点内嵌对象（数组项或对象字段本身）的属性表与 UI 元数据；无内嵌对象返回 None。
fn object_children(
    node: &SchemaNode,
) -> Option<(&[FieldUiMetadata], &BTreeMap<String, SchemaNode>)> {
    let object = match &node.kind {
        SchemaKind::Object { .. } => node,
        SchemaKind::Array { items, .. } => items.as_ref(),
        _ => return None,
    };
    match &object.kind {
        SchemaKind::Object { properties, ui, .. } => Some((ui.as_slice(), properties)),
        _ => None,
    }
}

/// 渲染可见性条件为「同级字段标签=值」。
///
/// 同级字段标签与枚举取值都按当前语言翻译；查不到同级字段、或该值不在枚举里时，
/// 分别回落为该字段的裸 key 与 JSON 值字面量。
fn condition_text(
    condition: &VisibleWhen,
    ui: &[FieldUiMetadata],
    properties: &BTreeMap<String, SchemaNode>,
    i18n: &I18nManager,
    lang: &str,
) -> String {
    let label = ui
        .iter()
        .find(|entry| entry.pointer.trim_start_matches('/') == condition.field)
        .map(|entry| i18n.t(lang, &entry.label))
        .unwrap_or_else(|| condition.field.clone());
    let value = match &condition.value {
        Value::String(raw) => enum_label(properties.get(&condition.field), raw, i18n, lang)
            .unwrap_or_else(|| raw.clone()),
        other => other.to_string(),
    };
    format!("{label}={value}")
}

/// 字符串枚举取值的已翻译标签；非枚举、未匹配或标签为空时返回 None。
fn enum_label(
    node: Option<&SchemaNode>,
    value: &str,
    i18n: &I18nManager,
    lang: &str,
) -> Option<String> {
    let SchemaKind::String {
        enum_values,
        enum_labels,
        ..
    } = &node?.kind
    else {
        return None;
    };
    let index = enum_values
        .iter()
        .position(|candidate| candidate == value)?;
    enum_labels
        .get(index)
        .filter(|label| !label.trim().is_empty())
        .map(|label| i18n.t(lang, label))
}

#[cfg(test)]
impl SettingsManual {
    /// 测试专用构造：绕过 schema 读取直接给定条目。
    pub fn from_parts(components: Vec<ManualComponent>, fields: Vec<ManualField>) -> Self {
        Self { components, fields }
    }
}

impl ManualField {
    /// 渲染单条字段说明行：`key` 标签｜界面位置｜界面状态｜说明｜补充｜取值摘要。
    ///
    /// 「位置」是字段所在分组名：`group_tabs` 为真时是组件内标签页，否则是页内分组标题；
    /// 未分组时不渲染。界面状态（设置页不直接显示 / 只读 / 仅当…时显示）与「补充」按
    /// schema 与译文原样给出，免得模型把用户看不见或改不了的字段当成普通可配项。
    fn render_line(&self, group_tabs: bool) -> String {
        let mut parts = vec![format!("`{}` {}", self.key, self.label)];
        if !self.group.is_empty() {
            let kind = if group_tabs { "标签页" } else { "分组" };
            parts.push(format!("位置：{kind}「{}」", self.group));
        }
        if self.hidden_in_settings {
            parts.push("设置页不直接显示".to_string());
        }
        if self.read_only {
            parts.push("只读".to_string());
        }
        if !self.condition.is_empty() {
            parts.push(format!("仅当 {} 时显示", self.condition));
        }
        if !self.description.is_empty() {
            parts.push(self.description.clone());
        }
        if !self.note.is_empty() {
            parts.push(format!("补充：{}", self.note));
        }
        if !self.value_hint.is_empty() {
            parts.push(format!("取值：{}", self.value_hint));
        }
        if self.sensitive {
            parts.push("敏感字段（当前值不可读）".to_string());
        }
        parts.join("｜")
    }
}

/// 字段匹配得分：key 精确 > key 前缀 > key 包含 > 标签 > 说明 > 补充 > 组件名；无关返回 None。
///
/// 补充说明参与匹配：它常是用户唯一会搜的字眼（如「立即生效」「重启」「小号字」）。
fn field_score(field: &ManualField, needle: &str) -> Option<u32> {
    let key = field.key.to_lowercase();
    if key == needle {
        return Some(100);
    }
    if key.starts_with(needle) {
        return Some(90);
    }
    if key.contains(needle) {
        return Some(70);
    }
    if field.label.to_lowercase().contains(needle) {
        return Some(50);
    }
    if field.description.to_lowercase().contains(needle) {
        return Some(30);
    }
    if field.note.to_lowercase().contains(needle) {
        return Some(25);
    }
    if field.component_name.to_lowercase().contains(needle) {
        return Some(20);
    }
    None
}

/// 生成字段取值摘要：类型 / 可选值 / 范围 + 默认值。
fn value_hint(node: &SchemaNode, i18n: &I18nManager, lang: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    match &node.kind {
        SchemaKind::String {
            enum_values,
            enum_labels,
            min_length,
            max_length,
            ..
        } => {
            if enum_values.is_empty() {
                parts.push("文本".to_string());
            } else {
                let options: Vec<String> = enum_values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| match enum_labels.get(index) {
                        // 选项标签是 i18n key，需按当前语言翻译后再给模型
                        Some(label) if !label.trim().is_empty() => {
                            format!("{value}={}", i18n.t(lang, label))
                        }
                        _ => value.clone(),
                    })
                    .collect();
                parts.push(format!("可选 {}", options.join(" / ")));
            }
            match (min_length, max_length) {
                (Some(min), Some(max)) => parts.push(format!("长度 {min}-{max}")),
                (Some(min), None) => parts.push(format!("长度 ≥{min}")),
                (None, Some(max)) => parts.push(format!("长度 ≤{max}")),
                (None, None) => {}
            }
        }
        SchemaKind::Number {
            minimum, maximum, ..
        } => parts.push(bounds_text(*minimum, *maximum)),
        SchemaKind::Integer {
            minimum, maximum, ..
        } => parts.push(bounds_text(*minimum, *maximum)),
        SchemaKind::Boolean => parts.push("开关（true/false）".to_string()),
        SchemaKind::Array {
            min_items,
            max_items,
            ..
        } => {
            parts.push("数组".to_string());
            match (min_items, max_items) {
                (Some(min), Some(max)) => parts.push(format!("{min}-{max} 项")),
                (Some(min), None) => parts.push(format!("至少 {min} 项")),
                (None, Some(max)) => parts.push(format!("至多 {max} 项")),
                (None, None) => {}
            }
        }
        SchemaKind::Object { properties, .. } => {
            parts.push(format!("对象（{} 个子字段）", properties.len()));
        }
    }
    if let Some(default) = &node.default {
        if !default.is_null() {
            let rendered = default.to_string();
            let shown: String = rendered.chars().take(40).collect();
            parts.push(format!("默认 {shown}"));
        }
    }
    parts.join("，")
}

/// 数值范围文本（整数与浮点共用）。
fn bounds_text<N: std::fmt::Display>(minimum: Option<N>, maximum: Option<N>) -> String {
    match (minimum, maximum) {
        (Some(min), Some(max)) => format!("数值 {min}-{max}"),
        (Some(min), None) => format!("数值 ≥{min}"),
        (None, Some(max)) => format!("数值 ≤{max}"),
        (None, None) => "数值".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// 构造可解析译文的管理器：写入独立的临时语言包并固定为简体中文。
    ///
    /// 目录名带自增序号：并行测试共用进程 id 会互相覆盖同一份语言包。
    fn test_i18n(catalog: serde_json::Value) -> I18nManager {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let index = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("zl-agent-manual-{}-{index}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("zh-Hans.json"), catalog.to_string()).unwrap();
        let manager = I18nManager::load(dir.clone());
        manager.set_language("zh-Hans");
        std::fs::remove_dir_all(&dir).ok();
        Arc::try_unwrap(manager).unwrap_or_else(|_| unreachable!("load 返回唯一 Arc"))
    }

    /// 构造一条测试用字段条目（可见、可编辑、无补充与条件）。
    fn field(key: &str, label: &str, description: &str) -> ManualField {
        ManualField {
            component_id: "demo".to_string(),
            component_name: "演示".to_string(),
            key: key.to_string(),
            label: label.to_string(),
            description: description.to_string(),
            note: String::new(),
            group: String::new(),
            read_only: false,
            hidden_in_settings: false,
            condition: String::new(),
            value_hint: String::new(),
            sensitive: false,
        }
    }

    /// 构造测试用 UI 元数据（可见、可编辑，label 传 i18n key）。
    fn ui(pointer: &str, label: &str) -> FieldUiMetadata {
        FieldUiMetadata {
            pointer: pointer.to_string(),
            label: label.to_string(),
            description: String::new(),
            note: None,
            group: None,
            order: 0,
            visible: true,
            read_only: false,
            sensitive: false,
            visible_when: None,
            widget: None,
            action: None,
            detail_action: None,
            drop: None,
        }
    }

    /// 构造只含字段与组件条目的说明书。
    fn manual(fields: Vec<ManualField>) -> SettingsManual {
        SettingsManual::from_parts(
            vec![ManualComponent {
                id: "demo".to_string(),
                name: "演示".to_string(),
                description: "演示组件".to_string(),
            }],
            fields,
        )
    }

    /// 检索：标签命中排在说明命中之前（分数更高），key 大小写不敏感，空查询返回空。
    #[test]
    fn search_ranks_label_over_description_and_ignores_case_and_blank() {
        let manual = manual(vec![
            field("theme", "外观模式", "界面主题深浅"),
            field("other", "主题颜色", "无关说明"),
            field("bm25_k1", "K1", "BM25 参数"),
        ]);
        // 命中分别来自 other 的标签与 theme 的说明，标签得分更高
        let hits = manual.search("主题", 10);
        assert_eq!(hits[0].field_key, "other");
        assert_eq!(hits[1].field_key, "theme");
        assert!(hits[0].score > hits[1].score);
        assert_eq!(manual.search("BM25_", 10)[0].field_key, "bm25_k1");
        assert!(manual.search("   ", 10).is_empty());
    }

    /// 渲染：组件与字段行、前端下发的设置页路径与触发方式、字段所在分组「位置」与「补充」都在场；
    /// 未分组字段不渲染「位置」，免得模型编出不存在的一步。
    #[test]
    fn render_includes_field_lines_path_trigger_and_position() {
        let mut grouped = field("show_subtitle", "显示副标题", "在结果项中显示副标题信息");
        grouped.group = "结果列表".to_string();
        let mut noted = field("footer_height", "底栏高度", "底栏高度(px)");
        noted.note = "设 0 会让各处小字变小到看不见".to_string();
        noted.group = "底栏".to_string();
        let manual = manual(vec![grouped, noted, field("theme", "主题", "界面主题")]);

        let text = manual.render(&HashMap::new(), &HashMap::new());
        assert!(text.contains("## 演示(demo)"), "{text}");
        assert!(text.contains("`theme` 主题｜界面主题"), "{text}");
        assert!(
            text.contains("`show_subtitle` 显示副标题｜位置：分组「结果列表」"),
            "{text}"
        );
        assert!(
            text.contains("补充：设 0 会让各处小字变小到看不见"),
            "{text}"
        );

        let navigation = HashMap::from([(
            "demo".to_string(),
            ComponentNavigation {
                path: "设置 › 外观设置 › 演示".to_string(),
                group_tabs: false,
            },
        )]);
        let triggers = HashMap::from([("demo".to_string(), "触发词：>".to_string())]);
        let text = manual.render(&navigation, &triggers);
        assert!(
            text.contains("## 演示(demo)｜设置页路径：设置 › 外观设置 › 演示｜触发词：>"),
            "{text}"
        );

        // 同一份字段在分组标签页形态下要标成「标签页」，模型据此把它插进点击链
        let tabs = HashMap::from([(
            "demo".to_string(),
            ComponentNavigation {
                path: "设置 › 外观设置 › 演示".to_string(),
                group_tabs: true,
            },
        )]);
        let text = manual.render(&tabs, &HashMap::new());
        assert!(
            text.contains("`show_subtitle` 显示副标题｜位置：标签页「结果列表」"),
            "{text}"
        );
    }

    /// 单组组件即使分类允许分组 tab，前端也会平铺字段（`showGroupTabs` 要求多于一组），
    /// 说明书必须同步标成分组标题，不能给出不存在的标签页。
    #[test]
    fn render_group_tabs_requires_multiple_groups() {
        let mut grouped = field("show_subtitle", "显示副标题", "在结果项中显示副标题信息");
        grouped.group = "结果列表".to_string();
        let manual = manual(vec![grouped]);
        let navigation = HashMap::from([(
            "demo".to_string(),
            ComponentNavigation {
                path: "设置 › 外观设置 › 图标覆盖".to_string(),
                group_tabs: true,
            },
        )]);

        let text = manual.render(&navigation, &HashMap::new());
        assert!(
            text.contains("`show_subtitle` 显示副标题｜位置：分组「结果列表」"),
            "{text}"
        );
    }

    /// 内嵌对象字段按 `父key.子key` 收集，界面状态照 UI 元数据标注：
    /// 只读、设置页不直接显示、显示条件（条件里的同级字段标签与枚举值都译成当前语言）。
    #[test]
    fn collect_fields_expands_nested_items_and_marks_ui_state() {
        let i18n = test_i18n(serde_json::json!({
            "components": {"demo": {
                "fields": {
                    "models": "模型清单", "name": "模型名", "kind": "角色",
                    "temperature": "温度", "extra": "附加", "flag": "开关"
                },
                "kinds": {"chat": "文本生成"}
            }}
        }));
        let mut name = ui("/name", "components.demo.fields.name");
        name.read_only = true;
        let mut kind = ui("/kind", "components.demo.fields.kind");
        kind.visible = false;
        // 内嵌对象字段：自身不可见且只读，用来验证子字段继承父级状态
        let mut extra = ui("/extra", "components.demo.fields.extra");
        extra.visible = false;
        extra.read_only = true;
        let mut temperature = ui("/temperature", "components.demo.fields.temperature");
        temperature.visible_when = Some(VisibleWhen {
            field: "kind".to_string(),
            value: serde_json::json!("chat"),
        });
        let kind_schema = SchemaNode {
            kind: SchemaKind::String {
                enum_values: vec!["chat".to_string(), "embedding".to_string()],
                enum_labels: vec!["components.demo.kinds.chat".to_string(), String::new()],
                min_length: None,
                max_length: None,
                pattern: None,
            },
            default: None,
        };
        let extra_schema = SchemaNode {
            kind: SchemaKind::Object {
                properties: BTreeMap::from([("flag".to_string(), SchemaNode::string())]),
                ui: vec![ui("/flag", "components.demo.fields.flag")],
                required: std::collections::BTreeSet::new(),
            },
            default: None,
        };
        let items = SchemaNode {
            kind: SchemaKind::Object {
                properties: BTreeMap::from([
                    ("name".to_string(), SchemaNode::string()),
                    ("kind".to_string(), kind_schema),
                    ("temperature".to_string(), SchemaNode::number()),
                    ("extra".to_string(), extra_schema),
                ]),
                ui: vec![name, kind, temperature, extra],
                required: std::collections::BTreeSet::new(),
            },
            default: None,
        };
        let properties = BTreeMap::from([(
            "models".to_string(),
            SchemaNode {
                kind: SchemaKind::Array {
                    items: Box::new(items),
                    item_widget: None,
                    min_items: None,
                    max_items: None,
                },
                default: None,
            },
        )]);
        let mut fields = Vec::new();
        collect_fields(
            &[ui("/models", "components.demo.fields.models")],
            &properties,
            &FieldScope {
                component_id: "demo",
                component_name: "演示",
                i18n: &i18n,
                lang: "zh-Hans",
                parent_key: "",
                parent_hidden: false,
                parent_read_only: false,
            },
            &mut fields,
        );

        let keys: Vec<&str> = fields.iter().map(|field| field.key.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "models",
                "models.name",
                "models.kind",
                "models.temperature",
                "models.extra",
                "models.extra.flag"
            ],
            "{keys:?}"
        );
        assert_eq!(fields[1].label, "模型名");
        assert!(
            fields[1].render_line(false).contains("只读"),
            "{}",
            fields[1].render_line(false)
        );
        assert!(fields[2].hidden_in_settings);
        assert!(fields[2].render_line(false).contains("设置页不直接显示"));
        assert_eq!(fields[3].condition, "角色=文本生成");
        assert!(
            fields[3]
                .render_line(false)
                .contains("仅当 角色=文本生成 时显示"),
            "{}",
            fields[3].render_line(false)
        );
        // 内嵌对象字段的子字段继承父级界面状态：extra 自身不可见且只读 → flag 两样都继承
        assert!(fields[4].hidden_in_settings && fields[4].read_only);
        assert!(
            fields[5].hidden_in_settings && fields[5].read_only,
            "子字段应继承父级的「设置页不直接显示」与只读"
        );
        assert!(fields[5].render_line(false).contains("设置页不直接显示"));
    }

    #[test]
    fn value_hint_translates_enum_labels_and_shows_default() {
        let i18n = test_i18n(serde_json::json!({
            "components": {"demo": {"enum": {"light": "浅色"}}}
        }));
        let node = SchemaNode {
            kind: SchemaKind::String {
                enum_values: vec!["light".to_string(), "dark".to_string()],
                enum_labels: vec!["components.demo.enum.light".to_string()],
                min_length: None,
                max_length: None,
                pattern: None,
            },
            default: Some(serde_json::json!("light")),
        };
        let hint = value_hint(&node, &i18n, "zh-Hans");
        assert!(hint.contains("light=浅色"), "{hint}");
        assert!(hint.contains("dark"), "{hint}");
        assert!(hint.contains("默认 \"light\""), "{hint}");
    }

    #[test]
    fn sensitive_keys_lists_only_marked_fields() {
        let mut sensitive = field("api_key", "API Key", "");
        sensitive.sensitive = true;
        let manual = manual(vec![sensitive, field("base_url", "地址", "")]);
        assert_eq!(manual.sensitive_keys(), vec!["api_key"]);
    }
}
