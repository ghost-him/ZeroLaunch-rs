use serde::{Deserialize, Serialize};
/// 配置动作声明，描述组件对外提供的可执行动作。
///
/// 仅由配置管理边界和字段动作绑定引用；具体执行仍由 Configurable 实现负责。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigActionDef {
    /// 动作唯一标识符，如 "detect_browsers"。
    #[serde(rename = "action", default)]
    pub action: String,
    /// 动作显示名称，用于设置界面按钮文本。
    #[serde(rename = "label", default)]
    pub label: String,
    /// 动作描述，用于解释动作效果。
    #[serde(rename = "description", default)]
    pub description: String,
}

/// 字段级动作绑定，区分数据注入和用户触发的副作用动作。
///
/// 仅作为 Settings schema 的 UI metadata 使用，不应被 core 配置逻辑直接依赖。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "binding")]
pub enum FieldAction {
    /// 查询或检测数据，并将返回值注入字段或兄弟字段。
    #[serde(rename = "data")]
    Data(DataActionBinding),
    /// 用户显式触发的轻量副作用动作，例如写入图标缓存。
    #[serde(rename = "effect")]
    Effect(EffectActionBinding),
}

/// 数据注入动作绑定，描述如何从 action 返回值填充设置字段。
///
/// 仅由 schema builder、IPC schema 和前端字段动作按钮使用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataActionBinding {
    /// 动作标识符，对应 `ConfigActionDef.action`。
    #[serde(rename = "action", default)]
    pub action: String,
    /// 动作所属组件 ID；None 表示当前组件。
    #[serde(rename = "component", default)]
    pub component: Option<String>,
    /// action 返回结果中用作显示标签的字段名。
    #[serde(rename = "labelField", default)]
    pub label_field: String,
    /// labelField 列的表头显示文本；labelField 在条目 schema 中无对应字段时使用。
    #[serde(rename = "labelFieldLabel", default)]
    pub label_field_label: String,
    /// action 返回结果中用作字段值的字段名。
    #[serde(rename = "valueField", default)]
    pub value_field: String,
    /// 返回结果字段到设置字段的映射，格式为 source → target。
    #[serde(rename = "fieldMapping", default)]
    pub field_mapping: Vec<(String, String)>,
}

/// 用户触发的副作用动作绑定，描述动作参数来源和临时字段语义。
///
/// 仅由设置字段渲染层调用；动作必须通过 `config_execute_action` 执行，
/// 不参与 staged/immediate 配置提交，也不修改组件 settings。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectActionBinding {
    /// 动作标识符，对应 `ConfigActionDef.action`。
    #[serde(rename = "action", default)]
    pub action: String,
    /// 动作所属组件 ID；None 表示当前组件。
    #[serde(rename = "component", default)]
    pub component: Option<String>,
    /// 表单字段到 action 参数的映射，格式为 source → target；为空时传递当前表单值。
    #[serde(rename = "fieldMapping", default)]
    pub field_mapping: Vec<(String, String)>,
    /// 是否只用于动作参数而不应写入持久化 settings。
    #[serde(rename = "transient", default)]
    pub transient: bool,
}

/// MasterDetail 详情面板的联动动作定义。
///
/// 仅由 MasterDetail schema 和详情预览组件使用；动作结果用于生成预览数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailActionDef {
    /// 选中左侧列表项时调用的动作名。
    #[serde(rename = "action", default)]
    pub action: String,
    /// 从选中项提取参数的字段名。
    #[serde(rename = "paramField", default)]
    pub param_field: String,
    /// 传给 action 的参数名。
    #[serde(rename = "paramKey", default)]
    pub param_key: String,
    /// 预览数据项的唯一标识字段。
    #[serde(rename = "previewItemKey", default)]
    pub preview_item_key: String,
    /// 预览数据项的显示标题字段。
    #[serde(rename = "previewItemLabel", default)]
    pub preview_item_label: String,
    /// 详情编辑结果写入的兄弟设置字段。
    #[serde(rename = "targetField", default)]
    pub target_field: String,
    /// 用于匹配已有覆盖项的字段名。
    #[serde(rename = "targetMatchKey", default)]
    pub target_match_key: String,
}

#[cfg(test)]
mod tests {
    use super::{DataActionBinding, EffectActionBinding, FieldAction};

    /// 验证字段 action 使用与 TypeScript contract 一致的 kind/binding 形状。
    #[test]
    fn field_action_uses_discriminated_shape() {
        let data = serde_json::to_value(FieldAction::Data(DataActionBinding {
            action: "search_candidates".into(),
            component: Some("candidate-registry".into()),
            label_field: "name".into(),
            label_field_label: "名称".into(),
            value_field: "target".into(),
            field_mapping: vec![("iconRequestJson".into(), "icon_request_json".into())],
        }))
        .expect("Data action must serialize");
        assert_eq!(data["kind"], "data");
        assert_eq!(data["binding"]["labelField"], "name");
        assert_eq!(data["binding"]["labelFieldLabel"], "名称");
        assert_eq!(data["binding"]["valueField"], "target");
        assert_eq!(data["binding"]["fieldMapping"][0][0], "iconRequestJson");

        let effect = serde_json::to_value(FieldAction::Effect(EffectActionBinding {
            action: "apply_override".into(),
            component: None,
            field_mapping: vec![("custom_icon_path".into(), "custom_icon_path".into())],
            transient: true,
        }))
        .expect("Effect action must serialize");
        assert_eq!(effect["kind"], "effect");
        assert_eq!(effect["binding"]["transient"], true);
        assert_eq!(effect["binding"]["fieldMapping"][0][1], "custom_icon_path");
    }
}
