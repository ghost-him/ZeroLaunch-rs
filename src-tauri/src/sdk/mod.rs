pub mod app;
pub mod autostart;
pub mod common;
pub mod host_api;
pub mod icon;
pub mod parameter;
pub mod path;
pub mod platform;
pub mod shell;
pub mod window;

pub use app::{AppEnumerator, AppInfo, AppLauncher};
pub use autostart::AutoStartManager;
pub use host_api::{
    CacheLevel, HostApi, HostApiBuilder, HostApiError, IconRequest, OpenTarget, PluginHandle,
    PluginSdkConfig,
};
pub use icon::icon_cache::IconCacheService;
pub use icon::icon_extractor::IconExtractor;
pub use parameter::{
    ParameterError, ParameterResolver, ParameterSnapshot, SystemParameterProvider,
};
pub use path::path_resolver::{KnownPath, PathResolver};
pub use platform::capabilities::{PlatformCapabilities, PlatformCapability};
pub use shell::{LnkResolver, ResourceLoader, ShellExecutor};
pub use window::WindowManager;
