//! 刷新调度器配置

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// 刷新调度器的部分配置（用于配置更新）
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PartialRefreshSchedulerConfig {
    /// 定时刷新间隔（分钟），0 表示禁用定时刷新
    pub auto_refresh_interval_mins: Option<u64>,
    /// 是否启用安装监控
    pub enable_installation_monitor: Option<bool>,
    /// 安装监控的 debounce 时间（秒）
    pub monitor_debounce_secs: Option<u64>,
}

/// 刷新调度器的完整配置（内部使用）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RefreshSchedulerConfigInner {
    /// 定时刷新间隔（分钟），0 表示禁用定时刷新
    #[serde(default = "RefreshSchedulerConfigInner::default_auto_refresh_interval_mins")]
    pub auto_refresh_interval_mins: u64,
    /// 是否启用安装监控
    #[serde(default = "RefreshSchedulerConfigInner::default_enable_installation_monitor")]
    pub enable_installation_monitor: bool,
    /// 安装监控的 debounce 时间（秒）
    #[serde(default = "RefreshSchedulerConfigInner::default_monitor_debounce_secs")]
    pub monitor_debounce_secs: u64,
}

impl Default for RefreshSchedulerConfigInner {
    fn default() -> Self {
        Self {
            auto_refresh_interval_mins: Self::default_auto_refresh_interval_mins(),
            enable_installation_monitor: Self::default_enable_installation_monitor(),
            monitor_debounce_secs: Self::default_monitor_debounce_secs(),
        }
    }
}

impl RefreshSchedulerConfigInner {
    pub(crate) fn default_auto_refresh_interval_mins() -> u64 {
        30
    }

    pub(crate) fn default_enable_installation_monitor() -> bool {
        false
    }

    pub(crate) fn default_monitor_debounce_secs() -> u64 {
        5
    }

    pub fn update(&mut self, partial: PartialRefreshSchedulerConfig) {
        if let Some(interval) = partial.auto_refresh_interval_mins {
            self.auto_refresh_interval_mins = interval;
        }
        if let Some(enable) = partial.enable_installation_monitor {
            self.enable_installation_monitor = enable;
        }
        if let Some(debounce) = partial.monitor_debounce_secs {
            self.monitor_debounce_secs = debounce;
        }
    }

    pub fn to_partial(&self) -> PartialRefreshSchedulerConfig {
        PartialRefreshSchedulerConfig {
            auto_refresh_interval_mins: Some(self.auto_refresh_interval_mins),
            enable_installation_monitor: Some(self.enable_installation_monitor),
            monitor_debounce_secs: Some(self.monitor_debounce_secs),
        }
    }
}

/// 刷新调度器配置（线程安全包装）
#[derive(Debug)]
pub struct RefreshSchedulerConfig {
    inner: RwLock<RefreshSchedulerConfigInner>,
}

impl Default for RefreshSchedulerConfig {
    fn default() -> Self {
        Self {
            inner: RwLock::new(RefreshSchedulerConfigInner::default()),
        }
    }
}

impl RefreshSchedulerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&self, partial: PartialRefreshSchedulerConfig) {
        self.inner.write().update(partial);
    }

    pub fn get_auto_refresh_interval_mins(&self) -> u64 {
        self.inner.read().auto_refresh_interval_mins
    }

    pub fn get_enable_installation_monitor(&self) -> bool {
        self.inner.read().enable_installation_monitor
    }

    pub fn get_monitor_debounce_secs(&self) -> u64 {
        self.inner.read().monitor_debounce_secs
    }

    pub fn to_partial(&self) -> PartialRefreshSchedulerConfig {
        self.inner.read().to_partial()
    }

    /// 获取内部配置的快照
    pub fn get_snapshot(&self) -> RefreshSchedulerConfigInner {
        self.inner.read().clone()
    }
}
