use crate::sdk::shell::lnk_resolver::LnkResolver;
use tracing::warn;

/// Windows 平台 Lnk 快捷方式解析器实现。
/// 使用 lnk crate 解析 .lnk 快捷方式文件，优先使用 GB18030 编码，失败后回退 UTF-16LE。
pub struct WindowsLnkResolver;

impl Default for WindowsLnkResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsLnkResolver {
    pub fn new() -> Self {
        Self
    }
}

impl LnkResolver for WindowsLnkResolver {
    /// 解析 .lnk 快捷方式文件的目标路径。
    /// 优先使用 GB18030 编码打开，失败后回退 UTF-16LE 编码。
    /// 参数：lnk_path - .lnk 文件的路径。
    /// 返回：解析成功返回目标路径，失败返回 None。
    fn resolve_lnk_target(&self, lnk_path: &str) -> Option<String> {
        let shell_link_result = lnk::ShellLink::open(lnk_path, encoding_rs::GB18030);

        let shell_link = match shell_link_result {
            Ok(link) => link,
            Err(e_gb18030) => {
                warn!(
                    "Failed to open LNK file '{}' with GB18030 encoding: {:?}",
                    lnk_path, e_gb18030
                );
                match lnk::ShellLink::open(lnk_path, encoding_rs::UTF_16LE) {
                    Ok(link) => {
                        warn!(
                            "在主要编码尝试失败后，成功使用 UTF-16LE 编码打开 LNK 文件: {}",
                            lnk_path
                        );
                        link
                    }
                    Err(e_utf16) => {
                        warn!(
                            "尝试使用 UTF-16LE 编码打开 LNK 文件 '{}' 再次失败: {:?}",
                            lnk_path, e_utf16
                        );
                        return None;
                    }
                }
            }
        };

        let link_info = match shell_link.link_info() {
            Some(info) => info,
            None => {
                warn!("无法从 LNK 文件 '{}' 获取 link_info。", lnk_path);
                return None;
            }
        };

        match link_info.local_base_path() {
            Some(path) => Some(path.to_string()),
            None => {
                warn!(
                    "无法从 LNK 文件 '{}' 获取基本路径 (local_base_path)。",
                    lnk_path
                );
                None
            }
        }
    }
}
