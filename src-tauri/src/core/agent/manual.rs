//! 设置说明书：把配置系统的 schema 与译文整理成可读、可检索的条目。
//!
//! 数据只有两个来源：`ConfigManager`（组件与字段 schema）与 `I18nManager`（译文），
//! 不另维护第二份文案 —— schema 或文案变了，说明书随之变化。
//! 条目同时服务两个消费方：拼装模型上下文、确定性检索（UI 搜索框与 agent 共用）。

use crate::core::config::ConfigManager;
use crate::core::i18n::I18nManager;
use std::collections::HashMap;
use zerolaunch_plugin_api::config::{SchemaKind, SchemaNode};

/// 组件级说明条目。
pub struct ManualComponent {
    /// 组件 id；模型在「相关设置」行中引用它。
    pub id: String,
    /// 组件显示名（已按当前语言翻译）。
    pub name: String,
    /// 组件说明（已按当前语言翻译）。
    pub description: String,
}

/// 字段级说明条目。
pub struct ManualField {
    /// 所属组件 id。
    pub component_id: String,
    /// 所属组件显示名。
    pub component_name: String,
    /// 字段 key（settings JSON 中的键）。
    pub key: String,
    /// 字段标签（已翻译）。
    pub label: String,
    /// 字段说明（已翻译）。
    pub description: String,
    /// 字段所在分组（已翻译；空表示未分组）。界面上是组件内的标签页或分组标题。
    pub group: String,
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
                for ui in &contribution.ui {
                    // pointer 为 "/<key>"；字段 key 允许的字符集不含需要转义的字符
                    let key = ui.pointer.trim_start_matches('/').to_string();
                    fields.push(ManualField {
                        component_id: info.component_id.clone(),
                        component_name: name.clone(),
                        value_hint: contribution
                            .properties
                            .get(&key)
                            .map(|node| value_hint(node, i18n, &lang))
                            .unwrap_or_default(),
                        key,
                        label: i18n.t(&lang, &ui.label),
                        description: i18n.t(&lang, &ui.description),
                        group: ui
                            .group
                            .as_deref()
                            .map(|group| i18n.t(&lang, group))
                            .unwrap_or_default(),
                        sensitive: ui.sensitive,
                    });
                }
            }
            components.push(ManualComponent {
                id: info.component_id,
                name,
                description,
            });
        }
        Self { components, fields }
    }

    /// 渲染为模型上下文文本：组件（id + 设置页路径 + 说明）后跟该组件的字段清单。
    ///
    /// 参数：navigation - 组件 id → 设置页点击路径（由前端按当前界面语言拼好）。
    /// 缺路径的组件只渲染 id 与说明，模型无法给出逐步导航时会退回组件名。
    /// 字段行带上所在分组（`位置`）：分组在界面上是组件内的标签页或分组标题，
    /// 模型据此把这一步插进点击路径。
    pub fn render(&self, navigation: &HashMap<String, String>) -> String {
        let mut out = String::new();
        for component in &self.components {
            out.push_str(&format!("## {}({})", component.name, component.id));
            if let Some(path) = navigation.get(&component.id) {
                out.push_str(&format!("｜设置页路径：{path}"));
            }
            out.push('\n');
            if !component.description.is_empty() {
                out.push_str(&format!("{}\n", component.description));
            }
            for field in self
                .fields
                .iter()
                .filter(|field| field.component_id == component.id)
            {
                out.push_str("- ");
                out.push_str(&field.render_line());
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

    /// 敏感字段 key 集合（脱敏取值时按 key 名匹配，兼容嵌套与数组项）。
    pub fn sensitive_keys(&self) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|field| field.sensitive)
            .map(|field| field.key.as_str())
            .collect()
    }
}

#[cfg(test)]
impl SettingsManual {
    /// 测试专用构造：绕过 schema 读取直接给定条目。
    pub fn from_parts(components: Vec<ManualComponent>, fields: Vec<ManualField>) -> Self {
        Self { components, fields }
    }
}

impl ManualField {
    /// 渲染单条字段说明行：`key` 标签｜界面位置｜说明｜取值摘要。
    ///
    /// 「位置」是字段所在分组名，即界面上该字段所在的标签页或分组标题（未分组时不渲染）。
    fn render_line(&self) -> String {
        let mut parts = vec![format!("`{}` {}", self.key, self.label)];
        if !self.group.is_empty() {
            parts.push(format!("位置：{}", self.group));
        }
        if !self.description.is_empty() {
            parts.push(self.description.clone());
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

/// 字段匹配得分：key 精确 > key 前缀 > key 包含 > 标签 > 说明 > 组件名；无关返回 None。
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

    /// 构造可解析译文的管理器：写入临时语言包并固定为简体中文。
    fn test_i18n(catalog: serde_json::Value) -> I18nManager {
        let dir = std::env::temp_dir().join(format!("zl-agent-manual-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("zh-Hans.json"), catalog.to_string()).unwrap();
        let manager = I18nManager::load(dir.clone());
        manager.set_language("zh-Hans");
        std::fs::remove_dir_all(&dir).ok();
        Arc::try_unwrap(manager).unwrap_or_else(|_| unreachable!("load 返回唯一 Arc"))
    }

    /// 构造一条测试用字段条目。
    fn field(key: &str, label: &str, description: &str) -> ManualField {
        ManualField {
            component_id: "demo".to_string(),
            component_name: "演示".to_string(),
            key: key.to_string(),
            label: label.to_string(),
            description: description.to_string(),
            group: String::new(),
            value_hint: String::new(),
            sensitive: false,
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

    #[test]
    fn search_prefers_label_hit_over_description_hit() {
        let manual = manual(vec![
            field("theme", "外观模式", "界面主题深浅"),
            field("other", "主题颜色", "无关说明"),
        ]);
        let hits = manual.search("主题", 10);
        // 命中分别来自 other 的标签与 theme 的说明，标签得分更高
        assert_eq!(hits[0].field_key, "other");
        assert_eq!(hits[1].field_key, "theme");
        assert!(hits[0].score > hits[1].score);
    }

    #[test]
    fn search_matches_key_case_insensitively_and_ignores_empty_query() {
        let manual = manual(vec![field("bm25_k1", "K1", "BM25 参数")]);
        assert_eq!(manual.search("BM25_", 10)[0].field_key, "bm25_k1");
        assert!(manual.search("   ", 10).is_empty());
    }

    #[test]
    fn render_includes_component_and_field_lines() {
        let manual = manual(vec![field("theme", "主题", "界面主题")]);
        let text = manual.render(&HashMap::new());
        assert!(text.contains("## 演示(demo)"));
        assert!(text.contains("`theme` 主题｜界面主题"));
    }

    #[test]
    fn render_includes_settings_page_path_when_provided() {
        let manual = manual(vec![field("theme", "主题", "界面主题")]);
        let navigation =
            HashMap::from([("demo".to_string(), "设置 › 外观设置 › 演示".to_string())]);
        let text = manual.render(&navigation);
        assert!(
            text.contains("## 演示(demo)｜设置页路径：设置 › 外观设置 › 演示"),
            "{text}"
        );
    }

    /// 字段所在分组必须进入上下文（模型据此补出组件内的中间一步）；
    /// 未分组字段不渲染「位置」，避免模型编出不存在的一步。
    #[test]
    fn render_marks_field_position_only_when_grouped() {
        let mut grouped = field("show_subtitle", "显示副标题", "在结果项中显示副标题信息");
        grouped.group = "结果列表".to_string();
        let manual = manual(vec![grouped, field("theme", "主题", "界面主题")]);
        let text = manual.render(&HashMap::new());
        assert!(
            text.contains("`show_subtitle` 显示副标题｜位置：结果列表"),
            "{text}"
        );
        assert!(text.contains("- `theme` 主题｜界面主题"), "{text}");
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
