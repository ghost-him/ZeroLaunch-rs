//! 固定偏移量（bias）配置的数据结构与转换函数 —— 核心配置类型。
//!
//! 从 `builtin_plugin/config/bias_config.rs` 下沉至此（2026-08-05 架构修复）：
//! 配置类型按 P1 放置约定属于 core/config/（L2），供 L3 框架层
//! （`SessionDispatcher` 管道重建）与 L6 编排层（`bootstrap` 启动加载）向下引用，
//! 消除 plugin_framework → builtin_plugin 的反向依赖。
//! 持久化键名与前端 SearchTable schema 一致（entries/bias/target/note），迁移不改契约。

use serde::{Deserialize, Serialize};

use crate::core::bias_rule::BiasRule;

/// 固定偏移量配置的根结构 —— 反序列化自 `bias-config` 组件的持久化设置。
///
/// 由 `BiasConfig`（builtin_plugin 侧 Configurable 组件）读写；
/// 由 `SessionDispatcher::rebuild_candidate_pipeline` 消费并转换为 `BiasRule`。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiasSettings {
    /// 偏移量规则条目列表（SearchTable UI 编辑，可为空）。
    #[serde(rename = "entries", default)]
    pub entries: Vec<BiasEntry>,
}

/// 单条固定偏移量规则 —— `BiasSettings.entries` 的元素。
///
/// 目标程序标识匹配候选项 payload，权重偏移在采集管道注入阶段生效。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasEntry {
    /// 目标程序标识，匹配 candidate.target.payload()。
    /// 由前端 SearchTable UI 自动填充，apply_settings 时归一化为 to_ascii_lowercase()。
    #[serde(rename = "target", default)]
    pub target: String,
    /// 权重偏移值，正值提升搜索结果位置，负值降低。
    /// 取值范围 [-10.0, 10.0]（schema 约束），缺省 0.0。
    #[serde(rename = "bias", default = "BiasEntry::default_bias")]
    pub bias: f64,
    /// 备注信息（可选，纯展示用途）。
    #[serde(rename = "note", default)]
    pub note: String,
}

impl BiasEntry {
    fn default_bias() -> f64 {
        0.0
    }
}

/// 将固定偏移量配置转换为管道注入用的 `BiasRule` 列表。
///
/// 纯函数（无 ConfigManager/插件 API 依赖）：读取与 from_value 解析由调用方
/// （SessionDispatcher / bootstrap）完成；此处仅做形状转换与 target 归一化。
pub(crate) fn bias_settings_to_rules(settings: &BiasSettings) -> Vec<BiasRule> {
    settings
        .entries
        .iter()
        .map(|e| BiasRule {
            target: e.target.to_ascii_lowercase(),
            bias: e.bias,
        })
        .collect()
}
