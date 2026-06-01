use crate::host::error::HostApiError;

/// 已知路径类型枚举，列出所有平台相关的用户目录。
/// 各平台实现将枚举值映射为实际文件系统路径。
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum KnownPath {
    // ========================================================================
    // 系统路径（平台相关）
    // ========================================================================
    /// 公共开始菜单目录（Windows: C:\ProgramData\Microsoft\Windows\Start Menu）
    CommonStartMenu,
    /// 当前用户开始菜单目录（Windows: %APPDATA%\Microsoft\Windows\Start Menu）
    UserStartMenu,
    /// 当前用户桌面目录（Windows: %USERPROFILE%\Desktop）
    UserDesktop,
    /// 当前用户 AppData\Roaming 目录（Windows: %APPDATA%）
    UserAppDataRoaming,

    // ========================================================================
    // 应用路径（ZeroLaunch-rs 专用）
    // ========================================================================
    /// 应用数据根目录
    /// 标准模式：FOLDERID_RoamingAppData/ZeroLaunch-rs
    /// 便携模式（feature="portable"）：可执行文件所在目录
    AppDataDir,
    /// 日志目录（AppDataDir/logs）
    AppLogDir,
    /// 图标缓存目录（AppDataDir/icons）
    AppIconCacheDir,
    /// 配置目录（AppDataDir/config）
    AppConfigDir,
}

/// 路径解析器 trait，定义平台原语。
/// 各平台实现通过系统 API 将 KnownPath 枚举映射为实际文件系统路径。
pub trait PathResolver: Send + Sync {
    /// 根据 KnownPath 类型解析实际文件系统路径。
    /// 参数：path - 已知路径类型枚举。
    /// 返回：解析后的路径字符串，失败返回 HostApiError。
    fn resolve_path(&self, path: KnownPath) -> Result<String, HostApiError>;
}
