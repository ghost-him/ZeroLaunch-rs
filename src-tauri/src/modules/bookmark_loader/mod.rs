pub mod config;

use config::{BookmarkLoaderConfig, BookmarkOverride};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, warn};

/// 浏览器信息（用于自动检测）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrowserInfo {
    pub name: String,
    pub bookmarks_path: String,
}

/// 单个书签数据
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
}

// ============ Chrome/Edge 书签解析相关结构 ============

#[derive(Debug, Deserialize)]
struct ChromeBookmarkNode {
    name: Option<String>,
    url: Option<String>,
    #[serde(rename = "type")]
    node_type: String,
    children: Option<Vec<ChromeBookmarkNode>>,
}

#[derive(Debug, Deserialize)]
struct ChromeBookmarksRoot {
    roots: std::collections::HashMap<String, ChromeBookmarkNode>,
}

// ============ URL 规范化 ============

/// 规范化 URL 以支持宽松匹配
/// - 移除末尾的 `/`（除了 `http://` 和 `https://` 后的第一个）
/// - 转换为小写
fn normalize_url(url: &str) -> String {
    let url = url.trim();

    // 移除末尾的 /（如果有，且不是协议部分）
    let url = if url.ends_with('/') && !url.ends_with("://") {
        &url[..url.len() - 1]
    } else {
        url
    };

    url.to_lowercase()
}

// ============ BookmarkLoader 内部实现 ============

#[derive(Debug, Default)]
struct BookmarkLoaderInner {
    /// 缓存的已启用书签数据，格式为 (title, url)
    enabled_bookmarks: Vec<(String, String)>,
}

// ============ BookmarkLoader 实现 ============

#[derive(Debug)]
pub struct BookmarkLoader {
    /// 内部缓存的书签数据
    inner: Arc<RwLock<BookmarkLoaderInner>>,
}

impl BookmarkLoader {
    pub fn new() -> Self {
        BookmarkLoader {
            inner: Arc::new(RwLock::new(BookmarkLoaderInner::default())),
        }
    }

    /// 根据配置加载书签数据，刷新内部缓存
    pub fn load_from_config(&self, config: &Arc<BookmarkLoaderConfig>) {
        let enabled_sources = config.get_enabled_sources();
        let overrides = config.get_overrides();

        // 构建规范化的 URL -> Override 的 HashMap，方便快速查找（支持URL变体匹配）
        let override_map: HashMap<String, &BookmarkOverride> = overrides
            .iter()
            .map(|o| (normalize_url(&o.url), o))
            .collect();

        let mut bookmarks = Vec::new();

        for source in enabled_sources {
            match Self::read_bookmarks_from_path(&source.bookmarks_path) {
                Ok(bookmark_list) => {
                    debug!(
                        "📚 从 {} 加载了 {} 个书签",
                        source.name,
                        bookmark_list.len()
                    );
                    for bookmark in bookmark_list {
                        if bookmark.title.trim().is_empty() || bookmark.url.trim().is_empty() {
                            continue;
                        }

                        // 使用规范化的URL进行查找，支持末尾斜杠等变体
                        let normalized_url = normalize_url(&bookmark.url);
                        if let Some(override_config) = override_map.get(&normalized_url) {
                            // 如果被排除，跳过
                            if override_config.excluded {
                                continue;
                            }
                            // 使用自定义标题或原始标题
                            let title = override_config
                                .custom_title
                                .as_ref()
                                .filter(|t| !t.trim().is_empty())
                                .cloned()
                                .unwrap_or(bookmark.title);
                            bookmarks.push((title, bookmark.url));
                        } else {
                            // 没有覆盖配置，使用原始数据
                            bookmarks.push((bookmark.title, bookmark.url));
                        }
                    }
                }
                Err(e) => {
                    warn!("⚠️ 读取书签失败 ({}): {}", source.name, e);
                }
            }
        }

        // 更新内部缓存
        self.inner.write().enabled_bookmarks = bookmarks;
    }

    /// 获取缓存的已启用书签数据
    /// 这些是从 load_from_config() 加载的已解析书签
    pub fn get_enabled_bookmarks(&self) -> Vec<(String, String)> {
        self.inner.read().enabled_bookmarks.clone()
    }

    /// 自动检测系统已安装的浏览器
    pub fn detect_installed_browsers() -> Vec<BrowserInfo> {
        let mut browsers = Vec::new();
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
        if local_app_data.is_empty() {
            return browsers;
        }

        let base_path = PathBuf::from(local_app_data);

        // Helper closure to create BrowserInfo
        let create_browser_info = |name: String, path: PathBuf| -> BrowserInfo {
            let bookmarks_path = path.join("User Data").join("Default").join("Bookmarks");
            BrowserInfo {
                name,
                bookmarks_path: bookmarks_path.to_string_lossy().to_string(),
            }
        };

        if let Ok(entries) = fs::read_dir(&base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                // Check Depth 1: %LOCALAPPDATA%/<Browser>/User Data/Default/Bookmarks
                let user_data = path.join("User Data");
                if user_data.exists() {
                    let bookmarks = user_data.join("Default").join("Bookmarks");
                    if bookmarks.exists() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        browsers.push(create_browser_info(name, path.clone()));
                    }
                }

                // Check Depth 2: %LOCALAPPDATA%/<Vendor>/<Browser>/User Data/Default/Bookmarks
                if let Ok(sub_entries) = fs::read_dir(&path) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        if !sub_path.is_dir() {
                            continue;
                        }

                        let sub_user_data = sub_path.join("User Data");
                        if sub_user_data.exists() {
                            let sub_bookmarks = sub_user_data.join("Default").join("Bookmarks");
                            if sub_bookmarks.exists() {
                                let parent_name = entry.file_name().to_string_lossy().to_string();
                                let child_name =
                                    sub_entry.file_name().to_string_lossy().to_string();
                                let name = format!("{} {}", parent_name, child_name);
                                browsers.push(create_browser_info(name, sub_path));
                            }
                        }
                    }
                }
            }
        }

        browsers
    }

    /// 从指定路径读取书签
    pub fn read_bookmarks_from_path(bookmarks_path: &str) -> Result<Vec<Bookmark>, String> {
        let path = PathBuf::from(bookmarks_path);
        if !path.exists() {
            return Err("Bookmarks file not found".to_string());
        }

        // 尝试直接读取，如果失败（可能是文件被占用），则尝试复制到临时文件再读取
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => {
                let temp_dir = std::env::temp_dir();
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                let temp_path = temp_dir.join(format!("zl_bookmarks_{}.tmp", timestamp));

                // 尝试复制文件（通常复制操作可以避开某些读锁）
                match fs::copy(&path, &temp_path) {
                    Ok(_) => {
                        let c = fs::read_to_string(&temp_path);
                        let _ = fs::remove_file(&temp_path);
                        c.map_err(|e| format!("读取临时书签副本失败: {}", e))?
                    }
                    Err(e) => return Err(format!("读取书签失败(文件可能被占用且无法复制): {}", e)),
                }
            }
        };

        // 如果文件为空，直接返回空列表
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let root: ChromeBookmarksRoot = match serde_json::from_str(&content) {
            Ok(r) => r,
            Err(e) => return Err(format!("Failed to parse bookmarks: {}", e)),
        };

        let mut bookmarks = Vec::new();

        // Iterate over roots (bookmark_bar, other, synced, etc.)
        for (_, node) in root.roots {
            Self::traverse_bookmark_node(&node, &mut bookmarks);
        }

        Ok(bookmarks)
    }

    /// 遍历书签树
    fn traverse_bookmark_node(node: &ChromeBookmarkNode, list: &mut Vec<Bookmark>) {
        if node.node_type == "url" {
            if let (Some(title), Some(url)) = (&node.name, &node.url) {
                list.push(Bookmark {
                    title: title.clone(),
                    url: url.clone(),
                });
            }
        } else if let Some(children) = &node.children {
            for child in children {
                Self::traverse_bookmark_node(child, list);
            }
        }
    }
}

impl Default for BookmarkLoader {
    fn default() -> Self {
        Self::new()
    }
}
