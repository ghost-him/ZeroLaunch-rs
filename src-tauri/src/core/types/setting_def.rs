use serde::{Deserialize, Serialize};

/// 组件配置项的字段定义。
/// 用于描述配置项的核心属性，可被 SettingDefinition 和 ArrayItem::Object 复用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub key: String,
    pub label: String,
    pub description: String,
    pub setting_type: SettingType,
    pub default_value: serde_json::Value,
    pub visible: bool,
    pub editable: bool,
}

impl Default for FieldDefinition {
    fn default() -> Self {
        Self {
            key: String::new(),
            label: String::new(),
            description: String::new(),
            setting_type: SettingType::Text,
            default_value: serde_json::Value::Null,
            visible: true,
            editable: true,
        }
    }
}

/// 组件配置项的声明式定义。
/// 服务于设置存储与动态设置界面生成。
///
/// 字段语义说明：
/// - `field.default_value`: 整个设置项的默认值（如整个数组的默认内容）
/// - `FieldDefinition.default_value`（在 ArrayItem::Object 内）: 新增一行对象时，该字段的默认值模板
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingDefinition {
    pub field: FieldDefinition,
    pub group: Option<String>,
    pub order: u32,
    /// 关联的配置动作标识符。
    /// 前端据此在配置项旁渲染操作按钮（如"自动检测"），
    /// 点击后调用该组件的 execute_config_action(action) 获取数据填充配置。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_action: Option<String>,
}

/// 数组元素的 UI 渲染提示。
/// 用于指导前端如何渲染数组类型的配置项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ArrayUiHint {
    #[default]
    Default,
    Table,
    MasterDetail,
    Tags,
}

/// 原始类型枚举，用于数组元素的类型定义。
/// 与 SettingType 类似，但不包含复合类型（Array）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimitiveType {
    Text,
    Number {
        min: Option<f64>,
        max: Option<f64>,
        step: Option<f64>,
    },
    Boolean,
    Select {
        options: Vec<String>,
    },
    Path {
        mode: PathMode,
    },
    Color,
}

/// 数组元素类型定义。
/// 用于区分数组元素是原始类型还是对象类型。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArrayItem {
    Primitive(PrimitiveType),
    Object(Vec<FieldDefinition>),
}

/// 组件设置项的输入控件类型。
/// 服务于设置表单渲染与取值校验。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettingType {
    Text,
    Number {
        min: Option<f64>,
        max: Option<f64>,
        step: Option<f64>,
    },
    Boolean,
    Select {
        options: Vec<String>,
    },
    Path {
        mode: PathMode,
    },
    Color,
    Json,
    Array {
        item: ArrayItem,
        min_items: Option<usize>,
        max_items: Option<usize>,
        ui_hint: ArrayUiHint,
    },
}

/// 路径选择模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PathMode {
    File,
    Directory,
}
