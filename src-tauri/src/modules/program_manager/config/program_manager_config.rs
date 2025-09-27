use super::image_loader_config::PartialImageLoaderConfig;
use super::image_loader_config::RuntimeImageLoaderConfig;
#[cfg(feature = "ai")]
use crate::core::ai::model_manager::ModelManager;
use crate::modules::program_manager::config::image_loader_config::ImageLoaderConfig;
use crate::program_manager::config::program_launcher_config::PartialProgramLauncherConfig;
use crate::program_manager::config::program_launcher_config::ProgramLauncherConfig;
use crate::program_manager::config::program_loader_config::PartialProgramLoaderConfig;
use crate::program_manager::config::program_loader_config::ProgramLoaderConfig;
use crate::program_manager::SearchModelConfig;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialProgramManagerConfig {
    pub launcher: Option<PartialProgramLauncherConfig>,
    pub loader: Option<PartialProgramLoaderConfig>,
    pub image_loader: Option<PartialImageLoaderConfig>,
    pub search_model: Option<Arc<SearchModelConfig>>,
}

#[derive(Debug)]
pub struct ProgramManagerConfigInner {
    pub launcher_config: Arc<ProgramLauncherConfig>,
    pub loader_config: Arc<ProgramLoaderConfig>,
    pub image_loader: Arc<ImageLoaderConfig>,
    pub search_model: Arc<SearchModelConfig>,
}

impl Default for ProgramManagerConfigInner {
    fn default() -> Self {
        ProgramManagerConfigInner {
            launcher_config: Arc::new(ProgramLauncherConfig::default()),
            loader_config: Arc::new(ProgramLoaderConfig::default()),
            image_loader: Arc::new(ImageLoaderConfig::default()),
            search_model: Arc::new(SearchModelConfig::default()),
        }
    }
}

impl ProgramManagerConfigInner {
    pub fn to_partial(&self) -> PartialProgramManagerConfig {
        PartialProgramManagerConfig {
            launcher: Some(self.launcher_config.to_partial()),
            loader: Some(self.loader_config.to_partial()),
            image_loader: Some(self.image_loader.to_partial()),
            search_model: Some(self.search_model.clone()),
        }
    }
    pub fn update(&mut self, partial_config: PartialProgramManagerConfig) {
        if let Some(partial_launcher) = partial_config.launcher {
            self.launcher_config.update(partial_launcher);
        }
        if let Some(partial_loader) = partial_config.loader {
            self.loader_config.update(partial_loader);
        }
        if let Some(partial_image_loader) = partial_config.image_loader {
            self.image_loader.update(partial_image_loader);
        }
        if let Some(new_search_model) = partial_config.search_model {
            self.search_model = new_search_model;
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

    pub fn get_launcher_config(&self) -> Arc<ProgramLauncherConfig> {
        self.inner.read().launcher_config.clone()
    }

    pub fn get_loader_config(&self) -> Arc<ProgramLoaderConfig> {
        self.inner.read().loader_config.clone()
    }

    pub fn get_image_loader_config(&self) -> Arc<ImageLoaderConfig> {
        self.inner.read().image_loader.clone()
    }

    pub fn get_search_model_config(&self) -> Arc<SearchModelConfig> {
        self.inner.read().search_model.clone()
    }

    pub fn update(&self, partial_config: PartialProgramManagerConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }
}

/// 运行时的配置信息，只会在程序初始化时被传入类，用于初始化相关的组件
pub struct RuntimeProgramConfig {
    /// 图片加载器的配置
    pub image_loader_config: RuntimeImageLoaderConfig,
    /// 模型管理器，用于语义搜索
    #[cfg(feature = "ai")]
    pub model_manager: Arc<ModelManager>,
    /// 启动时加载到内存的embedding缓存（二进制）
    pub embedding_cache_bytes: Option<Vec<u8>>,
}
