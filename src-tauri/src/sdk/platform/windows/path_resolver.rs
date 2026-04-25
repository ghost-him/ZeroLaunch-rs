use crate::sdk::host_api::HostApiError;
use crate::sdk::path::path_resolver::{KnownPath, PathResolver};
use std::path::Path;
use tracing::warn;
use windows::Win32::UI::Shell::{
    FOLDERID_CommonStartMenu, FOLDERID_Desktop, FOLDERID_RoamingAppData, FOLDERID_StartMenu,
    SHGetKnownFolderPath, KF_FLAG_DEFAULT,
};

/// 应用名称，用于构建应用数据目录
#[allow(dead_code)]
const APP_NAME: &str = "ZeroLaunch-rs";

/// Windows 路径解析器实现。
/// 通过 SHGetKnownFolderPath 将 KnownPath 枚举映射为 Windows 文件系统路径。
pub struct WindowsPathResolver;

impl Default for WindowsPathResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsPathResolver {
    pub fn new() -> Self {
        Self
    }

    /// 将 KnownPath 映射为 Windows KNOWNFOLDERID 常量引用。
    /// 参数：path - 已知路径类型枚举。
    /// 返回：对应的 Windows KNOWNFOLDERID 常量引用，应用路径返回 None。
    fn known_path_to_folder_id(path: KnownPath) -> Option<&'static windows_core::GUID> {
        match path {
            KnownPath::CommonStartMenu => Some(&FOLDERID_CommonStartMenu),
            KnownPath::UserStartMenu => Some(&FOLDERID_StartMenu),
            KnownPath::UserDesktop => Some(&FOLDERID_Desktop),
            KnownPath::UserAppDataRoaming => Some(&FOLDERID_RoamingAppData),
            // 应用路径不走此映射
            KnownPath::AppDataDir
            | KnownPath::AppLogDir
            | KnownPath::AppIconCacheDir
            | KnownPath::AppConfigDir => None,
        }
    }

    /// 判断是否为应用路径
    fn is_app_path(path: KnownPath) -> bool {
        matches!(
            path,
            KnownPath::AppDataDir
                | KnownPath::AppLogDir
                | KnownPath::AppIconCacheDir
                | KnownPath::AppConfigDir
        )
    }

    /// 解析应用数据根目录。
    /// 便携模式（feature="portable"）：可执行文件所在目录。
    /// 标准模式：FOLDERID_RoamingAppData/ZeroLaunch-rs。
    /// 参数：path_label - 路径标签，用于错误信息。
    /// 返回：解析后的路径字符串，失败返回 HostApiError。
    fn resolve_app_data_dir(&self) -> Result<String, HostApiError> {
        let path_label = format!("{:?}", KnownPath::AppDataDir);
        #[cfg(feature = "portable")]
        {
            let dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            dir.to_str()
                .map(|s| s.to_string())
                .ok_or_else(|| HostApiError::PathResolutionFailed {
                    path: path_label.to_string(),
                    reason: "便携模式路径转换失败".to_string(),
                })
        }

        #[cfg(not(feature = "portable"))]
        {
            unsafe {
                let pwstr = SHGetKnownFolderPath(&FOLDERID_RoamingAppData, KF_FLAG_DEFAULT, None)
                    .map_err(|e| {
                    warn!("获取 RoamingAppData 路径失败: {}", e);
                    HostApiError::PathResolutionFailed {
                        path: path_label.to_string(),
                        reason: format!("SHGetKnownFolderPath 调用失败: {}", e),
                    }
                })?;
                let roaming = pwstr.to_string().map_err(|e| {
                    warn!("RoamingAppData 路径转换失败: {}", e);
                    HostApiError::PathResolutionFailed {
                        path: path_label.to_string(),
                        reason: format!("路径字符串转换失败: {}", e),
                    }
                })?;
                Path::new(&roaming)
                    .join(APP_NAME)
                    .to_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| HostApiError::PathResolutionFailed {
                        path: path_label.to_string(),
                        reason: "应用数据目录路径转换失败".to_string(),
                    })
            }
        }
    }
}

impl PathResolver for WindowsPathResolver {
    /// 根据 KnownPath 类型解析实际文件系统路径。
    fn resolve_path(&self, path: KnownPath) -> Result<String, HostApiError> {
        let path_label = format!("{:?}", path);
        // 应用路径需要可变引用来缓存结果
        if Self::is_app_path(path) {
            // 这里需要内部可变性来缓存 app_data_dir
            // 使用 RefCell 或直接计算
            return self.resolve_app_path_internal(path);
        }

        // 系统路径
        let folder_id =
            Self::known_path_to_folder_id(path).expect("非应用路径应该有对应的 folder_id");

        unsafe {
            match SHGetKnownFolderPath(folder_id, KF_FLAG_DEFAULT, None) {
                Ok(pwstr) => match pwstr.to_string() {
                    Ok(s) => Ok(s),
                    Err(e) => {
                        warn!("路径转换失败 ({:?}): {}", path, e);
                        Err(HostApiError::PathResolutionFailed {
                            path: path_label,
                            reason: format!("路径字符串转换失败: {}", e),
                        })
                    }
                },
                Err(e) => {
                    warn!("获取已知路径失败 ({:?}): {}", path, e);
                    Err(HostApiError::PathResolutionFailed {
                        path: path_label,
                        reason: format!("SHGetKnownFolderPath 调用失败: {}", e),
                    })
                }
            }
        }
    }
}

impl WindowsPathResolver {
    /// 内部方法：解析应用路径
    fn resolve_app_path_internal(&self, path: KnownPath) -> Result<String, HostApiError> {
        match path {
            KnownPath::AppDataDir => self.resolve_app_data_dir(),
            KnownPath::AppLogDir => {
                let app_data = self.resolve_app_data_dir()?;
                Ok(Path::new(&app_data)
                    .join("logs")
                    .to_string_lossy()
                    .to_string())
            }
            KnownPath::AppIconCacheDir => {
                let app_data = self.resolve_app_data_dir()?;
                Ok(Path::new(&app_data)
                    .join("icons")
                    .to_string_lossy()
                    .to_string())
            }
            KnownPath::AppConfigDir => {
                let app_data = self.resolve_app_data_dir()?;
                Ok(Path::new(&app_data)
                    .join("config")
                    .to_string_lossy()
                    .to_string())
            }
            _ => unreachable!("非应用路径不应进入此方法"),
        }
    }
}
