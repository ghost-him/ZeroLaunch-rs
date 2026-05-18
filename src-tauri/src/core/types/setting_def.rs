use serde::{Deserialize, Serialize};

/// 组件配置项的字段定义。
/// 用于描述配置项的核心属性，可被 SettingDefinition 和 ArrayItem::Object 复用。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// MasterDetail 详情面板的联动动作定义。
/// 当用户在 MasterDetail 左侧列表选中一项时，前端据此定义：
/// - 调用哪个 config_action（带参数）
/// - 从选中项的哪个字段提取参数
/// - 将预览数据的编辑结果写入哪个兄弟设置字段
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailActionDef {
    /// 要调用的 config_action 标识符
    pub action: String,
    /// 从选中数组项中提取的字段 key，作为 params 的一部分传递给后端
    pub param_field: String,
    /// 传递给后端的参数键名（camelCase）
    pub param_key: String,
    /// 预览数据中每条记录的关联键
    pub preview_item_key: String,
    /// 预览数据中每条记录的显示键
    pub preview_item_label: String,
    /// 写入编辑结果的兄弟设置字段 key
    pub target_field: String,
    /// target_field 数组中，用于匹配预览项的键
    pub target_match_key: String,
}

/// 组件配置项的声明式定义。
/// 服务于设置存储与动态设置界面生成。
///
/// 字段语义说明：
/// - `field.default_value`: 整个设置项的默认值（如整个数组的默认内容）
/// - `FieldDefinition.default_value`（在 ArrayItem::Object 内）: 新增一行对象时，该字段的默认值模板
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettingDefinition {
    pub field: FieldDefinition,
    pub group: Option<String>,
    pub order: u32,
    /// 关联的配置动作标识符。
    /// 前端据此在配置项旁渲染操作按钮（如"自动检测"），
    /// 点击后调用该组件的 execute_config_action(action) 获取数据填充配置。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_action: Option<String>,
    /// MasterDetail 详情面板的联动动作定义。
    /// 仅对 Array + MasterDetail UI hint 有效。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail_action: Option<DetailActionDef>,
}

/// 数组元素的 UI 渲染提示。
/// 用于指导前端如何渲染数组类型的配置项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub enum ArrayItem {
    Primitive(PrimitiveType),
    Object(Vec<FieldDefinition>),
}

/// 组件设置项的输入控件类型。
/// 服务于设置表单渲染与取值校验。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    Image {
        accept: Vec<String>,
        max_size: Option<u64>,
    },
}

/// 路径选择模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PathMode {
    File,
    Directory,
}
