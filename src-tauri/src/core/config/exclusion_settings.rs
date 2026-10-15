//! 结果排除（exclusion）配置的存储类型与转换函数 —— 核心配置类型。
//!
//! 与 `bias_settings` 同层（P1：配置数据的存储类型/转换函数属于 `core/config/`）：
//! 供 L3 框架层（`plugin_framework::candidate_filter`）与 L6 编排层
//! （`SessionDispatcher` 读取 `exclusion-config`）向下引用，
//! 消除 `plugin_framework` → `builtin_plugin` 的反向依赖。
//! 持久化键名与前端 SearchTable schema 一致（rules / kind / target / note）。

use serde::{Deserialize, Serialize};

use crate::core::exclusion_rule::{normalize_path, normalize_text, ExclusionRules};

/// 排除规则的匹配方式 —— 持久化于 `exclusion-config` 的 `rules[].kind`。
///
/// 由设置页下拉框写入（序列化值 exact / path_prefix / contains），
/// 经转换函数决定规则值以何种方式参与候选比较。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ExclusionKind {
    /// 精确匹配 —— 候选执行目标与规则值全等（默认）。用于“隐藏某一条具体结果”。
    #[default]
    #[serde(rename = "exact")]
    Exact,
    /// 目录前缀 —— 候选位于该目录或其子目录（仅文件/路径目标）。用于“禁止某个文件夹”。
    #[serde(rename = "path_prefix")]
    PathPrefix,
    /// 关键字包含 —— 候选执行目标或显示名包含该子串。用于“形如 xxx 的一律排除”。
    #[serde(rename = "contains")]
    Contains,
}

/// 单条排除规则（存储形态）—— `exclusion-config` 的 `rules[]` 元素。
///
/// 由设置页 SearchTable UI 增删改。规则值字段沿用 `bias-config` 的 `target` 命名：
/// 设置页数据注入 action（`candidate-registry.search_candidates`）以 `target`
/// 作为结果值字段，而前端要求条目 schema 字段名与该字段一致（校验与回填都用它）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExclusionRuleEntry {
    /// 匹配方式，缺失时回退 `Exact`（右键隐藏写入的条目只带 target）。
    #[serde(rename = "kind", default)]
    pub kind: ExclusionKind,
    /// 规则值：目标路径/标识（exact、path_prefix）或关键字（contains）。
    /// 空串或纯空白条目在转换时被丢弃。
    #[serde(rename = "target", default)]
    pub target: String,
    /// 备注信息（可选，纯展示用途，不参与匹配）。
    #[serde(rename = "note", default)]
    pub note: String,
}

/// 结果排除配置的根结构 —— 反序列化自 `exclusion-config` 组件的持久化设置。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExclusionSettings {
    /// 规则条目列表。空列表表示不排除任何候选项。
    #[serde(rename = "rules", default)]
    pub rules: Vec<ExclusionRuleEntry>,
}

/// 将持久化配置转换为运行时排除规则集（纯函数，无 IO/无 ConfigManager）。
/// 参数：settings - `exclusion-config` 的持久化设置。
/// 返回：归一化并去重后的规则集；空串/纯空白条目被丢弃。
pub(crate) fn exclusion_settings_to_rules(settings: &ExclusionSettings) -> ExclusionRules {
    let mut rules = ExclusionRules::default();
    for entry in &settings.rules {
        let value = entry.target.trim();
        if value.is_empty() {
            continue;
        }
        match entry.kind {
            // 精确规则同时登记文本形态与路径归一化形态：前者覆盖非路径目标（App/Url/Command），
            // 后者覆盖文件/路径目标（容忍大小写、`/`↔`\` 与尾部分隔符差异）。
            ExclusionKind::Exact => {
                push_unique(&mut rules.exact, normalize_text(value));
                push_unique(&mut rules.exact_path, normalize_path(value));
            }
            ExclusionKind::PathPrefix => {
                push_unique(&mut rules.path_prefixes, normalize_path(value))
            }
            ExclusionKind::Contains => push_unique(&mut rules.contains, normalize_text(value)),
        }
    }
    rules
}

/// 追加去重：值已存在时跳过（配置表可能含重复条目，重复只会增加比较开销）。
fn push_unique(list: &mut Vec<String>, value: String) {
    if !list.contains(&value) {
        list.push(value);
    }
}
