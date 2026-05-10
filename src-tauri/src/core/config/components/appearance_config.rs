use crate::core::config::setting_builders::{
    bool_field, color_field, num_field, select_field, text_field,
};
use crate::core::types::setting_def::SettingDefinition;
use crate::core::types::{ComponentType, ConfigError, Configurable};
use parking_lot::RwLock;
use tracing::info;

/// 外观配置组件。
/// 管理主题（浅色/深色/跟随系统）、语言偏好、搜索栏/结果栏/底栏尺寸与字体、窗口参数、配色方案。
pub struct AppearanceConfigComponent {
    settings: RwLock<serde_json::Value>,
}

impl Default for AppearanceConfigComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl AppearanceConfigComponent {
    pub fn new() -> Self {
        Self {
            settings: RwLock::new(serde_json::Value::Null),
        }
    }
}

impl Configurable for AppearanceConfigComponent {
    fn component_id(&self) -> &str {
        "appearance"
    }

    fn component_name(&self) -> &str {
        "外观"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            // ---- 主题 & 语言 ----
            select_field(
                "theme",
                "主题",
                "选择浅色、深色或跟随系统主题",
                "theme",
                0,
                vec!["system", "light", "dark"],
                "system",
            ),
            select_field(
                "language",
                "语言",
                "界面显示语言",
                "theme",
                1,
                vec!["zh-Hans", "zh-Hant", "en"],
                "zh-Hans",
            ),
            // ---- 搜索栏 ----
            num_field(
                "searchBarHeight",
                "搜索栏高度",
                "搜索栏的高度(px)",
                "searchBar",
                10,
                72.0,
                40.0,
                120.0,
                1.0,
            ),
            num_field(
                "searchBarFontRatio",
                "搜索栏字体比例",
                "字体大小 = 搜索栏高度 × 此比例",
                "searchBar",
                11,
                0.56,
                0.3,
                0.8,
                0.01,
            ),
            text_field(
                "searchBarFontFamily",
                "搜索栏字体",
                "字体族名称，留空则跟随系统",
                "searchBar",
                12,
                "",
            ),
            text_field(
                "searchBarPlaceholder",
                "搜索栏占位符",
                "搜索栏的提示文本",
                "searchBar",
                13,
                "Hello, ZeroLaunch!",
            ),
            // ---- 结果栏 ----
            num_field(
                "resultItemHeight",
                "结果项高度",
                "单条结果的高度(px)",
                "resultList",
                20,
                54.0,
                36.0,
                80.0,
                1.0,
            ),
            num_field(
                "resultItemFontRatio",
                "结果标题字体比例",
                "标题字体大小 = 结果项高度 × 此比例",
                "resultList",
                21,
                0.30,
                0.2,
                0.5,
                0.01,
            ),
            num_field(
                "resultItemSubtitleFontRatio",
                "结果副标题字体比例",
                "副标题字体大小 = 结果项高度 × 此比例",
                "resultList",
                22,
                0.24,
                0.15,
                0.4,
                0.01,
            ),
            text_field(
                "resultItemFontFamily",
                "结果栏字体",
                "字体族名称，留空则跟随系统",
                "resultList",
                23,
                "",
            ),
            num_field(
                "maxVisibleResults",
                "最大可见结果数",
                "不滚动时最多显示的结果条数",
                "resultList",
                24,
                8.0,
                3.0,
                20.0,
                1.0,
            ),
            bool_field(
                "showLaunchCommand",
                "显示启动命令",
                "在结果项中显示启动命令路径",
                "resultList",
                25,
                false,
            ),
            // ---- 底栏 ----
            num_field(
                "footerHeight",
                "底栏高度",
                "底栏的高度(px)，设为0则隐藏",
                "footer",
                30,
                48.0,
                0.0,
                60.0,
                1.0,
            ),
            num_field(
                "footerFontRatio",
                "底栏字体比例",
                "字体大小 = 底栏高度 × 此比例",
                "footer",
                31,
                0.25,
                0.15,
                0.35,
                0.01,
            ),
            text_field(
                "footerFontFamily",
                "底栏字体",
                "字体族名称，留空则跟随系统",
                "footer",
                32,
                "",
            ),
            // ---- 窗口 ----
            num_field(
                "windowWidth",
                "窗口宽度",
                "搜索窗口的宽度(px)",
                "window",
                40,
                800.0,
                400.0,
                1200.0,
                10.0,
            ),
            num_field(
                "windowCornerRadius",
                "窗口圆角",
                "窗口圆角大小(px)",
                "window",
                41,
                12.0,
                0.0,
                24.0,
                1.0,
            ),
            num_field(
                "verticalPositionRatio",
                "垂直位置比例",
                "窗口在屏幕垂直方向的位置比例(0=顶部, 1=底部)",
                "window",
                42,
                0.28,
                0.0,
                1.0,
                0.01,
            ),
            // ---- 浅色配色 ----
            color_field(
                "bgPrimary",
                "主背景色",
                "浅色模式下的主背景颜色",
                "colorsLight",
                50,
                "#ffffff",
            ),
            color_field(
                "bgSecondary",
                "副背景色",
                "浅色模式下的次要背景颜色",
                "colorsLight",
                51,
                "#f5f5f5",
            ),
            color_field(
                "textPrimary",
                "主文字色",
                "浅色模式下的主要文字颜色",
                "colorsLight",
                52,
                "#1a1a1a",
            ),
            color_field(
                "textSecondary",
                "副文字色",
                "浅色模式下的次要文字颜色",
                "colorsLight",
                53,
                "#666666",
            ),
            color_field(
                "borderColor",
                "边框色",
                "浅色模式下的边框颜色",
                "colorsLight",
                54,
                "#e5e5e5",
            ),
            color_field(
                "accentColor",
                "强调色",
                "浅色模式下的强调/链接颜色",
                "colorsLight",
                55,
                "#2080f0",
            ),
            color_field(
                "hoverColor",
                "悬停色",
                "浅色模式下鼠标悬停的背景颜色",
                "colorsLight",
                56,
                "rgba(0,0,0,0.04)",
            ),
            // ---- 深色配色 ----
            color_field(
                "darkBgPrimary",
                "主背景色",
                "深色模式下的主背景颜色",
                "colorsDark",
                60,
                "#1a1a1a",
            ),
            color_field(
                "darkBgSecondary",
                "副背景色",
                "深色模式下的次要背景颜色",
                "colorsDark",
                61,
                "#2a2a2a",
            ),
            color_field(
                "darkTextPrimary",
                "主文字色",
                "深色模式下的主要文字颜色",
                "colorsDark",
                62,
                "#e5e5e5",
            ),
            color_field(
                "darkTextSecondary",
                "副文字色",
                "深色模式下的次要文字颜色",
                "colorsDark",
                63,
                "#999999",
            ),
            color_field(
                "darkBorderColor",
                "边框色",
                "深色模式下的边框颜色",
                "colorsDark",
                64,
                "#333333",
            ),
            color_field(
                "darkAccentColor",
                "强调色",
                "深色模式下的强调/链接颜色",
                "colorsDark",
                65,
                "#4098fc",
            ),
            color_field(
                "darkHoverColor",
                "悬停色",
                "深色模式下鼠标悬停的背景颜色",
                "colorsDark",
                66,
                "rgba(255,255,255,0.06)",
            ),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        *self.settings.write() = settings;
        Ok(())
    }

    fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
        // 主题
        if let Some(theme) = settings.get("theme").and_then(|v| v.as_str()) {
            if !["system", "light", "dark"].contains(&theme) {
                return Err(ConfigError::ValidationFailed(format!(
                    "Invalid theme value: {}",
                    theme
                )));
            }
        }
        // 语言
        if let Some(lang) = settings.get("language").and_then(|v| v.as_str()) {
            if !["zh-Hans", "zh-Hant", "en"].contains(&lang) {
                return Err(ConfigError::ValidationFailed(format!(
                    "Invalid language value: {}",
                    lang
                )));
            }
        }
        // 数值范围校验
        let numeric_validations: [(&str, f64, f64); 10] = [
            ("searchBarHeight", 40.0, 120.0),
            ("searchBarFontRatio", 0.3, 0.8),
            ("resultItemHeight", 36.0, 80.0),
            ("resultItemFontRatio", 0.2, 0.5),
            ("resultItemSubtitleFontRatio", 0.15, 0.4),
            ("maxVisibleResults", 3.0, 20.0),
            ("footerHeight", 0.0, 60.0),
            ("footerFontRatio", 0.15, 0.35),
            ("windowWidth", 400.0, 1200.0),
            ("windowCornerRadius", 0.0, 24.0),
        ];
        for (key, min, max) in &numeric_validations {
            if let Some(val) = settings.get(*key).and_then(|v| v.as_f64()) {
                if val < *min || val > *max {
                    return Err(ConfigError::ValidationFailed(format!(
                        "{} value {} is out of range [{}, {}]",
                        key, val, min, max
                    )));
                }
            }
        }
        // verticalPositionRatio
        if let Some(val) = settings
            .get("verticalPositionRatio")
            .and_then(|v| v.as_f64())
        {
            if !(0.0..=1.0).contains(&val) {
                return Err(ConfigError::ValidationFailed(format!(
                    "verticalPositionRatio value {} is out of range [0.0, 1.0]",
                    val
                )));
            }
        }
        Ok(())
    }

    fn on_settings_changed(&self) {
        info!("Appearance config changed — frontend will apply CSS variables via config-changed event");
    }

    fn default_enabled(&self) -> bool {
        true
    }
}
