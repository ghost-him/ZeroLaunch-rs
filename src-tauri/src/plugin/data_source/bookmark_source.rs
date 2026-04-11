use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::{
    ArrayItem, ArrayUiHint, ConfigActionDef, DataSource, ExecutionTarget, FieldDefinition,
    SearchCandidate, SettingType,
};
use crate::plugin_system::{ComponentType, ConfigError, Configurable, SettingDefinition};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, warn};

// ============ Chrome 书签解析相关结构 ============

/// Chrome 书签 JSON 中的节点结构
#[derive(Debug, Deserialize)]
struct ChromeBookmarkNode {
    name: Option<String>,
    url: Option<String>,
    #[serde(rename = "type")]
    node_type: String,
    children: Option<Vec<ChromeBookmarkNode>>,
}

/// Chrome 书签 JSON 根结构
#[derive(Debug, Deserialize)]
struct ChromeBookmarksRoot {
    roots: HashMap<String, ChromeBookmarkNode>,
}

// ============ 配置相关结构 ============

/// 单个书签源的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkSourceConfig {
    /// 浏览器名称
    pub name: String,
    /// 书签文件路径
    pub bookmarks_path: String,
    /// 是否启用
    pub enabled: bool,
}

/// 单个书签的覆盖配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkOverride {
    /// 匹配 URL
    pub url: String,
    /// 是否排除
    pub excluded: bool,
    /// 自定义标题
    pub custom_title: Option<String>,
}

/// 浏览器信息（自动检测结果）
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

// ============ URL 规范化 ============

/// 规范化 URL 以支持宽松匹配。
/// 移除末尾的 `/`（除了 `://` 后的），转换为小写。
fn normalize_url(url: &str) -> String {
    let url = url.trim();
    let url = if url.ends_with('/') && !url.ends_with("://") {
        &url[..url.len() - 1]
    } else {
        url
    };
    url.to_lowercase()
}

// ============ BookmarkSource 实现 ============

pub struct BookmarkSource {
    settings: RwLock<serde_json::Value>,
}

impl Default for BookmarkSource {
    fn default() -> Self {
        Self::new()
    }
}

impl BookmarkSource {
    pub fn new() -> Self {
        BookmarkSource {
            settings: RwLock::new(serde_json::Value::Null),
        }
    }

    /// 从 settings 中解析书签源配置
    fn parse_sources(&self) -> Vec<BookmarkSourceConfig> {
        self.settings
            .read()
            .get("sources")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| serde_json::from_value(item.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 从 settings 中解析覆盖配置
    fn parse_overrides(&self) -> Vec<BookmarkOverride> {
        self.settings
            .read()
            .get("overrides")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| serde_json::from_value(item.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 从指定路径读取书签文件并解析。
    /// 若文件被占用，则复制到临时文件再读取。
    fn read_bookmarks_from_path(bookmarks_path: &str) -> Result<Vec<Bookmark>, String> {
        let path = PathBuf::from(bookmarks_path);
        if !path.exists() {
            return Err("Bookmarks file not found".to_string());
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => {
                let temp_dir = std::env::temp_dir();
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                let temp_path = temp_dir.join(format!("zl_bookmarks_{}.tmp", timestamp));

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

        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let root: ChromeBookmarksRoot = match serde_json::from_str(&content) {
            Ok(r) => r,
            Err(e) => return Err(format!("Failed to parse bookmarks: {}", e)),
        };

        let mut bookmarks = Vec::new();
        for (_, node) in root.roots {
            Self::traverse_bookmark_node(&node, &mut bookmarks);
        }
        Ok(bookmarks)
    }

    /// 递归遍历书签树，提取所有 URL 节点
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

    /// 自动检测系统已安装的浏览器书签路径
    pub fn detect_installed_browsers() -> Vec<BrowserInfo> {
        let mut browsers = Vec::new();
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
        if local_app_data.is_empty() {
            return browsers;
        }

        let base_path = PathBuf::from(&local_app_data);

        let find_profiles =
            |browsers: &mut Vec<BrowserInfo>, browser_name: String, user_data_path: PathBuf| {
                if let Ok(entries) = fs::read_dir(&user_data_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let bookmarks = path.join("Bookmarks");
                            if bookmarks.exists() {
                                let profile_name = entry.file_name().to_string_lossy().to_string();
                                let display_name = format!("{} ({})", browser_name, profile_name);
                                browsers.push(BrowserInfo {
                                    name: display_name,
                                    bookmarks_path: bookmarks.to_string_lossy().to_string(),
                                });
                            }
                        }
                    }
                }
            };

        if let Ok(entries) = fs::read_dir(&base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let name = entry.file_name().to_string_lossy().to_string();

                // Depth 1: %LOCALAPPDATA%/<Browser>/User Data
                let user_data = path.join("User Data");
                if user_data.is_dir() {
                    find_profiles(&mut browsers, name.clone(), user_data);
                }

                // Depth 2: %LOCALAPPDATA%/<Vendor>/<Browser>/User Data
                if let Ok(sub_entries) = fs::read_dir(&path) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        if !sub_path.is_dir() {
                            continue;
                        }

                        let sub_user_data = sub_path.join("User Data");
                        if sub_user_data.is_dir() {
                            let sub_name = sub_entry.file_name().to_string_lossy().to_string();
                            let full_name = format!("{} {}", name, sub_name);
                            find_profiles(&mut browsers, full_name, sub_user_data);
                        }
                    }
                }
            }
        }

        browsers
    }
}

impl Configurable for BookmarkSource {
    fn component_id(&self) -> &str {
        "bookmark-source"
    }

    fn component_name(&self) -> &str {
        "书签数据源"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::DataSource
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SettingDefinition {
                field: FieldDefinition {
                    key: "sources".to_string(),
                    label: "书签源".to_string(),
                    description: "配置要索引的浏览器书签来源".to_string(),
                    setting_type: SettingType::Array {
                        item: ArrayItem::Object(vec![
                            FieldDefinition {
                                key: "name".to_string(),
                                label: "浏览器名称".to_string(),
                                description: "书签来源的浏览器名称".to_string(),
                                setting_type: SettingType::Text,
                                default_value: serde_json::json!(""),
                                visible: true,
                                editable: false,
                            },
                            FieldDefinition {
                                key: "bookmarks_path".to_string(),
                                label: "书签文件路径".to_string(),
                                description: "浏览器书签文件的完整路径".to_string(),
                                setting_type: SettingType::Path {
                                    mode: crate::plugin_system::types::PathMode::File,
                                },
                                default_value: serde_json::json!(""),
                                visible: true,
                                editable: false,
                            },
                            FieldDefinition {
                                key: "enabled".to_string(),
                                label: "启用".to_string(),
                                description: "是否启用此书签源".to_string(),
                                setting_type: SettingType::Boolean,
                                default_value: serde_json::json!(true),
                                visible: true,
                                editable: true,
                            },
                        ]),
                        min_items: None,
                        max_items: None,
                        ui_hint: ArrayUiHint::MasterDetail,
                    },
                    default_value: serde_json::json!([]),
                    visible: true,
                    editable: true,
                },
                group: Some("书签源".to_string()),
                order: 1,
                config_action: Some("detect_browsers".to_string()),
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "overrides".to_string(),
                    label: "覆盖配置".to_string(),
                    description: "对特定书签进行排除或自定义标题".to_string(),
                    setting_type: SettingType::Array {
                        item: ArrayItem::Object(vec![
                            FieldDefinition {
                                key: "url".to_string(),
                                label: "URL".to_string(),
                                description: "要匹配的书签 URL".to_string(),
                                setting_type: SettingType::Text,
                                default_value: serde_json::json!(""),
                                visible: true,
                                editable: false,
                            },
                            FieldDefinition {
                                key: "excluded".to_string(),
                                label: "排除".to_string(),
                                description: "是否排除此书签".to_string(),
                                setting_type: SettingType::Boolean,
                                default_value: serde_json::json!(false),
                                visible: true,
                                editable: true,
                            },
                            FieldDefinition {
                                key: "custom_title".to_string(),
                                label: "自定义标题".to_string(),
                                description: "替换原始标题的自定义标题，留空则使用原始标题"
                                    .to_string(),
                                setting_type: SettingType::Text,
                                default_value: serde_json::json!(""),
                                visible: true,
                                editable: true,
                            },
                        ]),
                        min_items: None,
                        max_items: None,
                        ui_hint: ArrayUiHint::Table,
                    },
                    default_value: serde_json::json!([]),
                    visible: true,
                    editable: true,
                },
                group: Some("覆盖配置".to_string()),
                order: 2,
                config_action: None,
            },
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        *self.settings.write() = settings;
        Ok(())
    }

    fn config_actions(&self) -> Vec<ConfigActionDef> {
        vec![ConfigActionDef {
            action: "detect_browsers".to_string(),
            label: "自动检测浏览器".to_string(),
            description: "扫描系统中已安装的浏览器书签路径".to_string(),
        }]
    }

    fn execute_config_action(&self, action: &str) -> Result<serde_json::Value, String> {
        match action {
            "detect_browsers" => {
                serde_json::to_value(Self::detect_installed_browsers()).map_err(|e| e.to_string())
            }
            _ => Err(format!("Unknown config action: {}", action)),
        }
    }
}

impl DataSource for BookmarkSource {
    fn fetch_candidates(&self) -> CachedCandidateData {
        let mut result = CachedCandidateData::new();

        let sources = self.parse_sources();
        let overrides = self.parse_overrides();

        // 过滤出已启用的书签源
        let enabled_sources: Vec<&BookmarkSourceConfig> =
            sources.iter().filter(|s| s.enabled).collect();

        if enabled_sources.is_empty() {
            return result;
        }

        // 构建规范化的 URL -> Override 映射
        let override_map: HashMap<String, &BookmarkOverride> = overrides
            .iter()
            .map(|o| (normalize_url(&o.url), o))
            .collect();

        for source in enabled_sources {
            match Self::read_bookmarks_from_path(&source.bookmarks_path) {
                Ok(bookmark_list) => {
                    debug!("从 {} 加载了 {} 个书签", source.name, bookmark_list.len());
                    for bookmark in bookmark_list {
                        if bookmark.title.trim().is_empty() || bookmark.url.trim().is_empty() {
                            continue;
                        }

                        let normalized_url = normalize_url(&bookmark.url);
                        let (title, url) =
                            if let Some(override_config) = override_map.get(&normalized_url) {
                                if override_config.excluded {
                                    continue;
                                }
                                let title = override_config
                                    .custom_title
                                    .as_ref()
                                    .filter(|t| !t.trim().is_empty())
                                    .cloned()
                                    .unwrap_or_else(|| bookmark.title.clone());
                                (title, bookmark.url)
                            } else {
                                (bookmark.title.clone(), bookmark.url)
                            };

                        let candidate = SearchCandidate {
                            id: 0,
                            name: title,
                            icon: url.clone(),
                            target: ExecutionTarget::Url(url),
                            keywords: Vec::new(),
                            bias: 0.0,
                        };

                        result.add_candidate(candidate);
                    }
                }
                Err(e) => {
                    warn!("读取书签失败 ({}): {}", source.name, e);
                }
            }
        }

        result
    }
}
