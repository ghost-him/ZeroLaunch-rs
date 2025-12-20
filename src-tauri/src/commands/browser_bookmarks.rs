use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub name: String,
    pub bookmarks_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
}

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

#[command]
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
                            let child_name = sub_entry.file_name().to_string_lossy().to_string();
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

#[command]
pub fn read_browser_bookmarks(bookmarks_path: String) -> Result<Vec<Bookmark>, String> {
    let path = PathBuf::from(bookmarks_path);
    if !path.exists() {
        return Err("Bookmarks file not found".to_string());
    }

    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let root: ChromeBookmarksRoot = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let mut bookmarks = Vec::new();

    // Helper function to traverse the tree
    fn traverse(node: &ChromeBookmarkNode, list: &mut Vec<Bookmark>) {
        if node.node_type == "url" {
            if let (Some(title), Some(url)) = (&node.name, &node.url) {
                list.push(Bookmark {
                    title: title.clone(),
                    url: url.clone(),
                });
            }
        } else if let Some(children) = &node.children {
            for child in children {
                traverse(child, list);
            }
        }
    }

    // Iterate over roots (bookmark_bar, other, synced, etc.)
    for (_, node) in root.roots {
        traverse(&node, &mut bookmarks);
    }

    Ok(bookmarks)
}
