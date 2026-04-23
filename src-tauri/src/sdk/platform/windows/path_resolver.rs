use crate::error::OptionExt;
use crate::sdk::host_api::HostApiError;
use crate::sdk::path::path_resolver::{KnownPath, PathResolver};
#[allow(unused_imports)]
use std::path::Path;
use tracing::warn;
use windows::Win32::UI::Shell::{
    FOLDERID_CommonStartMenu, FOLDERID_Desktop, FOLDERID_RoamingAppData, FOLDERID_StartMenu,
    SHGetKnownFolderPath, KF_FLAG_DEFAULT,
};

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
    /// 返回：对应的 Windows KNOWNFOLDERID 常量引用，AppDataDir 返回 None。
    /// 注意：AppDataDir 不走此映射，在 resolve_path 中单独处理。
    fn known_path_to_folder_id(path: KnownPath) -> Option<&'static windows_core::GUID> {
        match path {
            KnownPath::CommonStartMenu => Some(&FOLDERID_CommonStartMenu),
            KnownPath::UserStartMenu => Some(&FOLDERID_StartMenu),
            KnownPath::UserDesktop => Some(&FOLDERID_Desktop),
            KnownPath::UserAppDataRoaming => Some(&FOLDERID_RoamingAppData),
            KnownPath::AppDataDir => None,
        }
    }

    /// 解析应用数据目录。
    /// 便携模式（feature="portable"）：可执行文件所在目录。
    /// 标准模式：FOLDERID_RoamingAppData/ZeroLaunch-rs。
    /// 参数：path_label - 路径标签，用于错误信息。
    /// 返回：解析后的路径字符串，失败返回 HostApiError。
    fn resolve_app_data_dir(&self, path_label: &str) -> Result<String, HostApiError> {
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
                    .join("ZeroLaunch-rs")
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
    /// 通过 SHGetKnownFolderPath 获取 Windows 已知文件夹路径。
    /// AppDataDir 特殊处理：便携模式使用 exe 目录，标准模式使用 RoamingAppData/ZeroLaunch-rs。
    fn resolve_path(&self, path: KnownPath) -> Result<String, HostApiError> {
        let path_label = format!("{:?}", path);

        // AppDataDir 特殊处理：编译期决定便携/标准模式
        if path == KnownPath::AppDataDir {
            return self.resolve_app_data_dir(&path_label);
        }

        let folder_id = WindowsPathResolver::known_path_to_folder_id(path)
            .expect_programming("AppDataDir should be handled above");

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
