use serde::{Deserialize, Serialize};

/// 组件类型枚举，用于区分不同类型的可配置组件。
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComponentType {
    #[serde(rename = "DataSource")]
    DataSource,
    #[serde(rename = "KeywordOptimizer")]
    KeywordOptimizer,
    #[serde(rename = "KeywordInjector")]
    KeywordInjector,
    #[serde(rename = "SearchEngine")]
    SearchEngine,
    #[serde(rename = "ScoreBooster")]
    ScoreBooster,
    #[serde(rename = "ActionExecutor")]
    ActionExecutor,
    #[serde(rename = "Plugin")]
    Plugin,
    #[serde(rename = "BiasRule")]
    BiasRule,
    /// 结果排除规则 —— 候选采集末端的统一过滤配置（精确 / 目录 / 关键字）。
    /// 由内置 `exclusion-config` 组件声明，`SessionDispatcher` 在全量重建候选项时消费；
    /// 前端设置页按此类型归入搜索管道分类（`BiasRule` 的姊妹类型）。
    #[serde(rename = "ExclusionRule")]
    ExclusionRule,
    #[serde(rename = "Core")]
    Core,
}
