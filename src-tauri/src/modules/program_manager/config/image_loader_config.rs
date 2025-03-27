use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub struct RuntimeImageLoaderConfig {
    /// 默认的 app 图标的路径
    pub default_app_icon_path: String,
    /// 默认的 网址图标
    pub default_web_icon_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialImageLoaderConfig {
    pub enable_icon_cache: Option<bool>,
    pub enable_online: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ImageLoaderConfigInner {
    /// 要不要开启图片缓存
    #[serde(default = "ImageLoaderConfigInner::default_enable_icon_cache")]
    pub enable_icon_cache: bool,
    /// 要不要联网来获取网址的图标
    #[serde(default = "ImageLoaderConfigInner::default_enable_online")]
    pub enable_online: bool,
}

impl Default for ImageLoaderConfigInner {
    fn default() -> Self {
        Self {
            enable_icon_cache: Self::default_enable_icon_cache(),
            enable_online: Self::default_enable_online(),
        }
    }
}

impl ImageLoaderConfigInner {
    pub(crate) fn default_enable_icon_cache() -> bool {
        true
    }

    pub(crate) fn default_enable_online() -> bool {
        true
    }
}

impl ImageLoaderConfigInner {
    pub fn to_partial(&self) -> PartialImageLoaderConfig {
        PartialImageLoaderConfig {
            enable_icon_cache: Some(self.enable_icon_cache),
            enable_online: Some(self.enable_online),
        }
    }

    pub fn update(&mut self, partial_config: PartialImageLoaderConfig) {
        if let Some(enable) = partial_config.enable_icon_cache {
            self.enable_icon_cache = enable;
        }
        if let Some(enable) = partial_config.enable_online {
            self.enable_online = enable;
        }
    }
}

#[derive(Debug)]
pub struct ImageLoaderConfig {
    inner: RwLock<ImageLoaderConfigInner>,
}

impl Default for ImageLoaderConfig {
    fn default() -> Self {
        ImageLoaderConfig {
            inner: RwLock::new(ImageLoaderConfigInner::default()),
        }
    }
}

impl ImageLoaderConfig {
    pub fn to_partial(&self) -> PartialImageLoaderConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }

    pub fn get_enable_icon_cache(&self) -> bool {
        self.inner.read().enable_icon_cache
    }

    pub fn get_enable_online(&self) -> bool {
        self.inner.read().enable_online
    }

    pub fn update(&self, partial_config: PartialImageLoaderConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }
}
