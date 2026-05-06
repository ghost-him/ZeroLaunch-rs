use serde::{Deserialize, Serialize};

/// 组件类型枚举，用于区分不同类型的可配置组件。
/// 序列化为 PascalCase 以匹配前端 TypeScript 类型定义。
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComponentType {
    DataSource,
    KeywordOptimizer,
    SearchEngine,
    ScoreBooster,
    ActionExecutor,
    Plugin,
    Core,
}
