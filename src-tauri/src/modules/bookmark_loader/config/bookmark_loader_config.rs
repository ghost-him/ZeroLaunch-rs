use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// 单个书签来源的配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BookmarkSourceConfig {
    /// 浏览器名称（用于显示，如 "Google Chrome", "Microsoft Edge"）
    pub name: String,
    /// 书签文件的完整路径
    pub bookmarks_path: String,
    /// 是否启用自动导入（开关）
    pub enabled: bool,
}

/// 单个书签的覆盖配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BookmarkOverride {
    /// 要匹配的 URL（精确匹配）
    pub url: String,
    /// 是否排除此书签（不添加到搜索索引）
    pub excluded: bool,
    /// 自定义标题（用于搜索关键字）
    /// - Some(title) => 使用自定义标题
    /// - None => 使用原始标题
    pub custom_title: Option<String>,
}

/// Partial 配置，用于增量更新
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PartialBookmarkLoaderConfig {
    pub sources: Option<Vec<BookmarkSourceConfig>>,
    pub overrides: Option<Vec<BookmarkOverride>>,
}

/// 书签加载器的完整配置（内部）
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct BookmarkLoaderConfigInner {
    /// 书签来源列表
    #[serde(default)]
    pub sources: Vec<BookmarkSourceConfig>,
    /// 书签覆盖配置（排除和自定义标题）
    #[serde(default)]
    pub overrides: Vec<BookmarkOverride>,
}

impl BookmarkLoaderConfigInner {
    pub fn to_partial(&self) -> PartialBookmarkLoaderConfig {
        PartialBookmarkLoaderConfig {
            sources: Some(self.sources.clone()),
            overrides: Some(self.overrides.clone()),
        }
    }

    pub fn update(&mut self, partial: PartialBookmarkLoaderConfig) {
        if let Some(sources) = partial.sources {
            self.sources = sources;
        }
        if let Some(overrides) = partial.overrides {
            self.overrides = overrides;
        }
    }
}

/// 书签加载器配置（线程安全包装）
#[derive(Debug)]
pub struct BookmarkLoaderConfig {
    inner: RwLock<BookmarkLoaderConfigInner>,
}

impl Default for BookmarkLoaderConfig {
    fn default() -> Self {
        BookmarkLoaderConfig {
            inner: RwLock::new(BookmarkLoaderConfigInner::default()),
        }
    }
}

impl BookmarkLoaderConfig {
    pub fn to_partial(&self) -> PartialBookmarkLoaderConfig {
        self.inner.read().to_partial()
    }

    pub fn get_sources(&self) -> Vec<BookmarkSourceConfig> {
        self.inner.read().sources.clone()
    }

    pub fn get_enabled_sources(&self) -> Vec<BookmarkSourceConfig> {
        self.inner
            .read()
            .sources
            .iter()
            .filter(|s| s.enabled)
            .cloned()
            .collect()
    }

    pub fn get_overrides(&self) -> Vec<BookmarkOverride> {
        self.inner.read().overrides.clone()
    }

    pub fn update(&self, partial: PartialBookmarkLoaderConfig) {
        self.inner.write().update(partial);
    }
}
