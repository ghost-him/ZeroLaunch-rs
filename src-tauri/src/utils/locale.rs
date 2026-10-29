/// 系统语言探测与界面语言映射。
///
/// 读取系统首选语言使用跨平台库 `sys-locale`（Windows 走 `GetUserDefaultLocaleName`，
/// macOS 走 `NSLocale`，Unix 走 `LC_ALL`/`LANG`），不再手写各平台实现。
use tracing::info;

/// 读取系统首选语言标签（BCP-47，如 `zh-CN` / `en-US`）。
/// 返回：系统语言标签；系统未提供时返回 None。
pub fn get_system_locale() -> Option<String> {
    sys_locale::get_locale()
}

/// 将任意语言标签映射到本应用支持的语言码。
/// 参数：locale - 系统语言标签，分隔符可为 `-` 或 `_`。
/// 返回：`zh-Hant`（台湾/港澳/显式繁体）、`zh-Hans`（其余中文）、`en`（非中文与无法识别时的兜底）。
pub fn map_locale_to_language(locale: &str) -> String {
    // 统一分隔符后再匹配：Unix 侧可能返回 zh_CN.UTF-8 这类 POSIX 写法
    let normalized = locale.to_lowercase().replace('_', "-");

    // 语言码必须恰好是 zh：`zhx` 这类前缀相同的其他标签不算中文
    if normalized == "zh" || normalized.starts_with("zh-") {
        // 只取 "zh" 之后的部分，避免把语言码本身当成区域码误匹配
        let region = &normalized[2..];
        if ["-tw", "-hk", "-mo", "-hant"]
            .iter()
            .any(|tag| region.starts_with(tag))
        {
            return "zh-Hant".to_string();
        }
        // 包括 zh-Hans、zh-CN、zh-SG 及无区域码的 zh
        return "zh-Hans".to_string();
    }

    // 非中文一律英语：与支持语言集合取交集
    "en".to_string()
}

/// 获取应用的默认界面语言。
/// 返回：系统语言映射出的支持语言码；读取失败时回退英语。
pub fn get_default_app_language() -> String {
    match get_system_locale() {
        Some(locale) => map_locale_to_language(&locale),
        None => {
            info!("无法读取系统语言，使用英语作为默认");
            "en".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_simplified_chinese_locales() {
        for locale in ["zh-CN", "zh-Hans", "zh-SG", "zh", "zh_CN", "zh-Hans-CN"] {
            assert_eq!(map_locale_to_language(locale), "zh-Hans", "locale={locale}");
        }
    }

    #[test]
    fn maps_traditional_chinese_locales() {
        for locale in ["zh-TW", "zh-HK", "zh-MO", "zh-Hant", "zh_HK", "zh-Hant-TW"] {
            assert_eq!(map_locale_to_language(locale), "zh-Hant", "locale={locale}");
        }
    }

    #[test]
    fn maps_non_chinese_locales_to_english() {
        for locale in ["en-US", "en-GB", "en", "ja-JP", "de-DE", "", "zhx"] {
            assert_eq!(map_locale_to_language(locale), "en", "locale={locale}");
        }
    }

    #[test]
    fn default_language_is_one_of_supported() {
        let lang = get_default_app_language();
        assert!(
            ["zh-Hans", "zh-Hant", "en"].contains(&lang.as_str()),
            "unexpected default language: {lang}"
        );
    }
}
