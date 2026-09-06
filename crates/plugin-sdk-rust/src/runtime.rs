//! 第三方 Rust 插件的 JSON-RPC 运行时。
//!
//! 运行时采用三任务异步架构：
//! - `read_task`：唯一 stdin 读取者，解析 LSP 帧，路由响应到 pending_map，
//!   转发请求到 dispatch_task。
//! - `write_task`：唯一 stdout 写入者，将所有出站消息编码为 LSP 帧。
//! - `dispatch_task`：处理 plugin/* 请求，调用用户 Plugin trait 实现，
//!   将响应发到 write_task。
//!
//! HostProxy 通过共享的 pending_map 和 outbound_tx 发送 host/* 请求，
//! 避免了同步 BufReader 造成的死锁问题。

use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_api::{
    ActionExecutor, CachedCandidateData, DataSource, KeywordInjector, KeywordOptimizer, Plugin,
    ScoreBooster, SearchEngine,
};
use zerolaunch_plugin_protocol::codec::{
    encode_frame, summarize_message, MAX_FRAME_SIZE, MAX_HEADER_SIZE,
};
use zerolaunch_plugin_protocol::jsonrpc::{Message, Request, Response};
use zerolaunch_plugin_protocol::messages::*;
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;
use zerolaunch_plugin_protocol::{codes, JsonRpcError, PROTOCOL_VERSION};

use crate::host_proxy::HostProxy;
use crate::logging;

use std::sync::OnceLock;

// 全局 HostProxy，由 `run_async` 初始化。
static HOST_PROXY: OnceLock<Arc<HostProxy>> = OnceLock::new();

/// 返回当前运行时的 `HostProxy`。
/// 在 `run()` 之前调用会 panic。
pub fn host() -> Arc<HostProxy> {
    HOST_PROXY
        .get()
        .expect("host() 必须在 run() 之后调用")
        .clone()
}

/// 从 read task 路由到 dispatch task 的入站 JSON-RPC 请求。
struct IncomingRequest {
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// SDK 组件集合：一个 Plugin 主组件 + 任意个 DataSource / ActionExecutor /
/// SearchEngine / ScoreBooster / KeywordOptimizer / KeywordInjector 附加组件。
///
/// 与内置插件完全对等：每个组件都是独立的 Configurable（各自的 component_id、
/// schema、设置），进程向宿主声明全部组件（GET_COMPONENTS 返回全部）。
pub struct PluginApp {
    /// 插件主组件：进程级元数据、触发词、面板查询。
    pub(crate) plugin: Arc<dyn Plugin>,
    pub(crate) data_sources: Vec<Arc<dyn DataSource>>,
    pub(crate) executors: Vec<Arc<dyn ActionExecutor>>,
    /// 搜索引擎组件（与内置引擎互斥，仅允许一个启用）。
    pub(crate) search_engines: Vec<Arc<dyn SearchEngine>>,
    /// 分数增强器组件（引擎打分后追加修正）。
    pub(crate) score_boosters: Vec<Arc<dyn ScoreBooster>>,
    /// 关键词优化器组件（扩展候选关键词）。
    pub(crate) keyword_optimizers: Vec<Arc<dyn KeywordOptimizer>>,
    /// 关键词注入器组件（基于候选上下文注入关键词）。
    pub(crate) keyword_injectors: Vec<Arc<dyn KeywordInjector>>,
    /// component_id → 组件统一索引（全部组件类别皆入）。
    /// dispatch 按 component_id 以 O(1) 路由组件方法。
    by_id: HashMap<String, ComponentEntry>,
}

/// 组件索引条目：统一持有各 trait 对象，按需向上转型。
enum ComponentEntry {
    Plugin(Arc<dyn Plugin>),
    DataSource(Arc<dyn DataSource>),
    Executor(Arc<dyn ActionExecutor>),
    /// 搜索引擎组件（与内置引擎互斥，仅允许一个启用）。
    SearchEngine(Arc<dyn SearchEngine>),
    /// 分数增强器组件。
    ScoreBooster(Arc<dyn ScoreBooster>),
    /// 关键词优化器组件。
    KeywordOptimizer(Arc<dyn KeywordOptimizer>),
    /// 关键词注入器组件。
    KeywordInjector(Arc<dyn KeywordInjector>),
}

impl PluginApp {
    /// 以 Plugin 主组件构建应用（必须存在：进程级 metadata/触发词/面板查询属于它）。
    pub fn new(plugin: impl Plugin + 'static) -> Self {
        let plugin = Arc::new(plugin);
        let mut by_id = HashMap::new();
        by_id.insert(
            plugin.component_id().to_string(),
            ComponentEntry::Plugin(plugin.clone()),
        );
        Self {
            plugin,
            data_sources: Vec::new(),
            executors: Vec::new(),
            search_engines: Vec::new(),
            score_boosters: Vec::new(),
            keyword_optimizers: Vec::new(),
            keyword_injectors: Vec::new(),
            by_id,
        }
    }

    /// 附加 DataSource 组件（候选采集；与内置数据源完全对等）。
    /// 组件 id 重复属于插件编码错误，直接 panic 暴露。
    pub fn with_data_source(mut self, ds: impl DataSource + 'static) -> Self {
        let ds = Arc::new(ds);
        let component_id = ds.component_id().to_string();
        assert!(
            self.by_id
                .insert(component_id, ComponentEntry::DataSource(ds.clone()))
                .is_none(),
            "DataSource 组件 id 重复：{}",
            ds.component_id()
        );
        self.data_sources.push(ds);
        self
    }

    /// 附加 ActionExecutor 组件（候选执行；与内置执行器完全对等）。
    /// 组件 id 重复属于插件编码错误，直接 panic 暴露。
    pub fn with_executor(mut self, ex: impl ActionExecutor + 'static) -> Self {
        let ex = Arc::new(ex);
        let component_id = ex.component_id().to_string();
        assert!(
            self.by_id
                .insert(component_id, ComponentEntry::Executor(ex.clone()))
                .is_none(),
            "ActionExecutor 组件 id 重复：{}",
            ex.component_id()
        );
        self.executors.push(ex);
        self
    }
    /// 附加 SearchEngine 组件（搜索打分；与内置搜索引擎完全对等）。
    /// 组件 id 重复属于插件编码错误，直接 panic 暴露。
    pub fn with_search_engine(mut self, engine: impl SearchEngine + 'static) -> Self {
        let engine = Arc::new(engine);
        let component_id = engine.component_id().to_string();
        assert!(
            self.by_id
                .insert(component_id, ComponentEntry::SearchEngine(engine.clone()))
                .is_none(),
            "SearchEngine 组件 id 重复：{}",
            engine.component_id()
        );
        self.search_engines.push(engine);
        self
    }

    /// 附加 ScoreBooster 组件（分数增强；与内置分数增强器完全对等）。
    /// 组件 id 重复属于插件编码错误，直接 panic 暴露。
    pub fn with_score_booster(mut self, booster: impl ScoreBooster + 'static) -> Self {
        let booster = Arc::new(booster);
        let component_id = booster.component_id().to_string();
        assert!(
            self.by_id
                .insert(component_id, ComponentEntry::ScoreBooster(booster.clone()))
                .is_none(),
            "ScoreBooster 组件 id 重复：{}",
            booster.component_id()
        );
        self.score_boosters.push(booster);
        self
    }

    /// 附加 KeywordOptimizer 组件（关键词扩展；与内置优化器完全对等）。
    /// 组件 id 重复属于插件编码错误，直接 panic 暴露。
    pub fn with_keyword_optimizer(mut self, optimizer: impl KeywordOptimizer + 'static) -> Self {
        let optimizer = Arc::new(optimizer);
        let component_id = optimizer.component_id().to_string();
        assert!(
            self.by_id
                .insert(
                    component_id,
                    ComponentEntry::KeywordOptimizer(optimizer.clone()),
                )
                .is_none(),
            "KeywordOptimizer 组件 id 重复：{}",
            optimizer.component_id()
        );
        self.keyword_optimizers.push(optimizer);
        self
    }

    /// 附加 KeywordInjector 组件（基于候选上下文注入关键词；与内置注入器完全对等）。
    /// 组件 id 重复属于插件编码错误，直接 panic 暴露。
    pub fn with_keyword_injector(mut self, injector: impl KeywordInjector + 'static) -> Self {
        let injector = Arc::new(injector);
        let component_id = injector.component_id().to_string();
        assert!(
            self.by_id
                .insert(
                    component_id,
                    ComponentEntry::KeywordInjector(injector.clone()),
                )
                .is_none(),
            "KeywordInjector 组件 id 重复：{}",
            injector.component_id()
        );
        self.keyword_injectors.push(injector);
        self
    }

    /// 运行 JSON-RPC stdio 循环（阻塞当前线程直到进程退出）。
    pub fn run(self) {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");

        rt.block_on(async move {
            run_async(self).await;
        });
    }
}

/// 使用给定的 Plugin 实现运行 JSON-RPC stdio 循环。
/// 等价于 `PluginApp::new(plugin).run()`。
pub fn run(plugin: impl Plugin + 'static) {
    PluginApp::new(plugin).run()
}

async fn run_async(mut app: PluginApp) {
    // 初始化日志系统（双写：stderr → 文件 + WARN/ERROR → host/log 转发）
    let mut log_rx = logging::init_logging();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // 通道
    let (request_tx, mut request_rx) = mpsc::channel::<IncomingRequest>(64);
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<Vec<u8>>(64);
    let pending: Arc<DashMap<u64, oneshot::Sender<Result<serde_json::Value, JsonRpcError>>>> =
        Arc::new(DashMap::new());
    // 创建 HostProxy 并写入全局，供 dispatch/read/write 三任务共享访问。
    let host_proxy = Arc::new(HostProxy::new(pending.clone(), outbound_tx.clone()));
    let _ = HOST_PROXY.set(host_proxy);
    let hp_for_logs = HOST_PROXY.get().expect("HOST_PROXY 已设置").clone();

    // 插件状态
    let mut plugin_context: Option<zerolaunch_plugin_api::PluginContext> = None;

    // --- 日志转发后台任务：将 WARN/ERROR 非阻塞转发到宿主 ---
    tokio::spawn(async move {
        while let Some(entry) = log_rx.recv().await {
            hp_for_logs.log_no_wait(&entry.level, &entry.message);
        }
    });

    // --- 读任务：stdin → pending_map（响应）或 request_tx（新请求）---
    let pending_r = pending.clone();
    let request_tx_clone = request_tx.clone();
    let mut read_handle = tokio::spawn(async move {
        let reader = BufReader::new(stdin);
        let mut stdin = reader;
        while let Ok(body) = read_frame(&mut stdin).await {
            tracing::debug!(
                "收到宿主原始帧 ({} bytes): {:?}",
                body.len(),
                String::from_utf8_lossy(&body)
            );
            let msg: Message = match serde_json::from_slice(&body) {
                Ok(m) => m,
                Err(e) => {
                    tracing::debug!(
                        "解析宿主消息失败: {}; raw frame: {:?}",
                        e,
                        String::from_utf8_lossy(&body)
                    );
                    continue;
                }
            };
            match msg {
                Message::Response(resp) => {
                    if let Some((_, tx)) = pending_r.remove(&resp.id) {
                        // 错误走 Err 分支携带结构化 JsonRpcError，不再伪装成字符串结果：
                        // 宿主拒绝/模型错误等可被调用方区分，模型方法 from_value 不再被
                        // "invalid type: string" 淹没真实错误。
                        let result = if let Some(err) = resp.error {
                            Err(err)
                        } else {
                            Ok(resp.result.unwrap_or(serde_json::Value::Null))
                        };
                        let _ = tx.send(result);
                    } else {
                        // 如果没有对应的 pending channel，说明响应已经超时或被取消，忽略。同时打印一下被忽略的信息
                        tracing::warn!(
                            "收到未知的响应 id={}，可能已超时或被取消: {:?}",
                            resp.id,
                            resp
                        );
                    }
                }
                Message::Request(req) => {
                    let ret = request_tx_clone
                        .send(IncomingRequest {
                            id: req.id,
                            method: req.method,
                            params: req.params,
                        })
                        .await;
                    // 如果 dispatch task 已退出，说明插件可能已经崩溃或被关闭，无法处理请求。打印警告信息。
                    if ret.is_err() {
                        tracing::warn!(
                            "无法将请求发送到 dispatch task，可能 dispatch task 已退出: {:?}",
                            ret
                        );
                    }
                }
                Message::Notification(_) => {
                    tracing::trace!("忽略通知");
                }
            }
        }
    });

    // --- 分发任务：plugin/* 请求 → 用户 Plugin → 响应到 outbound_tx ---
    let outbound_dispatch = outbound_tx.clone();
    let mut dispatch_handle = tokio::spawn(async move {
        while let Some(incoming) = request_rx.recv().await {
            let req = Request::new(incoming.id, &incoming.method, incoming.params);
            tracing::debug!(
                "dispatch 收到请求: {:?}",
                summarize_message(&Message::Request(req.clone()))
            );
            // 调用用户实现的 Plugin trait 处理，并将响应发送到 outbound_tx。
            let result = handle_request(&mut app, &req, &mut plugin_context).await;
            if let Ok(payload) = serde_json::to_vec(&result) {
                if outbound_dispatch.send(payload).await.is_err() {
                    tracing::warn!(
                        "dispatch 发送响应失败（outbound channel 关闭），dispatch task 退出"
                    );
                    break;
                }
            } else {
                tracing::error!("dispatch 序列化响应失败，请求被丢弃");
            }
        }
        tracing::warn!("dispatch task 退出（request channel 关闭或发送失败）");
    });

    // --- 写任务：outbound_rx → stdout ---
    let write_handle = tokio::spawn(async move {
        let mut writer = stdout;
        while let Some(payload) = outbound_rx.recv().await {
            match serde_json::from_slice::<Message>(&payload) {
                Ok(msg) => tracing::debug!("发送给宿主的消息: {:?}", summarize_message(&msg)),
                Err(e) => tracing::debug!(
                    "发送给宿主的原始负载 ({} bytes, 解析失败: {}): {:?}",
                    payload.len(),
                    e,
                    String::from_utf8_lossy(&payload)
                ),
            }
            let frame = encode_frame(&payload);
            if writer.write_all(&frame).await.is_err() {
                break;
            }
            if writer.flush().await.is_err() {
                break;
            }
        }
    });

    // 等待读任务结束（传输层关闭），或 dispatch task 崩溃（插件代码 panic）。
    tokio::select! {
        // 正常退出：stdin 关闭（宿主 shutdown / EOF）→ 读任务结束。
        _ = &mut read_handle => {
            // 释放 request_tx → dispatch_task 在当前请求处理完后
            // 通过 channel 关闭优雅退出。
            drop(request_tx);
            // 写任务无法通过 channel 关闭退出，因为 HOST_PROXY 全局
            // 仍持有 outbound_tx 的 clone，因此直接 abort。
            write_handle.abort();
            let _ = (&mut dispatch_handle).await;
        }
        // 插件代码 panic：dispatch task 死亡，进程退出由宿主按 auto_restart 重启。
        r = &mut dispatch_handle => {
            if r.is_err() {
                tracing::error!("dispatch task 因 panic 退出，插件进程终止（宿主将按 auto_restart 重启）");
            }
        }
    }
}

/// 读取单条 LSP 风格 Content-Length 帧消息。
/// 返回原始 JSON 字节，或在解析/大小/IO 失败时返回错误字符串。
async fn read_frame<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R) -> Result<Vec<u8>, String> {
    let mut content_length: Option<usize> = None;
    let mut total_header_len = 0usize;
    loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| format!("read error: {}", e))?;
        if n == 0 {
            return Err("transport closed".into());
        }
        total_header_len += n;
        if total_header_len > MAX_HEADER_SIZE {
            return Err("header too long".into());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|e| format!("bad Content-Length: {}", e))?,
            );
        }
    }
    let len = content_length.ok_or("missing Content-Length")?;
    if len > MAX_FRAME_SIZE {
        return Err(format!("Content-Length too large: {}", len));
    }
    let mut body = vec![0u8; len];
    reader
        .read_exact(&mut body)
        .await
        .map_err(|e| format!("read body: {}", e))?;
    Ok(body)
}

// / 处理单条 plugin/* 请求，返回响应 Message。
async fn handle_request(
    app: &mut PluginApp,
    req: &Request,
    plugin_ctx: &mut Option<zerolaunch_plugin_api::PluginContext>,
) -> Message {
    let id = req.id;
    let result = dispatch(app, &req.method, &req.params, plugin_ctx).await;
    match result {
        Ok(value) => Message::Response(Response::ok(id, value)),
        Err(err) => Message::Response(Response::err(id, err)),
    }
}

async fn dispatch(
    app: &mut PluginApp,
    method: &str,
    params: &serde_json::Value,
    plugin_ctx: &mut Option<zerolaunch_plugin_api::PluginContext>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        // 初始化请求，设置 plugin_ctx 并返回插件版本信息。
        plugin_methods::INITIALIZE => {
            let p: InitializeParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            // 记录插件 id：t_key() 据此自动补 `plugin.<id>.` 前缀
            crate::set_plugin_id(&p.plugin_id);
            *plugin_ctx = Some(zerolaunch_plugin_api::PluginContext {
                trace_id: "init".into(),
                query_id: None,
                plugin_id: Some(p.plugin_id),
                // 远端插件无宿主查询版本门控，恒视为最新。
                query_revision_gate: None,
                // 远端插件会话由宿主经 RPC 下发通道，未收到时缺省视为 GUI 通道。
                query_channel: zerolaunch_plugin_api::QueryChannel::Ui,
                // 宿主语言在握手时下发（InitializeParams.locale），写入会话上下文。
                locale: p.locale,
            });
            let result = InitializeResult {
                plugin_version: app.plugin.metadata().version.clone(),
                protocol_version: PROTOCOL_VERSION.to_string(),
            };
            Ok(serde_json::to_value(result).unwrap_or_default())
        }
        // 返回 metadata
        plugin_methods::GET_METADATA => {
            Ok(serde_json::to_value(app.plugin.metadata()).unwrap_or(serde_json::Value::Null))
        }
        // 返回这个插件实现的全部组件（Plugin + 附加 DataSource / ActionExecutor）
        plugin_methods::GET_COMPONENTS => {
            let mut components = vec![ComponentDescriptor {
                component_id: app.plugin.component_id().to_string(),
                component_name: app.plugin.component_name().to_string(),
                component_description: app.plugin.metadata().description.clone(),
                component_type: app.plugin.component_type(),
                kind: ComponentKind::Plugin {
                    trigger_keywords: app.plugin.metadata().trigger_keywords.clone(),
                },
                priority: app.plugin.metadata().priority,
            }];
            for ds in &app.data_sources {
                components.push(ComponentDescriptor {
                    component_id: ds.component_id().to_string(),
                    component_name: ds.component_name().to_string(),
                    component_description: ds.component_description().to_string(),
                    component_type: ds.component_type(),
                    kind: ComponentKind::DataSource,
                    priority: ds.priority(),
                });
            }
            for ex in &app.executors {
                components.push(ComponentDescriptor {
                    component_id: ex.component_id().to_string(),
                    component_name: ex.component_name().to_string(),
                    component_description: ex.component_description().to_string(),
                    component_type: ex.component_type(),
                    kind: ComponentKind::ActionExecutor {
                        target_types: ex.supported_target_types(),
                    },
                    priority: ex.priority(),
                });
            }
            for engine in &app.search_engines {
                components.push(ComponentDescriptor {
                    component_id: engine.component_id().to_string(),
                    component_name: engine.component_name().to_string(),
                    component_description: engine.component_description().to_string(),
                    component_type: engine.component_type(),
                    kind: ComponentKind::SearchEngine,
                    priority: engine.priority(),
                });
            }
            for booster in &app.score_boosters {
                components.push(ComponentDescriptor {
                    component_id: booster.component_id().to_string(),
                    component_name: booster.component_name().to_string(),
                    component_description: booster.component_description().to_string(),
                    component_type: booster.component_type(),
                    kind: ComponentKind::ScoreBooster,
                    priority: booster.priority(),
                });
            }
            for optimizer in &app.keyword_optimizers {
                components.push(ComponentDescriptor {
                    component_id: optimizer.component_id().to_string(),
                    component_name: optimizer.component_name().to_string(),
                    component_description: optimizer.component_description().to_string(),
                    component_type: optimizer.component_type(),
                    kind: ComponentKind::KeywordOptimizer,
                    priority: optimizer.priority(),
                });
            }
            for injector in &app.keyword_injectors {
                components.push(ComponentDescriptor {
                    component_id: injector.component_id().to_string(),
                    component_name: injector.component_name().to_string(),
                    component_description: injector.component_description().to_string(),
                    component_type: injector.component_type(),
                    kind: ComponentKind::KeywordInjector,
                    priority: injector.priority(),
                });
            }
            Ok(serde_json::to_value(components).unwrap_or_default())
        }
        // 返回指定组件的注册配置项
        plugin_methods::GET_SETTINGS_SCHEMA => {
            let p: GetSettingsSchemaParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let conf = find_configurable(app, &p.component_id)?;
            Ok(serde_json::to_value(conf.setting_schema()).unwrap_or(serde_json::Value::Null))
        }
        // 返回指定组件当前的配置值
        plugin_methods::GET_SETTINGS => {
            let p: GetSettingsParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let conf = find_configurable(app, &p.component_id)?;
            Ok(conf.get_settings())
        }
        // 返回指定组件的默认启用状态
        plugin_methods::GET_DEFAULT_ENABLED => {
            let p: GetDefaultEnabledParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let conf = find_configurable(app, &p.component_id)?;
            Ok(serde_json::to_value(conf.default_enabled()).unwrap_or_default())
        }
        // 宿主下发新的配置值，指定组件据此更新自身行为
        plugin_methods::APPLY_SETTINGS => {
            let p: ApplySettingsParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let conf = find_configurable(app, &p.component_id)?;
            conf.apply_settings(p.settings)
                .await
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
            Ok(serde_json::Value::Null)
        }
        // 验证一组配置值是否合法（不会实际应用），返回验证结果或错误信息
        plugin_methods::VALIDATE_SETTINGS => {
            let p: ValidateSettingsParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let conf = find_configurable(app, &p.component_id)?;
            let result = match conf.validate_settings(&p.settings).await {
                Ok(()) => ValidateSettingsResult { error: None },
                Err(e) => ValidateSettingsResult {
                    error: Some(e.to_string()),
                },
            };
            Ok(serde_json::to_value(result).unwrap_or_default())
        }
        // 返回裸数组：宿主 discover 流程以 `Vec<ConfigActionDef>` 反序列化
        // （与 get_settings_schema 的裸数组约定一致），包装结构会解析失败。
        plugin_methods::CONFIG_ACTIONS => {
            let p: ConfigActionsParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let conf = find_configurable(app, &p.component_id)?;
            Ok(serde_json::to_value(conf.config_actions()).unwrap_or_default())
        }
        plugin_methods::EXECUTE_CONFIG_ACTION => {
            let p: ExecuteConfigActionParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let conf = find_configurable(app, &p.component_id)?;
            conf.execute_config_action(&p.action, &p.params)
                .await
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))
        }
        plugin_methods::QUERY => {
            let p: QueryParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let response = app
                .plugin
                .query(&p.ctx, &p.query)
                .await
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
            Ok(serde_json::to_value(response).unwrap_or_default())
        }
        plugin_methods::EXECUTE_ACTION => {
            let p: ExecuteActionParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            app.plugin
                .execute_action(&p.ctx, &p.action_id, p.payload)
                .await
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
            Ok(serde_json::Value::Null)
        }
        // 插件初始化钩子：宿主在注册完成后调用（内置插件在启动期统一 init）。
        // 远端进程无宿主 PluginHandle（跨进程不可序列化），以 None 传入，
        // 平台能力经 host() 的 host/* RPC 访问——与内置插件语义对等。
        plugin_methods::INIT => {
            let p: InitParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            app.plugin
                .init(&p.ctx, None)
                .await
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
            Ok(serde_json::Value::Null)
        }
        // 插件交互策略（查询触发方式/防抖/按键绑定）：宿主在会话推送时读取。
        plugin_methods::INTERACTION_POLICY => {
            let _p: InteractionPolicyParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            Ok(serde_json::to_value(app.plugin.interaction_policy()).unwrap_or_default())
        }
        // DataSource 组件：采集候选项（与内置数据源对等）
        plugin_methods::FETCH_CANDIDATES => {
            let p: FetchCandidatesParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let ds = find_data_source(app, &p.component_id)?;
            let cache = ds.fetch_candidates().await;
            Ok(serde_json::to_value(FetchCandidatesResult {
                candidates: cache.get_candidates().clone(),
            })
            .unwrap_or_default())
        }
        // ActionExecutor 组件：支持的目标类型列表
        plugin_methods::SUPPORTED_TARGET_TYPES => {
            let p: SupportedTargetTypesParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let ex = find_executor(app, &p.component_id)?;
            Ok(serde_json::to_value(ex.supported_target_types()).unwrap_or_default())
        }
        // ActionExecutor 组件：支持的动作列表（与内置 supported_actions() 语义一致，
        // 不区分 target_type——宿主按 (id, label) 去重合并）
        plugin_methods::SUPPORTED_ACTIONS => {
            let p: SupportedActionsParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let ex = find_executor(app, &p.component_id)?;
            Ok(serde_json::to_value(ex.supported_actions()).unwrap_or_default())
        }
        // ActionExecutor 组件：执行动作（完整 ExecutionContext 原样下发）
        plugin_methods::EXECUTOR_EXECUTE => {
            let p: ExecutorExecuteParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let ex = find_executor(app, &p.component_id)?;
            let result = match ex.execute(&p.execution_ctx, &p.action_id).await {
                Ok(()) => ExecutorExecuteResult { error: None },
                Err(e) => ExecutorExecuteResult {
                    error: Some(e.to_string()),
                },
            };
            Ok(serde_json::to_value(result).unwrap_or_default())
        }
        // SearchEngine 组件：对缓存候选计算分数（候选快照保真还原，
        // 插件进程内按原 id / 原顺序可回查）
        plugin_methods::CALCULATE_SCORES => {
            let p: CalculateScoresParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let engine = find_search_engine(app, &p.component_id)?;
            let cache = CachedCandidateData::from_data(p.candidates);
            let scored = engine.calculate_scores(&cache, &p.query).await;
            Ok(serde_json::to_value(scored).unwrap_or_default())
        }
        // ScoreBooster 组件：增强分数（返回增强后的完整列表，索引与传入一致）
        plugin_methods::BOOSTER_BOOST => {
            let p: BoosterBoostParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let booster = find_score_booster(app, &p.component_id)?;
            let cache = CachedCandidateData::from_data(p.candidates);
            let mut scored = p.scored;
            booster.boost(&mut scored, &cache, &p.query).await;
            Ok(serde_json::to_value(scored).unwrap_or_default())
        }
        // ScoreBooster 组件：记录用户确认（选中候选 → 学习用户习惯）
        plugin_methods::BOOSTER_RECORD => {
            let p: BoosterRecordParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let booster = find_score_booster(app, &p.component_id)?;
            let cache = CachedCandidateData::from_data(p.candidates);
            booster.record(p.candidate_id, &cache, &p.query).await;
            Ok(serde_json::Value::Null)
        }
        // KeywordOptimizer 组件：声明属性（input_source / priority 经 info RPC 上报）
        plugin_methods::KEYWORD_OPTIMIZER_INFO => {
            let p: KeywordOptimizerInfoParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let optimizer = find_keyword_optimizer(app, &p.component_id)?;
            Ok(serde_json::to_value(KeywordOptimizerInfo {
                input_source: optimizer.input_source(),
                priority: optimizer.get_priority(),
            })
            .unwrap_or_default())
        }
        // KeywordOptimizer 组件：优化单个关键词
        plugin_methods::KEYWORD_OPTIMIZE => {
            let p: KeywordOptimizeParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let optimizer = find_keyword_optimizer(app, &p.component_id)?;
            Ok(serde_json::to_value(optimizer.optimize(&p.keyword).await).unwrap_or_default())
        }
        // KeywordInjector 组件：对单个候选注入关键词
        plugin_methods::KEYWORD_INJECT => {
            let p: KeywordInjectParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let injector = find_keyword_injector(app, &p.component_id)?;
            Ok(
                serde_json::to_value(injector.inject_keywords(&p.candidate).await)
                    .unwrap_or_default(),
            )
        }
        _ => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("method not found: {}", method),
        )),
    }
}

/// 按 component_id 查找 Configurable 组件（Plugin / DataSource / ActionExecutor 皆可）。
/// 组件未注册时返回 METHOD_NOT_FOUND。
fn find_configurable<'a>(
    app: &'a PluginApp,
    component_id: &str,
) -> Result<&'a dyn Configurable, JsonRpcError> {
    let entry = app.by_id.get(component_id).ok_or_else(|| {
        JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("component not found: {component_id}"),
        )
    })?;
    Ok(match entry {
        ComponentEntry::Plugin(p) => p.as_ref() as &dyn Configurable,
        ComponentEntry::DataSource(ds) => ds.as_ref() as &dyn Configurable,
        ComponentEntry::Executor(ex) => ex.as_ref() as &dyn Configurable,
        ComponentEntry::SearchEngine(engine) => engine.as_ref() as &dyn Configurable,
        ComponentEntry::ScoreBooster(booster) => booster.as_ref() as &dyn Configurable,
        ComponentEntry::KeywordOptimizer(optimizer) => optimizer.as_ref() as &dyn Configurable,
        ComponentEntry::KeywordInjector(injector) => injector.as_ref() as &dyn Configurable,
    })
}

/// 按 component_id 查找 DataSource 组件。
fn find_data_source<'a>(
    app: &'a PluginApp,
    component_id: &str,
) -> Result<&'a dyn DataSource, JsonRpcError> {
    match app.by_id.get(component_id) {
        Some(ComponentEntry::DataSource(ds)) => Ok(ds.as_ref()),
        Some(_) => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("component is not a data source: {component_id}"),
        )),
        None => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("data source not found: {component_id}"),
        )),
    }
}

/// 按 component_id 查找 ActionExecutor 组件。
fn find_executor<'a>(
    app: &'a PluginApp,
    component_id: &str,
) -> Result<&'a dyn ActionExecutor, JsonRpcError> {
    match app.by_id.get(component_id) {
        Some(ComponentEntry::Executor(ex)) => Ok(ex.as_ref()),
        Some(_) => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("component is not an executor: {component_id}"),
        )),
        None => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("executor not found: {component_id}"),
        )),
    }
}
/// 按 component_id 查找 SearchEngine 组件。
fn find_search_engine<'a>(
    app: &'a PluginApp,
    component_id: &str,
) -> Result<&'a dyn SearchEngine, JsonRpcError> {
    match app.by_id.get(component_id) {
        Some(ComponentEntry::SearchEngine(engine)) => Ok(engine.as_ref()),
        Some(_) => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("component is not a search engine: {component_id}"),
        )),
        None => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("search engine not found: {component_id}"),
        )),
    }
}

/// 按 component_id 查找 ScoreBooster 组件。
fn find_score_booster<'a>(
    app: &'a PluginApp,
    component_id: &str,
) -> Result<&'a dyn ScoreBooster, JsonRpcError> {
    match app.by_id.get(component_id) {
        Some(ComponentEntry::ScoreBooster(booster)) => Ok(booster.as_ref()),
        Some(_) => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("component is not a score booster: {component_id}"),
        )),
        None => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("score booster not found: {component_id}"),
        )),
    }
}

/// 按 component_id 查找 KeywordOptimizer 组件。
fn find_keyword_optimizer<'a>(
    app: &'a PluginApp,
    component_id: &str,
) -> Result<&'a dyn KeywordOptimizer, JsonRpcError> {
    match app.by_id.get(component_id) {
        Some(ComponentEntry::KeywordOptimizer(optimizer)) => Ok(optimizer.as_ref()),
        Some(_) => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("component is not a keyword optimizer: {component_id}"),
        )),
        None => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("keyword optimizer not found: {component_id}"),
        )),
    }
}

/// 按 component_id 查找 KeywordInjector 组件。
fn find_keyword_injector<'a>(
    app: &'a PluginApp,
    component_id: &str,
) -> Result<&'a dyn KeywordInjector, JsonRpcError> {
    match app.by_id.get(component_id) {
        Some(ComponentEntry::KeywordInjector(injector)) => Ok(injector.as_ref()),
        Some(_) => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("component is not a keyword injector: {component_id}"),
        )),
        None => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("keyword injector not found: {component_id}"),
        )),
    }
}
