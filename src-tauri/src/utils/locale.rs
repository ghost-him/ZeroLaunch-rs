/// 系统区域设置和语言检测工具
use tracing::{debug, info};

/// 获取系统区域设置。
///
/// 平台无关实现：经 `sys_locale` crate 查询（Windows 走
/// GetUserDefaultLocaleName、macOS 走 NSLocale、Linux 走 LANG 等），
/// GUI 应用下也能拿到真实系统区域，失败返回 None。
pub fn get_system_locale() -> Option<String> {
    let locale = sys_locale::get_locale()?;
    debug!("检测到系统语言: {}", locale);
    Some(locale)
}

pub fn map_locale_to_language(locale: &str) -> String {
    // 转换为小写以便于匹配
    let locale_lower = locale.to_lowercase();

    if locale_lower.starts_with("zh-") {
        let traditional_locales = ["zh-tw", "zh-hk", "zh-mo", "zh-hant"];

        for traditional in &traditional_locales {
            if locale_lower.starts_with(traditional) {
                return "zh-Hant".to_string();
            }
        }

        // 默认其他中文 locale 为简体中文
        return "zh-Hans".to_string();
    }

    // 英语处理
    if locale_lower.starts_with("en-") || locale_lower == "en" {
        debug!("系统语言 {} 映射为英语", locale);
        return "en".to_string();
    }

    "en".to_string()
}

/// 获取适合应用的默认语言
///
/// 尝试检测系统语言并映射到应用支持的语言,如果检测失败则返回英语
pub fn get_default_app_language() -> String {
    match get_system_locale() {
        Some(locale) => map_locale_to_language(&locale),
        None => {
            info!("无法检测系统语言，使用英语作为默认");
            "en".to_string()
        }
    }
}
