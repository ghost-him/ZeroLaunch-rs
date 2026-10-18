//! HostProxy — 第三方插件调用 host/* API 的代理。
//!
//! 每个方法经共享出站通道发送 LSP 帧 JSON-RPC 请求，并通过注册在共享
//! pending map 中的 oneshot 等待响应。该设计将 stdin 读取与 stdout 写入
//! 集中到独立任务，避免旧同步 stdin 锁方案的死锁。
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use zerolaunch_plugin_api::host::PluginSdkConfig;
use zerolaunch_plugin_api::platform::PlatformCapability;
use zerolaunch_plugin_api::services::focus_monitor::FocusEvent;
use zerolaunch_plugin_api::services::hotkey::types::{HotkeyEvent, HotkeyEventFilter};
use zerolaunch_plugin_api::services::installation_monitor::types::InstallationEvent;
use zerolaunch_plugin_api::services::model::{
    ModelChatRequest, ModelChatResponse, ModelEmbeddingRequest, ModelEmbeddingResponse, ModelInfo,
    ModelSimilarityRequest, ModelSimilarityResponse,
};
use zerolaunch_plugin_api::services::shell::CommandRunMode;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_protocol::messages::{
    AppIconPathParams, AppLaunchParams, CacheCleanupParams, CacheDeleteParams, CacheGetParams,
    CachePutParams, ClipboardSetTextParams, FocusSubscribeParams, FocusUnsubscribeParams,
    HotkeySubscribeParams, HotkeyUnsubscribeParams, IconOverrideCacheParams, IconRequestParams,
    InstallationSubscribeParams, InstallationUnsubscribeParams, ParameterResolveParams,
    ParameterTemplateParams, PathLocalizedNamesParams, PathResolveLnkParams, SdkConfigUpdateParams,
    TimerCancelAllParams, TimerCancelParams, TimerSetIntervalParams, TimerSetTimeoutParams,
    WindowActivateByProcessParams, WindowActivateByTitleParams, WindowActivateParams,
};
use zerolaunch_plugin_protocol::methods::host;
use zerolaunch_plugin_protocol::JsonRpcError;

use crate::events;

/// Proxy for calling host-side APIs from a plugin subprocess.
/// Does NOT access stdin/stdout directly — uses channel-based I/O.
pub struct HostProxy {
    /// 请求 id 分配器（单调递增，从 1 开始）。
    next_id: AtomicU64,
    /// 在途请求表：id → 响应通道。read_task 完成响应后移除条目。
    /// 值类型为 `Result`：宿主错误经 Err(JsonRpcError) 返回，调用方区分错误与结果。
    pending: Arc<DashMap<u64, oneshot::Sender<Result<serde_json::Value, JsonRpcError>>>>,
    /// 出站帧通道：write_task 独占消费并写 stdout。
    outbound_tx: mpsc::Sender<Vec<u8>>,
}

impl HostProxy {
    pub fn new(
        pending: Arc<DashMap<u64, oneshot::Sender<Result<serde_json::Value, JsonRpcError>>>>,
        outbound_tx: mpsc::Sender<Vec<u8>>,
    ) -> Self {
        Self {
            next_id: AtomicU64::new(1),
            pending,
            outbound_tx,
        }
    }

    /// Send a host/* request via the shared stdout channel and await the response
    /// through the shared pending map.
    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // 模型类方法耗时可能远超普通 host/*（LLM 生成 / 大批量 embedding），
        // 单独放宽超时；其余方法维持 30s。
        let is_model_call = method.starts_with("host/model.");
        let timeout = if is_model_call {
            Duration::from_secs(300)
        } else {
            Duration::from_secs(30)
        };
        self.send_request_inner(method, params, timeout).await
    }

    /// 发送请求并等待响应；超时/通道关闭返回错误字符串。
    async fn send_request_inner(
        &self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let payload = serde_json::to_vec(&request).map_err(|e| e.to_string())?;

        // Register pending response
        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, tx);

        // Send via the shared channel (write_task writes to stdout).
        // 注意：此处只投递干净 JSON，分帧统一由 write_task 完成，
        // 若在此 encode_frame 会导致双重分帧（帧内嵌套帧），宿主无法解析。
        if self.outbound_tx.send(payload).await.is_err() {
            self.pending.remove(&id);
            return Err("write channel closed".to_string());
        }

        // Await the response (read_task completes the oneshot with Result).
        // Apply a timeout so the plugin doesn't hang forever if
        // the host crashes during request processing.
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(value))) => Ok(value),
            Ok(Ok(Err(err))) => Err(format!(
                "host call failed (code {}): {}",
                err.code, err.message
            )),
            Ok(Err(_)) => Err("response channel closed".to_string()),
            Err(_) => {
                // 超时：移除 pending 条目，防止长驻插件无界泄漏。
                self.pending.remove(&id);
                Err("host call timed out".to_string())
            }
        }
    }

    pub async fn log(&self, level: &str, message: &str) -> Result<(), String> {
        self.send_request(
            host::LOG,
            serde_json::json!({ "level": level, "message": message }),
        )
        .await?;
        Ok(())
    }

    /// 发送 host/log 请求但不等待响应（fire-and-forget）。
    ///
    /// pending 条目在宿主响应到达时由 read_task 自动清理。
    /// 若 outbound 通道已满，日志被静默丢弃并从 pending 中移除，
    /// 避免阻塞调用者（通常来自 tracing subscriber 的回调）。
    pub fn log_no_wait(&self, level: &str, message: &str) {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let Ok(payload) = serde_json::to_vec(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": host::LOG,
            "params": { "level": level, "message": message },
        })) else {
            return;
        };

        let (tx, _rx) = oneshot::channel(); // _rx 立即 drop → fire-and-forget
        self.pending.insert(id, tx);

        // 非阻塞投递：通道满了则丢弃并清理 pending。
        // 同 send_request：只投递干净 JSON，分帧由 write_task 统一完成。
        if self.outbound_tx.try_send(payload).is_err() {
            self.pending.remove(&id);
        }
    }

    pub async fn shell_open(&self, target: &str) -> Result<(), String> {
        self.send_request(host::SHELL_OPEN, serde_json::json!({ "target": target }))
            .await?;
        Ok(())
    }

    /// 获取图标字节（base64 字符串）。
    /// 提取失败时由宿主回退默认图标（永不报错）；需要严格失败语义用 extract_icon。
    /// 返回：图标字节的 base64（WebP，回退可能为 PNG）；失败为空字符串。
    pub async fn get_icon(&self, path: &str) -> Result<String, String> {
        let params = serde_json::to_value(IconRequestParams {
            request: IconRequest::Path(path.to_string()),
        })
        .map_err(|e| e.to_string())?;
        let result = self.send_request(host::ICON_GET, params).await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// 执行命令字符串。
    /// 参数：cmd - 命令字符串；mode - 执行方式（后台静默 / 可见控制台窗口）。
    pub async fn shell_execute_command(
        &self,
        cmd: &str,
        mode: CommandRunMode,
    ) -> Result<(), String> {
        self.send_request(
            host::SHELL_EXECUTE_COMMAND,
            serde_json::json!({ "cmd": cmd, "mode": mode }),
        )
        .await?;
        Ok(())
    }

    pub async fn shell_open_folder(&self, path: &str) -> Result<(), String> {
        self.send_request(host::SHELL_OPEN_FOLDER, serde_json::json!({ "path": path }))
            .await?;
        Ok(())
    }

    pub async fn shell_execute_elevation(&self, path: &str) -> Result<(), String> {
        self.send_request(
            host::SHELL_EXECUTE_ELEVATION,
            serde_json::json!({ "path": path }),
        )
        .await?;
        Ok(())
    }

    pub async fn notify(&self, title: &str, message: &str) -> Result<(), String> {
        self.send_request(
            host::NOTIFY,
            serde_json::json!({ "title": title, "message": message }),
        )
        .await?;
        Ok(())
    }

    /// 获取宿主当前界面语言（如 "zh-Hans"），供插件生成本地化文本。
    pub async fn get_locale(&self) -> Result<String, String> {
        let result = self
            .send_request(host::GET_LOCALE, serde_json::json!(null))
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// 查询宿主当前实际生效主题，返回 `light` 或 `dark`。
    pub async fn get_theme(&self) -> Result<String, String> {
        let result = self
            .send_request(host::GET_THEME, serde_json::Value::Null)
            .await?;
        result
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "host theme response is not a string".to_string())
    }

    /// 全网模型清单（聚合所有提供方）。
    pub async fn model_list(&self) -> Result<Vec<ModelInfo>, String> {
        let result = self
            .send_request(host::MODEL_LIST, serde_json::Value::Null)
            .await?;
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// 按 model_id 调用文本生成。
    pub async fn model_chat(&self, req: ModelChatRequest) -> Result<ModelChatResponse, String> {
        let params = serde_json::to_value(req).map_err(|e| e.to_string())?;
        let result = self.send_request(host::MODEL_CHAT, params).await?;
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// 按 model_id 调用文本向量化（task_type 必填，宿主对缺失/未知值返回错误）。
    pub async fn model_embedding(
        &self,
        req: ModelEmbeddingRequest,
    ) -> Result<ModelEmbeddingResponse, String> {
        let params = serde_json::to_value(req).map_err(|e| e.to_string())?;
        let result = self.send_request(host::MODEL_EMBEDDING, params).await?;
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// 按 model_id 计算查询向量与多个目标向量的相似度。
    pub async fn model_similarity(
        &self,
        req: ModelSimilarityRequest,
    ) -> Result<ModelSimilarityResponse, String> {
        let params = serde_json::to_value(req).map_err(|e| e.to_string())?;
        let result = self.send_request(host::MODEL_SIMILARITY, params).await?;
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    pub async fn enumerate_apps(&self) -> Result<serde_json::Value, String> {
        self.send_request(host::APP_ENUMERATE, serde_json::json!(null))
            .await
    }

    pub async fn resolve_path(&self, kind: &str) -> Result<String, String> {
        let result = self
            .send_request(host::PATH_RESOLVE, serde_json::json!({ "kind": kind }))
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// 上传插件本地文件到宿主资源空间。
    /// 直接传递文件路径，由宿主负责读取。
    pub async fn resource_upload(
        &self,
        resource_id: &str,
        file_path: &str,
        max_size: Option<u64>,
    ) -> Result<String, String> {
        let result = self
            .send_request(
                host::RESOURCE_UPLOAD,
                serde_json::json!({
                    "resourceId": resource_id,
                    "filePath": file_path,
                    "maxSize": max_size,
                }),
            )
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    pub async fn resource_get(&self, resource_id: &str) -> Result<Vec<u8>, String> {
        let result = self
            .send_request(
                host::RESOURCE_GET,
                serde_json::json!({
                    "resourceId": resource_id,
                }),
            )
            .await?;
        let b64 = result.as_str().unwrap_or("");
        STANDARD
            .decode(b64)
            .map_err(|e| format!("base64 decode failed: {}", e))
    }

    /// 直接写入资源字节数据（无需临时文件），base64 编解码由 SDK 内部处理。
    pub async fn resource_put(&self, resource_id: &str, data: &[u8]) -> Result<(), String> {
        let b64 = STANDARD.encode(data);
        self.send_request(
            host::RESOURCE_PUT,
            serde_json::json!({
                "resourceId": resource_id,
                "bytesB64": b64,
            }),
        )
        .await?;
        Ok(())
    }

    /// 删除资源文件。
    pub async fn resource_delete(&self, resource_id: &str) -> Result<(), String> {
        self.send_request(
            host::RESOURCE_DELETE,
            serde_json::json!({
                "resourceId": resource_id,
            }),
        )
        .await?;
        Ok(())
    }

    /// 列出本插件的所有资源标识符。
    pub async fn resource_list(&self) -> Result<Vec<String>, String> {
        let result = self
            .send_request(host::RESOURCE_LIST, serde_json::json!({}))
            .await?;
        serde_json::from_value(result).map_err(|e| format!("parse resource list failed: {}", e))
    }

    // ===== 平台与 SDK 配置 =====

    /// 查询宿主平台能力集，插件据此降级不可用功能。
    pub async fn capabilities(&self) -> Result<Vec<PlatformCapability>, String> {
        let result = self
            .send_request(host::PLATFORM_CAPABILITIES, serde_json::Value::Null)
            .await?;
        serde_json::from_value(result).map_err(|e| format!("parse capabilities failed: {}", e))
    }

    /// 更新本插件的 SDK 配置（立即生效，影响后续图标缓存等级等行为）。
    pub async fn update_sdk_config(&self, config: PluginSdkConfig) -> Result<(), String> {
        let params =
            serde_json::to_value(SdkConfigUpdateParams { config }).map_err(|e| e.to_string())?;
        self.send_request(host::SDK_CONFIG_UPDATE, params).await?;
        Ok(())
    }

    // ===== 图标（严格变体）=====

    /// 严格提取图标：提取失败返回错误（与 get_icon 的默认图标回退不同）。
    /// 返回：图标字节的 base64（WebP，回退路径可能为 PNG）。
    pub async fn extract_icon(&self, request: &IconRequest) -> Result<String, String> {
        let params = serde_json::to_value(IconRequestParams {
            request: request.clone(),
        })
        .map_err(|e| e.to_string())?;
        let result = self.send_request(host::ICON_EXTRACT, params).await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// 强制从磁盘提取图标并写回缓存（跳过缓存读取）。
    /// 返回：图标字节的 base64（WebP，回退路径可能为 PNG）。
    pub async fn refresh_icon(&self, request: &IconRequest) -> Result<String, String> {
        let params = serde_json::to_value(IconRequestParams {
            request: request.clone(),
        })
        .map_err(|e| e.to_string())?;
        let result = self.send_request(host::ICON_REFRESH, params).await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// 用自定义图标文件覆盖指定图标请求的缓存条目。
    pub async fn override_icon_cache(
        &self,
        request: &IconRequest,
        custom_icon_path: &str,
    ) -> Result<(), String> {
        let params = serde_json::to_value(IconOverrideCacheParams {
            request: request.clone(),
            custom_icon_path: custom_icon_path.to_string(),
        })
        .map_err(|e| e.to_string())?;
        self.send_request(host::ICON_OVERRIDE_CACHE, params).await?;
        Ok(())
    }

    // ===== 窗口 =====

    /// 按进程名（如 "chrome.exe"）激活已存在的窗口。
    /// 返回：成功激活 true，未找到窗口 false。
    pub async fn activate_window_by_process(&self, process_name: &str) -> Result<bool, String> {
        let params = serde_json::to_value(WindowActivateByProcessParams {
            process_name: process_name.to_string(),
        })
        .map_err(|e| e.to_string())?;
        let result = self
            .send_request(host::WINDOW_ACTIVATE_BY_PROCESS, params)
            .await?;
        result
            .as_bool()
            .ok_or_else(|| "host window response is not a bool".to_string())
    }

    /// 按进程 PID 激活已存在的窗口。
    /// 返回：成功激活 true，未找到窗口 false。
    pub async fn activate_window_by_pid(&self, pid: u32) -> Result<bool, String> {
        let params =
            serde_json::to_value(WindowActivateParams { pid }).map_err(|e| e.to_string())?;
        let result = self
            .send_request(host::WINDOW_ACTIVATE_BY_PID, params)
            .await?;
        result
            .as_bool()
            .ok_or_else(|| "host window response is not a bool".to_string())
    }

    /// 按窗口标题片段激活已存在的窗口。
    /// 返回：成功激活 true，未找到窗口 false。
    pub async fn activate_window_by_title(&self, title: &str) -> Result<bool, String> {
        let params = serde_json::to_value(WindowActivateByTitleParams {
            title: title.to_string(),
        })
        .map_err(|e| e.to_string())?;
        let result = self
            .send_request(host::WINDOW_ACTIVATE_BY_TITLE, params)
            .await?;
        result
            .as_bool()
            .ok_or_else(|| "host window response is not a bool".to_string())
    }

    // ===== 剪贴板 / 应用 / 路径 / 主题 / 参数 =====

    /// 将文本写入系统剪贴板。
    pub async fn set_clipboard_text(&self, text: &str) -> Result<(), String> {
        let params = serde_json::to_value(ClipboardSetTextParams {
            text: text.to_string(),
        })
        .map_err(|e| e.to_string())?;
        self.send_request(host::CLIPBOARD_SET_TEXT, params).await?;
        Ok(())
    }

    /// 启动已安装应用（args 为 None 表示不传参数）。
    /// 返回：启动进程的 PID。
    pub async fn launch_app(&self, app_id: &str, args: Option<&[String]>) -> Result<u32, String> {
        let params = serde_json::to_value(AppLaunchParams {
            app_id: app_id.to_string(),
            args: args.map(|a| a.to_vec()),
        })
        .map_err(|e| e.to_string())?;
        let result = self.send_request(host::APP_LAUNCH, params).await?;
        let pid = result
            .as_u64()
            .ok_or_else(|| "host app.launch response is not a number".to_string())?;
        Ok(pid as u32)
    }

    /// 查询内置应用图标资源的文件系统路径；未注册返回 None。
    pub async fn app_icon_path(&self, name: &str) -> Result<Option<String>, String> {
        let params = serde_json::to_value(AppIconPathParams {
            name: name.to_string(),
        })
        .map_err(|e| e.to_string())?;
        let result = self.send_request(host::APP_ICON_PATH, params).await?;
        Ok(result.as_str().map(str::to_string))
    }

    /// 解析 .lnk 快捷方式文件的目标路径；失败返回 None。
    pub async fn resolve_lnk_target(&self, lnk_path: &str) -> Result<Option<String>, String> {
        let params = serde_json::to_value(PathResolveLnkParams {
            lnk_path: lnk_path.to_string(),
        })
        .map_err(|e| e.to_string())?;
        let result = self.send_request(host::PATH_RESOLVE_LNK, params).await?;
        Ok(result.as_str().map(str::to_string))
    }

    /// 解析目录下 desktop.ini 的本地化文件名表。
    pub async fn localized_names_from_dir(
        &self,
        dir_path: &str,
    ) -> Result<HashMap<String, String>, String> {
        let params = serde_json::to_value(PathLocalizedNamesParams {
            dir_path: dir_path.to_string(),
        })
        .map_err(|e| e.to_string())?;
        let result = self
            .send_request(host::PATH_LOCALIZED_NAMES, params)
            .await?;
        serde_json::from_value(result).map_err(|e| format!("parse localized names failed: {}", e))
    }

    /// 查询系统主题（未应用宿主显式 light/dark 配置），返回 `light` 或 `dark`。
    pub async fn get_system_theme(&self) -> Result<String, String> {
        let result = self
            .send_request(host::THEME_GET_SYSTEM, serde_json::Value::Null)
            .await?;
        result
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "host system theme response is not a string".to_string())
    }

    /// 解析参数模板（宿主注入系统参数、拼装用户参数），返回填充后的字符串。
    pub async fn resolve_parameters(
        &self,
        template: &str,
        user_args: &[String],
    ) -> Result<String, String> {
        let params = serde_json::to_value(ParameterResolveParams {
            user_args: user_args.to_vec(),
            template: template.to_string(),
        })
        .map_err(|e| e.to_string())?;
        let result = self.send_request(host::PARAMETER_RESOLVE, params).await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// 统计模板中需要用户输入的参数数量。
    pub async fn count_user_parameters(&self, template: &str) -> Result<usize, String> {
        let params = serde_json::to_value(ParameterTemplateParams {
            template: template.to_string(),
        })
        .map_err(|e| e.to_string())?;
        let result = self
            .send_request(host::PARAMETER_COUNT_USER, params)
            .await?;
        let count = result
            .as_u64()
            .ok_or_else(|| "host parameter.count_user response is not a number".to_string())?;
        Ok(count as usize)
    }

    /// 判断模板是否包含系统参数。
    pub async fn has_system_parameters(&self, template: &str) -> Result<bool, String> {
        let params = serde_json::to_value(ParameterTemplateParams {
            template: template.to_string(),
        })
        .map_err(|e| e.to_string())?;
        let result = self
            .send_request(host::PARAMETER_HAS_SYSTEM, params)
            .await?;
        result
            .as_bool()
            .ok_or_else(|| "host parameter.has_system response is not a bool".to_string())
    }

    // ===== 插件本地缓存（作用域 <AppCacheDir>/<plugin_id>/<domain>/<key>）=====

    /// 写入插件本地缓存（可再生的本地数据，不经存储后端）。
    pub async fn cache_put(&self, domain: &str, key: &str, data: &[u8]) -> Result<(), String> {
        let params = serde_json::to_value(CachePutParams {
            domain: domain.to_string(),
            key: key.to_string(),
            bytes_b64: STANDARD.encode(data),
        })
        .map_err(|e| e.to_string())?;
        self.send_request(host::CACHE_PUT, params).await?;
        Ok(())
    }

    /// 读取插件本地缓存；不存在返回 None（区别于空字节数据）。
    pub async fn cache_get(&self, domain: &str, key: &str) -> Result<Option<Vec<u8>>, String> {
        let params = serde_json::to_value(CacheGetParams {
            domain: domain.to_string(),
            key: key.to_string(),
        })
        .map_err(|e| e.to_string())?;
        let result = self.send_request(host::CACHE_GET, params).await?;
        let Some(b64) = result.as_str() else {
            return Ok(None);
        };
        STANDARD
            .decode(b64)
            .map(Some)
            .map_err(|e| format!("base64 decode failed: {}", e))
    }

    /// 删除插件本地缓存条目；不存在视为成功。
    pub async fn cache_delete(&self, domain: &str, key: &str) -> Result<(), String> {
        let params = serde_json::to_value(CacheDeleteParams {
            domain: domain.to_string(),
            key: key.to_string(),
        })
        .map_err(|e| e.to_string())?;
        self.send_request(host::CACHE_DELETE, params).await?;
        Ok(())
    }

    /// 容量控制：缓存域条目超过 max_entries 时按修改时间删除最旧条目。
    pub async fn cache_cleanup(&self, domain: &str, max_entries: usize) -> Result<(), String> {
        let params = serde_json::to_value(CacheCleanupParams {
            domain: domain.to_string(),
            max_entries,
        })
        .map_err(|e| e.to_string())?;
        self.send_request(host::CACHE_CLEANUP, params).await?;
        Ok(())
    }

    // ===== 定时器（到期事件由宿主推送，经 events 注册表分发）=====

    /// 创建一次性定时器；到期时调用 handler（入参为定时器 ID）。
    /// 返回：宿主分配的定时器 ID（可用于 cancel_timer）。
    pub async fn set_timeout(
        &self,
        delay: Duration,
        handler: impl Fn(u64) + Send + Sync + 'static,
    ) -> Result<u64, String> {
        let params = serde_json::to_value(TimerSetTimeoutParams {
            delay_ms: delay.as_millis() as u64,
        })
        .map_err(|e| e.to_string())?;
        let result = self.send_request(host::TIMER_SET_TIMEOUT, params).await?;
        let id = result
            .as_u64()
            .ok_or_else(|| "host timer response is not a number".to_string())?;
        events::register_timer(id, false, Arc::new(handler));
        Ok(id)
    }

    /// 创建重复定时器；每次到期调用 handler（入参为定时器 ID）。
    /// 返回：宿主分配的定时器 ID（可用于 cancel_timer）。
    pub async fn set_interval(
        &self,
        interval: Duration,
        handler: impl Fn(u64) + Send + Sync + 'static,
    ) -> Result<u64, String> {
        let params = serde_json::to_value(TimerSetIntervalParams {
            interval_ms: interval.as_millis() as u64,
        })
        .map_err(|e| e.to_string())?;
        let result = self.send_request(host::TIMER_SET_INTERVAL, params).await?;
        let id = result
            .as_u64()
            .ok_or_else(|| "host timer response is not a number".to_string())?;
        events::register_timer(id, true, Arc::new(handler));
        Ok(id)
    }

    /// 取消指定定时器（本地处理函数登记随取消成功一并移除）。
    pub async fn cancel_timer(&self, id: u64) -> Result<(), String> {
        let params = serde_json::to_value(TimerCancelParams { id }).map_err(|e| e.to_string())?;
        self.send_request(host::TIMER_CANCEL, params).await?;
        events::unregister_timer(id);
        Ok(())
    }

    /// 取消本插件的全部定时器（不影响其他插件）。
    pub async fn cancel_all_timers(&self) -> Result<(), String> {
        let params = serde_json::to_value(TimerCancelAllParams {}).map_err(|e| e.to_string())?;
        self.send_request(host::TIMER_CANCEL_ALL, params).await?;
        events::clear_timers();
        Ok(())
    }

    // ===== 事件订阅（事件由宿主推送，按插件内 ID 路由到 handler）=====

    /// 订阅按键事件；事件到达时调用 handler。
    /// 参数：id - 插件内订阅 ID（退订时使用同一 ID）；filter - 关注的事件过滤器。
    pub async fn subscribe_hotkey(
        &self,
        id: &str,
        filter: HotkeyEventFilter,
        handler: impl Fn(HotkeyEvent) + Send + Sync + 'static,
    ) -> Result<(), String> {
        let params = serde_json::to_value(HotkeySubscribeParams {
            id: id.to_string(),
            filter,
        })
        .map_err(|e| e.to_string())?;
        self.send_request(host::HOTKEY_SUBSCRIBE, params).await?;
        events::register_hotkey(id.to_string(), Arc::new(handler));
        Ok(())
    }

    /// 退订按键事件。
    pub async fn unsubscribe_hotkey(&self, id: &str) -> Result<(), String> {
        let params = serde_json::to_value(HotkeyUnsubscribeParams { id: id.to_string() })
            .map_err(|e| e.to_string())?;
        self.send_request(host::HOTKEY_UNSUBSCRIBE, params).await?;
        events::unregister_hotkey(id);
        Ok(())
    }

    /// 订阅安装监控事件（开始菜单等目录变化）；事件到达时调用 handler。
    pub async fn subscribe_installation(
        &self,
        id: &str,
        handler: impl Fn(InstallationEvent) + Send + Sync + 'static,
    ) -> Result<(), String> {
        let params = serde_json::to_value(InstallationSubscribeParams { id: id.to_string() })
            .map_err(|e| e.to_string())?;
        self.send_request(host::INSTALLATION_SUBSCRIBE, params)
            .await?;
        events::register_installation(id.to_string(), Arc::new(handler));
        Ok(())
    }

    /// 退订安装监控事件。
    pub async fn unsubscribe_installation(&self, id: &str) -> Result<(), String> {
        let params = serde_json::to_value(InstallationUnsubscribeParams { id: id.to_string() })
            .map_err(|e| e.to_string())?;
        self.send_request(host::INSTALLATION_UNSUBSCRIBE, params)
            .await?;
        events::unregister_installation(id);
        Ok(())
    }

    /// 订阅窗口焦点事件；事件到达时调用 handler。
    pub async fn subscribe_focus(
        &self,
        id: &str,
        handler: impl Fn(FocusEvent) + Send + Sync + 'static,
    ) -> Result<(), String> {
        let params = serde_json::to_value(FocusSubscribeParams { id: id.to_string() })
            .map_err(|e| e.to_string())?;
        self.send_request(host::FOCUS_SUBSCRIBE, params).await?;
        events::register_focus(id.to_string(), Arc::new(handler));
        Ok(())
    }

    /// 退订窗口焦点事件。
    pub async fn unsubscribe_focus(&self, id: &str) -> Result<(), String> {
        let params = serde_json::to_value(FocusUnsubscribeParams { id: id.to_string() })
            .map_err(|e| e.to_string())?;
        self.send_request(host::FOCUS_UNSUBSCRIBE, params).await?;
        events::unregister_focus(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proxy() -> (Arc<HostProxy>, mpsc::Receiver<Vec<u8>>) {
        let (tx, rx) = mpsc::channel::<Vec<u8>>(16);
        let pending: Arc<DashMap<u64, oneshot::Sender<Result<serde_json::Value, JsonRpcError>>>> =
            Arc::new(DashMap::new());
        (Arc::new(HostProxy::new(pending, tx)), rx)
    }

    /// 回归测试：host/* 请求必须以**干净 JSON** 投递到 outbound 通道，
    /// 分帧由 write_task 统一完成。
    ///
    /// 修复前 send_request/log_no_wait 在投递前先 encode_frame，
    /// 导致双重分帧（帧内嵌帧）：写出的字节为
    /// `Content-Length: M\r\n\r\nContent-Length: N\r\n\r\n{JSON}`，
    /// 宿主 read_frame 后 body 以 `Content-Length:` 开头，serde_json 解析失败
    /// （`expected value at line 1 column 1`），host/* 调用全部超时。
    #[tokio::test]
    async fn send_request_posts_clean_json_without_embedded_frame() {
        let (proxy, mut rx) = make_proxy();

        // log_no_wait 是同步 fire-and-forget：直接检查通道里的字节。
        proxy.log_no_wait("warn", "test message");
        let bytes = rx.recv().await.expect("log_no_wait 应投递一条消息");
        let text = String::from_utf8(bytes.clone()).expect("UTF-8");
        assert!(
            !text.contains("Content-Length"),
            "outbound 通道中的消息不得包含帧头（双重分帧）: {:?}",
            text
        );
        // 必须是合法 JSON-RPC 消息
        let v: serde_json::Value = serde_json::from_slice(&bytes).expect("干净 JSON 可解析");
        assert_eq!(v["method"], host::LOG);
        assert_eq!(v["params"]["message"], "test message");
    }

    /// send_request 投递的也必须是干净 JSON（await 路径）。
    #[tokio::test]
    async fn send_request_await_path_posts_clean_json() {
        let (proxy, mut rx) = make_proxy();
        // send_request 会 await 响应（oneshot 永不完成→挂起），
        // 因此只在通道上取一条消息验证负载，然后丢弃任务。
        let task = tokio::spawn(async move {
            let _ = proxy.model_list().await; // 挂起直到超时（30s），任务随测试结束
        });
        let bytes = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("send_request 应投递一条消息")
            .expect("通道未关闭");
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains("Content-Length"),
            "outbound 通道中的消息不得包含帧头（双重分帧）: {:?}",
            text
        );
        let v: serde_json::Value = serde_json::from_slice(&bytes).expect("干净 JSON 可解析");
        assert_eq!(v["method"], host::MODEL_LIST);
        // 清理：中止挂起的任务
        task.abort();
    }
}
