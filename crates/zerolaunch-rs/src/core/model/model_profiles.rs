use zerolaunch_plugin_api::services::model::SemanticTask;

/// 内置 embedding 模型档案：按模型 id 前缀匹配。
///
/// `task_templates` 是语义任务 → 输入模板的映射（`{0}` = input 文本，
/// 仅支持 `{0}` 占位符，其余字符原样输出）。
pub struct EmbeddingModelProfile {
    /// 模型 id 前缀（匹配 `{provider}/{name}` 中的 name 部分，如 "qwen3-embedding"）。
    pub id_prefix: &'static str,
    /// 语义任务 → 输入模板。
    pub task_templates: &'static [(SemanticTask, &'static str)],
}

/// gemma-embedding 档案模板表（Google EmbeddingGemma 官方模板）。
///
/// 查询侧 `task: {task} | query: {0}`，文档侧 `title: {title:none} | text: {0}`
/// （sentence-transformers 官方 prompts：`question: task: search result | query: `、
/// `passage_text: title: none | text: `；文档侧 title 缺省为字面量 none，
/// 调用方传 `template_args.title` 时覆盖）。
const GEMMA_TEMPLATES: &[(SemanticTask, &str)] = &[
    (
        SemanticTask::RetrievalDocument,
        "title: {title:none} | text: {0}",
    ),
    (
        SemanticTask::RetrievalQuery,
        "task: search result | query: {0}",
    ),
    (
        SemanticTask::SemanticSimilarity,
        "task: sentence similarity | query: {0}",
    ),
    (
        SemanticTask::Classification,
        "task: classification | query: {0}",
    ),
    (SemanticTask::Clustering, "task: clustering | query: {0}"),
    (SemanticTask::PlainText, "{0}"),
];

/// 内置模型档案表。
///
/// 当前收录 qwen3-embedding（Instruct 模板）与 gemma-embedding（title+text 双变量模板）。
/// 模板遵循官方文档：
/// - qwen3：查询侧 `Instruct: {task}\nQuery:{text}`（`{0}` = text），文档侧裸传
///   （官方 README 明确 "No need to add instruction for retrieval documents"）。
/// - gemma：查询侧 `task: {task} | query: {text}`，文档侧 `title: none | text: {text}`
///   （文档侧 title 为官方固定字面量 none，不引用模板参数）。
const PROFILES: &[EmbeddingModelProfile] = &[
    EmbeddingModelProfile {
        id_prefix: "qwen3-embedding",
        task_templates: &[
            (SemanticTask::RetrievalDocument, "{0}"),
            (
                SemanticTask::RetrievalQuery,
                "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery:{0}",
            ),
            (
                SemanticTask::SemanticSimilarity,
                "Instruct: Given two sentences, determine how similar they are to each other\nQuery:{0}",
            ),
            (
                SemanticTask::Classification,
                "Instruct: Given a text, classify it into the appropriate category\nQuery:{0}",
            ),
            (
                SemanticTask::Clustering,
                "Instruct: Given a text, determine the cluster it belongs to\nQuery:{0}",
            ),
            (
                SemanticTask::PlainText,
                "{0}",
            ),
        ],
    },
    EmbeddingModelProfile {
        id_prefix: "gemma-embedding",
        task_templates: GEMMA_TEMPLATES,
    },
];

/// 按模型 id 查内置档案。
///
/// 匹配规则：取模型 id 中 provider 前缀之后的 name 部分（如 `ollama/qwen3-embedding:0.6b`
/// → `qwen3-embedding:0.6b`），与档案 `id_prefix` 前缀匹配。
pub fn profile_for(model_id: &str) -> Option<&'static EmbeddingModelProfile> {
    let name = model_id.split('/').next_back().unwrap_or(model_id);
    PROFILES.iter().find(|p| name.starts_with(p.id_prefix))
}

/// 语义任务 → 模板：命中内置档案返回档案模板，否则 None（裸传）。
pub fn template_for(task: SemanticTask, model_id: &str) -> Option<&'static str> {
    profile_for(model_id).and_then(|p| {
        p.task_templates
            .iter()
            .find(|(semantic, _)| *semantic == task)
            .map(|(_, template)| *template)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_matches_qwen3_with_provider_prefix() {
        let p = profile_for("ollama/qwen3-embedding:0.6b").expect("qwen3 应有档案");
        assert_eq!(p.id_prefix, "qwen3-embedding");
    }

    #[test]
    fn profile_matches_gemma() {
        let p = profile_for("ollama/gemma-embedding:300m").expect("gemma 应有档案");
        assert_eq!(p.id_prefix, "gemma-embedding");
    }

    #[test]
    fn profile_unknown_returns_none() {
        assert!(profile_for("ollama/unknown-model").is_none());
        assert!(profile_for("openai/text-embedding-3-large").is_none());
    }

    #[test]
    fn qwen3_retrieval_query_template_is_instruct() {
        let t = template_for(SemanticTask::RetrievalQuery, "ollama/qwen3-embedding:0.6b")
            .expect("qwen3 retrieval_query 应有模板");
        assert!(t.starts_with("Instruct: "));
        assert!(t.contains("{0}"));
    }

    #[test]
    fn template_for_unknown_model_returns_none() {
        // 无档案模型：所有任务均返回 None（裸传），不再套任何前缀。
        assert_eq!(
            template_for(SemanticTask::RetrievalQuery, "ollama/unknown-model"),
            None
        );
        assert_eq!(
            template_for(SemanticTask::PlainText, "openai/text-embedding-3-large"),
            None
        );
    }
}
