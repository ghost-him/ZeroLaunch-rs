use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::Shortcut;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PartialShortcutConfig {
    pub open_search_bar: Option<Shortcut>,
    pub switch_to_everything: Option<Shortcut>,
    pub arrow_up: Option<Shortcut>,
    pub arrow_down: Option<Shortcut>,
    pub arrow_left: Option<Shortcut>,
    pub arrow_right: Option<Shortcut>,
}

/// 快捷键配置
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ShortcutConfigInner {
    #[serde(default = "ShortcutConfigInner::default_open_search_bar")]
    pub open_search_bar: Shortcut,
    #[serde(default = "ShortcutConfigInner::default_switch_to_everything")]
    pub switch_to_everything: Shortcut,
    #[serde(default = "ShortcutConfigInner::default_arrow_up")]
    pub arrow_up: Shortcut,
    #[serde(default = "ShortcutConfigInner::default_arrow_down")]
    pub arrow_down: Shortcut,
    #[serde(default = "ShortcutConfigInner::default_arrow_left")]
    pub arrow_left: Shortcut,
    #[serde(default = "ShortcutConfigInner::default_arrow_right")]
    pub arrow_right: Shortcut,
}

impl Default for ShortcutConfigInner {
    fn default() -> Self {
        Self {
            open_search_bar: Self::default_open_search_bar(),
            switch_to_everything: Self::default_switch_to_everything(),
            arrow_up: Self::default_arrow_up(),
            arrow_down: Self::default_arrow_down(),
            arrow_left: Self::default_arrow_left(),
            arrow_right: Self::default_arrow_right(),
        }
    }
}

impl ShortcutConfigInner {
    pub(crate) fn default_open_search_bar() -> Shortcut {
        let mut shortcut = Shortcut::new();
        shortcut.key = "Space".to_string();
        shortcut.alt = true;
        shortcut
    }

    pub(crate) fn default_switch_to_everything() -> Shortcut {
        let mut shortcut = Shortcut::new();
        shortcut.key = "e".to_string();
        shortcut.ctrl = true;
        shortcut
    }

    pub(crate) fn default_arrow_up() -> Shortcut {
        let mut shortcut = Shortcut::new();
        shortcut.key = "k".to_string();
        shortcut.ctrl = true;
        shortcut
    }

    pub(crate) fn default_arrow_down() -> Shortcut {
        let mut shortcut = Shortcut::new();
        shortcut.key = "j".to_string();
        shortcut.ctrl = true;
        shortcut
    }

    pub(crate) fn default_arrow_left() -> Shortcut {
        let mut shortcut = Shortcut::new();
        shortcut.key = "h".to_string();
        shortcut.ctrl = true;
        shortcut
    }

    pub(crate) fn default_arrow_right() -> Shortcut {
        let mut shortcut = Shortcut::new();
        shortcut.key = "l".to_string();
        shortcut.ctrl = true;
        shortcut
    }

    pub fn update(&mut self, partial: PartialShortcutConfig) {
        if let Some(shortcut) = partial.open_search_bar {
            self.open_search_bar = shortcut;
        }
        if let Some(shortcut) = partial.switch_to_everything {
            self.switch_to_everything = shortcut;
        }
        if let Some(shortcut) = partial.arrow_up {
            self.arrow_up = shortcut;
        }
        if let Some(shortcut) = partial.arrow_down {
            self.arrow_down = shortcut;
        }
        if let Some(shortcut) = partial.arrow_left {
            self.arrow_left = shortcut;
        }
        if let Some(shortcut) = partial.arrow_right {
            self.arrow_right = shortcut;
        }
    }

    pub fn to_partial(&self) -> PartialShortcutConfig {
        PartialShortcutConfig {
            open_search_bar: Some(self.open_search_bar.clone()),
            switch_to_everything: Some(self.switch_to_everything.clone()),
            arrow_up: Some(self.arrow_up.clone()),
            arrow_down: Some(self.arrow_down.clone()),
            arrow_left: Some(self.arrow_left.clone()),
            arrow_right: Some(self.arrow_right.clone()),
        }
    }
}

#[derive(Debug)]
pub struct ShortcutConfig {
    inner: RwLock<ShortcutConfigInner>,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        ShortcutConfig {
            inner: RwLock::new(ShortcutConfigInner::default()),
        }
    }
}

impl ShortcutConfig {
    pub fn update(&self, partial: PartialShortcutConfig) {
        let mut inner = self.inner.write();
        inner.update(partial);
    }

    pub fn get_open_search_bar(&self) -> Shortcut {
        let inner = self.inner.read();
        inner.open_search_bar.clone()
    }

    pub fn get_switch_to_everything(&self) -> Shortcut {
        let inner = self.inner.read();
        inner.switch_to_everything.clone()
    }

    pub fn get_arrow_up(&self) -> Shortcut {
        let inner = self.inner.read();
        inner.arrow_up.clone()
    }

    pub fn get_arrow_down(&self) -> Shortcut {
        let inner = self.inner.read();
        inner.arrow_down.clone()
    }

    pub fn get_arrow_left(&self) -> Shortcut {
        let inner = self.inner.read();
        inner.arrow_left.clone()
    }

    pub fn get_arrow_right(&self) -> Shortcut {
        let inner = self.inner.read();
        inner.arrow_right.clone()
    }

    pub fn to_partial(&self) -> PartialShortcutConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }
}
