//! 设置助手调试轨迹：记录每次提问下发给模型的上下文与工具调用结果。
//!
//! 只在内存里保留最近 `MAX_TRACES` 次提问（环形缓冲），供设置窗调试页查看；
//! 唯一写者是 `SettingsAgent::ask`，唯一读者是 `commands::debug` 的 `debug_get_agent_traces`。
//! 录制不参与回答逻辑，失败时也照常留下上下文与出错位置。

use parking_lot::Mutex;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::LazyLock;
use zerolaunch_plugin_api::services::model::ModelChatRole;

/// 内存里保留的提问次数上限，超出后丢弃最旧的一条。
const MAX_TRACES: usize = 10;

/// 最近的提问轨迹（按发生顺序，最新在尾部）。
static TRACES: LazyLock<Mutex<VecDeque<AgentTrace>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));

/// 一次提问的调试轨迹：下发给模型的上下文 + 每轮模型输出与工具执行结果。
///
/// 由 `SettingsAgent::ask` 在提问结束时写入全局缓冲，经 `debug_get_agent_traces` 下发调试页；
/// 其他模块不读不写。
#[derive(Debug, Clone, Serialize)]
pub struct AgentTrace {
    /// 记录时刻的本地时间（`%Y-%m-%d %H:%M:%S`）。
    #[serde(rename = "time")]
    pub time: String,
    /// 本次使用的模型全局 id（含 provider 前缀）。
    #[serde(rename = "modelId")]
    pub model_id: String,
    /// 首轮随请求下发的工具名清单。
    #[serde(rename = "tools")]
    pub tools: Vec<String>,
    /// 首轮下发的完整上下文（系统提示词 + 历史对话 + 当前问题），时间正序。
    #[serde(rename = "messages")]
    pub messages: Vec<TraceMessage>,
    /// 每次模型调用的输出与随后的工具执行，按调用顺序排列。
    #[serde(rename = "rounds")]
    pub rounds: Vec<TraceRound>,
    /// 模型调用失败信息；`None` 表示本次提问没有发生模型调用错误。
    #[serde(rename = "error")]
    pub error: Option<String>,
}

/// 轨迹里的一条消息，对应首轮请求消息序列中的一项。
#[derive(Debug, Clone, Serialize)]
pub struct TraceMessage {
    /// 消息角色（序列化为 `system` / `user` / `assistant` / `tool`）。
    #[serde(rename = "role")]
    pub role: ModelChatRole,
    /// 消息正文。
    #[serde(rename = "content")]
    pub content: String,
}

/// 轨迹里的一轮模型调用。
#[derive(Debug, Clone, Serialize)]
pub struct TraceRound {
    /// 模型本轮返回的正文（纯工具调用轮可能为空串）。
    #[serde(rename = "content")]
    pub content: String,
    /// 本轮模型请求的工具调用及其执行结果。
    #[serde(rename = "toolCalls")]
    pub tool_calls: Vec<TraceToolCall>,
}

/// 轨迹里的一次工具调用。
#[derive(Debug, Clone, Serialize)]
pub struct TraceToolCall {
    /// 工具名。
    #[serde(rename = "name")]
    pub name: String,
    /// 模型给出的参数原文（JSON 文本，可能非法）。
    #[serde(rename = "arguments")]
    pub arguments: String,
    /// 工具回喂给模型的文本。
    #[serde(rename = "result")]
    pub result: String,
}

impl AgentTrace {
    /// 创建一条空轨迹（时间与模型 id 在此固定，其余字段由循环逐轮填充）。
    ///
    /// 参数：model_id - 本次使用的模型全局 id。
    /// 返回：上下文、轮次均为空的轨迹。
    pub fn new(model_id: &str) -> Self {
        Self {
            time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            model_id: model_id.to_string(),
            tools: Vec::new(),
            messages: Vec::new(),
            rounds: Vec::new(),
            error: None,
        }
    }
}

/// 记入一条轨迹；超出上限时先丢弃最旧的一条。
///
/// 参数：trace - 一次提问结束时的完整轨迹。
pub fn push(trace: AgentTrace) {
    let mut traces = TRACES.lock();
    if traces.len() >= MAX_TRACES {
        traces.pop_front();
    }
    traces.push_back(trace);
}

/// 读取已记录的轨迹，最新在前。
///
/// 返回：轨迹快照（最多 `MAX_TRACES` 条）；未提问过时为空。
pub fn snapshot() -> Vec<AgentTrace> {
    TRACES.lock().iter().rev().cloned().collect()
}
