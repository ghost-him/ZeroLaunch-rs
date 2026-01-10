use crate::modules::bookmark_loader::config::{
    BookmarkOverride, BookmarkSourceConfig, PartialBookmarkLoaderConfig,
};
use crate::modules::bookmark_loader::{Bookmark, BookmarkLoader, BrowserInfo};
use crate::state::app_state::AppState;
use std::sync::Arc;
use tauri::{command, State};

#[command]
pub fn detect_installed_browsers() -> Vec<BrowserInfo> {
    BookmarkLoader::detect_installed_browsers()
}

#[command]
pub fn read_browser_bookmarks(bookmarks_path: String) -> Result<Vec<Bookmark>, String> {
    BookmarkLoader::read_bookmarks_from_path(&bookmarks_path)
}

/// 获取当前书签源配置
#[command]
pub fn get_bookmark_sources(state: State<Arc<AppState>>) -> Vec<BookmarkSourceConfig> {
    state
        .get_runtime_config()
        .get_bookmark_loader_config()
        .get_sources()
}

/// 更新书签源配置
#[command]
pub fn update_bookmark_sources(state: State<Arc<AppState>>, sources: Vec<BookmarkSourceConfig>) {
    let partial = PartialBookmarkLoaderConfig {
        sources: Some(sources),
        overrides: None,
    };
    state
        .get_runtime_config()
        .get_bookmark_loader_config()
        .update(partial);
}

/// 获取书签覆盖配置
#[command]
pub fn get_bookmark_overrides(state: State<Arc<AppState>>) -> Vec<BookmarkOverride> {
    state
        .get_runtime_config()
        .get_bookmark_loader_config()
        .get_overrides()
}

/// 更新书签覆盖配置
#[command]
pub fn update_bookmark_overrides(state: State<Arc<AppState>>, overrides: Vec<BookmarkOverride>) {
    let partial = PartialBookmarkLoaderConfig {
        sources: None,
        overrides: Some(overrides),
    };
    state
        .get_runtime_config()
        .get_bookmark_loader_config()
        .update(partial);
}
