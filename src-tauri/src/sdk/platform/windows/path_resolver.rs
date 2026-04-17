use crate::sdk::host_api::HostApiError;
use crate::sdk::path::path_resolver::{KnownPath, PathResolver};
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
    /// 返回：对应的 Windows KNOWNFOLDERID 常量引用。
    fn known_path_to_folder_id(path: KnownPath) -> &'static windows_core::GUID {
        match path {
            KnownPath::CommonStartMenu => &FOLDERID_CommonStartMenu,
            KnownPath::UserStartMenu => &FOLDERID_StartMenu,
            KnownPath::UserDesktop => &FOLDERID_Desktop,
            KnownPath::UserAppDataRoaming => &FOLDERID_RoamingAppData,
        }
    }
}

impl PathResolver for WindowsPathResolver {
    /// 根据 KnownPath 类型解析实际文件系统路径。
    /// 通过 SHGetKnownFolderPath 获取 Windows 已知文件夹路径。
    fn resolve_path(&self, path: KnownPath) -> Result<String, HostApiError> {
        let folder_id = Self::known_path_to_folder_id(path);
        let path_label = format!("{:?}", path);

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
