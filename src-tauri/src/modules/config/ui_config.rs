/// 与程序页面设置有关的，比如窗口的大小，显示的界面等
use crate::modules::config::{Height, Width};
use parking_lot::RwLock;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialUiConfig {
    pub item_width_scale_factor: Option<f64>,
    pub item_height_scale_factor: Option<f64>,
    pub selected_item_color: Option<String>,
    pub item_font_color: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiConfigInner {
    /// 显示器的大小与窗口的大小的比例
    /// 窗口的高的比例
    pub item_width_scale_factor: f64,
    /// 窗口的宽度的比例
    pub item_height_scale_factor: f64,
    /// 选中项的颜色
    pub selected_item_color: String,
    /// 选项中的字体的颜色
    pub item_font_color: String,
}

impl Default for UiConfigInner {
    fn default() -> Self {
        UiConfigInner {
            item_width_scale_factor: 0.5,
            item_height_scale_factor: 0.0555,
            selected_item_color: "#d55d1d".to_string(),
            item_font_color: "#000000".to_string(),
        }
    }
}

impl UiConfigInner {
    pub fn update(&mut self, partial_ui_config: PartialUiConfig) {
        if let Some(item_width_scale_factor) = partial_ui_config.item_width_scale_factor {
            self.item_width_scale_factor = item_width_scale_factor;
        }
        if let Some(item_height_scale_factor) = partial_ui_config.item_height_scale_factor {
            self.item_height_scale_factor = item_height_scale_factor;
        }
        if let Some(selected_item_color) = partial_ui_config.selected_item_color {
            self.selected_item_color = selected_item_color;
        }
        if let Some(item_font_color) = partial_ui_config.item_font_color {
            self.item_font_color = item_font_color;
        }
    }

    pub fn get_item_width_scale_factor(&self) -> f64 {
        self.item_width_scale_factor
    }

    pub fn get_item_height_scale_factor(&self) -> f64 {
        self.item_height_scale_factor
    }

    pub fn get_selected_item_color(&self) -> String {
        self.selected_item_color.clone()
    }
    pub fn get_item_font_color(&self) -> String {
        self.item_font_color.clone()
    }

    pub fn to_partial(&self) -> PartialUiConfig {
        PartialUiConfig {
            item_width_scale_factor: Some(self.item_width_scale_factor),
            item_height_scale_factor: Some(self.item_height_scale_factor),
            selected_item_color: Some(self.selected_item_color.clone()),
            item_font_color: Some(self.item_font_color.clone()),
        }
    }
}
#[derive(Debug)]
pub struct UiConfig {
    inner: RwLock<UiConfigInner>,
}

impl Default for UiConfig {
    fn default() -> Self {
        UiConfig {
            inner: RwLock::new(UiConfigInner::default()),
        }
    }
}

impl UiConfig {
    pub fn update(&self, partial_ui_config: PartialUiConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_ui_config);
    }

    pub fn get_item_width_scale_factor(&self) -> f64 {
        let inner = self.inner.read();
        inner.item_width_scale_factor
    }

    pub fn get_item_height_scale_factor(&self) -> f64 {
        let inner = self.inner.read();
        inner.item_height_scale_factor
    }

    pub fn get_selected_item_color(&self) -> String {
        let inner = self.inner.read();
        inner.selected_item_color.clone()
    }
    pub fn get_item_font_color(&self) -> String {
        let inner = self.inner.read();
        inner.item_font_color.clone()
    }

    pub fn to_partial(&self) -> PartialUiConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }
}

// // 手动实现序列化
// impl Serialize for UiConfig {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         // 获取读锁后序列化内部数据
//         let inner = self.inner.read();
//         inner.serialize(serializer)
//     }
// }

// // 手动实现反序列化
// impl<'de> Deserialize<'de> for UiConfig {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         // 先反序列化出内部数据
//         let inner = UiConfigInner::deserialize(deserializer)?;
//         // 用 RwLock 包装后返回
//         Ok(UiConfig {
//             inner: RwLock::new(inner),
//         })
//     }
// }

// // 手动实现 Clone
// impl Clone for UiConfig {
//     fn clone(&self) -> Self {
//         // 获取读锁后克隆内部数据
//         let inner_data = self.inner.read().clone();
//         UiConfig {
//             inner: RwLock::new(inner_data),
//         }
//     }
// }
