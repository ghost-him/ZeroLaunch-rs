use crate::modules::program_manager::config::program_ranker_config::PartialProgramRankerConfig;
use crate::modules::program_manager::config::program_ranker_config::ProgramRankerConfig;
use crate::modules::program_manager::semantic_manager::EmbeddingBackend;
use crate::program_manager::config::program_loader_config::PartialProgramLoaderConfig;
use crate::program_manager::config::program_loader_config::ProgramLoaderConfig;
use crate::program_manager::SearchModelConfig;
use crate::IconManager;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialProgramManagerConfig {
    pub ranker: Option<PartialProgramRankerConfig>,
    pub loader: Option<PartialProgramLoaderConfig>,
    pub search_model: Option<Arc<SearchModelConfig>>,
    pub enable_lru_search_cache: Option<bool>,
    pub search_cache_capacity: Option<usize>,
}

#[derive(Debug)]
pub struct ProgramManagerConfigInner {
    pub ranker_config: Arc<ProgramRankerConfig>,
    pub loader_config: Arc<ProgramLoaderConfig>,
    pub search_model: Arc<SearchModelConfig>,
    pub enable_lru_search_cache: bool,
    pub search_cache_capacity: usize,
}

impl Default for ProgramManagerConfigInner {
    fn default() -> Self {
        ProgramManagerConfigInner {
            ranker_config: Arc::new(ProgramRankerConfig::default()),
            loader_config: Arc::new(ProgramLoaderConfig::default()),
            search_model: Arc::new(SearchModelConfig::default()),
            enable_lru_search_cache: false,
            search_cache_capacity: 120,
        }
    }
}

impl ProgramManagerConfigInner {
    pub fn to_partial(&self) -> PartialProgramManagerConfig {
        PartialProgramManagerConfig {
            ranker: Some(self.ranker_config.to_partial()),
            loader: Some(self.loader_config.to_partial()),
            search_model: Some(self.search_model.clone()),
            enable_lru_search_cache: Some(self.enable_lru_search_cache),
            search_cache_capacity: Some(self.search_cache_capacity),
        }
    }
    pub fn update(&mut self, partial_config: PartialProgramManagerConfig) {
        if let Some(partial_ranker) = partial_config.ranker {
            self.ranker_config.update(partial_ranker);
        }
        if let Some(partial_loader) = partial_config.loader {
            self.loader_config.update(partial_loader);
        }
        if let Some(new_search_model) = partial_config.search_model {
            self.search_model = new_search_model;
        }
        if let Some(enable_cache) = partial_config.enable_lru_search_cache {
            self.enable_lru_search_cache = enable_cache;
        }
        if let Some(capacity) = partial_config.search_cache_capacity {
            if capacity == 0 {
                self.search_cache_capacity = 1;
            } else {
                self.search_cache_capacity = capacity;
            }
        }
    }
}
#[derive(Debug)]
pub struct ProgramManagerConfig {
    inner: RwLock<ProgramManagerConfigInner>,
}

impl Default for ProgramManagerConfig {
    fn default() -> Self {
        ProgramManagerConfig {
            inner: RwLock::new(ProgramManagerConfigInner::default()),
        }
    }
}

impl ProgramManagerConfig {
    pub fn to_partial(&self) -> PartialProgramManagerConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }

    pub fn get_ranker_config(&self) -> Arc<ProgramRankerConfig> {
        self.inner.read().ranker_config.clone()
    }

    pub fn get_loader_config(&self) -> Arc<ProgramLoaderConfig> {
        self.inner.read().loader_config.clone()
    }

    pub fn get_search_model_config(&self) -> Arc<SearchModelConfig> {
        self.inner.read().search_model.clone()
    }

    pub fn is_lru_search_cache_enabled(&self) -> bool {
        self.inner.read().enable_lru_search_cache
    }

    pub fn get_search_cache_capacity(&self) -> usize {
        self.inner.read().search_cache_capacity
    }

    pub fn update(&self, partial_config: PartialProgramManagerConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }
}

/// 运行时的配置信息，只会在程序初始化时被传入类，用于初始化相关的组件
pub struct RuntimeProgramConfig {
    /// 语义搜索后端（启用 AI 时存在）
    pub embedding_backend: Option<Arc<dyn EmbeddingBackend>>,
    /// 启动时加载到内存的embedding缓存（二进制）
    pub embedding_cache_bytes: Option<Vec<u8>>,
    /// 图标管理器
    pub icon_manager: Arc<IconManager>,
}
