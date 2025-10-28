/// 轻量级国际化 (i18n) 翻译管理器
///
/// 该模块提供了一个简单的翻译系统，用于后端系统托盘和通知的多语言支持。
/// 翻译器由 AppState 管理，在运行时加载JSON翻译文件，并提供快速的翻译查找功能。
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, warn};

use crate::core::storage::utils::read_str;
use crate::utils::service_locator::ServiceLocator;

/// 翻译管理器
pub struct Translator {
    /// 当前语言
    current_language: String,
    /// 翻译数据缓存 (扁平化的键值对)
    translations: HashMap<String, String>,
    /// 是否已初始化
    initialized: bool,
}

impl Default for Translator {
    fn default() -> Self {
        Self::new()
    }
}

impl Translator {
    /// 创建新的翻译管理器
    pub fn new() -> Self {
        Self {
            current_language: "en".to_string(),
            translations: HashMap::new(),
            initialized: false,
        }
    }

    /// 从JSON文件加载翻译
    /// * `language` - 语言代码 (zh-Hans, zh-Hant, en)
    pub fn load_language(&mut self, language: &str) {
        let locale_path = self.get_locale_file_path(language);

        match read_str(&locale_path) {
            Ok(content) => match serde_json::from_str::<Value>(&content) {
                Ok(json) => {
                    self.translations.clear();
                    self.flatten_json(&json, String::new());
                    self.current_language = language.to_string();
                    self.initialized = true;
                    debug!(
                        "成功加载语言: {}, 翻译键数量: {}",
                        language,
                        self.translations.len()
                    );
                }
                Err(e) => {
                    warn!("解析语言文件失败 {}: {:?}", language, e);
                }
            },
            Err(e) => {
                warn!("读取语言文件失败 {}: {:?}", locale_path, e);
            }
        }
    }

    /// 获取语言文件路径
    fn get_locale_file_path(&self, language: &str) -> String {
        // 使用 tauri 的资源目录
        let resource_path = PathBuf::from("locales").join(format!("{}.json", language));

        // 尝试从可执行文件旁边的 locales 目录读取
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let locale_file = exe_dir.join(&resource_path);
                if locale_file.exists() {
                    return locale_file.to_string_lossy().to_string();
                }
            }
        }

        // 回退到相对路径
        format!("src-tauri/locales/{}.json", language)
    }

    /// 将嵌套的JSON扁平化为点分隔的键
    /// 例如: {"tray": {"show_settings": "xxx"}} -> "tray.show_settings" = "xxx"
    fn flatten_json(&mut self, value: &Value, prefix: String) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    let new_prefix = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    self.flatten_json(val, new_prefix);
                }
            }
            Value::String(s) => {
                self.translations.insert(prefix, s.clone());
            }
            _ => {
                // 忽略非字符串值
            }
        }
    }

    /// 获取翻译文本
    pub fn get(&self, key: &str) -> String {
        self.translations.get(key).cloned().unwrap_or_else(|| {
            warn!("翻译键未找到: {}", key);
            key.to_string()
        })
    }

    /// 获取翻译文本并替换占位符
    pub fn get_with_replacements(
        &self,
        key: &str,
        replacements: &HashMap<String, String>,
    ) -> String {
        let mut text = self.get(key);

        for (placeholder, value) in replacements {
            let placeholder_key = format!("{{{}}}", placeholder);
            text = text.replace(&placeholder_key, value);
        }

        text
    }

    /// 获取当前语言
    pub fn current_language(&self) -> &str {
        &self.current_language
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

// ========== 公共 API ==========

/// 初始化翻译系统
///
/// 应该在应用启动时调用一次
///
/// # 参数
/// * `language` - 初始语言代码 (zh-Hans, zh-Hant, en)
pub fn init_translator(language: &str) {
    let state = ServiceLocator::get_state();
    let translator_arc = state.get_translator();
    let mut translator = translator_arc.write();
    translator.load_language(language);
    debug!("翻译系统已初始化，语言: {}", language);
}

/// 切换语言
///
/// # 参数
/// * `language` - 新的语言代码
pub fn switch_language(language: &str) {
    let state = ServiceLocator::get_state();
    let translator_arc = state.get_translator();
    let mut translator = translator_arc.write();
    if translator.current_language() != language {
        translator.load_language(language);
        debug!("语言已切换到: {}", language);
    }
}

/// 获取翻译文本 (简化版)
///
/// # 参数
/// * `key` - 翻译键 (如 "tray.show_settings")
///
/// # 示例
/// ```
/// let text = t("tray.show_settings");
/// ```
pub fn t(key: &str) -> String {
    let state = ServiceLocator::get_state();
    let translator_arc = state.get_translator();
    let translator = translator_arc.read();
    if !translator.is_initialized() {
        warn!("翻译系统未初始化，使用键作为默认值: {}", key);
        return key.to_string();
    }
    translator.get(key)
}

/// 获取翻译文本并替换占位符
///
/// # 参数
/// * `key` - 翻译键
/// * `replacements` - 占位符替换数组 (如 &[("program", "VSCode"), ("index", "1")])
///
/// # 示例
/// ```
/// let text = t_with("notifications.error", &[("program", "VSCode")]);
/// ```
pub fn t_with(key: &str, replacements: &[(&str, &str)]) -> String {
    let state = ServiceLocator::get_state();
    let translator_arc = state.get_translator();
    let translator = translator_arc.read();
    if !translator.is_initialized() {
        warn!("翻译系统未初始化，使用键作为默认值: {}", key);
        return key.to_string();
    }

    let replacement_map: HashMap<String, String> = replacements
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    translator.get_with_replacements(key, &replacement_map)
}

/// 获取当前语言
pub fn current_language() -> String {
    let state = ServiceLocator::get_state();
    let translator_arc = state.get_translator();
    let translator = translator_arc.read();
    translator.current_language().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatten_json() {
        let mut translator = Translator::new();
        let json: Value = serde_json::from_str(
            r#"
        {
            "tray": {
                "show_settings": "Open Settings",
                "exit": "Exit"
            },
            "notifications": {
                "success": "Success"
            }
        }
        "#,
        )
        .unwrap();

        translator.flatten_json(&json, String::new());

        assert_eq!(translator.get("tray.show_settings"), "Open Settings");
        assert_eq!(translator.get("tray.exit"), "Exit");
        assert_eq!(translator.get("notifications.success"), "Success");
    }

    #[test]
    fn test_replacements() {
        let mut translator = Translator::new();
        translator.translations.insert(
            "test.message".to_string(),
            "Hello {name}, you have {count} messages".to_string(),
        );

        let mut replacements = HashMap::new();
        replacements.insert("name".to_string(), "Alice".to_string());
        replacements.insert("count".to_string(), "5".to_string());

        let result = translator.get_with_replacements("test.message", &replacements);
        assert_eq!(result, "Hello Alice, you have 5 messages");
    }
}
