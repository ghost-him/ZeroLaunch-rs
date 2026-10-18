//! TauriHostCallHandler — 将插件的 `host/*` RPC 调用分发给插件句柄。
//! 注册的定时器到期与订阅事件经注入的 `PluginEventSink` 以 `plugin/*`
//! 通知推送给插件进程；通道在进程 spawn 后注入，卸载/崩溃前 detach 清理。

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use parking_lot::RwLock;
use serde_json::{from_value, json, Value};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;

use zerolaunch_plugin_api::host::OpenTarget;
use zerolaunch_plugin_api::services::focus_monitor::{FocusCallback, FocusEvent};
use zerolaunch_plugin_api::services::hotkey::types::{HotkeyCallback, HotkeyEvent};
use zerolaunch_plugin_api::services::installation_monitor::types::{
    InstallationCallback, InstallationEvent,
};
use zerolaunch_plugin_api::services::model::{
    ModelChatRequest, ModelEmbeddingRequest, ModelSimilarityRequest,
};
use zerolaunch_plugin_api::services::parameter::types::ParameterSnapshot;
use zerolaunch_plugin_api::services::path::KnownPath;
use zerolaunch_plugin_api::services::theme::Theme;
use zerolaunch_plugin_api::services::timer::types::{TimerCallback, TimerId};
use zerolaunch_plugin_host::host_dispatch::{HostCallHandler, PluginEventSink};
use zerolaunch_plugin_protocol::codec::summarize_value;
use zerolaunch_plugin_protocol::methods::{event, host};
use zerolaunch_plugin_protocol::{
    codes, AppIconPathParams, AppLaunchParams, CacheCleanupParams, CacheDeleteParams,
    CacheGetParams, CachePutParams, ClipboardSetTextParams, FocusEventParams, FocusSubscribeParams,
    FocusUnsubscribeParams, HotkeyEventParams, HotkeySubscribeParams, HotkeyUnsubscribeParams,
    IconOverrideCacheParams, IconRequestParams, InstallationEventParams,
    InstallationSubscribeParams, InstallationUnsubscribeParams, JsonRpcError, LogParams,
    NotifyParams, ParameterResolveParams, ParameterTemplateParams, PathLocalizedNamesParams,
    PathResolveLnkParams, PathResolveParams, ResourceDeleteParams, ResourceGetParams,
    ResourceListParams, ResourcePutParams, ResourceUploadParams, SdkConfigUpdateParams,
    ShellExecuteCommandParams, ShellExecuteElevationParams, ShellOpenFolderParams, ShellOpenParams,
    TimerCancelAllParams, TimerCancelParams, TimerFiredParams, TimerSetIntervalParams,
    TimerSetTimeoutParams, WindowActivateByProcessParams, WindowActivateByTitleParams,
    WindowActivateParams,
};

use crate::core::i18n::I18nManager;
use crate::sdk::HostApi;

/// 本处理器为单个插件登记的订阅 ID 集合（按事件族分组）。
/// 只由 `handle_host_call`（订阅/退订分支）写入，`detach_event_sink` 读取并在卸载时注销，
/// 避免插件进程退出后残留指向失效推送通道的回调。
/// 设计债：与 HostApi 内按 "{plugin_id}:{id}" 前缀隔离的回调注册表构成双权威。
#[derive(Default)]
struct Subscriptions {
    /// 已注册的按键事件订阅 ID（未前缀化的插件内 ID）。
    hotkey: BTreeSet<String>,
    /// 已注册的安装监控订阅 ID（未前缀化的插件内 ID）。
    installation: BTreeSet<String>,
    /// 已注册的窗口焦点订阅 ID（未前缀化的插件内 ID）。
    focus: BTreeSet<String>,
}

/// 将插件的 `host/*` RPC 调用分发给本地的 PluginHandle。
pub(crate) struct TauriHostCallHandler {
    pub(crate) host_api: Arc<HostApi>,
    pub(crate) plugin_id: String,
    pub(crate) app_handle: Option<Arc<AppHandle>>,
    /// 后端翻译服务（host/i18n.get_locale 查询当前语言）
    pub(crate) i18n: Arc<I18nManager>,
    /// 宿主 → 插件事件推送通道。由 plugin-host 在进程 spawn 后注入（重启覆盖）；
    /// None 表示进程未就绪或已卸载，此时不推送事件（订阅/定时器注册返回错误）。
    event_sink: RwLock<Option<Arc<dyn PluginEventSink>>>,
    /// 本处理器为该插件登记的订阅 ID（卸载时用于注销）。
    subscriptions: RwLock<Subscriptions>,
}

impl TauriHostCallHandler {
    /// 构造分发处理器（事件推送通道由 plugin-host 在 spawn 后注入）。
    /// 参数：host_api - 宿主执行节点；plugin_id - 插件 ID；app_handle - 通知等
    ///       Tauri 能力所需的句柄（None 表示无 UI 环境）；i18n - 语言查询服务。
    /// 返回：尚未绑定推送通道的处理器。
    pub(crate) fn new(
        host_api: Arc<HostApi>,
        plugin_id: String,
        app_handle: Option<Arc<AppHandle>>,
        i18n: Arc<I18nManager>,
    ) -> Self {
        Self {
            host_api,
            plugin_id,
            app_handle,
            i18n,
            event_sink: RwLock::new(None),
            subscriptions: RwLock::new(Subscriptions::default()),
        }
    }

    /// 取当前事件推送通道。
    /// 返回：已注入返回通道引用；未注入（进程未就绪/已卸载）返回宿主错误。
    fn require_sink(&self) -> Result<Arc<dyn PluginEventSink>, JsonRpcError> {
        self.event_sink.read().clone().ok_or_else(|| {
            JsonRpcError::new(codes::PLUGIN_ERROR, "事件推送通道未就绪（插件进程未运行）")
        })
    }

    /// 构造一次性/重复定时器的到期回调：把 `plugin/timer.fired` 推给本插件进程。
    /// 参数：id - 到期时回调收到的定时器 ID（回传给插件）。
    /// 返回：推送事件到注册时刻通道的定时器回调。
    fn timer_callback(sink: Arc<dyn PluginEventSink>) -> TimerCallback {
        Arc::new(move |id: TimerId| {
            match serde_json::to_value(TimerFiredParams { id: id.value() }) {
                Ok(payload) => sink.notify(event::TIMER_FIRED, payload),
                Err(e) => tracing::warn!("序列化定时器事件失败: {e}"),
            }
        })
    }
}

#[async_trait::async_trait]
impl HostCallHandler for TauriHostCallHandler {
    async fn handle_host_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        tracing::debug!(
            "收到插件 host/* 调用: plugin={} method={} params={}",
            self.plugin_id,
            method,
            summarize_value(&params)
        );

        let handle = self
            .host_api
            .get_plugin_handle(&self.plugin_id)
            .ok_or_else(|| JsonRpcError::new(codes::PLUGIN_ERROR, "PluginHandle not found"))?;

        match method {
            host::LOG => {
                let p: LogParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                match p.level.as_str() {
                    "error" => tracing::error!("[plugin {}] {}", self.plugin_id, p.message),
                    "warn" => tracing::warn!("[plugin {}] {}", self.plugin_id, p.message),
                    "debug" => tracing::debug!("[plugin {}] {}", self.plugin_id, p.message),
                    _ => tracing::info!("[plugin {}] {}", self.plugin_id, p.message),
                }
                Ok(Value::Null)
            }
            host::NOTIFY => {
                let p: NotifyParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                if let Some(app_handle) = &self.app_handle {
                    use tauri_plugin_notification::NotificationExt;
                    let _ = app_handle
                        .notification()
                        .builder()
                        .title(&p.title)
                        .body(&p.message)
                        .show();
                }
                Ok(Value::Null)
            }
            host::SHELL_OPEN => {
                let p: ShellOpenParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .shell_open(OpenTarget::File(p.target))
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            host::SHELL_OPEN_FOLDER => {
                let p: ShellOpenFolderParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .shell_open_folder(&p.path)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            host::SHELL_EXECUTE_ELEVATION => {
                let p: ShellExecuteElevationParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .shell_execute_elevation(&p.path)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            host::SHELL_EXECUTE_COMMAND => {
                let p: ShellExecuteCommandParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .shell_execute_command(&p.cmd, p.mode)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            // 按进程名激活（方法名与语义一致：processName）。
            host::WINDOW_ACTIVATE_BY_PROCESS => {
                let p: WindowActivateByProcessParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let activated = handle
                    .activate_window_by_process(&p.process_name)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(json!(activated))
            }
            // 按 PID 激活。
            host::WINDOW_ACTIVATE_BY_PID => {
                let p: WindowActivateParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let activated = handle
                    .activate_window_by_pid(p.pid)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(json!(activated))
            }
            // 按窗口标题片段激活。
            host::WINDOW_ACTIVATE_BY_TITLE => {
                let p: WindowActivateByTitleParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let activated = handle
                    .activate_window_by_title(&p.title)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(json!(activated))
            }
            // 提取失败回退默认图标（与 icon.extract 的严格语义不同）。
            host::ICON_GET => {
                let p: IconRequestParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let bytes = handle.get_icon_or_default(p.request).await;
                Ok(json!(STANDARD.encode(&bytes)))
            }
            // 严格提取：失败返回错误，不回退默认图标。
            host::ICON_EXTRACT => {
                let p: IconRequestParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let bytes = handle
                    .get_icon(p.request)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(json!(STANDARD.encode(&bytes)))
            }
            // 强制从磁盘提取并写回缓存。
            host::ICON_REFRESH => {
                let p: IconRequestParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let bytes = handle
                    .get_icon_and_update_cache(p.request)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(json!(STANDARD.encode(&bytes)))
            }
            // 用自定义图标文件覆盖缓存条目。
            host::ICON_OVERRIDE_CACHE => {
                let p: IconOverrideCacheParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .override_icon_cache(&p.request, &p.custom_icon_path)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            host::APP_ENUMERATE => {
                let apps = handle.enumerate_apps().await;
                Ok(serde_json::to_value(apps).unwrap_or_default())
            }
            // 启动应用（返回进程 PID）。
            host::APP_LAUNCH => {
                let p: AppLaunchParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let pid = handle
                    .launch_app(&p.app_id, p.args.as_deref())
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(json!(pid))
            }
            // 内置应用图标资源路径；未注册返回 null。
            host::APP_ICON_PATH => {
                let p: AppIconPathParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                Ok(json!(handle.get_app_icon_path(&p.name)))
            }
            host::PATH_RESOLVE => {
                let p: PathResolveParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let known_path = match p.kind.as_str() {
                    "AppDataDir" => KnownPath::AppDataDir,
                    "AppConfigDir" => KnownPath::AppConfigDir,
                    "AppCacheDir" => KnownPath::AppCacheDir,
                    "AppLogDir" => KnownPath::AppLogDir,
                    "AppIconCacheDir" => KnownPath::AppIconCacheDir,
                    // 未知 kind：返回错误而非静默回退 AppDataDir，避免路径语义漂移。
                    _ => {
                        return Err(JsonRpcError::new(
                            codes::INVALID_PARAMS,
                            format!("未知的路径类型: {}", p.kind),
                        ));
                    }
                };
                let path = handle
                    .resolve_path(known_path)
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(json!(path))
            }
            // 解析 .lnk 目标；失败返回 null。
            host::PATH_RESOLVE_LNK => {
                let p: PathResolveLnkParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                Ok(json!(handle.resolve_lnk_target(&p.lnk_path)))
            }
            // 目录 desktop.ini 的本地化文件名表。
            host::PATH_LOCALIZED_NAMES => {
                let p: PathLocalizedNamesParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let names = handle.parse_localized_names_from_dir(Path::new(&p.dir_path));
                Ok(serde_json::to_value(names).unwrap_or_default())
            }
            // 写系统剪贴板。
            host::CLIPBOARD_SET_TEXT => {
                let p: ClipboardSetTextParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .set_clipboard_text(&p.text)
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            // 平台能力集（camelCase 名称数组），插件据此降级。
            host::PLATFORM_CAPABILITIES => Ok(json!(handle.capabilities().to_list())),
            // 更新本插件的 SDK 配置（如图标缓存等级）。
            host::SDK_CONFIG_UPDATE => {
                let p: SdkConfigUpdateParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle.update_config(p.config);
                Ok(Value::Null)
            }
            //
            // 插件                           JSON-RPC 线上              宿主
            // ────                          ────────────              ────
            // upload:        传文件路径 → host/resource.upload ──→ resource_upload(file_path) → 读文件 → 存储
            // put:           raw bytes ──(encode)──→ base64 ──(decode)──→ resource_put(bytes) → 存储
            // get:           raw bytes ←──(decode)── base64 ←──(encode)── resource_get(id)
            //
            host::RESOURCE_UPLOAD => {
                let p: ResourceUploadParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let result = handle
                    .resource_upload(&p.resource_id, &p.file_path, p.max_size.map(|s| s as u64))
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(json!(result))
            }
            host::RESOURCE_PUT => {
                let p: ResourcePutParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let bytes = STANDARD
                    .decode(&p.bytes_b64)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .resource_put(&p.resource_id, &bytes)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            host::RESOURCE_GET => {
                let p: ResourceGetParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let data = handle
                    .resource_get(&p.resource_id)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(json!(STANDARD.encode(&data)))
            }
            host::RESOURCE_DELETE => {
                let p: ResourceDeleteParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .resource_delete(&p.resource_id)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            host::RESOURCE_LIST => {
                let _p: ResourceListParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let keys = handle
                    .resource_list()
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(serde_json::to_value(keys).unwrap_or_default())
            }
            // 插件本地缓存：不经 StorageService，仅落在本地缓存目录。
            host::CACHE_PUT => {
                let p: CachePutParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let bytes = STANDARD
                    .decode(&p.bytes_b64)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .cache_put(&p.domain, &p.key, &bytes)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            // 缓存未命中返回 null（区别于空字节数据）。
            host::CACHE_GET => {
                let p: CacheGetParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let data = handle
                    .cache_get(&p.domain, &p.key)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(match data {
                    Some(bytes) => json!(STANDARD.encode(&bytes)),
                    None => Value::Null,
                })
            }
            host::CACHE_DELETE => {
                let p: CacheDeleteParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .cache_delete(&p.domain, &p.key)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            host::CACHE_CLEANUP => {
                let p: CacheCleanupParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .cache_cleanup(&p.domain, p.max_entries)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            // 定时器：宿主保管回调，到期经推送通道发 plugin/timer.fired。
            host::TIMER_SET_TIMEOUT => {
                let p: TimerSetTimeoutParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let sink = self.require_sink()?;
                let id = handle
                    .set_timeout(
                        Duration::from_millis(p.delay_ms),
                        Self::timer_callback(sink),
                    )
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(json!(id.value()))
            }
            host::TIMER_SET_INTERVAL => {
                let p: TimerSetIntervalParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let sink = self.require_sink()?;
                let id = handle
                    .set_interval(
                        Duration::from_millis(p.interval_ms),
                        Self::timer_callback(sink),
                    )
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(json!(id.value()))
            }
            host::TIMER_CANCEL => {
                let p: TimerCancelParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .cancel_timer(TimerId::new(p.id))
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            host::TIMER_CANCEL_ALL => {
                let _p: TimerCancelAllParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle
                    .cancel_all_timers()
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(Value::Null)
            }
            // 订阅：宿主保存回调，事件到达时经推送通道转发；（取消）订阅按插件内 ID 前缀隔离。
            host::HOTKEY_SUBSCRIBE => {
                let p: HotkeySubscribeParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let sink = self.require_sink()?;
                let sub_id = p.id.clone();
                let callback: HotkeyCallback = Arc::new(move |ev: HotkeyEvent| {
                    match serde_json::to_value(HotkeyEventParams {
                        id: sub_id.clone(),
                        event: ev,
                    }) {
                        Ok(payload) => sink.notify(event::HOTKEY_EVENT, payload),
                        Err(e) => tracing::warn!("序列化按键事件失败: {e}"),
                    }
                });
                handle.register_hotkey_callback(&p.id, p.filter, callback);
                self.subscriptions.write().hotkey.insert(p.id);
                Ok(Value::Null)
            }
            host::HOTKEY_UNSUBSCRIBE => {
                let p: HotkeyUnsubscribeParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle.unregister_hotkey_callback(&p.id);
                self.subscriptions.write().hotkey.remove(&p.id);
                Ok(Value::Null)
            }
            host::INSTALLATION_SUBSCRIBE => {
                let p: InstallationSubscribeParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let sink = self.require_sink()?;
                let sub_id = p.id.clone();
                let callback: InstallationCallback = Arc::new(move |ev: InstallationEvent| {
                    match serde_json::to_value(InstallationEventParams {
                        id: sub_id.clone(),
                        event: ev,
                    }) {
                        Ok(payload) => sink.notify(event::INSTALLATION_EVENT, payload),
                        Err(e) => tracing::warn!("序列化安装事件失败: {e}"),
                    }
                });
                handle.register_installation_callback(&p.id, callback);
                self.subscriptions.write().installation.insert(p.id);
                Ok(Value::Null)
            }
            host::INSTALLATION_UNSUBSCRIBE => {
                let p: InstallationUnsubscribeParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle.unregister_installation_callback(&p.id);
                self.subscriptions.write().installation.remove(&p.id);
                Ok(Value::Null)
            }
            host::FOCUS_SUBSCRIBE => {
                let p: FocusSubscribeParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let sink = self.require_sink()?;
                let sub_id = p.id.clone();
                let callback: FocusCallback = Arc::new(move |ev: FocusEvent| {
                    match serde_json::to_value(FocusEventParams {
                        id: sub_id.clone(),
                        event: ev,
                    }) {
                        Ok(payload) => sink.notify(event::FOCUS_EVENT, payload),
                        Err(e) => tracing::warn!("序列化焦点事件失败: {e}"),
                    }
                });
                handle.register_focus_callback(&p.id, callback);
                self.subscriptions.write().focus.insert(p.id);
                Ok(Value::Null)
            }
            host::FOCUS_UNSUBSCRIBE => {
                let p: FocusUnsubscribeParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                handle.unregister_focus_callback(&p.id);
                self.subscriptions.write().focus.remove(&p.id);
                Ok(Value::Null)
            }
            // 查询宿主当前界面语言（如 "zh-Hans"），插件按需本地化动态文本。
            host::GET_LOCALE => Ok(json!(self.i18n.current_language())),
            // 查询宿主当前实际生效主题，system 模式由 PluginHandle 解析。
            host::GET_THEME => Ok(json!(match handle
                .get_theme()
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?
            {
                Theme::Light => "light",
                Theme::Dark => "dark",
            })),
            // 查询系统主题（未应用宿主显式 light/dark 配置）。
            host::THEME_GET_SYSTEM => Ok(json!(match handle
                .get_system_theme()
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?
            {
                Theme::Light => "light",
                Theme::Dark => "dark",
            })),
            // 参数模板：完整解析 / 统计用户参数数量 / 判断是否含系统参数。
            host::PARAMETER_RESOLVE => {
                let p: ParameterResolveParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let snapshot = ParameterSnapshot::empty();
                let result = handle
                    .resolve_parameters(&p.template, &p.user_args, &snapshot)
                    .await
                    .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
                Ok(json!(result))
            }
            host::PARAMETER_COUNT_USER => {
                let p: ParameterTemplateParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                Ok(json!(handle.count_user_parameters(&p.template)))
            }
            host::PARAMETER_HAS_SYSTEM => {
                let p: ParameterTemplateParams = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                Ok(json!(handle.has_system_parameters(&p.template)))
            }
            // 全网模型清单（聚合缓存）。
            host::MODEL_LIST => Ok(serde_json::to_value(handle.model_list()).unwrap_or_default()),
            // 按 model_id 调用文本生成。
            host::MODEL_CHAT => {
                let req: ModelChatRequest = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let resp = handle.model_chat(req).await.map_err(model_error_to_rpc)?;
                Ok(serde_json::to_value(resp).unwrap_or_default())
            }
            // 按 model_id 调用文本向量化。
            host::MODEL_EMBEDDING => {
                let req: ModelEmbeddingRequest = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let resp = handle
                    .model_embedding(req)
                    .await
                    .map_err(model_error_to_rpc)?;
                Ok(serde_json::to_value(resp).unwrap_or_default())
            }
            // 按 model_id 计算查询向量与多个目标向量的相似度。
            host::MODEL_SIMILARITY => {
                let req: ModelSimilarityRequest = from_value(params)
                    .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
                let resp = handle
                    .model_similarity(req)
                    .await
                    .map_err(model_error_to_rpc)?;
                Ok(serde_json::to_value(resp).unwrap_or_default())
            }
            _ => Err(JsonRpcError::new(
                codes::METHOD_NOT_FOUND,
                format!("host method not found: {}", method),
            )),
        }
    }

    /// 记录插件进程的推送通道（spawn 后由 plugin-host 注入；重启时覆盖为新通道）。
    fn attach_event_sink(&self, sink: Arc<dyn PluginEventSink>) {
        *self.event_sink.write() = Some(sink);
    }

    /// 推送通道失效（进程卸载/崩溃）：清空通道，注销该插件的订阅回调并取消其定时器。
    /// 在插件句柄注销（HostApi::unregister）之前调用。
    fn detach_event_sink(&self) {
        *self.event_sink.write() = None;
        let subs = std::mem::take(&mut *self.subscriptions.write());
        let Some(handle) = self.host_api.get_plugin_handle(&self.plugin_id) else {
            return;
        };
        for id in subs.hotkey {
            handle.unregister_hotkey_callback(&id);
        }
        for id in subs.installation {
            handle.unregister_installation_callback(&id);
        }
        for id in subs.focus {
            handle.unregister_focus_callback(&id);
        }
        // 取消该插件的全部定时器（不影响其他插件）。
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = handle.cancel_all_timers().await {
                tracing::warn!("卸载时取消插件定时器失败: {e}");
            }
        });
    }
}

/// 将模型服务错误映射为 JSON-RPC 错误（专用错误码 + 原始消息）。
/// 插件经 host/model.* 调用可区分 ModelNotFound / ProviderUnavailable 等语义。
fn model_error_to_rpc(e: zerolaunch_plugin_api::services::model::ModelError) -> JsonRpcError {
    use zerolaunch_plugin_api::services::model::ModelError;
    let (code, message) = match e {
        ModelError::NotSupported => (codes::MODEL_NOT_SUPPORTED, e.to_string()),
        ModelError::ModelNotFound(_) => (codes::MODEL_NOT_FOUND, e.to_string()),
        ModelError::ProviderUnavailable(_) => (codes::MODEL_PROVIDER_UNAVAILABLE, e.to_string()),
        ModelError::InvalidRequest(_) => (codes::MODEL_INVALID_REQUEST, e.to_string()),
        ModelError::Transport(_) => (codes::MODEL_TRANSPORT, e.to_string()),
        ModelError::Internal(_) => (codes::INTERNAL_ERROR, e.to_string()),
    };
    JsonRpcError::new(code, message)
}
