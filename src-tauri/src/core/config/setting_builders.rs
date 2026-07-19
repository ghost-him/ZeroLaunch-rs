//! Schema 构建器 — 链式调用 API。
//!
//! 提供统一的 `SchemaBuilder` 来构建 `SettingDefinition` 和 `FieldDefinition`，
//! 覆盖全部 8 种 SettingType，消除旧自由函数的位置参数混乱。
//!
//! # 使用示例
//!
//! ```ignore
//! // 简单文本字段
//! SchemaBuilder::text("key", "Label", "Description")
//!     .group("Group").order(0).default("default").build()
//!
//! // 带约束的数值字段
//! SchemaBuilder::number("height", "Height", "Height in px")
//!     .group("Layout").order(1).default(72.0)
//!     .min(40.0).max(120.0).step(1.0).build()
//!
//! // Array + Object + config_action
//! SchemaBuilder::array("sources", "Sources", "Browser sources")
//!     .group("Sources").order(2).config_action("detect_browsers")
//!     .object_items(vec![
//!         SchemaBuilder::text("name", "Name", "Name").default("").build_field(),
//!     ])
//!     .master_detail().default(serde_json::json!([])).build()
//! ```

use serde_json::Value;
use zerolaunch_plugin_api::config::PrimitiveType;
use zerolaunch_plugin_api::config::{
    ArrayItem, ArrayUiHint, DetailActionDef, FieldDefinition, PathMode, SettingDefinition,
    SettingType,
};

pub struct SchemaBuilder {
    field_def: FieldDefinition,
    group: Option<String>,
    order: u32,
    config_action: Option<String>,
    detail_action: Option<DetailActionDef>,
}

impl SchemaBuilder {
    // ── constructors ──────────────────────────────────────────────

    pub fn text(key: &str, label: &str, desc: &str) -> Self {
        Self::new(key, label, desc, SettingType::Text)
    }

    pub fn number(key: &str, label: &str, desc: &str) -> Self {
        Self::new(
            key,
            label,
            desc,
            SettingType::Number {
                min: None,
                max: None,
                step: None,
            },
        )
    }

    pub fn boolean(key: &str, label: &str, desc: &str) -> Self {
        Self::new(key, label, desc, SettingType::Boolean)
    }

    pub fn select(key: &str, label: &str, desc: &str) -> Self {
        Self::new(key, label, desc, SettingType::Select { options: vec![] })
    }

    pub fn color(key: &str, label: &str, desc: &str) -> Self {
        Self::new(key, label, desc, SettingType::Color)
    }

    pub fn path(key: &str, label: &str, desc: &str) -> Self {
        Self::new(
            key,
            label,
            desc,
            SettingType::Path {
                mode: PathMode::File,
            },
        )
    }

    pub fn json(key: &str, label: &str, desc: &str) -> Self {
        Self::new(key, label, desc, SettingType::Json)
    }

    pub fn array(key: &str, label: &str, desc: &str) -> Self {
        Self::new(
            key,
            label,
            desc,
            SettingType::Array {
                item: ArrayItem::Primitive(PrimitiveType::Text),
                min_items: None,
                max_items: None,
                ui_hint: ArrayUiHint::Default,
            },
        )
    }

    pub fn image(key: &str, label: &str, desc: &str) -> Self {
        Self::new(
            key,
            label,
            desc,
            SettingType::Image {
                accept: vec!["png".into(), "jpg".into(), "jpeg".into(), "webp".into()],
                max_size: Some(2 * 1024 * 1024),
            },
        )
    }

    fn new(key: &str, label: &str, desc: &str, setting_type: SettingType) -> Self {
        Self {
            field_def: FieldDefinition {
                key: key.to_string(),
                label: label.to_string(),
                description: desc.to_string(),
                setting_type,
                default_value: Value::Null,
                visible: true,
                editable: true,
                config_action: None,
            },
            group: None,
            order: 0,
            config_action: None,
            detail_action: None,
        }
    }

    // ── universal methods ─────────────────────────────────────────

    pub fn group(mut self, group: &str) -> Self {
        self.group = Some(group.to_string());
        self
    }

    pub fn order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }

    pub fn config_action(mut self, action: &str) -> Self {
        self.config_action = Some(action.to_string());
        self
    }

    /// 为 MasterDetail 数组配置详情面板联动动作。
    /// 选中列表项时，前端将调用指定的 config_action，
    /// 并从选中项中提取指定字段作为参数，将用户编辑结果写入指定的兄弟设置字段。
    pub fn detail_action(mut self, def: DetailActionDef) -> Self {
        self.detail_action = Some(def);
        self
    }

    pub fn default(mut self, value: impl Into<Value>) -> Self {
        self.field_def.default_value = value.into();
        self
    }

    pub fn visible(mut self, v: bool) -> Self {
        self.field_def.visible = v;
        self
    }

    pub fn editable(mut self, v: bool) -> Self {
        self.field_def.editable = v;
        self
    }

    // ── Number ────────────────────────────────────────────────────

    pub fn min(mut self, v: f64) -> Self {
        if let SettingType::Number { min, .. } = &mut self.field_def.setting_type {
            *min = Some(v);
        }
        self
    }

    pub fn max(mut self, v: f64) -> Self {
        if let SettingType::Number { max, .. } = &mut self.field_def.setting_type {
            *max = Some(v);
        }
        self
    }

    pub fn step(mut self, v: f64) -> Self {
        if let SettingType::Number { step, .. } = &mut self.field_def.setting_type {
            *step = Some(v);
        }
        self
    }

    // ── Select ────────────────────────────────────────────────────

    pub fn options(mut self, options: &[&str]) -> Self {
        if let SettingType::Select { options: opts } = &mut self.field_def.setting_type {
            *opts = options.iter().map(|s| s.to_string()).collect();
        }
        self
    }

    // ── Path ──────────────────────────────────────────────────────

    pub fn file(mut self) -> Self {
        if let SettingType::Path { mode } = &mut self.field_def.setting_type {
            *mode = PathMode::File;
        }
        self
    }

    pub fn directory(mut self) -> Self {
        if let SettingType::Path { mode } = &mut self.field_def.setting_type {
            *mode = PathMode::Directory;
        }
        self
    }

    // ── Array ─────────────────────────────────────────────────────

    pub fn primitive_item(mut self, item: PrimitiveType) -> Self {
        if let SettingType::Array {
            item: arr_item,
            ui_hint,
            ..
        } = &mut self.field_def.setting_type
        {
            if matches!(ui_hint, ArrayUiHint::Tags) {
                debug_assert!(
                    matches!(item, PrimitiveType::Text),
                    "tags_ui is only supported for Text primitive arrays, cannot set primitive_item to {:?}",
                    item
                );
            }
            *arr_item = ArrayItem::Primitive(item);
        }
        self
    }

    pub fn object_items(mut self, items: Vec<FieldDefinition>) -> Self {
        if let SettingType::Array { item: arr_item, .. } = &mut self.field_def.setting_type {
            *arr_item = ArrayItem::Object(items);
        }
        self
    }

    pub fn min_items(mut self, n: usize) -> Self {
        if let SettingType::Array { min_items, .. } = &mut self.field_def.setting_type {
            *min_items = Some(n);
        }
        self
    }

    pub fn max_items(mut self, n: usize) -> Self {
        if let SettingType::Array { max_items, .. } = &mut self.field_def.setting_type {
            *max_items = Some(n);
        }
        self
    }

    pub fn default_ui(mut self) -> Self {
        if let SettingType::Array { ui_hint, .. } = &mut self.field_def.setting_type {
            *ui_hint = ArrayUiHint::Default;
        }
        self
    }

    pub fn table_ui(mut self) -> Self {
        if let SettingType::Array { ui_hint, .. } = &mut self.field_def.setting_type {
            *ui_hint = ArrayUiHint::Table;
        }
        self
    }

    pub fn master_detail(mut self) -> Self {
        if let SettingType::Array { ui_hint, .. } = &mut self.field_def.setting_type {
            *ui_hint = ArrayUiHint::MasterDetail;
        }
        self
    }

    /// 切换到 Tags UI。仅支持 Text 类型的 primitive array，
    /// 因为 Naive UI 的 n-dynamic-tags 组件本质上是基于字符串的。
    pub fn tags_ui(mut self) -> Self {
        if let SettingType::Array { item, ui_hint, .. } = &mut self.field_def.setting_type {
            debug_assert!(
                matches!(item, ArrayItem::Primitive(PrimitiveType::Text)),
                "tags_ui is only supported for Text primitive arrays"
            );
            *ui_hint = ArrayUiHint::Tags;
        }
        self
    }

    /// 切换到 SearchTable UI。用于搜索已索引程序并配置别名的数组。
    /// `source_component` 是提供搜索服务的组件 ID（如 "candidate-registry"），
    /// `source_action` 是搜索动作名（如 "search_candidates"）。
    /// `field_mapping` 是候选结果字段到编辑表单字段的映射，每项 `(candidateField, formField)`。
    pub fn search_table_ui(
        mut self,
        source_component: &str,
        source_action: &str,
        field_mapping: &[(&str, &str)],
    ) -> Self {
        if let SettingType::Array { ui_hint, .. } = &mut self.field_def.setting_type {
            *ui_hint = ArrayUiHint::SearchTable {
                source_component: source_component.to_string(),
                source_action: source_action.to_string(),
                field_mapping: field_mapping
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            };
        }
        self
    }

    // ── Image ──────────────────────────────────────────────────────

    pub fn accept(mut self, formats: &[&str]) -> Self {
        if let SettingType::Image { accept, .. } = &mut self.field_def.setting_type {
            *accept = formats.iter().map(|s| s.to_string()).collect();
        }
        self
    }

    pub fn max_image_size(mut self, bytes: u64) -> Self {
        if let SettingType::Image { max_size, .. } = &mut self.field_def.setting_type {
            *max_size = Some(bytes);
        }
        self
    }

    // ── build ─────────────────────────────────────────────────────

    pub fn build(self) -> SettingDefinition {
        SettingDefinition {
            field: self.field_def,
            group: self.group,
            order: self.order,
            config_action: self.config_action,
            detail_action: self.detail_action,
        }
    }

    pub fn build_field(self) -> FieldDefinition {
        self.field_def
    }
}
