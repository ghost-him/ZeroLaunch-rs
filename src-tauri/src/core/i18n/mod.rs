//! 后端翻译服务。
//!
//! 内置 catalog 读取 `resource_dir/locales/<lang>.json`（vite 构建时从
//! `src-ui/i18n/locales` 复制、并经 tauri.conf.json 打包进资源），供托盘菜单、
//! CLI 等后端用户可见文本查表；插件 catalog 由 PluginManager 在插件加载/卸载时
//! 注册/移除，经 `i18n_get_plugin_translations` IPC 下发前端合并
//! （命名空间 `plugin.<id>.`）。
//!
//! 当前界面语言由本管理器内部持有：启动时按系统语言初始化，之后由 bootstrap
//! 在持久化配置加载完成及 appearance-config 语言变更时同步（`set_language`），
//! 消费方（托盘、host_handler、session_dispatcher）仅依赖本管理器，
//! 无需各自访问 ConfigManager。

use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::warn;

use crate::utils::locale::get_default_app_language;

/// 宿主支持的语言列表。
pub const SUPPORTED_LANGUAGES: &[&str] = &["zh-Hans", "zh-Hant", "en"];

/// 插件语言包单文件大小上限（64 KiB），防止异常插件拖垮下发链路。
const PLUGIN_CATALOG_MAX_BYTES: u64 = 64 * 1024;

/// 后端翻译服务。
pub struct I18nManager {
    /// 内置 catalog：语言码 → 扁平 key → 译文
    builtin: HashMap<String, HashMap<String, String>>,
    /// 插件 catalog：plugin_id → 语言码 → 原始嵌套 JSON（保留结构，下发时统一加前缀）
    plugins: RwLock<HashMap<String, HashMap<String, Value>>>,
    /// 当前界面语言（如 "zh-Hans"）
    current: RwLock<String>,
}

impl I18nManager {
    /// 加载内置语言包并创建管理器。
    /// 参数：locales_dir - 资源目录下的 locales 目录（含 `<lang>.json` 三份）。
    /// 返回：管理器实例；单份语言包缺失/损坏仅告警，不影响启动。
    pub fn load(locales_dir: PathBuf) -> Arc<Self> {
        let mut builtin = HashMap::new();
        for lang in SUPPORTED_LANGUAGES {
            let path = locales_dir.join(format!("{lang}.json"));
            let mut flat: HashMap<String, String> = HashMap::new();
            match std::fs::read_to_string(&path) {
                Ok(text) => match serde_json::from_str::<Value>(&text) {
                    Ok(Value::Object(map)) => flatten_json(&Value::Object(map), "", &mut flat),
                    Ok(_) => warn!("语言包 {} 顶层不是对象，已跳过", lang),
                    Err(e) => warn!("语言包 {} 解析失败: {}", lang, e),
                },
                Err(e) => warn!("语言包 {} 读取失败（{}），t() 将回退到 key 原文", lang, e),
            }
            builtin.insert(lang.to_string(), flat);
        }
        Arc::new(Self {
            builtin,
            plugins: RwLock::new(HashMap::new()),
            current: RwLock::new(get_default_app_language()),
        })
    }

    /// 按语言解析 key：当前语言 → en → 原样返回 key。
    /// 参数：lang - 语言码；key - 翻译键（如 "tray.showSettings"）。
    /// 返回：译文；未命中时依次回退 en 与 key 原文。
    pub fn t(&self, lang: &str, key: &str) -> String {
        if let Some(catalog) = self.builtin.get(lang) {
            if let Some(value) = catalog.get(key) {
                return value.clone();
            }
        }
        if lang != "en" {
            if let Some(catalog) = self.builtin.get("en") {
                if let Some(value) = catalog.get(key) {
                    return value.clone();
                }
            }
        }
        key.to_string()
    }

    /// 当前界面语言。
    pub fn current_language(&self) -> String {
        self.current.read().clone()
    }

    /// 更新当前界面语言（bootstrap 在配置加载与语言变更时调用）。
    /// 非法值忽略并告警，保持原语言不变。
    pub fn set_language(&self, lang: &str) {
        if SUPPORTED_LANGUAGES.contains(&lang) {
            *self.current.write() = lang.to_string();
        } else {
            warn!("非法语言值 '{}'，忽略语言更新", lang);
        }
    }

    /// 注册插件翻译目录（`<plugin_dir>/i18n/<lang>.json`）。
    /// 参数：plugin_id - 插件 id（下发时作为 `plugin.<id>.` 前缀）；dir - 插件目录。
    /// 未发现 i18n/ 目录时静默跳过（旧插件无翻译资源）。
    pub fn register_plugin_catalog(&self, plugin_id: &str, dir: &Path) {
        let i18n_dir = dir.join("i18n");
        if !i18n_dir.is_dir() {
            return;
        }
        let entries = match std::fs::read_dir(&i18n_dir) {
            Ok(entries) => entries,
            Err(e) => {
                warn!("读取插件 {} 的 i18n 目录失败: {}", plugin_id, e);
                return;
            }
        };
        let mut catalogs = HashMap::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(lang) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            if !SUPPORTED_LANGUAGES.contains(&lang.as_str()) {
                warn!(
                    "插件 {} 的语言包 '{}' 不在支持列表，已跳过",
                    plugin_id, lang
                );
                continue;
            }
            if entry
                .metadata()
                .is_ok_and(|m| m.len() > PLUGIN_CATALOG_MAX_BYTES)
            {
                warn!(
                    "插件 {} 的语言包 '{}' 超过 {} 字节限制，已跳过",
                    plugin_id, lang, PLUGIN_CATALOG_MAX_BYTES
                );
                continue;
            }
            let parsed = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|s| serde_json::from_str::<Value>(&s).map_err(|e| e.to_string()));
            match parsed {
                Ok(value @ Value::Object(_)) if validate_plugin_catalog(&value) => {
                    catalogs.insert(lang, value);
                }
                Ok(_) => warn!(
                    "插件 {} 的语言包 '{}' 结构非法（仅允许对象与字符串），已跳过",
                    plugin_id, lang
                ),
                Err(e) => warn!("插件 {} 的语言包 '{}' 解析失败: {}", plugin_id, lang, e),
            }
        }
        if catalogs.is_empty() {
            return;
        }
        self.plugins.write().insert(plugin_id.to_string(), catalogs);
    }

    /// 移除插件翻译目录（插件卸载时调用）。
    pub fn unregister_plugin_catalog(&self, plugin_id: &str) {
        self.plugins.write().remove(plugin_id);
    }

    /// 合并某语言下所有插件 catalog：返回 `{"plugin": {"<id 按点嵌套>": {…}}}`。
    ///
    /// 插件 id 为反向域名（含 `.`），而 vue-i18n 以 `.` 为路径分隔符，
    /// 故将 id 按点拆为嵌套对象（如 `com.example.hello-world` →
    /// `{"com": {"example": {"hello-world": …}}}`），共享前缀的插件深合并；
    /// 顶层 `plugin` 命名空间与内置命名空间不冲突。
    pub fn plugin_catalog_for(&self, lang: &str) -> Value {
        let plugins = self.plugins.read();
        let mut root = serde_json::Map::new();
        for (plugin_id, catalogs) in plugins.iter() {
            if let Some(catalog) = catalogs.get(lang) {
                let segments: Vec<&str> = plugin_id.split('.').collect();
                let mut current = &mut root;
                for seg in &segments[..segments.len() - 1] {
                    current = current
                        .entry((*seg).to_string())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()))
                        .as_object_mut()
                        .expect("插件 id 段嵌套必须是对象");
                }
                let last = segments.last().expect("插件 id 非空").to_string();
                // 末段与中间段同规则合并：短 id（如 com.example）的目录对象
                // 可能已被长 id（com.example.hello-world）作为中间节点占用，
                // 此时需键级合并而非覆盖，保证结果与处理顺序无关。
                match current.get_mut(&last) {
                    Some(Value::Object(existing)) => {
                        if let Value::Object(incoming) = catalog {
                            for (k, v) in incoming {
                                existing.insert(k.clone(), v.clone());
                            }
                        }
                    }
                    _ => {
                        current.insert(last, catalog.clone());
                    }
                }
            }
        }
        serde_json::json!({ "plugin": root })
    }
}

/// 递归展平嵌套 JSON 对象为 `.` 连接 key 的字符串 map；非字符串叶子跳过。
fn flatten_json(value: &Value, prefix: &str, out: &mut HashMap<String, String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_json(v, &key, out);
            }
        }
        Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        _ => {}
    }
}

/// 校验插件语言包结构：仅允许嵌套对象与字符串叶子。
fn validate_plugin_catalog(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.values().all(validate_plugin_catalog),
        Value::String(_) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造不读盘的管理器（测试直接注入插件目录）。
    fn test_manager() -> Arc<I18nManager> {
        Arc::new(I18nManager {
            builtin: HashMap::new(),
            plugins: RwLock::new(HashMap::new()),
            current: RwLock::new("zh-Hans".to_string()),
        })
    }

    /// 在临时目录写入插件语言包并注册。
    fn register(mgr: &I18nManager, id: &str, zh: &str) {
        let dir = std::env::temp_dir().join(format!("zl-i18n-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(id).join("i18n")).unwrap();
        std::fs::write(
            dir.join(id).join("i18n/zh-Hans.json"),
            format!("{{\"greeting\": \"{zh}\"}}"),
        )
        .unwrap();
        mgr.register_plugin_catalog(id, &dir.join(id));
    }

    /// 共享前缀插件（com.example 与 com.example.hello-world）的合并
    /// 必须与注册顺序无关，两个插件的 key 都可达。
    #[test]
    fn plugin_catalog_prefix_merge_is_order_independent() {
        for ids in [
            ["com.example", "com.example.hello-world"],
            ["com.example.hello-world", "com.example"],
        ] {
            let mgr = test_manager();
            // 值绑定插件 id（而非注册顺序），断言期望随 id 固定
            let short_zh = if ids[0] == "com.example" { "A" } else { "B" };
            let long_zh = if ids[0] == "com.example.hello-world" {
                "A"
            } else {
                "B"
            };
            register(&mgr, "com.example", short_zh);
            register(&mgr, "com.example.hello-world", long_zh);
            let catalog = mgr.plugin_catalog_for("zh-Hans");
            let plugin = &catalog["plugin"];
            assert_eq!(plugin["com"]["example"]["greeting"], short_zh);
            assert_eq!(plugin["com"]["example"]["hello-world"]["greeting"], long_zh);
            std::fs::remove_dir_all(
                std::env::temp_dir().join(format!("zl-i18n-test-{}", std::process::id())),
            )
            .ok();
        }
    }

    /// 无 i18n/ 目录的插件静默跳过，不产生 catalog 条目。
    #[test]
    fn plugin_without_i18n_dir_is_skipped() {
        let mgr = test_manager();
        let dir = std::env::temp_dir().join(format!("zl-i18n-test-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        mgr.register_plugin_catalog("com.example.plain", &dir);
        assert!(mgr.plugin_catalog_for("zh-Hans")["plugin"]
            .as_object()
            .unwrap()
            .is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
