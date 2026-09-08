//! PluginHost — 宿主操作契约（插件 → 宿主的单向中转接缝）。
//!
//! 插件持有的 `PluginHandle` 只保存插件身份/配置/能力集，不保存任何服务实现；
//! 全部可调用操作通过本契约委托给宿主执行（进程内实现为 src-tauri 的 `HostApi`，
//! 测试实现为 mock 桩宿主）。宿主在实现中决定「直接处理宿主级服务」还是
//! 「转发到平台实现」（`crates/plugin-api/src/platform` 的 PlatformServices）。
//!
//! 作用域约定：需要按插件隔离的操作（资源/缓存路径、回调 ID 前缀、图标缓存等级）
//! 由委托方（PluginHandle / RPC 宿主侧接收端）显式传入插件身份参数，
//! 使宿主可复用同一执行节点服务所有插件。

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;

use crate::host::{CacheLevel, HostApiError, OpenTarget};
use crate::services::app::AppInfo;
use crate::services::focus_monitor::FocusCallback;
use crate::services::hotkey::types::{HotkeyCallback, HotkeyEventFilter};
use crate::services::installation_monitor::types::InstallationCallback;
use crate::services::model::{
    ModelChatRequest, ModelChatResponse, ModelEmbeddingRequest, ModelEmbeddingResponse, ModelError,
    ModelInfo, ModelSimilarityRequest, ModelSimilarityResponse,
};
use crate::services::parameter::types::ParameterSnapshot;
use crate::services::path::path_resolver::KnownPath;
use crate::services::theme::Theme;
use crate::services::timer::types::{TimerCallback, TimerId};
use crate::services::IconRequest;

/// 宿主操作契约。实现方为宿主（真实 HostApi）或测试桩宿主。
#[async_trait]
pub trait PluginHost: Send + Sync {
    // ===== 图标服务 =====

    /// 根据图标请求提取图标数据，行为由缓存等级决定。
    /// 参数：request - 图标请求；level - 插件配置的缓存等级。
    /// 返回：WebP 格式的图标字节数据（回退路径可能为 PNG），失败返回 HostApiError。
    async fn get_icon(
        &self,
        request: &IconRequest,
        level: CacheLevel,
    ) -> Result<Vec<u8>, HostApiError>;

    /// 提取图标数据，失败时回退到默认图标（永不返回错误）。
    async fn get_icon_or_default(&self, request: &IconRequest, level: CacheLevel) -> Vec<u8>;

    /// 强制从磁盘提取图标数据并根据缓存等级更新缓存（跳过缓存读取）。
    async fn get_icon_and_update_cache(
        &self,
        request: &IconRequest,
        level: CacheLevel,
    ) -> Result<Vec<u8>, HostApiError>;

    /// 覆盖指定 IconRequest 的缓存图标为自定义图标文件。
    /// 参数：original_request - 需要覆盖图标的原始 IconRequest；
    ///       custom_icon_path - 用户选择的自定义图标文件路径。
    async fn override_icon_cache(
        &self,
        original_request: &IconRequest,
        custom_icon_path: &str,
    ) -> Result<(), HostApiError>;

    // ===== Shell 服务 =====

    /// 使用系统默认方式打开目标（文件/网址/文件夹）。
    async fn shell_open(&self, target: OpenTarget) -> Result<(), HostApiError>;

    /// 在文件资源管理器中打开指定路径的父目录并选中该文件。
    async fn shell_open_folder(&self, path: &str) -> Result<(), HostApiError>;

    /// 以管理员权限启动程序。
    async fn shell_execute_elevation(&self, path: &str) -> Result<(), HostApiError>;

    /// 执行命令字符串（后台运行，无窗口）。
    async fn shell_execute_command(&self, command: &str) -> Result<(), HostApiError>;

    // ===== 窗口服务 =====

    /// 根据进程名（如 "chrome.exe"）激活已存在的窗口。
    /// 返回：成功激活返回 Ok(true)，未找到窗口返回 Ok(false)。
    async fn activate_window_by_process(&self, process_name: &str) -> Result<bool, HostApiError>;

    /// 根据窗口标题的部分内容激活已存在的窗口。
    async fn activate_window_by_title(&self, title: &str) -> Result<bool, HostApiError>;

    /// 根据进程 PID 激活已存在的窗口。
    async fn activate_window_by_pid(&self, pid: u32) -> Result<bool, HostApiError>;

    // ===== 路径服务 =====

    /// 根据已知路径类型解析实际文件系统路径。
    fn resolve_path(&self, path: KnownPath) -> Result<String, HostApiError>;

    // ===== 剪贴板服务 =====

    /// 将文本写入系统剪贴板。
    fn set_clipboard_text(&self, text: &str) -> Result<(), HostApiError>;

    // ===== 应用服务 =====

    /// 枚举当前平台已安装的应用。
    async fn enumerate_apps(&self) -> Vec<AppInfo>;

    /// 启动指定应用。返回：成功返回 Ok(pid)。
    async fn launch_app(&self, app_id: &str, args: Option<&[String]>) -> Result<u32, HostApiError>;

    // ===== 应用资源服务 =====

    /// 根据名称获取内置图标资源的文件系统路径，未注册则返回 None。
    fn get_app_icon_path(&self, name: &str) -> Option<String>;

    // ===== 快捷方式解析 =====

    /// 解析 .lnk 快捷方式文件的目标路径，失败返回 None。
    fn resolve_lnk_target(&self, lnk_path: &str) -> Option<String>;

    /// 解析指定目录下的 desktop.ini 文件，提取 [LocalizedFileNames] 部分。
    fn parse_localized_names_from_dir(&self, dir_path: &Path) -> HashMap<String, String>;

    // ===== 主题服务 =====

    /// 查询宿主当前实际生效的界面主题。
    /// 显式 light/dark 配置直接返回；system 模式委托平台读取系统主题。
    fn get_theme(&self) -> Result<Theme, HostApiError>;

    /// 查询系统主题（未应用宿主显式 light/dark 配置），供前端 system 模式跟随。
    fn get_system_theme(&self) -> Result<Theme, HostApiError>;

    // ===== 模型服务 =====

    /// 全网模型清单（聚合缓存，含所有已注册提供方）。
    fn model_list(&self) -> Vec<ModelInfo>;

    /// 按 model_id 调用文本生成。
    async fn model_chat(&self, req: ModelChatRequest) -> Result<ModelChatResponse, ModelError>;

    /// 按 model_id 调用文本向量化（task_type 必填，宿主对缺失/未知值返回 InvalidRequest）。
    async fn model_embedding(
        &self,
        req: ModelEmbeddingRequest,
    ) -> Result<ModelEmbeddingResponse, ModelError>;

    /// 按 model_id 计算查询向量与多个目标向量的相似度。
    async fn model_similarity(
        &self,
        req: ModelSimilarityRequest,
    ) -> Result<ModelSimilarityResponse, ModelError>;

    // ===== 参数解析服务 =====

    /// 解析参数模板。返回：填充后的完整字符串。
    async fn resolve_parameters(
        &self,
        template: &str,
        user_args: &[String],
        snapshot: &ParameterSnapshot,
    ) -> Result<String, HostApiError>;

    /// 统计模板中需要用户输入的参数数量。
    fn count_user_parameters(&self, template: &str) -> usize;

    /// 检查模板是否包含系统参数。
    fn has_system_parameters(&self, template: &str) -> bool;

    // ===== 定时器服务 =====

    /// 创建一个一次性定时器，在指定延迟后触发回调。
    async fn set_timeout(
        &self,
        delay: Duration,
        callback: TimerCallback,
    ) -> Result<TimerId, HostApiError>;

    /// 创建一个重复定时器，每隔指定间隔触发回调。
    async fn set_interval(
        &self,
        interval: Duration,
        callback: TimerCallback,
    ) -> Result<TimerId, HostApiError>;

    /// 取消指定 ID 的定时器。
    async fn cancel_timer(&self, id: TimerId) -> Result<(), HostApiError>;

    /// 取消所有定时器。
    async fn cancel_all_timers(&self) -> Result<(), HostApiError>;

    // ===== 资源管理（插件作用域） =====

    /// 上传资源文件到指定插件的资源空间。
    /// 参数：plugin_id - 插件 ID（决定存储命名空间）。
    async fn resource_upload(
        &self,
        plugin_id: &str,
        resource_id: &str,
        file_path: &str,
        max_size: Option<u64>,
    ) -> Result<String, HostApiError>;

    /// 直接写入资源字节数据，无需本地路径。
    async fn resource_put(
        &self,
        plugin_id: &str,
        resource_id: &str,
        data: &[u8],
    ) -> Result<(), HostApiError>;

    /// 获取资源文件内容。
    async fn resource_get(
        &self,
        plugin_id: &str,
        resource_id: &str,
    ) -> Result<Vec<u8>, HostApiError>;

    /// 删除资源文件。
    async fn resource_delete(&self, plugin_id: &str, resource_id: &str)
        -> Result<(), HostApiError>;

    /// 列出指定插件的所有资源。
    async fn resource_list(&self, plugin_id: &str) -> Result<Vec<String>, HostApiError>;

    // ===== 本地缓存（插件作用域） =====

    /// 写入插件本地缓存（可再生的本地数据，不经 StorageService 远端同步）。
    async fn cache_put(
        &self,
        plugin_id: &str,
        domain: &str,
        key: &str,
        data: &[u8],
    ) -> Result<(), HostApiError>;

    /// 读取插件本地缓存；缓存不存在时返回 Ok(None)。
    async fn cache_get(
        &self,
        plugin_id: &str,
        domain: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, HostApiError>;

    /// 删除插件本地缓存条目；缓存不存在时视为成功。
    async fn cache_delete(
        &self,
        plugin_id: &str,
        domain: &str,
        key: &str,
    ) -> Result<(), HostApiError>;

    /// 容量控制：缓存域条目超过 max_entries 时按修改时间删除最旧条目。
    async fn cache_cleanup(
        &self,
        plugin_id: &str,
        domain: &str,
        max_entries: usize,
    ) -> Result<(), HostApiError>;

    // ===== 推送式回调注册（插件作用域） =====

    /// 注册按键事件回调。ID 自动前缀化为 "{plugin_id}:{id}"。
    fn register_hotkey_callback(
        &self,
        plugin_id: &str,
        id: &str,
        filter: HotkeyEventFilter,
        callback: HotkeyCallback,
    );

    /// 注销按键事件回调。
    fn unregister_hotkey_callback(&self, plugin_id: &str, id: &str);

    /// 注册安装事件回调。ID 自动前缀化为 "{plugin_id}:{id}"。
    fn register_installation_callback(
        &self,
        plugin_id: &str,
        id: &str,
        callback: InstallationCallback,
    );

    /// 注销安装事件回调。
    fn unregister_installation_callback(&self, plugin_id: &str, id: &str);

    /// 注册焦点事件回调。ID 自动前缀化为 "{plugin_id}:{id}"。
    fn register_focus_callback(&self, plugin_id: &str, id: &str, callback: FocusCallback);

    /// 注销焦点事件回调。
    fn unregister_focus_callback(&self, plugin_id: &str, id: &str);
}
