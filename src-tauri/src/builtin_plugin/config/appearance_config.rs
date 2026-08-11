use crate::core::config::setting_builders::SchemaBuilder;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};

// ============ 默认值函数 ============
// 为所有非类型原生的业务默认值提供函数，供 #[serde(default = "...")] 引用。

fn default_theme() -> String {
    "system".to_string()
}
fn default_search_bar_height() -> f64 {
    72.0
}
fn default_search_bar_font_ratio() -> f64 {
    0.56
}
fn default_search_bar_placeholder() -> String {
    "Hello, ZeroLaunch! ヾ(≧▽≦*)o".to_string()
}
fn default_result_item_height() -> f64 {
    54.0
}
fn default_result_item_font_ratio() -> f64 {
    0.30
}
fn default_result_item_subtitle_font_ratio() -> f64 {
    0.24
}
fn default_show_subtitle() -> bool {
    true
}
fn default_result_item_icon_ratio() -> f64 {
    0.72
}
fn default_max_visible_results() -> f64 {
    8.0
}
fn default_footer_height() -> f64 {
    48.0
}
fn default_footer_font_ratio() -> f64 {
    0.25
}
fn default_window_width() -> f64 {
    800.0
}
fn default_window_corner_radius() -> f64 {
    12.0
}
fn default_vertical_position_ratio() -> f64 {
    0.28
}
fn default_bg_primary() -> String {
    "#ffffff".to_string()
}
fn default_bg_secondary() -> String {
    "#f5f5f5".to_string()
}
fn default_text_primary() -> String {
    "#1a1a1a".to_string()
}
fn default_text_secondary() -> String {
    "#666666".to_string()
}
fn default_border_color() -> String {
    "#e5e5e5".to_string()
}
fn default_accent_color() -> String {
    "#2080f0".to_string()
}
fn default_hover_color() -> String {
    "rgba(0,0,0,0.04)".to_string()
}
fn default_dark_bg_primary() -> String {
    "#1a1a1a".to_string()
}
fn default_dark_bg_secondary() -> String {
    "#2a2a2a".to_string()
}
fn default_dark_text_primary() -> String {
    "#e5e5e5".to_string()
}
fn default_dark_text_secondary() -> String {
    "#999999".to_string()
}
fn default_dark_border_color() -> String {
    "#333333".to_string()
}
fn default_dark_accent_color() -> String {
    "#4098fc".to_string()
}
fn default_dark_hover_color() -> String {
    "rgba(255,255,255,0.06)".to_string()
}
fn default_bg_size() -> String {
    "cover".to_string()
}
fn default_bg_position() -> String {
    "center".to_string()
}
fn default_bg_repeat() -> String {
    "no-repeat".to_string()
}
fn default_bg_opacity() -> f64 {
    1.0
}

// ============ 强类型配置结构 ============

/// 外观设置的强类型配置结构。
/// 每个字段标注 `#[serde(default)]`，确保老 JSON 缺失新字段时回退到业务默认值，
/// 且 `get_settings()` 返回的 JSON 始终包含完整字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    // ---- 主题 ----
    #[serde(rename = "theme", default = "default_theme")]
    pub theme: String,

    // ---- 搜索栏 ----
    #[serde(rename = "search_bar_height", default = "default_search_bar_height")]
    pub search_bar_height: f64,
    #[serde(
        rename = "search_bar_font_ratio",
        default = "default_search_bar_font_ratio"
    )]
    pub search_bar_font_ratio: f64,
    #[serde(rename = "search_bar_font_family", default)]
    pub search_bar_font_family: String,
    #[serde(
        rename = "search_bar_placeholder",
        default = "default_search_bar_placeholder"
    )]
    pub search_bar_placeholder: String,

    // ---- 结果栏 ----
    #[serde(rename = "result_item_height", default = "default_result_item_height")]
    pub result_item_height: f64,
    #[serde(
        rename = "result_item_font_ratio",
        default = "default_result_item_font_ratio"
    )]
    pub result_item_font_ratio: f64,
    #[serde(
        rename = "result_item_subtitle_font_ratio",
        default = "default_result_item_subtitle_font_ratio"
    )]
    pub result_item_subtitle_font_ratio: f64,
    #[serde(
        rename = "result_item_icon_ratio",
        default = "default_result_item_icon_ratio"
    )]
    pub result_item_icon_ratio: f64,
    #[serde(rename = "result_item_font_family", default)]
    pub result_item_font_family: String,
    #[serde(
        rename = "max_visible_results",
        default = "default_max_visible_results"
    )]
    pub max_visible_results: f64,
    #[serde(rename = "show_launch_command", default)]
    pub show_launch_command: bool,
    #[serde(rename = "show_subtitle", default = "default_show_subtitle")]
    pub show_subtitle: bool,

    // ---- 底栏 ----
    #[serde(rename = "footer_height", default = "default_footer_height")]
    pub footer_height: f64,
    #[serde(rename = "footer_font_ratio", default = "default_footer_font_ratio")]
    pub footer_font_ratio: f64,
    #[serde(rename = "footer_font_family", default)]
    pub footer_font_family: String,

    // ---- 窗口 ----
    #[serde(rename = "window_width", default = "default_window_width")]
    pub window_width: f64,
    #[serde(
        rename = "window_corner_radius",
        default = "default_window_corner_radius"
    )]
    pub window_corner_radius: f64,
    #[serde(
        rename = "vertical_position_ratio",
        default = "default_vertical_position_ratio"
    )]
    pub vertical_position_ratio: f64,

    // ---- 浅色配色 ----
    #[serde(rename = "bg_primary", default = "default_bg_primary")]
    pub bg_primary: String,
    #[serde(rename = "bg_secondary", default = "default_bg_secondary")]
    pub bg_secondary: String,
    #[serde(rename = "text_primary", default = "default_text_primary")]
    pub text_primary: String,
    #[serde(rename = "text_secondary", default = "default_text_secondary")]
    pub text_secondary: String,
    #[serde(rename = "border_color", default = "default_border_color")]
    pub border_color: String,
    #[serde(rename = "accent_color", default = "default_accent_color")]
    pub accent_color: String,
    #[serde(rename = "hover_color", default = "default_hover_color")]
    pub hover_color: String,

    // ---- 深色配色 ----
    #[serde(rename = "dark_bg_primary", default = "default_dark_bg_primary")]
    pub dark_bg_primary: String,
    #[serde(rename = "dark_bg_secondary", default = "default_dark_bg_secondary")]
    pub dark_bg_secondary: String,
    #[serde(rename = "dark_text_primary", default = "default_dark_text_primary")]
    pub dark_text_primary: String,
    #[serde(
        rename = "dark_text_secondary",
        default = "default_dark_text_secondary"
    )]
    pub dark_text_secondary: String,
    #[serde(rename = "dark_border_color", default = "default_dark_border_color")]
    pub dark_border_color: String,
    #[serde(rename = "dark_accent_color", default = "default_dark_accent_color")]
    pub dark_accent_color: String,
    #[serde(rename = "dark_hover_color", default = "default_dark_hover_color")]
    pub dark_hover_color: String,

    // ---- 背景图片 ----
    #[serde(rename = "bg_image", default)]
    pub bg_image: String,
    #[serde(rename = "bg_image_dark", default)]
    pub bg_image_dark: String,
    #[serde(rename = "bg_size", default = "default_bg_size")]
    pub bg_size: String,
    #[serde(rename = "bg_position", default = "default_bg_position")]
    pub bg_position: String,
    #[serde(rename = "bg_repeat", default = "default_bg_repeat")]
    pub bg_repeat: String,
    #[serde(rename = "bg_opacity", default = "default_bg_opacity")]
    pub bg_opacity: f64,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            search_bar_height: default_search_bar_height(),
            search_bar_font_ratio: default_search_bar_font_ratio(),
            search_bar_font_family: String::new(),
            search_bar_placeholder: default_search_bar_placeholder(),
            result_item_height: default_result_item_height(),
            result_item_font_ratio: default_result_item_font_ratio(),
            result_item_subtitle_font_ratio: default_result_item_subtitle_font_ratio(),
            result_item_icon_ratio: default_result_item_icon_ratio(),
            result_item_font_family: String::new(),
            max_visible_results: default_max_visible_results(),
            show_launch_command: false,
            show_subtitle: true,
            footer_height: default_footer_height(),
            footer_font_ratio: default_footer_font_ratio(),
            footer_font_family: String::new(),
            window_width: default_window_width(),
            window_corner_radius: default_window_corner_radius(),
            vertical_position_ratio: default_vertical_position_ratio(),
            bg_primary: default_bg_primary(),
            bg_secondary: default_bg_secondary(),
            text_primary: default_text_primary(),
            text_secondary: default_text_secondary(),
            border_color: default_border_color(),
            accent_color: default_accent_color(),
            hover_color: default_hover_color(),
            dark_bg_primary: default_dark_bg_primary(),
            dark_bg_secondary: default_dark_bg_secondary(),
            dark_text_primary: default_dark_text_primary(),
            dark_text_secondary: default_dark_text_secondary(),
            dark_border_color: default_dark_border_color(),
            dark_accent_color: default_dark_accent_color(),
            dark_hover_color: default_dark_hover_color(),
            bg_image: String::new(),
            bg_image_dark: String::new(),
            bg_size: default_bg_size(),
            bg_position: default_bg_position(),
            bg_repeat: default_bg_repeat(),
            bg_opacity: default_bg_opacity(),
        }
    }
}

// ============ 配置组件 ============

/// 外观配置组件。
/// 管理主题（浅色/深色/跟随系统）、语言偏好、搜索栏/结果栏/底栏尺寸与字体、窗口参数、配色方案。
pub struct AppearanceConfigComponent {
    core: ComponentCore,
    settings: RwLock<AppearanceSettings>,
}

impl Default for AppearanceConfigComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl AppearanceConfigComponent {
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "appearance-config".to_string(),
                t_key!("appearance-config", "name").to_string(),
                t_key!("appearance-config", "description").to_string(),
                ComponentType::Core,
                0,
            ),
            settings: RwLock::new(AppearanceSettings::default()),
        }
    }
}

#[async_trait]
impl Configurable for AppearanceConfigComponent {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            // ---- 主题 & 语言 ----
            SchemaBuilder::select(
                "theme",
                t_key!("appearance-config", "fields.theme.label"),
                t_key!("appearance-config", "fields.theme.desc"),
            )
            .group(t_key!("appearance-config", "groups.theme"))
            .order(0)
            // 主题选项标签由 i18n 控制（跟随系统/浅色/深色），不直接显示原始枚举值
            .options_with_labels(&[
                (
                    "system",
                    t_key!("appearance-config", "options.theme.system"),
                ),
                ("light", t_key!("appearance-config", "options.theme.light")),
                ("dark", t_key!("appearance-config", "options.theme.dark")),
            ])
            .default("system")
            .build(),
            // ---- 搜索栏 ----
            SchemaBuilder::number(
                "search_bar_height",
                t_key!("appearance-config", "fields.search_bar_height.label"),
                t_key!("appearance-config", "fields.search_bar_height.desc"),
            )
            .group(t_key!("appearance-config", "groups.searchBar"))
            .order(10)
            .default(72.0)
            .min(40.0)
            .max(120.0)
            .step(1.0)
            .build(),
            SchemaBuilder::number(
                "search_bar_font_ratio",
                t_key!("appearance-config", "fields.search_bar_font_ratio.label"),
                t_key!("appearance-config", "fields.search_bar_font_ratio.desc"),
            )
            .group(t_key!("appearance-config", "groups.searchBar"))
            .order(11)
            .default(0.56)
            .min(0.3)
            .max(0.8)
            .step(0.01)
            .build(),
            SchemaBuilder::text(
                "search_bar_font_family",
                t_key!("appearance-config", "fields.search_bar_font_family.label"),
                t_key!("appearance-config", "fields.search_bar_font_family.desc"),
            )
            .group(t_key!("appearance-config", "groups.searchBar"))
            .order(12)
            .default("")
            .build(),
            SchemaBuilder::text(
                "search_bar_placeholder",
                t_key!("appearance-config", "fields.search_bar_placeholder.label"),
                t_key!("appearance-config", "fields.search_bar_placeholder.desc"),
            )
            .group(t_key!("appearance-config", "groups.searchBar"))
            .order(13)
            .default("Hello, ZeroLaunch! ヾ(≧▽≦*)o")
            .build(),
            // ---- 结果栏 ----
            SchemaBuilder::number(
                "result_item_height",
                t_key!("appearance-config", "fields.result_item_height.label"),
                t_key!("appearance-config", "fields.result_item_height.desc"),
            )
            .group(t_key!("appearance-config", "groups.resultList"))
            .order(20)
            .default(54.0)
            .min(36.0)
            .max(80.0)
            .step(1.0)
            .build(),
            SchemaBuilder::number(
                "result_item_font_ratio",
                t_key!("appearance-config", "fields.result_item_font_ratio.label"),
                t_key!("appearance-config", "fields.result_item_font_ratio.desc"),
            )
            .group(t_key!("appearance-config", "groups.resultList"))
            .order(21)
            .default(0.30)
            .min(0.2)
            .max(0.5)
            .step(0.01)
            .build(),
            SchemaBuilder::number(
                "result_item_subtitle_font_ratio",
                t_key!(
                    "appearance-config",
                    "fields.result_item_subtitle_font_ratio.label"
                ),
                t_key!(
                    "appearance-config",
                    "fields.result_item_subtitle_font_ratio.desc"
                ),
            )
            .group(t_key!("appearance-config", "groups.resultList"))
            .order(22)
            .default(0.24)
            .min(0.15)
            .max(0.4)
            .step(0.01)
            .build(),
            SchemaBuilder::number(
                "result_item_icon_ratio",
                t_key!("appearance-config", "fields.result_item_icon_ratio.label"),
                t_key!("appearance-config", "fields.result_item_icon_ratio.desc"),
            )
            .group(t_key!("appearance-config", "groups.resultList"))
            .order(23)
            .default(0.72)
            .min(0.3)
            .max(0.9)
            .step(0.01)
            .build(),
            SchemaBuilder::text(
                "result_item_font_family",
                t_key!("appearance-config", "fields.result_item_font_family.label"),
                t_key!("appearance-config", "fields.result_item_font_family.desc"),
            )
            .group(t_key!("appearance-config", "groups.resultList"))
            .order(24)
            .default("")
            .build(),
            SchemaBuilder::number(
                "max_visible_results",
                t_key!("appearance-config", "fields.max_visible_results.label"),
                t_key!("appearance-config", "fields.max_visible_results.desc"),
            )
            .group(t_key!("appearance-config", "groups.resultList"))
            .order(25)
            .default(8.0)
            .min(3.0)
            .max(20.0)
            .step(1.0)
            .build(),
            SchemaBuilder::boolean(
                "show_launch_command",
                t_key!("appearance-config", "fields.show_launch_command.label"),
                t_key!("appearance-config", "fields.show_launch_command.desc"),
            )
            .group(t_key!("appearance-config", "groups.resultList"))
            .order(26)
            .default(false)
            .build(),
            SchemaBuilder::boolean(
                "show_subtitle",
                t_key!("appearance-config", "fields.show_subtitle.label"),
                t_key!("appearance-config", "fields.show_subtitle.desc"),
            )
            .group(t_key!("appearance-config", "groups.resultList"))
            .order(27)
            .default(true)
            .build(),
            // ---- 底栏 ----
            SchemaBuilder::number(
                "footer_height",
                t_key!("appearance-config", "fields.footer_height.label"),
                t_key!("appearance-config", "fields.footer_height.desc"),
            )
            .group(t_key!("appearance-config", "groups.footer"))
            .order(30)
            .default(48.0)
            .min(0.0)
            .max(60.0)
            .step(1.0)
            .build(),
            SchemaBuilder::number(
                "footer_font_ratio",
                t_key!("appearance-config", "fields.footer_font_ratio.label"),
                t_key!("appearance-config", "fields.footer_font_ratio.desc"),
            )
            .group(t_key!("appearance-config", "groups.footer"))
            .order(31)
            .default(0.25)
            .min(0.15)
            .max(0.35)
            .step(0.01)
            .build(),
            SchemaBuilder::text(
                "footer_font_family",
                t_key!("appearance-config", "fields.footer_font_family.label"),
                t_key!("appearance-config", "fields.footer_font_family.desc"),
            )
            .group(t_key!("appearance-config", "groups.footer"))
            .order(32)
            .default("")
            .build(),
            // ---- 窗口 ----
            SchemaBuilder::number(
                "window_width",
                t_key!("appearance-config", "fields.window_width.label"),
                t_key!("appearance-config", "fields.window_width.desc"),
            )
            .group(t_key!("appearance-config", "groups.window"))
            .order(40)
            .default(800.0)
            .min(400.0)
            .max(1200.0)
            .step(10.0)
            .build(),
            SchemaBuilder::number(
                "window_corner_radius",
                t_key!("appearance-config", "fields.window_corner_radius.label"),
                t_key!("appearance-config", "fields.window_corner_radius.desc"),
            )
            .group(t_key!("appearance-config", "groups.window"))
            .order(41)
            .default(12.0)
            .min(0.0)
            .max(24.0)
            .step(1.0)
            .build(),
            SchemaBuilder::number(
                "vertical_position_ratio",
                t_key!("appearance-config", "fields.vertical_position_ratio.label"),
                t_key!("appearance-config", "fields.vertical_position_ratio.desc"),
            )
            .group(t_key!("appearance-config", "groups.window"))
            .order(42)
            .default(0.28)
            .min(0.0)
            .max(1.0)
            .step(0.01)
            .build(),
            // ---- 浅色配色 ----
            SchemaBuilder::color(
                "bg_primary",
                t_key!("appearance-config", "fields.bg_primary.label"),
                t_key!("appearance-config", "fields.bg_primary.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsLight"))
            .order(50)
            .default("#ffffff")
            .build(),
            SchemaBuilder::color(
                "bg_secondary",
                t_key!("appearance-config", "fields.bg_secondary.label"),
                t_key!("appearance-config", "fields.bg_secondary.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsLight"))
            .order(51)
            .default("#f5f5f5")
            .build(),
            SchemaBuilder::color(
                "text_primary",
                t_key!("appearance-config", "fields.text_primary.label"),
                t_key!("appearance-config", "fields.text_primary.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsLight"))
            .order(52)
            .default("#1a1a1a")
            .build(),
            SchemaBuilder::color(
                "text_secondary",
                t_key!("appearance-config", "fields.text_secondary.label"),
                t_key!("appearance-config", "fields.text_secondary.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsLight"))
            .order(53)
            .default("#666666")
            .build(),
            SchemaBuilder::color(
                "border_color",
                t_key!("appearance-config", "fields.border_color.label"),
                t_key!("appearance-config", "fields.border_color.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsLight"))
            .order(54)
            .default("#e5e5e5")
            .build(),
            SchemaBuilder::color(
                "accent_color",
                t_key!("appearance-config", "fields.accent_color.label"),
                t_key!("appearance-config", "fields.accent_color.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsLight"))
            .order(55)
            .default("#2080f0")
            .build(),
            SchemaBuilder::color(
                "hover_color",
                t_key!("appearance-config", "fields.hover_color.label"),
                t_key!("appearance-config", "fields.hover_color.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsLight"))
            .order(56)
            .default("rgba(0,0,0,0.04)")
            .build(),
            // ---- 深色配色 ----
            SchemaBuilder::color(
                "dark_bg_primary",
                t_key!("appearance-config", "fields.dark_bg_primary.label"),
                t_key!("appearance-config", "fields.dark_bg_primary.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsDark"))
            .order(60)
            .default("#1a1a1a")
            .build(),
            SchemaBuilder::color(
                "dark_bg_secondary",
                t_key!("appearance-config", "fields.dark_bg_secondary.label"),
                t_key!("appearance-config", "fields.dark_bg_secondary.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsDark"))
            .order(61)
            .default("#2a2a2a")
            .build(),
            SchemaBuilder::color(
                "dark_text_primary",
                t_key!("appearance-config", "fields.dark_text_primary.label"),
                t_key!("appearance-config", "fields.dark_text_primary.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsDark"))
            .order(62)
            .default("#e5e5e5")
            .build(),
            SchemaBuilder::color(
                "dark_text_secondary",
                t_key!("appearance-config", "fields.dark_text_secondary.label"),
                t_key!("appearance-config", "fields.dark_text_secondary.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsDark"))
            .order(63)
            .default("#999999")
            .build(),
            SchemaBuilder::color(
                "dark_border_color",
                t_key!("appearance-config", "fields.dark_border_color.label"),
                t_key!("appearance-config", "fields.dark_border_color.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsDark"))
            .order(64)
            .default("#333333")
            .build(),
            SchemaBuilder::color(
                "dark_accent_color",
                t_key!("appearance-config", "fields.dark_accent_color.label"),
                t_key!("appearance-config", "fields.dark_accent_color.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsDark"))
            .order(65)
            .default("#4098fc")
            .build(),
            SchemaBuilder::color(
                "dark_hover_color",
                t_key!("appearance-config", "fields.dark_hover_color.label"),
                t_key!("appearance-config", "fields.dark_hover_color.desc"),
            )
            .group(t_key!("appearance-config", "groups.colorsDark"))
            .order(66)
            .default("rgba(255,255,255,0.06)")
            .build(),
            // ---- 背景图片 ----
            SchemaBuilder::image(
                "bg_image",
                t_key!("appearance-config", "fields.bg_image.label"),
                t_key!("appearance-config", "fields.bg_image.desc"),
            )
            .group(t_key!("appearance-config", "groups.background"))
            .order(70)
            .default("")
            .max_image_size(20 * 1024 * 1024) // 限制最大图片大小为 20MB
            .build(),
            SchemaBuilder::image(
                "bg_image_dark",
                t_key!("appearance-config", "fields.bg_image_dark.label"),
                t_key!("appearance-config", "fields.bg_image_dark.desc"),
            )
            .group(t_key!("appearance-config", "groups.background"))
            .order(71)
            .default("")
            .max_image_size(20 * 1024 * 1024) // 限制最大图片大小为 20MB
            .build(),
            SchemaBuilder::select(
                "bg_size",
                t_key!("appearance-config", "fields.bg_size.label"),
                t_key!("appearance-config", "fields.bg_size.desc"),
            )
            .group(t_key!("appearance-config", "groups.background"))
            .order(72)
            .options(&["cover", "contain", "auto", "100% auto"])
            .default("cover")
            .build(),
            SchemaBuilder::select(
                "bg_position",
                t_key!("appearance-config", "fields.bg_position.label"),
                t_key!("appearance-config", "fields.bg_position.desc"),
            )
            .group(t_key!("appearance-config", "groups.background"))
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
            SchemaBuilder::select(
                "bg_repeat",
                t_key!("appearance-config", "fields.bg_repeat.label"),
                t_key!("appearance-config", "fields.bg_repeat.desc"),
            )
            .group(t_key!("appearance-config", "groups.background"))
            .order(74)
            .options(&["no-repeat", "repeat", "repeat-x", "repeat-y"])
            .default("no-repeat")
            .build(),
            SchemaBuilder::number(
                "bg_opacity",
                t_key!("appearance-config", "fields.bg_opacity.label"),
                t_key!("appearance-config", "fields.bg_opacity.desc"),
            )
            .group(t_key!("appearance-config", "groups.background"))
            .order(75)
            .default(1.0)
            .min(0.0)
            .max(1.0)
            .step(0.01)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: AppearanceSettings = serde_json::from_value(settings).unwrap_or_else(|e| {
            warn!(
                "failed to parse settings for {}, using defaults: {e}",
                self.component_id()
            );
            Default::default()
        });
        *self.settings.write() = parsed;
        Ok(())
    }

    async fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
        // 主题
        if let Some(theme) = settings.get("theme").and_then(|v| v.as_str()) {
            if !["system", "light", "dark"].contains(&theme) {
                return Err(ConfigError::ValidationFailed(format!(
                    "Invalid theme value: {}",
                    theme
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

use crate::plugin_framework::builtin_registry::{ConfigEntry, InventoryContext};

fn build_appearance_config(_ctx: &InventoryContext) -> std::sync::Arc<dyn Configurable> {
    std::sync::Arc::new(AppearanceConfigComponent::new())
}

::inventory::submit! {
    ConfigEntry {
        component_id: "appearance-config",
        priority: 20,
        factory: build_appearance_config,
    }
}
