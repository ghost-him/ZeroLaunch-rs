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
            SchemaBuilder::number("searchBarHeight", "搜索栏高度", "搜索栏的高度(px)")
                .group("searchBar")
                .order(10)
                .default(72.0)
                .min(40.0)
                .max(120.0)
                .step(1.0)
                .build(),
            SchemaBuilder::number(
                "searchBarFontRatio",
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
                "searchBarFontFamily",
                "搜索栏字体",
                "字体族名称，留空则跟随系统",
            )
            .group("searchBar")
            .order(12)
            .default("")
            .build(),
            SchemaBuilder::text("searchBarPlaceholder", "搜索栏占位符", "搜索栏的提示文本")
                .group("searchBar")
                .order(13)
                .default("Hello, ZeroLaunch! ヾ(≧▽≦*)o")
                .build(),
            // ---- 结果栏 ----
            SchemaBuilder::number("resultItemHeight", "结果项高度", "单条结果的高度(px)")
                .group("resultList")
                .order(20)
                .default(54.0)
                .min(36.0)
                .max(80.0)
                .step(1.0)
                .build(),
            SchemaBuilder::number(
                "resultItemFontRatio",
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
                "resultItemSubtitleFontRatio",
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
            SchemaBuilder::text(
                "resultItemFontFamily",
                "结果栏字体",
                "字体族名称，留空则跟随系统",
            )
            .group("resultList")
            .order(23)
            .default("")
            .build(),
            SchemaBuilder::number(
                "maxVisibleResults",
                "最大可见结果数",
                "不滚动时最多显示的结果条数",
            )
            .group("resultList")
            .order(24)
            .default(8.0)
            .min(3.0)
            .max(20.0)
            .step(1.0)
            .build(),
            SchemaBuilder::boolean(
                "showLaunchCommand",
                "显示启动命令",
                "在结果项中显示启动命令路径",
            )
            .group("resultList")
            .order(25)
            .default(false)
            .build(),
            // ---- 底栏 ----
            SchemaBuilder::number("footerHeight", "底栏高度", "底栏的高度(px)，设为0则隐藏")
                .group("footer")
                .order(30)
                .default(48.0)
                .min(0.0)
                .max(60.0)
                .step(1.0)
                .build(),
            SchemaBuilder::number(
                "footerFontRatio",
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
            SchemaBuilder::text("footerFontFamily", "底栏字体", "字体族名称，留空则跟随系统")
                .group("footer")
                .order(32)
                .default("")
                .build(),
            // ---- 窗口 ----
            SchemaBuilder::number("windowWidth", "窗口宽度", "搜索窗口的宽度(px)")
                .group("window")
                .order(40)
                .default(800.0)
                .min(400.0)
                .max(1200.0)
                .step(10.0)
                .build(),
            SchemaBuilder::number("windowCornerRadius", "窗口圆角", "窗口圆角大小(px)")
                .group("window")
                .order(41)
                .default(12.0)
                .min(0.0)
                .max(24.0)
                .step(1.0)
                .build(),
            SchemaBuilder::number(
                "verticalPositionRatio",
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
            SchemaBuilder::color("bgPrimary", "主背景色", "浅色模式下的主背景颜色")
                .group("colorsLight")
                .order(50)
                .default("#ffffff")
                .build(),
            SchemaBuilder::color("bgSecondary", "副背景色", "浅色模式下的次要背景颜色")
                .group("colorsLight")
                .order(51)
                .default("#f5f5f5")
                .build(),
            SchemaBuilder::color("textPrimary", "主文字色", "浅色模式下的主要文字颜色")
                .group("colorsLight")
                .order(52)
                .default("#1a1a1a")
                .build(),
            SchemaBuilder::color("textSecondary", "副文字色", "浅色模式下的次要文字颜色")
                .group("colorsLight")
                .order(53)
                .default("#666666")
                .build(),
            SchemaBuilder::color("borderColor", "边框色", "浅色模式下的边框颜色")
                .group("colorsLight")
                .order(54)
                .default("#e5e5e5")
                .build(),
            SchemaBuilder::color("accentColor", "强调色", "浅色模式下的强调/链接颜色")
                .group("colorsLight")
                .order(55)
                .default("#2080f0")
                .build(),
            SchemaBuilder::color("hoverColor", "悬停色", "浅色模式下鼠标悬停的背景颜色")
                .group("colorsLight")
                .order(56)
                .default("rgba(0,0,0,0.04)")
                .build(),
            // ---- 深色配色 ----
            SchemaBuilder::color("darkBgPrimary", "主背景色", "深色模式下的主背景颜色")
                .group("colorsDark")
                .order(60)
                .default("#1a1a1a")
                .build(),
            SchemaBuilder::color("darkBgSecondary", "副背景色", "深色模式下的次要背景颜色")
                .group("colorsDark")
                .order(61)
                .default("#2a2a2a")
                .build(),
            SchemaBuilder::color("darkTextPrimary", "主文字色", "深色模式下的主要文字颜色")
                .group("colorsDark")
                .order(62)
                .default("#e5e5e5")
                .build(),
            SchemaBuilder::color("darkTextSecondary", "副文字色", "深色模式下的次要文字颜色")
                .group("colorsDark")
                .order(63)
                .default("#999999")
                .build(),
            SchemaBuilder::color("darkBorderColor", "边框色", "深色模式下的边框颜色")
                .group("colorsDark")
                .order(64)
                .default("#333333")
                .build(),
            SchemaBuilder::color("darkAccentColor", "强调色", "深色模式下的强调/链接颜色")
                .group("colorsDark")
                .order(65)
                .default("#4098fc")
                .build(),
            SchemaBuilder::color("darkHoverColor", "悬停色", "深色模式下鼠标悬停的背景颜色")
                .group("colorsDark")
                .order(66)
                .default("rgba(255,255,255,0.06)")
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
