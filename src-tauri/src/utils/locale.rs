/// 系统区域设置和语言检测工具
use tracing::{debug, info, warn};
use windows::Win32::Globalization::GetUserDefaultLocaleName;

/// 使用 Windows API GetUserDefaultLocaleName 来获取用户的默认区域设置
pub fn get_system_locale() -> Option<String> {
    unsafe {
        const LOCALE_NAME_MAX_LENGTH: usize = 85;
        let mut locale_name: [u16; LOCALE_NAME_MAX_LENGTH] = [0; LOCALE_NAME_MAX_LENGTH];

        let result = GetUserDefaultLocaleName(&mut locale_name);

        if result > 0 {
            // 找到第一个 null 终止符
            let len = locale_name
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(result as usize);
            let locale_string = String::from_utf16_lossy(&locale_name[..len]);
            debug!("检测到系统语言: {}", locale_string);
            Some(locale_string)
        } else {
            warn!("无法获取系统语言设置");
            None
        }
    }
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
