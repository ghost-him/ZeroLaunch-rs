//! 设置助手 agent：最小「模型 ↔ 工具」循环。
//!
//! 循环形状（对齐 CoreCoder 的教学型骨架）：
//! 用户问题 → 模型（带工具清单）→ 有 tool_calls 就执行并回填成 tool 消息 → 再问，
//! 直到模型给出纯文本答复；轮数上限兜底，超限后再补一次无工具请求强制收尾。
//!
//! 工具面：读设置（`get_settings`）、把确认后的需求或 bug 报告开成预填 issue 草稿（`open_issue_draft`）。
//! 说明书由 schema + 译文现场生成，无文件系统访问。
//! 契约：本模块不写配置；任何配置写入必须经 `ConfigManager` 公开写接口
//! （validate → apply → 副作用 → 广播 → 持久化），不得在此直调配置写路径。
//! issue 工具的外部副作用（开浏览器、写剪贴板）一律经 `PluginHandle` 转交宿主。

mod manual;
mod prompt;
mod settings;
mod tools;
mod trace;
mod wiki;

pub use manual::{ComponentNavigation, ManualHit, SettingsManual};
pub use settings::{HelpAgentSettings, HELP_AGENT_CONFIG_ID};
pub use tools::AgentTool;
pub use tools::MASK;
pub use trace::{snapshot as trace_snapshot, AgentTrace};

use crate::core::config::ConfigManager;
use crate::core::i18n::I18nManager;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tools::{GetSettingsTool, OpenIssueTool, ReadWikiTool};
use trace::{TraceMessage, TraceRound, TraceToolCall};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::model::{
    ModelChatMessage, ModelChatRequest, ModelChatResponse, ModelChatRole, ModelError, ModelService,
    ModelTool, ModelToolCall,
};

/// 单次提问允许的最大工具轮数。
const MAX_ROUNDS: usize = 12;

/// 答复中「相关设置」引用条目的数量上限。
const MAX_REFERENCES: usize = 12;

/// 一次提问的答复与可跳转引用（`help_ask` 直接下发，跨 IPC 序列化）。
#[derive(Debug, Serialize)]
pub struct AgentAnswer {
    /// 面向用户的最终答复文本（已剔除引用行）。序列化键名 `answer`。
    #[serde(rename = "answer")]
    pub answer: String,
    /// 工具调用轮里模型写下的过渡正文（按发生顺序，每条单独成一条对话消息）。
    /// 序列化键名 `intermediates`。
    #[serde(rename = "intermediates")]
    pub intermediates: Vec<String>,
    /// 有效引用（已按说明书校验，臆造项被丢弃）。序列化键名 `references`。
    #[serde(rename = "references")]
    pub references: Vec<AgentReference>,
}

/// 答复中引用到的设置位置（前端渲染为可跳转链接，跨 IPC 序列化）。
#[derive(Debug, Serialize)]
pub struct AgentReference {
    /// 组件 id。序列化键名 `componentId`。
    #[serde(rename = "componentId")]
    pub component_id: String,
    /// 组件显示名（已翻译）。序列化键名 `componentName`。
    #[serde(rename = "componentName")]
    pub component_name: String,
    /// 字段 key；空串表示只引用到组件。序列化键名 `fieldKey`。
    #[serde(rename = "fieldKey")]
    pub field_key: String,
    /// 字段标签（已翻译）；组件级引用时为空串。序列化键名 `fieldLabel`。
    #[serde(rename = "fieldLabel")]
    pub field_label: String,
}

/// 助手调用错误。
#[derive(Debug)]
pub enum AgentError {
    /// 问题为空。
    QuestionEmpty,
    /// 未配置助手模型（`help-agent-config` 的 model_id 为空）。
    ModelNotConfigured,
    /// 配置的模型不在当前模型清单中。
    ModelUnknown(String),
    /// 模型调用失败。
    Model(ModelError),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QuestionEmpty => write!(f, "问题不能为空"),
            Self::ModelNotConfigured => write!(f, "AI 助手未配置模型"),
            Self::ModelUnknown(model_id) => write!(
                f,
                "模型 {model_id} 不在当前模型清单中，请在模型配置里获取模型列表后重试"
            ),
            Self::Model(error) => write!(f, "AI 助手调用失败: {error}"),
        }
    }
}

/// 一次提问的全部输入：服务、配置来源与本次会话上下文。
///
/// 由 IPC 命令层（`commands::help`）装配后交给 `ask_from_config`；
/// `history` 已按角色白名单与长度裁剪，这里不再校验。
pub struct AskRequest<'a> {
    /// 模型服务（按 model_id 路由到提供方）。
    pub service: Arc<dyn ModelService>,
    /// 配置管理器：助手配置与说明书 schema 的来源。
    pub config: Arc<ConfigManager>,
    /// 后端翻译服务：说明书文案的来源。
    pub i18n: Arc<I18nManager>,
    /// 宿主句柄：issue 工具打开浏览器、写剪贴板。
    pub handle: Arc<PluginHandle>,
    /// 用户本次提问（处理时去除首尾空白）。
    pub question: &'a str,
    /// 各组件的设置页完整点击链与字段分组形态（前端按当前语言拼好，用于给出逐步导航）。
    pub navigation: &'a [ComponentNavigation],
    /// 组件 id → 触发方式文本（触发词与触发说明，来自插件元数据，写入说明书）。
    pub triggers: &'a HashMap<String, String>,
    /// 本次提问之前的对话（时间正序，角色仅 user/assistant）。
    pub history: Vec<ModelChatMessage>,
}

/// 按当前配置回答一个问题：解析助手模型 → 校验 → 驱动工具循环。
///
/// 参数：request - 一次提问的全部输入（见 `AskRequest`）。
/// 返回：答复与引用；问题为空、模型未配置/不在清单、或模型调用失败时返回 `AgentError`。
pub async fn ask_from_config(request: AskRequest<'_>) -> Result<AgentAnswer, AgentError> {
    let AskRequest {
        service,
        config,
        i18n,
        handle,
        question,
        navigation,
        triggers,
        history,
    } = request;
    let question = question.trim();
    if question.is_empty() {
        return Err(AgentError::QuestionEmpty);
    }
    let model_id = config
        .get_settings(HELP_AGENT_CONFIG_ID)
        .and_then(|value| serde_json::from_value::<HelpAgentSettings>(value).ok())
        .unwrap_or_default()
        .model_id
        .trim()
        .to_string();
    if model_id.is_empty() {
        return Err(AgentError::ModelNotConfigured);
    }
    if service.model_info(&model_id).is_none() {
        return Err(AgentError::ModelUnknown(model_id));
    }
    SettingsAgent::new(service, model_id, config, i18n, handle)
        .ask(question, navigation, triggers, history)
        .await
        .map_err(AgentError::Model)
}

/// 设置助手：绑定模型服务、模型 id、配置/翻译来源与宿主句柄。
pub struct SettingsAgent {
    /// 模型服务（按 model_id 路由到提供方）。
    service: Arc<dyn ModelService>,
    /// 使用的模型 id（全局 id，含 provider 前缀）。
    model_id: String,
    /// 配置管理器：说明书 schema 与设置取值的来源。
    config: Arc<ConfigManager>,
    /// 后端翻译服务：说明书文案的来源。
    i18n: Arc<I18nManager>,
    /// 宿主句柄：issue 工具的外部副作用出口（开浏览器、写剪贴板）。
    handle: Arc<PluginHandle>,
}

impl SettingsAgent {
    /// 创建设置助手。
    /// 参数：service - 模型服务；model_id - 模型全局 id；config - 配置管理器；
    /// i18n - 翻译服务；handle - 宿主句柄（供 issue 工具使用）。
    pub fn new(
        service: Arc<dyn ModelService>,
        model_id: String,
        config: Arc<ConfigManager>,
        i18n: Arc<I18nManager>,
        handle: Arc<PluginHandle>,
    ) -> Self {
        Self {
            service,
            model_id,
            config,
            i18n,
            handle,
        }
    }

    /// 回答一个问题。
    ///
    /// 参数：question - 用户提问文本；navigation - 各组件的设置页完整点击链与字段分组形态（写入说明书）；
    /// triggers - 组件 id → 触发方式文本（写入说明书）；
    /// history - 本次提问之前的对话（时间正序，插在系统消息与当前问题之间）。
    /// 返回：答复与有效引用；模型调用失败时返回 `ModelError`。
    pub async fn ask(
        &self,
        question: &str,
        navigation: &[ComponentNavigation],
        triggers: &HashMap<String, String>,
        history: Vec<ModelChatMessage>,
    ) -> Result<AgentAnswer, ModelError> {
        let mut trace = AgentTrace::new(&self.model_id);
        let result = self
            .run(question, navigation, triggers, history, &mut trace)
            .await;
        if let Err(error) = &result {
            trace.error = Some(error.to_string());
        }
        trace::push(trace);
        result
    }

    /// 驱动「模型 ↔ 工具」循环，并把每一步记入 `trace`（调试页据此复盘上下文与工具调用）。
    ///
    /// 参数：question - 用户提问文本；navigation - 各组件的设置页完整点击链与字段分组形态（写入说明书）；
    /// triggers - 组件 id → 触发方式文本（写入说明书）；
    /// history - 本次提问之前的对话；trace - 本次提问的轨迹，就地写入首轮上下文、每轮模型输出与工具结果。
    /// 返回：答复与有效引用；模型调用失败时返回 `ModelError`（已收集的上下文仍留在 `trace` 里）。
    async fn run(
        &self,
        question: &str,
        navigation: &[ComponentNavigation],
        triggers: &HashMap<String, String>,
        history: Vec<ModelChatMessage>,
        trace: &mut AgentTrace,
    ) -> Result<AgentAnswer, ModelError> {
        let manual = SettingsManual::build(&self.config, &self.i18n);
        let tools: Vec<Box<dyn AgentTool>> = vec![
            Box::new(GetSettingsTool::new(
                self.config.clone(),
                manual
                    .sensitive_keys()
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            )),
            Box::new(OpenIssueTool::new(
                self.handle.clone(),
                self.model_id.clone(),
            )),
            Box::new(ReadWikiTool),
        ];
        let mut messages = vec![ModelChatMessage::text(
            ModelChatRole::System,
            prompt::system_prompt(&manual.render(navigation, triggers), &wiki::index_text()),
        )];
        // 历史插在系统消息之后、当前问题之前：模型据此接着上一轮澄清继续谈需求
        messages.extend(history);
        messages.push(ModelChatMessage::text(ModelChatRole::User, question));
        // 首轮消息即完整上下文；后续轮次追加的助手/工具消息都记在 rounds 里
        trace.tools = tools.iter().map(|tool| tool.name().to_string()).collect();
        trace.messages = messages
            .iter()
            .map(|message| TraceMessage {
                role: message.role,
                content: message.content.clone(),
            })
            .collect();

        let mut answer: Vec<String> = Vec::new();
        let mut intermediates: Vec<String> = Vec::new();

        for _ in 0..MAX_ROUNDS {
            let response = self.chat(messages.clone(), tool_specs(&tools)).await?;
            let mut round = TraceRound {
                content: response.content.clone(),
                tool_calls: Vec::new(),
            };
            // 带工具调用的一轮里写的正文是「我先查一下」这类过渡语，
            // 单独成一条消息；只有不再调用工具的终轮正文才并入最终答复。
            if !response.content.trim().is_empty() {
                if response.tool_calls.is_empty() {
                    answer.push(response.content.clone());
                } else {
                    intermediates.push(response.content.trim().to_string());
                }
            }
            if response.tool_calls.is_empty() {
                trace.rounds.push(round);
                return Ok(Self::finish(&answer.join("\n"), &manual, intermediates));
            }
            // 助手轮次原样回填（含工具调用），保证下一轮消息序列合法
            messages.push(ModelChatMessage {
                role: ModelChatRole::Assistant,
                content: response.content.clone(),
                tool_calls: response.tool_calls.clone(),
                tool_call_id: None,
            });
            for call in &response.tool_calls {
                let output = run_tool(&tools, call).await;
                round.tool_calls.push(TraceToolCall {
                    name: call.name.clone(),
                    arguments: call.arguments.clone(),
                    result: output.clone(),
                });
                messages.push(ModelChatMessage {
                    role: ModelChatRole::Tool,
                    content: output,
                    tool_calls: Vec::new(),
                    tool_call_id: Some(call.id.clone()),
                });
            }
            trace.rounds.push(round);
        }

        // 轮数用尽仍在调工具：去掉工具再问一次，强制收尾为文本答复
        let response = self.chat(messages, Vec::new()).await?;
        trace.rounds.push(TraceRound {
            content: response.content.clone(),
            tool_calls: Vec::new(),
        });
        if !response.content.trim().is_empty() {
            answer.push(response.content);
        }
        Ok(Self::finish(&answer.join("\n"), &manual, intermediates))
    }

    /// 发起一次 chat 请求（非流式：工具调用需要完整载荷，不做分片重组）。
    async fn chat(
        &self,
        messages: Vec<ModelChatMessage>,
        tools: Vec<ModelTool>,
    ) -> Result<ModelChatResponse, ModelError> {
        self.service
            .chat(ModelChatRequest {
                model_id: self.model_id.clone(),
                messages,
                temperature: None,
                max_tokens: None,
                reasoning_effort: None,
                tools,
            })
            .await
    }

    /// 收尾：解析「相关设置」引用并构造答复。
    fn finish(content: &str, manual: &SettingsManual, intermediates: Vec<String>) -> AgentAnswer {
        let (answer, references) = split_references(content, manual);
        AgentAnswer {
            answer,
            intermediates,
            references,
        }
    }
}

/// 工具清单的模型侧声明。
fn tool_specs(tools: &[Box<dyn AgentTool>]) -> Vec<ModelTool> {
    tools.iter().map(|tool| tool.spec()).collect()
}

/// 执行一次工具调用。
///
/// 参数：tools - 本次可用工具；call - 模型给出的调用。
/// 返回：工具结果文本；未知工具、参数非法等失败同样以文本返回，由模型自行调整。
async fn run_tool(tools: &[Box<dyn AgentTool>], call: &ModelToolCall) -> String {
    let Some(tool) = tools.iter().find(|tool| tool.name() == call.name) else {
        let names: Vec<&str> = tools.iter().map(|tool| tool.name()).collect();
        return format!("未知工具 {}；可用工具：{}", call.name, names.join(", "));
    };
    // 部分提供方（如 Ollama）允许模型给出非法或不完整参数，交给工具自己兜底
    match serde_json::from_str::<serde_json::Value>(&call.arguments) {
        Ok(arguments) => tool.execute(&arguments).await,
        Err(_) => tool.execute(&serde_json::Value::Null).await,
    }
}

/// 拆出答复中的「相关设置」引用行。
///
/// 约定：行首为 `相关设置` 且其后紧跟 `:` 或 `：`（冒号必需，避免把正文误当引用），
/// 条目以 `,`/`，`/`、` 分隔，形如 `<组件id>/<字段key>` 或 `<组件id>`；
/// 只保留说明书中真实存在的位置，整行无有效位置时按正文保留。
/// 返回：剔除引用行后的答复文本 + 去重后的引用列表。
fn split_references(answer: &str, manual: &SettingsManual) -> (String, Vec<AgentReference>) {
    let mut body_lines: Vec<&str> = Vec::new();
    let mut references: Vec<AgentReference> = Vec::new();
    for line in answer.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix(prompt::REFERENCES_MARKER) else {
            body_lines.push(line);
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix([':', '：']) else {
            body_lines.push(line);
            continue;
        };
        let rest = rest.trim();
        let mut recognized = false;
        for token in rest.split([',', '，', '、', ';', '；']) {
            let token = token.trim().trim_matches(['`', '"', '\'']).trim();
            if token.is_empty() {
                continue;
            }
            let (component_id, field_key) = match token.split_once('/') {
                Some((component_id, field_key)) => (component_id.trim(), field_key.trim()),
                None => (token, ""),
            };
            let valid = if field_key.is_empty() {
                manual.has_component(component_id)
            } else {
                manual.has_field(component_id, field_key)
            };
            if !valid {
                continue;
            }
            recognized = true;
            if references.iter().any(|existing| {
                existing.component_id == component_id && existing.field_key == field_key
            }) {
                continue;
            }
            if references.len() >= MAX_REFERENCES {
                continue;
            }
            references.push(AgentReference {
                component_id: component_id.to_string(),
                component_name: manual
                    .component_name(component_id)
                    .unwrap_or_default()
                    .to_string(),
                field_key: field_key.to_string(),
                field_label: manual
                    .field_label(component_id, field_key)
                    .unwrap_or_default()
                    .to_string(),
            });
        }
        if !recognized {
            body_lines.push(line);
        }
    }
    (body_lines.join("\n").trim_end().to_string(), references)
}

#[cfg(test)]
mod tests {
    use super::*;
    use manual::{ManualComponent, ManualField};

    /// 构造含两个组件、三个字段的说明书。
    fn manual() -> SettingsManual {
        SettingsManual::from_parts(
            vec![
                ManualComponent {
                    id: "appearance-config".to_string(),
                    name: "外观".to_string(),
                    description: String::new(),
                },
                ManualComponent {
                    id: "general-config".to_string(),
                    name: "通用".to_string(),
                    description: String::new(),
                },
            ],
            vec![
                ManualField {
                    component_id: "appearance-config".to_string(),
                    component_name: "外观".to_string(),
                    key: "theme".to_string(),
                    label: "主题".to_string(),
                    description: String::new(),
                    note: String::new(),
                    group: "主题".to_string(),
                    read_only: false,
                    hidden_in_settings: false,
                    condition: String::new(),
                    value_hint: String::new(),
                    sensitive: false,
                },
                ManualField {
                    component_id: "general-config".to_string(),
                    component_name: "通用".to_string(),
                    key: "language".to_string(),
                    label: "语言".to_string(),
                    description: String::new(),
                    note: String::new(),
                    group: String::new(),
                    read_only: false,
                    hidden_in_settings: false,
                    condition: String::new(),
                    value_hint: String::new(),
                    sensitive: true,
                },
            ],
        )
    }

    /// 引用行：合法项保留（含组件级引用、全角/半角分隔符与反引号包装），臆造项与重复项丢弃。
    #[test]
    fn split_references_keeps_valid_drops_invented_and_dedups() {
        let answer = "把主题改成深色即可。\n相关设置: appearance-config/theme, fake-config/x\n\
相关设置：`general-config`、appearance-config/theme";
        let (body, references) = split_references(answer, &manual());
        assert_eq!(body, "把主题改成深色即可。");
        assert_eq!(references.len(), 2);
        assert_eq!(references[0].component_id, "appearance-config");
        assert_eq!(references[0].field_key, "theme");
        assert_eq!(references[1].component_id, "general-config");
        assert_eq!(references[1].field_key, "");
    }

    /// 正文里的「相关设置」不成引用格式时原样保留，不被吃掉。
    #[test]
    fn split_references_without_marker_keeps_answer_intact() {
        let answer = "没有任何相关设置。\n相关设置这行没冒号";
        let (body, references) = split_references(answer, &manual());
        assert_eq!(body, answer);
        assert!(references.is_empty());
    }

    /// 工具循环按名称分发到 read_wiki，并返回整页正文。
    #[tokio::test]
    async fn run_tool_dispatches_read_wiki() {
        let tools: Vec<Box<dyn AgentTool>> = vec![Box::new(ReadWikiTool)];
        let output = run_tool(
            &tools,
            &ModelToolCall {
                id: "call-1".to_string(),
                name: "read_wiki".to_string(),
                arguments: r#"{"id":"Home"}"#.to_string(),
            },
        )
        .await;
        assert!(output.starts_with("# "), "应返回手册正文");
    }
}
