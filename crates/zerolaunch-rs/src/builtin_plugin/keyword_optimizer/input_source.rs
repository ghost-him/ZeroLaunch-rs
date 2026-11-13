//! 关键词优化器输入来源（KeywordInputSource）的配置化辅助。
//!
//! 每个优化器把「消费哪一层关键词产物」建模为可持久化设置：
//! - `input_source`：来源选择（扁平 snake_case 字符串，见下方常量），
//!   对应 `KeywordInputSource` 四变体之一；
//! - `producer_id`：当 `input_source = optimizer_output` 时指定被引用
//!   生产者优化器的 component_id。
//!
//! 配置层用扁平字符串而非 `KeywordInputSource` 的 IPC serde 形态
//! （camelCase 嵌套对象），保证配置键值全 snake_case 且可被 schema
//! select 直接表示。本模块负责扁平值 ↔ 枚举的映射。

use zerolaunch_plugin_api::KeywordInputSource;

/// 输入来源 select 的持久化值（snake_case，与 config-naming 规则一致）。
pub const SOURCE_ORIGINAL_NAME: &str = "original_name";
pub const SOURCE_NORMALIZED_BASE: &str = "normalized_base";
pub const SOURCE_REFINED: &str = "refined";
pub const SOURCE_OPTIMIZER_OUTPUT: &str = "optimizer_output";

/// 全部可选来源值，供 schema select 的 enum 使用。
pub const ALL_SOURCES: [&str; 4] = [
    SOURCE_ORIGINAL_NAME,
    SOURCE_NORMALIZED_BASE,
    SOURCE_REFINED,
    SOURCE_OPTIMIZER_OUTPUT,
];

/// 将扁平配置值映射为 `KeywordInputSource`。
/// `optimizer_output` 需要配套的 producer_id；其余来源忽略该参数。
/// 未知来源值回退 `Refined`（幂等精化层，最安全语义），
/// 使损坏/过期配置不致让优化器失效。
pub fn resolve_input_source(input_source: &str, producer_id: &str) -> KeywordInputSource {
    match input_source {
        SOURCE_ORIGINAL_NAME => KeywordInputSource::OriginalName,
        SOURCE_NORMALIZED_BASE => KeywordInputSource::NormalizedBase,
        // producer_id 为空（如迁移遗留的旧 JSON 缺该字段）时，
        // 不构造指向空 id 的引用（查无产物即静默失效），回退 refined。
        SOURCE_OPTIMIZER_OUTPUT if !producer_id.is_empty() => KeywordInputSource::OptimizerOutput {
            producer_id: producer_id.to_string(),
        },
        SOURCE_OPTIMIZER_OUTPUT => KeywordInputSource::Refined,
        // refined 与未知值
        _ => KeywordInputSource::Refined,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_flat_source_to_enum() {
        assert_eq!(
            resolve_input_source(SOURCE_ORIGINAL_NAME, ""),
            KeywordInputSource::OriginalName
        );
        assert_eq!(
            resolve_input_source(SOURCE_NORMALIZED_BASE, ""),
            KeywordInputSource::NormalizedBase
        );
        assert_eq!(
            resolve_input_source(SOURCE_REFINED, ""),
            KeywordInputSource::Refined
        );
        assert_eq!(
            resolve_input_source(SOURCE_OPTIMIZER_OUTPUT, "pinyin-converter"),
            KeywordInputSource::OptimizerOutput {
                producer_id: "pinyin-converter".to_string()
            }
        );
    }

    #[test]
    fn unknown_source_falls_back_to_refined() {
        assert_eq!(
            resolve_input_source("corrupted-value", ""),
            KeywordInputSource::Refined
        );
        // 空来源也回退 refined，而非 optimizer_output 缺 producer 的空引用
        assert_eq!(resolve_input_source("", ""), KeywordInputSource::Refined);
    }

    #[test]
    fn optimizer_output_with_empty_producer_falls_back_to_refined() {
        // 迁移遗留/畸形配置：optimizer_output 但 producer_id 为空，
        // 回退 refined，避免构造查无产物的空引用导致优化器静默失效。
        assert_eq!(
            resolve_input_source(SOURCE_OPTIMIZER_OUTPUT, ""),
            KeywordInputSource::Refined
        );
    }
}
