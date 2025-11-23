use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub struct RuntimeIconManagerConfig {
    /// 默认的 app 图标的路径
    pub default_app_icon_path: String,
    /// 默认的 网址图标
    pub default_web_icon_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialIconManagerConfig {
    pub enable_icon_cache: Option<bool>,
    pub enable_online: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct IconManagerConfigInner {
    /// 要不要开启图片缓存
    #[serde(default = "IconManagerConfigInner::default_enable_icon_cache")]
    pub enable_icon_cache: bool,
    /// 要不要联网来获取网址的图标
    #[serde(default = "IconManagerConfigInner::default_enable_online")]
    pub enable_online: bool,
}

impl Default for IconManagerConfigInner {
    fn default() -> Self {
        Self {
            enable_icon_cache: Self::default_enable_icon_cache(),
            enable_online: Self::default_enable_online(),
        }
    }
}

impl IconManagerConfigInner {
    pub(crate) fn default_enable_icon_cache() -> bool {
        true
    }

    pub(crate) fn default_enable_online() -> bool {
        true
    }
}

impl IconManagerConfigInner {
    pub fn to_partial(&self) -> PartialIconManagerConfig {
        PartialIconManagerConfig {
            enable_icon_cache: Some(self.enable_icon_cache),
            enable_online: Some(self.enable_online),
        }
    }

    pub fn update(&mut self, partial_config: PartialIconManagerConfig) {
        if let Some(enable) = partial_config.enable_icon_cache {
            self.enable_icon_cache = enable;
        }
        if let Some(enable) = partial_config.enable_online {
            self.enable_online = enable;
        }
    }
}

#[derive(Debug)]
pub struct IconManagerConfig {
    inner: RwLock<IconManagerConfigInner>,
}

impl Default for IconManagerConfig {
    fn default() -> Self {
        IconManagerConfig {
            inner: RwLock::new(IconManagerConfigInner::default()),
        }
    }
}

impl IconManagerConfig {
    pub fn to_partial(&self) -> PartialIconManagerConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }

    pub fn get_enable_icon_cache(&self) -> bool {
        self.inner.read().enable_icon_cache
    }

    pub fn get_enable_online(&self) -> bool {
        self.inner.read().enable_online
    }

    pub fn update(&self, partial_config: PartialIconManagerConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }
}
