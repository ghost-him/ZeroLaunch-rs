use crate::config::{DetailActionDef, FieldAction};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Mutex;

/// 编译后的正则表达式缓存，按 pattern 字符串共享。
/// validate_node 中每次匹配都从该缓存取编译结果，避免重复编译。
static REGEX_CACHE: Lazy<Mutex<HashMap<String, Regex>>> = Lazy::new(|| Mutex::new(HashMap::new()));
/// Settings schema 版本号，用于向前兼容判断。
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CommitPolicy {
    /// 用户必须点击"保存"按钮才提交。
    #[default]
    #[serde(rename = "staged")]
    Staged,
    /// 值变更时立即提交，跳过暂存。
    #[serde(rename = "immediateAllowed")]
    ImmediateAllowed,
}

/// 路径选择模式。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PathMode {
    /// 文件选择。
    #[serde(rename = "file")]
    File,
    /// 目录选择。
    #[serde(rename = "directory")]
    Directory,
}

/// UI 控件提示 — 告诉前端这个字段应该用什么控件渲染。
///
/// 与 `SchemaKind` 正交：SchemaKind 描述数据形状，WidgetHint 描述呈现方式。
/// 同一份数据（如 string）可以透过多样的 WidgetHint 渲染为不同控件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum WidgetHint {
    /// 单行文本输入框。
    #[serde(rename = "text")]
    Text,
    /// 多行文本域。
    #[serde(rename = "textarea")]
    Textarea,
    /// 数字输入框（步进器）。
    #[serde(rename = "number")]
    Number,
    /// 切换开关（用于 boolean 字段）。
    #[serde(rename = "toggle")]
    Toggle,
    /// 下拉选择器。
    #[serde(rename = "select")]
    Select,
    /// 路径选择器（文件/目录）。
    #[serde(rename = "path")]
    Path {
        /// 选择模式：文件或目录。
        #[serde(rename = "mode")]
        mode: PathMode,
    },
    /// 颜色选择器。
    #[serde(rename = "color")]
    Color,
    /// 图片选择器。
    #[serde(rename = "image")]
    Image {
        /// 允许的文件扩展名列表。
        #[serde(rename = "accept")]
        accept: Vec<String>,
        /// 最大文件大小（字节）。
        #[serde(rename = "maxSize")]
        #[serde(default)]
        max_size: Option<u64>,
    },
    /// 字体选择器 — 通过组件 config action 列出系统字体供用户直接选择。
    #[serde(rename = "font")]
    Font {
        /// 列出系统字体的 config action 名称（如 `list_fonts`）。
        #[serde(rename = "action")]
        action: String,
        /// 提供该 action 的组件 id；None 表示字段所属组件自身。
        #[serde(rename = "component", default)]
        component: Option<String>,
    },
    /// 普通列表编辑器（默认的数组 UI）。
    #[serde(rename = "list")]
    List,
    /// 标签式编辑器。
    #[serde(rename = "tags")]
    Tags,
    /// 表格编辑器。
    #[serde(rename = "table")]
    Table,
    /// 卡片式编辑器。
    #[serde(rename = "cards")]
    Cards,
    /// 主从详情面板 — 左侧列表选择，右侧编辑详情。
    #[serde(rename = "masterDetail")]
    MasterDetail,
    /// 搜索弹窗表格 — 通过 action 搜索并添加行。
    #[serde(rename = "searchTable")]
    SearchTable,
}

/// Schema 类型节点 — 描述数据的形状和校验规则。
///
/// 采用 tagged union 格式，通过 `type` 字段区分。
/// 设计灵感来自 JSON Schema，但简化为只包含本项目需要的约束。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SchemaKind {
    /// 字符串类型。
    #[serde(rename = "string")]
    String {
        /// 枚举值列表，非空时限定输入只能从这些值中选择。
        #[serde(rename = "enum", default)]
        enum_values: Vec<String>,
        /// 枚举值对应的可选展示标签，与 enum_values 等长。
        /// 前端优先使用标签展示，缺失时回退到 enum_values 本身。
        #[serde(rename = "enumLabels", default)]
        enum_labels: Vec<String>,
        /// 最小长度。
        #[serde(rename = "minLength", default)]
        min_length: Option<usize>,
        /// 最大长度。
        #[serde(rename = "maxLength", default)]
        max_length: Option<usize>,
        /// 正则表达式约束。
        #[serde(rename = "pattern", default)]
        pattern: Option<String>,
    },
    /// 浮点数类型。
    #[serde(rename = "number")]
    Number {
        /// 最小值（含）。
        #[serde(rename = "minimum", default)]
        minimum: Option<f64>,
        /// 最大值（含）。
        #[serde(rename = "maximum", default)]
        maximum: Option<f64>,
        /// 步长约束（值必须是 multiple_of 的整数倍）。
        #[serde(rename = "multipleOf", default)]
        multiple_of: Option<f64>,
    },
    /// 整数类型。
    #[serde(rename = "integer")]
    Integer {
        /// 最小值（含）。
        #[serde(rename = "minimum", default)]
        minimum: Option<i64>,
        /// 最大值（含）。
        #[serde(rename = "maximum", default)]
        maximum: Option<i64>,
        /// 步长约束。
        #[serde(rename = "multipleOf", default)]
        multiple_of: Option<i64>,
    },
    /// 布尔类型。
    #[serde(rename = "boolean")]
    Boolean,
    /// 数组类型。
    #[serde(rename = "array")]
    Array {
        /// 元素的 schema。
        #[serde(rename = "items")]
        items: Box<SchemaNode>,
        /// 数组元素的 UI 提示，用于恢复 Path/Color 等 item-level 控件。
        #[serde(rename = "itemWidget", default)]
        item_widget: Option<WidgetHint>,
        /// 最小元素数量。
        #[serde(rename = "minItems", default)]
        min_items: Option<usize>,
        /// 最大元素数量。
        #[serde(rename = "maxItems", default)]
        max_items: Option<usize>,
    },
    /// 对象类型。
    #[serde(rename = "object")]
    Object {
        /// 对象属性定义。
        #[serde(rename = "properties")]
        properties: BTreeMap<String, SchemaNode>,
        /// 嵌套字段的 UI 元数据，与 properties 一一对应。
        #[serde(rename = "ui", default)]
        ui: Vec<FieldUiMetadata>,
        /// 必需属性的 key 集合。
        #[serde(rename = "required", default)]
        required: BTreeSet<String>,
    },
}

/// Schema 节点 — 包含类型定义和默认值。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaNode {
    /// 类型定义（flatten 到父级 JSON 中）。
    #[serde(flatten)]
    pub kind: SchemaKind,
    /// 默认值。未设置时为 None。
    #[serde(rename = "default", default)]
    pub default: Option<Value>,
}

impl SchemaNode {
    /// 创建一个无约束的字符串 schema 节点。
    pub fn string() -> Self {
        Self {
            kind: SchemaKind::String {
                enum_values: Vec::new(),
                enum_labels: Vec::new(),
                min_length: None,
                max_length: None,
                pattern: None,
            },
            default: None,
        }
    }

    /// 创建一个无约束的浮点数 schema 节点。
    pub fn number() -> Self {
        Self {
            kind: SchemaKind::Number {
                minimum: None,
                maximum: None,
                multiple_of: None,
            },
            default: None,
        }
    }

    /// 创建一个无约束的整数 schema 节点。
    pub fn integer() -> Self {
        Self {
            kind: SchemaKind::Integer {
                minimum: None,
                maximum: None,
                multiple_of: None,
            },
            default: None,
        }
    }

    /// 创建一个布尔 schema 节点。
    pub fn boolean() -> Self {
        Self {
            kind: SchemaKind::Boolean,
            default: None,
        }
    }
}

/// 字段 UI 元数据 — 描述前端如何渲染和展示一个配置字段。
///
/// 通过 `pointer`（JSON Pointer 格式）关联到 `SettingsContribution.properties` 中的 schema 节点。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldUiMetadata {
    /// 指向 schema 属性的 JSON Pointer（如 `"/theme"`）。
    #[serde(rename = "pointer")]
    pub pointer: String,
    /// 字段显示标签。
    #[serde(rename = "label")]
    pub label: String,
    /// 字段描述文本。
    #[serde(rename = "description", default)]
    pub description: String,
    /// 分组名称，相同 group 的字段在前端渲染在同一区域。
    #[serde(rename = "group", default)]
    pub group: Option<String>,
    /// 组内排序序号，越小越靠前。
    #[serde(rename = "order", default)]
    pub order: u32,
    /// 是否可见。
    #[serde(rename = "visible", default = "default_true")]
    pub visible: bool,
    /// 是否只读。
    #[serde(rename = "readOnly", default)]
    pub read_only: bool,
    /// UI 控件提示。None 时前端根据 SchemaKind 选择默认控件。
    #[serde(rename = "widget", default)]
    pub widget: Option<WidgetHint>,
    /// 运行时数据注入绑定。Some 时前端渲染搜索/检测按钮。
    #[serde(rename = "action", default)]
    pub action: Option<FieldAction>,
    /// MasterDetail 详情面板联动动作定义。
    /// 仅当 widget 为 MasterDetail 时有效。选中列表项时，
    /// 前端调用指定的 config_action 获取预览数据，
    /// 用户编辑结果写入 `targetField` 指定的兄弟设置字段。
    #[serde(rename = "detailAction", default)]
    pub detail_action: Option<DetailActionDef>,
}

fn default_true() -> bool {
    true
}

/// 配置项定义 — 组件 `setting_schema()` 返回的单个配置字段描述。
///
/// 包含三部分：标识键、数据 schema、UI 元数据。
/// 经 `SettingsContribution::from_entries()` 处理后拆分为 properties map + ui 数组。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingDefinition {
    /// 配置项键名（snake_case），作为 settings JSON 中的 key。
    #[serde(rename = "key")]
    pub key: String,
    /// 数据 schema 和校验规则。
    #[serde(rename = "schema")]
    pub schema: SchemaNode,
    /// UI 呈现元数据。
    #[serde(rename = "ui")]
    pub ui: FieldUiMetadata,
}

/// 配置贡献 — 组件对外暴露的完整 schema 描述。
///
/// 包含 schema 版本号、属性定义（键 → schema）、UI 元数据列表、提交策略。
/// 前端接收此结构后按需渲染表单、校验输入。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsContribution {
    /// Schema 版本号，用于向前兼容。
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    /// 属性定义：字段 key → schema 节点。
    #[serde(rename = "properties")]
    pub properties: BTreeMap<String, SchemaNode>,
    /// UI 元数据列表（每个字段一条）。
    #[serde(rename = "ui")]
    pub ui: Vec<FieldUiMetadata>,
    /// 提交策略。
    #[serde(rename = "commitPolicy", default)]
    pub commit_policy: CommitPolicy,
}

impl SettingsContribution {
    /// 从 `SettingDefinition` 列表构建 `SettingsContribution`。
    ///
    /// 校验 schema 合法性后，将 key+schema 拆入 properties map，ui 保留为数组。
    pub fn from_entries(entries: Vec<SettingDefinition>) -> Result<Self, String> {
        validate_setting_definitions(&entries)?;
        let mut properties = BTreeMap::new();
        let mut ui = Vec::with_capacity(entries.len());
        for entry in entries {
            properties.insert(entry.key, entry.schema);
            ui.push(entry.ui);
        }
        ui.sort_by_key(|field| field.order);
        Ok(Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            properties,
            ui,
            commit_policy: CommitPolicy::Staged,
        })
    }

    /// 创建一个空的 SettingsContribution。
    pub fn empty() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            properties: BTreeMap::new(),
            ui: Vec::new(),
            commit_policy: CommitPolicy::Staged,
        }
    }

    /// 从 schema 中收集所有默认值。
    pub fn default_settings(&self) -> Value {
        let values = self
            .properties
            .iter()
            .filter_map(|(key, node)| {
                let value = node.default.as_ref()?;
                if value.is_null() {
                    None
                } else {
                    Some((key.clone(), value.clone()))
                }
            })
            .collect();
        Value::Object(values)
    }

    /// 校验一个 settings JSON 值是否符合本 schema。
    pub fn validate_values(&self, value: &Value) -> Result<(), String> {
        let object = value
            .as_object()
            .ok_or_else(|| "settings root must be an object".to_string())?;
        for key in object.keys() {
            if !self.properties.contains_key(key) {
                return Err(format!("unknown setting key: {}", key));
            }
        }
        for (key, node) in &self.properties {
            if let Some(value) = object.get(key) {
                validate_node(node, value, &format!("/{}", escape_pointer(key)), 0)?;
            }
        }
        Ok(())
    }
}

// ── 校验函数 ──

/// 校验一组 `SettingDefinition` 的合法性。
pub fn validate_setting_definitions(entries: &[SettingDefinition]) -> Result<(), String> {
    if entries.len() > 128 {
        return Err("too many top-level settings (max 128)".to_string());
    }
    let mut keys = HashSet::new();
    for entry in entries {
        if entry.key.is_empty() || entry.key.len() > 128 {
            return Err("setting key length is invalid".to_string());
        }
        if !entry
            .key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        {
            return Err(format!("invalid setting key: {}", entry.key));
        }
        if !keys.insert(entry.key.clone()) {
            return Err(format!("duplicate setting key: {}", entry.key));
        }
        let expected_pointer = format!("/{}", escape_pointer(&entry.key));
        if entry.ui.pointer != expected_pointer {
            return Err(format!(
                "UI pointer '{}' does not match key '{}'",
                entry.ui.pointer, entry.key
            ));
        }
        validate_ui(&entry.ui)?;
        let mut node_count = 0usize;
        validate_schema_node(&entry.schema, 0, &mut node_count)?;
        if let Some(default) = &entry.schema.default {
            validate_node(&entry.schema, default, &expected_pointer, 0)?;
        }
    }
    Ok(())
}

/// 递归校验 schema 节点的结构、约束和嵌套深度。
fn validate_schema_node(
    node: &SchemaNode,
    depth: usize,
    node_count: &mut usize,
) -> Result<(), String> {
    if depth > 4 {
        return Err("schema nesting exceeds limit (max 4)".to_string());
    }
    *node_count += 1;
    if *node_count > 512 {
        return Err("schema node count exceeds limit".to_string());
    }
    match &node.kind {
        SchemaKind::String {
            enum_values,
            min_length,
            max_length,
            pattern,
            ..
        } => {
            if enum_values.len() > 256 || enum_values.iter().any(|v| v.len() > 4096) {
                return Err("string enum exceeds limit".to_string());
            }
            if let (Some(min), Some(max)) = (min_length, max_length) {
                if min > max {
                    return Err("minLength cannot exceed maxLength".to_string());
                }
            }
            if pattern.as_ref().is_some_and(|p| p.len() > 512) {
                return Err("pattern exceeds limit".to_string());
            }
            if let Some(pattern) = pattern {
                if regex::Regex::new(pattern).is_err() {
                    return Err("pattern is not a valid regular expression".to_string());
                }
            }
        }
        SchemaKind::Number {
            minimum,
            maximum,
            multiple_of,
        } => {
            if let (Some(min), Some(max)) = (minimum, maximum) {
                if min > max {
                    return Err("minimum cannot exceed maximum".to_string());
                }
            }
            if multiple_of.is_some_and(|v| v <= 0.0 || !v.is_finite()) {
                return Err("multipleOf must be finite and positive".to_string());
            }
        }
        SchemaKind::Integer {
            minimum,
            maximum,
            multiple_of,
        } => {
            if let (Some(min), Some(max)) = (minimum, maximum) {
                if min > max {
                    return Err("minimum cannot exceed maximum".to_string());
                }
            }
            if multiple_of.is_some_and(|v| v <= 0) {
                return Err("multipleOf must be positive".to_string());
            }
        }
        // item_widget 是 UI 渲染提示，不影响 schema 结构校验，此处无需关注。
        SchemaKind::Array {
            items,
            min_items,
            max_items,
            ..
        } => {
            if max_items.unwrap_or(1024) > 1024 {
                return Err("maxItems exceeds limit".to_string());
            }
            if let (Some(min), Some(max)) = (min_items, max_items) {
                if min > max {
                    return Err("minItems cannot exceed maxItems".to_string());
                }
            }
            validate_schema_node(items, depth + 1, node_count)?;
        }
        SchemaKind::Object {
            properties,
            ui,
            required,
        } => {
            validate_object_ui(properties, ui)?;
            if properties.len() > 128 {
                return Err("object property count exceeds limit".to_string());
            }
            for key in required {
                if !properties.contains_key(key) {
                    return Err(format!("required property does not exist: {}", key));
                }
            }
            for (key, child) in properties {
                if key.is_empty() || key.len() > 128 {
                    return Err("invalid property key".to_string());
                }
                validate_schema_node(child, depth + 1, node_count)?;
            }
        }
        SchemaKind::Boolean => {}
    }
    Ok(())
}
fn validate_node(
    node: &SchemaNode,
    value: &Value,
    pointer: &str,
    depth: usize,
) -> Result<(), String> {
    if depth > 4 {
        return Err(format!("{} exceeds nesting limit", pointer));
    }
    match &node.kind {
        SchemaKind::String {
            enum_values,
            min_length,
            max_length,
            pattern,
            ..
        } => {
            let text = value
                .as_str()
                .ok_or_else(|| format!("{} must be a string", pointer))?;
            let len = text.chars().count();
            if let Some(min) = min_length {
                if len < *min {
                    return Err(format!("{} is too short (min {})", pointer, min));
                }
            }
            if let Some(max) = max_length {
                if len > *max {
                    return Err(format!("{} is too long (max {})", pointer, max));
                }
            }
            if !enum_values.is_empty() && !enum_values.iter().any(|v| v == text) {
                return Err(format!("{} is not an allowed value", pointer));
            }
            if let Some(pattern) = pattern {
                let mut cache = REGEX_CACHE.lock().unwrap();
                let regex = cache.entry(pattern.clone()).or_insert_with(|| {
                    Regex::new(pattern)
                        .expect("pattern 已在 schema 构建时通过 validate_schema_node 校验")
                });
                if !regex.is_match(text) {
                    return Err(format!("{} does not match pattern", pointer));
                }
            }
        }
        SchemaKind::Number {
            minimum,
            maximum,
            multiple_of,
        } => {
            let number = value
                .as_f64()
                .filter(|n| n.is_finite())
                .ok_or_else(|| format!("{} must be a finite number", pointer))?;
            if let Some(min) = minimum {
                if number < *min {
                    return Err(format!("{} is below minimum {}", pointer, min));
                }
            }
            if let Some(max) = maximum {
                if number > *max {
                    return Err(format!("{} is above maximum {}", pointer, max));
                }
            }
            if let Some(step) = multiple_of {
                let quotient = number / step;
                if (quotient - quotient.round()).abs() > 1e-9 {
                    return Err(format!("{} is not a multiple of {}", pointer, step));
                }
            }
        }
        SchemaKind::Integer {
            minimum,
            maximum,
            multiple_of,
        } => {
            let number = value
                .as_f64()
                .filter(|v| v.is_finite() && v.fract() == 0.0)
                .filter(|v| *v >= i64::MIN as f64 && *v <= i64::MAX as f64)
                .map(|v| v as i64)
                .ok_or_else(|| format!("{} must be an integer", pointer))?;
            if let Some(min) = minimum {
                if number < *min {
                    return Err(format!("{} is below minimum {}", pointer, min));
                }
            }
            if let Some(max) = maximum {
                if number > *max {
                    return Err(format!("{} is above maximum {}", pointer, max));
                }
            }
            if let Some(step) = multiple_of {
                if number % step != 0 {
                    return Err(format!("{} is not a multiple of {}", pointer, step));
                }
            }
        }
        SchemaKind::Boolean => {
            if !value.is_boolean() {
                return Err(format!("{} must be a boolean", pointer));
            }
        }
        // item_widget 是 UI 渲染提示，不影响值校验，此处无需关注。
        SchemaKind::Array {
            items,
            min_items,
            max_items,
            ..
        } => {
            let array = value
                .as_array()
                .ok_or_else(|| format!("{} must be an array", pointer))?;
            if let Some(min) = min_items {
                if array.len() < *min {
                    return Err(format!("{} has too few items (min {})", pointer, min));
                }
            }
            if let Some(max) = max_items {
                if array.len() > *max {
                    return Err(format!("{} has too many items (max {})", pointer, max));
                }
            }
            for (index, item) in array.iter().enumerate() {
                validate_node(items, item, &format!("{}/{}", pointer, index), depth + 1)?;
            }
        }
        SchemaKind::Object {
            properties,
            required,
            ..
        } => {
            let object = value
                .as_object()
                .ok_or_else(|| format!("{} must be an object", pointer))?;
            for key in required {
                if !object.contains_key(key) {
                    return Err(format!("{}/{} is required", pointer, escape_pointer(key)));
                }
            }
            for key in object.keys() {
                if !properties.contains_key(key) {
                    return Err(format!(
                        "unknown property: {}/{}",
                        pointer,
                        escape_pointer(key)
                    ));
                }
            }
            for (key, child) in properties {
                if let Some(child_value) = object.get(key) {
                    validate_node(
                        child,
                        child_value,
                        &format!("{}/{}", pointer, escape_pointer(key)),
                        depth + 1,
                    )?;
                }
            }
        }
    }
    Ok(())
}
fn validate_ui(ui: &FieldUiMetadata) -> Result<(), String> {
    if ui.label.is_empty() || ui.label.len() > 512 || ui.description.len() > 4096 {
        return Err(format!("invalid UI metadata at {}", ui.pointer));
    }
    if ui.group.as_ref().is_some_and(|g| g.len() > 512) {
        return Err(format!("UI group exceeds limit at {}", ui.pointer));
    }
    Ok(())
}
/// 校验嵌套 object 的 UI 元数据是否与 properties 完整对应。
fn validate_object_ui(
    properties: &BTreeMap<String, SchemaNode>,
    ui: &[FieldUiMetadata],
) -> Result<(), String> {
    if ui.len() != properties.len() {
        return Err("object UI metadata must match properties".to_string());
    }
    let mut seen = HashSet::new();
    for metadata in ui {
        validate_ui(metadata)?;
        if !seen.insert(metadata.pointer.clone()) {
            return Err(format!("duplicate object UI pointer: {}", metadata.pointer));
        }
        let matches_property = properties
            .keys()
            .any(|key| format!("/{}", escape_pointer(key)) == metadata.pointer);
        if !matches_property {
            return Err(format!(
                "object UI pointer does not match properties: {}",
                metadata.pointer
            ));
        }
    }
    Ok(())
}

fn escape_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

/// 原始类型枚举，用于 builder 的 `primitive_item()` 方法，
/// 快速指定数组元素的数据类型。
///
/// 不参与 JSON 序列化，调用后立即展开为 `SchemaNode`。
/// ——调用方只需说"我要 text/number/integer/boolean 类型的数组元素"，
///
/// 覆盖所有合理的数组元素类型：
/// - 字符串（Text / Path / Color / Select）
/// - 数值（Number / Integer）
/// - 布尔（Boolean）
#[derive(Debug, Clone)]
pub enum PrimitiveType {
    /// 字符串。
    Text,
    /// 浮点数，附带可选的范围/步长约束。
    Number {
        /// 最小值。
        min: Option<f64>,
        /// 最大值。
        max: Option<f64>,
        /// 步长。
        step: Option<f64>,
    },
    /// 整数，附带可选的范围/步长约束。
    Integer {
        /// 最小值。
        min: Option<i64>,
        /// 最大值。
        max: Option<i64>,
        /// 步长。
        step: Option<i64>,
    },
    /// 布尔值。
    Boolean,
    /// 单选下拉，附带可选值列表。
    Select {
        /// 可选值列表。
        options: Vec<String>,
    },
    /// 路径选择。
    Path {
        /// 选择模式。
        mode: PathMode,
    },
    /// 颜色。
    Color,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_settings() {
        let contribution = SettingsContribution::from_entries(vec![SettingDefinition {
            key: "enabled".into(),
            schema: SchemaNode {
                kind: SchemaKind::Boolean,
                default: Some(Value::Bool(true)),
            },
            ui: FieldUiMetadata {
                pointer: "/enabled".into(),
                label: "Enabled".into(),
                description: String::new(),
                group: None,
                order: 0,
                visible: true,
                read_only: false,
                widget: None,
                action: None,
                detail_action: None,
            },
        }])
        .unwrap();
        assert!(contribution
            .validate_values(&serde_json::json!({"other": true}))
            .is_err());
    }

    #[test]
    fn rejects_mismatched_ui_pointer() {
        let result = SettingsContribution::from_entries(vec![SettingDefinition {
            key: "enabled".into(),
            schema: SchemaNode::boolean(),
            ui: FieldUiMetadata {
                pointer: "/other".into(),
                label: "Enabled".into(),
                description: String::new(),
                group: None,
                order: 0,
                visible: true,
                read_only: false,
                widget: None,
                action: None,
                detail_action: None,
            },
        }]);
        assert!(result.is_err());
    }

    #[test]
    fn collects_defaults() {
        let contribution = SettingsContribution::from_entries(vec![SettingDefinition {
            key: "theme".into(),
            schema: SchemaNode {
                kind: SchemaKind::String {
                    enum_values: vec!["light".into(), "dark".into()],
                    enum_labels: Vec::new(),
                    min_length: None,
                    max_length: None,
                    pattern: None,
                },
                default: Some(Value::String("light".into())),
            },
            ui: FieldUiMetadata {
                pointer: "/theme".into(),
                label: "Theme".into(),
                description: String::new(),
                group: None,
                order: 0,
                visible: true,
                read_only: false,
                widget: None,
                action: None,
                detail_action: None,
            },
        }])
        .unwrap();
        let defaults = contribution.default_settings();
        assert_eq!(defaults, serde_json::json!({"theme": "light"}));
    }
    /// 构造测试用的最小字段 UI 元数据。
    fn test_ui(pointer: &str) -> FieldUiMetadata {
        FieldUiMetadata {
            pointer: pointer.into(),
            label: pointer.trim_start_matches('/').into(),
            description: String::new(),
            group: None,
            order: 0,
            visible: true,
            read_only: false,
            widget: None,
            action: None,
            detail_action: None,
        }
    }

    /// 验证嵌套 object 按完整 schema 递归校验并接受合法默认值。
    #[test]
    fn validates_nested_object_schema() {
        let mut properties = BTreeMap::new();
        properties.insert("name".into(), SchemaNode::string());
        let schema = SchemaNode {
            kind: SchemaKind::Object {
                properties,
                ui: vec![test_ui("/name")],
                required: BTreeSet::from(["name".into()]),
            },
            default: Some(serde_json::json!({"name": "default"})),
        };
        let contribution = SettingsContribution::from_entries(vec![SettingDefinition {
            key: "profile".into(),
            schema,
            ui: test_ui("/profile"),
        }])
        .unwrap();
        assert!(contribution
            .validate_values(&serde_json::json!({
                "profile": {"name": "value"}
            }))
            .is_ok());
    }

    /// 验证嵌套 object 缺失 UI metadata 时被拒绝而不是静默降级。
    #[test]
    fn rejects_nested_object_without_ui_metadata() {
        let mut properties = BTreeMap::new();
        properties.insert("name".into(), SchemaNode::string());
        let result = SettingsContribution::from_entries(vec![SettingDefinition {
            key: "profile".into(),
            schema: SchemaNode {
                kind: SchemaKind::Object {
                    properties,
                    ui: Vec::new(),
                    required: BTreeSet::new(),
                },
                default: None,
            },
            ui: test_ui("/profile"),
        }]);
        assert!(result.is_err());
    }
}
