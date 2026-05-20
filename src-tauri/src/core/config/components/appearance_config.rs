use crate::core::config::setting_builders::SchemaBuilder;
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
            SchemaBuilder::select("theme", "主题", "选择浅色、深色或跟随系统主题")
                .group("theme")
                .order(0)
                .options(&["system", "light", "dark"])
                .default("system")
                .build(),
            SchemaBuilder::select("language", "语言", "界面显示语言")
                .group("theme")
                .order(1)
                .options(&["zh-Hans", "zh-Hant", "en"])
                .default("zh-Hans")
                .build(),
            // ---- 搜索栏 ----
            SchemaBuilder::number("search_bar_height", "搜索栏高度", "搜索栏的高度(px)")
                .group("searchBar")
                .order(10)
                .default(72.0)
                .min(40.0)
                .max(120.0)
                .step(1.0)
                .build(),
            SchemaBuilder::number(
                "search_bar_font_ratio",
                "搜索栏字体比例",
                "字体大小 = 搜索栏高度 × 此比例",
            )
            .group("searchBar")
            .order(11)
            .default(0.56)
            .min(0.3)
            .max(0.8)
            .step(0.01)
            .build(),
            SchemaBuilder::text(
                "search_bar_font_family",
                "搜索栏字体",
                "字体族名称，留空则跟随系统",
            )
            .group("searchBar")
            .order(12)
            .default("")
            .build(),
            SchemaBuilder::text("search_bar_placeholder", "搜索栏占位符", "搜索栏的提示文本")
                .group("searchBar")
                .order(13)
                .default("Hello, ZeroLaunch! ヾ(≧▽≦*)o")
                .build(),
            // ---- 结果栏 ----
            SchemaBuilder::number("result_item_height", "结果项高度", "单条结果的高度(px)")
                .group("resultList")
                .order(20)
                .default(54.0)
                .min(36.0)
                .max(80.0)
                .step(1.0)
                .build(),
            SchemaBuilder::number(
                "result_item_font_ratio",
                "结果标题字体比例",
                "标题字体大小 = 结果项高度 × 此比例",
            )
            .group("resultList")
            .order(21)
            .default(0.30)
            .min(0.2)
            .max(0.5)
            .step(0.01)
            .build(),
            SchemaBuilder::number(
                "result_item_subtitle_font_ratio",
                "结果副标题字体比例",
                "副标题字体大小 = 结果项高度 × 此比例",
            )
            .group("resultList")
            .order(22)
            .default(0.24)
            .min(0.15)
            .max(0.4)
            .step(0.01)
            .build(),
            SchemaBuilder::number(
                "result_item_icon_ratio",
                "图标大小比例",
                "图标大小 = 结果项高度 × 此比例",
            )
            .group("resultList")
            .order(23)
            .default(0.72)
            .min(0.3)
            .max(0.9)
            .step(0.01)
            .build(),
            SchemaBuilder::text(
                "result_item_font_family",
                "结果栏字体",
                "字体族名称，留空则跟随系统",
            )
            .group("resultList")
            .order(24)
            .default("")
            .build(),
            SchemaBuilder::number(
                "max_visible_results",
                "最大可见结果数",
                "不滚动时最多显示的结果条数",
            )
            .group("resultList")
            .order(25)
            .default(8.0)
            .min(3.0)
            .max(20.0)
            .step(1.0)
            .build(),
            SchemaBuilder::boolean(
                "show_launch_command",
                "显示启动命令",
                "在结果项中显示启动命令路径",
            )
            .group("resultList")
            .order(26)
            .default(false)
            .build(),
            // ---- 底栏 ----
            SchemaBuilder::number("footer_height", "底栏高度", "底栏的高度(px)，设为0则隐藏")
                .group("footer")
                .order(30)
                .default(48.0)
                .min(0.0)
                .max(60.0)
                .step(1.0)
                .build(),
            SchemaBuilder::number(
                "footer_font_ratio",
                "底栏字体比例",
                "字体大小 = 底栏高度 × 此比例",
            )
            .group("footer")
            .order(31)
            .default(0.25)
            .min(0.15)
            .max(0.35)
            .step(0.01)
            .build(),
            SchemaBuilder::text(
                "footer_font_family",
                "底栏字体",
                "字体族名称，留空则跟随系统",
            )
            .group("footer")
            .order(32)
            .default("")
            .build(),
            // ---- 窗口 ----
            SchemaBuilder::number("window_width", "窗口宽度", "搜索窗口的宽度(px)")
                .group("window")
                .order(40)
                .default(800.0)
                .min(400.0)
                .max(1200.0)
                .step(10.0)
                .build(),
            SchemaBuilder::number("window_corner_radius", "窗口圆角", "窗口圆角大小(px)")
                .group("window")
                .order(41)
                .default(12.0)
                .min(0.0)
                .max(24.0)
                .step(1.0)
                .build(),
            SchemaBuilder::number(
                "vertical_position_ratio",
                "垂直位置比例",
                "窗口在屏幕垂直方向的位置比例(0=顶部, 1=底部)",
            )
            .group("window")
            .order(42)
            .default(0.28)
            .min(0.0)
            .max(1.0)
            .step(0.01)
            .build(),
            // ---- 浅色配色 ----
            SchemaBuilder::color("bg_primary", "主背景色", "浅色模式下的主背景颜色")
                .group("colorsLight")
                .order(50)
                .default("#ffffff")
                .build(),
            SchemaBuilder::color("bg_secondary", "副背景色", "浅色模式下的次要背景颜色")
                .group("colorsLight")
                .order(51)
                .default("#f5f5f5")
                .build(),
            SchemaBuilder::color("text_primary", "主文字色", "浅色模式下的主要文字颜色")
                .group("colorsLight")
                .order(52)
                .default("#1a1a1a")
                .build(),
            SchemaBuilder::color("text_secondary", "副文字色", "浅色模式下的次要文字颜色")
                .group("colorsLight")
                .order(53)
                .default("#666666")
                .build(),
            SchemaBuilder::color("border_color", "边框色", "浅色模式下的边框颜色")
                .group("colorsLight")
                .order(54)
                .default("#e5e5e5")
                .build(),
            SchemaBuilder::color("accent_color", "强调色", "浅色模式下的强调/链接颜色")
                .group("colorsLight")
                .order(55)
                .default("#2080f0")
                .build(),
            SchemaBuilder::color("hover_color", "悬停色", "浅色模式下鼠标悬停的背景颜色")
                .group("colorsLight")
                .order(56)
                .default("rgba(0,0,0,0.04)")
                .build(),
            // ---- 深色配色 ----
            SchemaBuilder::color("dark_bg_primary", "主背景色", "深色模式下的主背景颜色")
                .group("colorsDark")
                .order(60)
                .default("#1a1a1a")
                .build(),
            SchemaBuilder::color("dark_bg_secondary", "副背景色", "深色模式下的次要背景颜色")
                .group("colorsDark")
                .order(61)
                .default("#2a2a2a")
                .build(),
            SchemaBuilder::color("dark_text_primary", "主文字色", "深色模式下的主要文字颜色")
                .group("colorsDark")
                .order(62)
                .default("#e5e5e5")
                .build(),
            SchemaBuilder::color(
                "dark_text_secondary",
                "副文字色",
                "深色模式下的次要文字颜色",
            )
            .group("colorsDark")
            .order(63)
            .default("#999999")
            .build(),
            SchemaBuilder::color("dark_border_color", "边框色", "深色模式下的边框颜色")
                .group("colorsDark")
                .order(64)
                .default("#333333")
                .build(),
            SchemaBuilder::color("dark_accent_color", "强调色", "深色模式下的强调/链接颜色")
                .group("colorsDark")
                .order(65)
                .default("#4098fc")
                .build(),
            SchemaBuilder::color("dark_hover_color", "悬停色", "深色模式下鼠标悬停的背景颜色")
                .group("colorsDark")
                .order(66)
                .default("rgba(255,255,255,0.06)")
                .build(),
            // ---- 背景图片 ----
            SchemaBuilder::image(
                "bg_image",
                "背景图片",
                "浅色模式下的背景图片，留空则不使用背景图",
            )
            .group("background")
            .order(70)
            .default("")
            .build(),
            SchemaBuilder::image(
                "bg_image_dark",
                "深色背景图片",
                "深色模式下的背景图片，留空则回退到浅色背景图",
            )
            .group("background")
            .order(71)
            .default("")
            .build(),
            SchemaBuilder::select("bg_size", "背景尺寸", "CSS background-size 属性")
                .group("background")
                .order(72)
                .options(&["cover", "contain", "auto", "100% auto"])
                .default("cover")
                .build(),
            SchemaBuilder::select("bg_position", "背景位置", "CSS background-position 属性")
                .group("background")
                .order(73)
                .options(&[
                    "center",
                    "top",
                    "bottom",
                    "left",
                    "right",
                    "top left",
                    "top right",
                    "bottom left",
                    "bottom right",
                ])
                .default("center")
                .build(),
            SchemaBuilder::select("bg_repeat", "背景重复", "CSS background-repeat 属性")
                .group("background")
                .order(74)
                .options(&["no-repeat", "repeat", "repeat-x", "repeat-y"])
                .default("no-repeat")
                .build(),
            SchemaBuilder::number(
                "bg_opacity",
                "背景不透明度",
                "背景图片的不透明度 (0.0 ~ 1.0)",
            )
            .group("background")
            .order(75)
            .default(1.0)
            .min(0.0)
            .max(1.0)
            .step(0.01)
            .build(),
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
        let numeric_validations: [(&str, f64, f64); 11] = [
            ("search_bar_height", 40.0, 120.0),
            ("search_bar_font_ratio", 0.3, 0.8),
            ("result_item_height", 36.0, 80.0),
            ("result_item_font_ratio", 0.2, 0.5),
            ("result_item_subtitle_font_ratio", 0.15, 0.4),
            ("result_item_icon_ratio", 0.3, 0.9),
            ("max_visible_results", 3.0, 20.0),
            ("footer_height", 0.0, 60.0),
            ("footer_font_ratio", 0.15, 0.35),
            ("window_width", 400.0, 1200.0),
            ("window_corner_radius", 0.0, 24.0),
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
            .get("vertical_position_ratio")
            .and_then(|v| v.as_f64())
        {
            if !(0.0..=1.0).contains(&val) {
                return Err(ConfigError::ValidationFailed(format!(
                    "verticalPositionRatio value {} is out of range [0.0, 1.0]",
                    val
                )));
            }
        }
        // bgOpacity
        if let Some(val) = settings.get("bg_opacity").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&val) {
                return Err(ConfigError::ValidationFailed(format!(
                    "bgOpacity value {} is out of range [0.0, 1.0]",
                    val
                )));
            }
        }
        // 背景图片 CSS 属性枚举校验
        let bg_size_opts = ["cover", "contain", "auto", "100% auto"];
        if let Some(v) = settings.get("bg_size").and_then(|v| v.as_str()) {
            if !bg_size_opts.contains(&v) {
                return Err(ConfigError::ValidationFailed(format!(
                    "Invalid bgSize value: {}",
                    v
                )));
            }
        }
        let bg_position_opts = [
            "center",
            "top",
            "bottom",
            "left",
            "right",
            "top left",
            "top right",
            "bottom left",
            "bottom right",
        ];
        if let Some(v) = settings.get("bg_position").and_then(|v| v.as_str()) {
            if !bg_position_opts.contains(&v) {
                return Err(ConfigError::ValidationFailed(format!(
                    "Invalid bgPosition value: {}",
                    v
                )));
            }
        }
        let bg_repeat_opts = ["no-repeat", "repeat", "repeat-x", "repeat-y"];
        if let Some(v) = settings.get("bg_repeat").and_then(|v| v.as_str()) {
            if !bg_repeat_opts.contains(&v) {
                return Err(ConfigError::ValidationFailed(format!(
                    "Invalid bgRepeat value: {}",
                    v
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
