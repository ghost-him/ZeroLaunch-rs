//! embedding 请求文本组装：语义任务模板渲染。
//!
//! 组装责任在 provider（契约：不同模型输入格式不同，由 provider 内部组装）；
//! 本模块提供两个 provider 共用的模板渲染。

use super::model_profiles::template_for;
use zerolaunch_plugin_api::services::model::{
    EmbeddingCapability, EmbeddingTemplateArgs, ModelError, SemanticTask,
};

/// 替换命名占位符：`{name}` 与 `{name:default}` 两种形态。
///
/// 值缺失时 `{name:default}` 用 default，`{name}` 用空串；模板中不存在的占位符不动。
/// 未闭合的 `{name:`（无 `}`）原样保留。
fn replace_named(template: &str, name: &str, value: Option<&str>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    loop {
        let Some(pos) = rest.find(&format!("{{{}", name)) else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..pos]);
        let after = &rest[pos + name.len() + 1..];
        // 形态判定：`}`（{name}）或 `:`（{name:default}）后续闭合。
        if let Some(rest_of) = after.strip_prefix('}') {
            out.push_str(value.unwrap_or(""));
            rest = rest_of;
        } else if let Some(colon_rest) = after.strip_prefix(':') {
            match colon_rest.find('}') {
                Some(close) => {
                    let default = &colon_rest[..close];
                    out.push_str(value.unwrap_or(default));
                    rest = &colon_rest[close + 1..];
                }
                None => {
                    // 未闭合：`{name:` 原样保留。
                    out.push_str(&rest[pos..]);
                    break;
                }
            }
        } else {
            // 非占位符形态（如 {nameX）：原样保留该片段继续扫描。
            out.push_str(&rest[pos..pos + name.len() + 1]);
            rest = &rest[pos + name.len() + 1..];
        }
    }
    out
}

/// 渲染模板：`{0}` 替换为 text；`{title}` / `{title:none}` 替换为 args.title（缺省回退）。
/// 其余字符（含未声明的命名占位符与非数字占位符）原样保留。
fn render_template(template: &str, text: &str, args: Option<&EmbeddingTemplateArgs>) -> String {
    let out = template.replace("{0}", text);
    replace_named(&out, "title", args.and_then(|a| a.title.as_deref()))
}

/// 校验单项 embedding 能力声明。
///
/// 输入：模型能力清单、所需能力及其显示名称；未声明时返回 InvalidRequest。
pub(crate) fn require_embedding_capability(
    capabilities: &[EmbeddingCapability],
    capability: EmbeddingCapability,
    label: &str,
) -> Result<(), ModelError> {
    if capabilities.contains(&capability) {
        return Ok(());
    }
    Err(ModelError::InvalidRequest(format!(
        "模型未声明支持 {label} 能力"
    )))
}

/// 输入：原始文本列表、已解析的语义任务、模型 id（查内置档案）、
/// 与 input 一一对应的模板填充参数（长度不足的项视为无额外参数）。
/// 返回：按模型模板渲染后的文本列表（长度与 input 一致）。
///
/// 模板规则：命中档案且任务在档案模板表中 → 渲染（`{0}` 替换 input 文本、
/// 命名占位符替换 args 字段，缺省按占位符默认值回退）；
/// 档案缺失或档案缺该任务 → 原样透传 input。
pub(crate) fn compose_embedding_texts(
    input: &[String],
    task: SemanticTask,
    model_id: &str,
    template_args: Option<&[EmbeddingTemplateArgs]>,
) -> Result<Vec<String>, ModelError> {
    match template_for(task, model_id) {
        Some(template) => Ok(input
            .iter()
            .enumerate()
            .map(|(idx, text)| {
                render_template(template, text, template_args.and_then(|args| args.get(idx)))
            })
            .collect()),
        None => Ok(input.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_task_passes_through() {
        // plain_text 任务：qwen3 档案模板为 {0}，渲染后原文直传。
        let out = compose_embedding_texts(
            &["聊天".to_string()],
            SemanticTask::PlainText,
            "ollama/qwen3-embedding:0.6b",
            None,
        )
        .unwrap();
        assert_eq!(out, vec!["聊天".to_string()]);
    }

    #[test]
    fn unknown_model_all_tasks_pass_through() {
        // 无档案模型：所有任务一律裸传，不套任何前缀。
        for task in [SemanticTask::RetrievalQuery, SemanticTask::PlainText] {
            let out =
                compose_embedding_texts(&["聊天".to_string()], task, "ollama/unknown-model", None)
                    .unwrap();
            assert_eq!(out, vec!["聊天".to_string()]);
        }
    }

    #[test]
    fn qwen3_retrieval_query_uses_instruct_template() {
        let out = compose_embedding_texts(
            &["聊天".to_string()],
            SemanticTask::RetrievalQuery,
            "ollama/qwen3-embedding:0.6b",
            None,
        )
        .unwrap();
        assert_eq!(
            out,
            vec!["Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery:聊天".to_string()]
        );
    }

    #[test]
    fn qwen3_retrieval_document_passes_through() {
        let out = compose_embedding_texts(
            &["body".to_string()],
            SemanticTask::RetrievalDocument,
            "ollama/qwen3-embedding:0.6b",
            None,
        )
        .unwrap();
        assert_eq!(out, vec!["body".to_string()]);
    }

    #[test]
    fn gemma_document_template_uses_official_title_none() {
        // 官方模板文档侧 title 缺省为字面量 none。
        let out = compose_embedding_texts(
            &["body".to_string()],
            SemanticTask::RetrievalDocument,
            "ollama/gemma-embedding:300m",
            None,
        )
        .unwrap();
        assert_eq!(out, vec!["title: none | text: body".to_string()]);
    }

    #[test]
    fn gemma_document_template_uses_provided_title() {
        // 调用方传 template_args.title 时覆盖缺省 none。
        let args = vec![EmbeddingTemplateArgs {
            title: Some("文档标题".to_string()),
        }];
        let out = compose_embedding_texts(
            &["body".to_string()],
            SemanticTask::RetrievalDocument,
            "ollama/gemma-embedding:300m",
            Some(&args),
        )
        .unwrap();
        assert_eq!(out, vec!["title: 文档标题 | text: body".to_string()]);
    }

    #[test]
    fn template_other_placeholders_preserved_verbatim() {
        // 仅替换 {0}；{1} 与非数字占位符原样保留。
        let out = compose_embedding_texts(
            &["text".to_string()],
            SemanticTask::RetrievalQuery,
            "ollama/qwen3-embedding:0.6b",
            None,
        )
        .unwrap();
        assert!(out[0].contains("Query:text"));
    }
}
