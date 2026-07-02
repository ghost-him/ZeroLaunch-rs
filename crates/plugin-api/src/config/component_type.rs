use serde::{Deserialize, Serialize};

/// 组件类型枚举，用于区分不同类型的可配置组件。
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComponentType {
    #[serde(rename = "DataSource")]
    DataSource,
    #[serde(rename = "KeywordOptimizer")]
    KeywordOptimizer,
    #[serde(rename = "SearchEngine")]
    SearchEngine,
    #[serde(rename = "ScoreBooster")]
    ScoreBooster,
    #[serde(rename = "ActionExecutor")]
    ActionExecutor,
    #[serde(rename = "Plugin")]
    Plugin,
    #[serde(rename = "Core")]
    Core,
}
