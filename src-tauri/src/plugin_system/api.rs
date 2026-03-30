use super::types::{LogLevel, PluginAPI, PluginContext};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DefaultPluginAPI {
    settings: DashMap<String, String>,
    refresh_callback: RwLock<Option<Arc<dyn Fn() + Send + Sync>>>,
    hide_window_callback: RwLock<Option<Arc<dyn Fn() + Send + Sync>>>,
}

impl DefaultPluginAPI {
    /// 创建一个默认的插件宿主 API 实例。
    /// 参数：无。
    /// 返回：初始化后的 DefaultPluginAPI。
    pub fn new() -> Self {
        Self {
            settings: DashMap::new(),
            refresh_callback: RwLock::new(None),
            hide_window_callback: RwLock::new(None),
        }
    }

    /// 设置“刷新程序列表”的回调。
    /// 参数：callback - 刷新时要执行的函数。
    /// 返回：无。
    pub async fn set_refresh_callback<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut cb = self.refresh_callback.write().await;
        *cb = Some(Arc::new(callback));
    }

    /// 设置“隐藏窗口”的回调。
    /// 参数：callback - 隐藏窗口时要执行的函数。
    /// 返回：无。
    pub async fn set_hide_window_callback<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut cb = self.hide_window_callback.write().await;
        *cb = Some(Arc::new(callback));
    }
}

impl Default for DefaultPluginAPI {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PluginAPI for DefaultPluginAPI {
    /// 记录插件日志。
    /// 参数：ctx - 当前插件上下文；level - 日志级别；message - 日志内容。
    /// 返回：无。
    async fn log(&self, ctx: &PluginContext, level: LogLevel, message: &str) {
        let level_str = match level {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };
        tracing::info!(
            trace_id = %ctx.trace_id,
            query_id = ?ctx.query_id,
            level = %level_str,
            "{}",
            message
        );
    }

    /// 发送插件通知。
    /// 参数：ctx - 当前插件上下文；title - 通知标题；message - 通知内容。
    /// 返回：无。
    async fn notify(&self, _ctx: &PluginContext, title: &str, message: &str) {
        tracing::info!("Notification: {} - {}", title, message);
    }

    /// 获取某个插件的设置值。
    /// 参数：plugin_id - 插件 ID；key - 设置键。
    /// 返回：找到则返回设置值，找不到则返回 None。
    async fn get_setting(&self, plugin_id: &str, key: &str) -> Option<String> {
        let setting_key = format!("{}.{}", plugin_id, key);
        self.settings.get(&setting_key).map(|v| v.value().clone())
    }

    /// 设置某个插件的设置值。
    /// 参数：plugin_id - 插件 ID；key - 设置键；value - 设置值。
    /// 返回：无。
    async fn set_setting(&self, plugin_id: &str, key: &str, value: &str) {
        let setting_key = format!("{}.{}", plugin_id, key);
        self.settings.insert(setting_key, value.to_string());
    }

    /// 执行刷新程序列表回调。
    /// 参数：无。
    /// 返回：无。
    async fn refresh_programs(&self) {
        if let Some(callback) = self.refresh_callback.read().await.as_ref() {
            callback();
        }
    }

    /// 执行隐藏窗口回调。
    /// 参数：无。
    /// 返回：无。
    async fn hide_window(&self) {
        if let Some(callback) = self.hide_window_callback.read().await.as_ref() {
            callback();
        }
    }
}
