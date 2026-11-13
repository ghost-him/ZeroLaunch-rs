pub mod first_letter_extractor;
pub mod input_source;
pub mod pinyin_converter;
pub mod space_normalizer;
pub mod space_remover;
pub mod symbol_remover;
pub mod upper_case_letter_extractor;
pub mod version_number_remover;

pub use first_letter_extractor::FirstLetterExtractor;
pub use pinyin_converter::PinyinConverter;
pub use space_normalizer::SpaceNormalizer;
pub use space_remover::SpaceRemover;
pub use symbol_remover::SymbolRemover;
pub use upper_case_letter_extractor::UpperCaseLetterExtractor;
pub use version_number_remover::VersionNumberRemover;

#[cfg(test)]
mod tests {
    use super::first_letter_extractor::FirstLetterExtractor;
    use super::pinyin_converter::PinyinConverter;
    use super::space_normalizer::SpaceNormalizer;
    use super::space_remover::SpaceRemover;
    use super::symbol_remover::SymbolRemover;
    use super::upper_case_letter_extractor::UpperCaseLetterExtractor;
    use super::version_number_remover::VersionNumberRemover;
    use zerolaunch_plugin_api::config::Configurable;
    use zerolaunch_plugin_api::KeywordInputSource;
    use zerolaunch_plugin_api::KeywordOptimizer;

    /// 收集各优化器并验证默认输入来源与 schema 一致性。
    fn collect() -> Vec<(String, Box<dyn KeywordOptimizer>, KeywordInputSource)> {
        vec![
            (
                "first-letter-extractor".into(),
                Box::new(FirstLetterExtractor::new()),
                KeywordInputSource::OptimizerOutput {
                    producer_id: "pinyin-converter".to_string(),
                },
            ),
            (
                "pinyin-converter".into(),
                Box::new(PinyinConverter::new()),
                KeywordInputSource::NormalizedBase,
            ),
            (
                "space-normalizer".into(),
                Box::new(SpaceNormalizer::new()),
                KeywordInputSource::NormalizedBase,
            ),
            (
                "space-remover".into(),
                Box::new(SpaceRemover::new()),
                KeywordInputSource::Refined,
            ),
            (
                "symbol-remover".into(),
                Box::new(SymbolRemover::new()),
                KeywordInputSource::Refined,
            ),
            (
                "upper-case-letter-extractor".into(),
                Box::new(UpperCaseLetterExtractor::new()),
                KeywordInputSource::OriginalName,
            ),
            (
                "version-number-remover".into(),
                Box::new(VersionNumberRemover::new()),
                KeywordInputSource::NormalizedBase,
            ),
        ]
    }

    /// 所有优化器默认 input_source 与改动前硬编码值一致（老用户零感知），
    /// schema 与 settings 均含 input_source/producer_id 字段且默认一致。
    #[test]
    fn optimizer_input_source_defaults_and_schema_align() {
        for (id, opt, expected) in collect() {
            assert_eq!(opt.input_source(), expected, "{id} 默认来源");
            let settings = opt.get_settings();
            assert!(
                settings.get("input_source").is_some(),
                "{id} settings 缺 input_source"
            );
            assert!(
                settings.get("producer_id").is_some(),
                "{id} settings 缺 producer_id"
            );
            let schema = opt.setting_schema();
            let keys: Vec<&str> = schema.iter().map(|d| d.key.as_str()).collect();
            assert!(
                keys.contains(&"input_source"),
                "{id} schema 缺 input_source"
            );
            assert!(keys.contains(&"producer_id"), "{id} schema 缺 producer_id");
            let input_schema = schema.iter().find(|d| d.key == "input_source").unwrap();
            let default = input_schema
                .schema
                .default
                .clone()
                .unwrap_or_else(|| panic!("{id} input_source schema default"));
            assert_eq!(
                default.as_str().unwrap(),
                settings["input_source"].as_str().unwrap(),
                "{id} schema default 与 settings 不一致"
            );
            // producer_id 字段应仅在 input_source = optimizer_output 时可见
            let producer_schema = schema.iter().find(|d| d.key == "producer_id").unwrap();
            let visible_when = producer_schema
                .ui
                .visible_when
                .clone()
                .unwrap_or_else(|| panic!("{id} producer_id 应配置 visible_when"));
            assert_eq!(visible_when.field, "input_source");
            assert_eq!(visible_when.value.as_str().unwrap(), "optimizer_output");
        }
    }

    /// 应用用户自定义来源后 input_source() 反映新配置。
    #[tokio::test]
    async fn optimizer_input_source_applies_custom_config() {
        // 单测直连 apply_settings 需 Configurable：此处经 trait 对象调用。
        let pinyin = PinyinConverter::new();
        let configurable: &dyn Configurable = &pinyin;
        configurable
            .apply_settings(serde_json::json!({
                "priority": 25,
                "input_source": "refined",
                "producer_id": ""
            }))
            .await
            .unwrap();
        assert_eq!(pinyin.input_source(), KeywordInputSource::Refined);

        let first_letter = FirstLetterExtractor::new();
        let configurable: &dyn Configurable = &first_letter;
        configurable
            .apply_settings(serde_json::json!({
                "priority": 50,
                "input_source": "optimizer_output",
                "producer_id": "space-remover"
            }))
            .await
            .unwrap();
        assert_eq!(
            first_letter.input_source(),
            KeywordInputSource::OptimizerOutput {
                producer_id: "space-remover".to_string()
            }
        );
    }
}
