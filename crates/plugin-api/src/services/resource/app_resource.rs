use dashmap::DashMap;
use std::path::PathBuf;

/// 应用内置资源路径服务。
/// 使用 Tauri 内置的资源解析能力，替代硬编码的资源目录路径拼接。
/// 核心程序和插件均可通过 HostApi / PluginHandle 获取图标路径。
pub struct AppResourceService {
    /// 图标名称到文件系统路径的映射
    icon_paths: DashMap<String, String>,
    /// 图标资源目录
    icons_dir: String,
}

impl AppResourceService {
    /// 创建应用资源服务，注册所有内置图标的路径映射。
    ///
    /// 参数：
    /// - icons_dir: 图标资源目录的完整路径（由调用方通过 Tauri 资源解析获得）
    ///
    /// 返回：初始化后的 AppResourceService。
    pub fn new(icons_dir: String) -> Self {
        let icon_paths = DashMap::new();

        let icons_dir_path = PathBuf::from(&icons_dir);

        let icons = [
            ("tray_icon", "32x32.png"),
            ("tray_icon_white", "32x32-white.png"),
            ("web_pages", "web_pages.png"),
            ("tips", "tips.png"),
            ("terminal", "terminal.png"),
            ("settings", "settings.png"),
            ("refresh", "refresh.png"),
            ("register", "register.png"),
            ("game", "game.png"),
            ("exit", "exit.png"),
        ];

        for (name, filename) in icons {
            let path = icons_dir_path.join(filename);
            icon_paths.insert(name.to_string(), path.to_string_lossy().to_string());
        }

        Self {
            icon_paths,
            icons_dir,
        }
    }

    /// 根据图标名称获取对应的文件系统路径。
    ///
    /// 参数：
    /// - name: 图标名称（如 "tray_icon", "web_pages", "tips" 等）
    ///
    /// 返回：若图标已注册则返回路径，否则返回 None。
    pub fn get_icon_path(&self, name: &str) -> Option<String> {
        self.icon_paths.get(name).map(|v| v.value().clone())
    }

    /// 获取图标资源目录。
    ///
    /// 返回：图标目录路径字符串引用。
    pub fn icons_dir(&self) -> &str {
        &self.icons_dir
    }
}
