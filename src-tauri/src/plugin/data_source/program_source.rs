use crate::modules::program_manager::localization_translation::parse_localized_names_from_dir;
use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::{
    ArrayItem, ArrayUiHint, DataSource, FieldDefinition, LaunchMethod, PrimitiveType,
    SearchCandidate,
};
use crate::plugin_system::{
    ComponentType, ConfigError, Configurable, SettingDefinition, SettingType,
};
use globset::GlobSetBuilder;
use regex::RegexSet;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, warn};
use walkdir::WalkDir;

#[derive(Debug, Clone, serde::Deserialize, Default)]
enum SymlinkMode {
    #[default]
    ExplicitOnly,
    Auto,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct DirectoryConfig {
    root_path: String,
    max_depth: u32,
    pattern: Vec<String>,
    pattern_type: String,
    excluded_keywords: Vec<String>,
    #[serde(default)]
    forbidden_paths: Vec<String>,
    #[serde(default)]
    symlink_mode: SymlinkMode,
}

struct PathChecker {
    glob: Option<globset::GlobSet>,
    regex: Option<RegexSet>,
    excluded_keys: Vec<String>,
    is_glob: bool,
}

impl PathChecker {
    fn new(
        patterns: &[String],
        pattern_type: &str,
        excluded_keys: &[String],
    ) -> Result<PathChecker, String> {
        let excluded_keys = excluded_keys
            .iter()
            .map(|item| item.to_lowercase())
            .collect();

        match pattern_type {
            "Wildcard" => {
                let mut builder = GlobSetBuilder::new();
                for pattern in patterns {
                    match globset::Glob::new(pattern) {
                        Ok(glob) => {
                            builder.add(glob);
                        }
                        Err(e) => {
                            warn!("添加通配符失败: {}", e);
                            return Err(format!("添加通配符失败：{:?}", e.to_string()));
                        }
                    }
                }

                match builder.build() {
                    Ok(globset) => Ok(PathChecker {
                        glob: Some(globset),
                        regex: None,
                        excluded_keys,
                        is_glob: true,
                    }),
                    Err(e) => {
                        warn!("编译通配符检查器失败: {}", e);
                        Err(format!("编译通配符检查器失败：{:?}", e.to_string()))
                    }
                }
            }
            "Regex" => match RegexSet::new(patterns) {
                Ok(regex) => Ok(PathChecker {
                    glob: None,
                    regex: Some(regex),
                    excluded_keys,
                    is_glob: false,
                }),
                Err(e) => {
                    warn!("编译正则表达式失败: {}", e);
                    Err(format!("编译正则表达式失败：{:?}", e.to_string()))
                }
            },
            _ => Err(format!("无当前该匹配项：{}", pattern_type)),
        }
    }

    fn is_match(&self, path: &str) -> bool {
        let path = path.to_lowercase();
        if self.excluded_keys.iter().any(|item| path.contains(item)) {
            return false;
        }

        if self.is_glob {
            if let Some(ref glob_set) = self.glob {
                return glob_set.is_match(path);
            }
        } else if let Some(ref regex_set) = self.regex {
            return regex_set.is_match(&path);
        }
        false
    }
}

pub struct ProgramSource {
    settings: serde_json::Value,
}

impl Default for ProgramSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgramSource {
    pub fn new() -> Self {
        ProgramSource {
            settings: serde_json::Value::Null,
        }
    }

    fn contains_excluded_keywords(&self, file_name: &str, excluded_keywords: &[String]) -> bool {
        let file_name_lower = file_name.to_lowercase();
        excluded_keywords.iter().any(|keyword| {
            if keyword.is_empty() {
                return false;
            }
            file_name_lower.contains(&keyword.to_lowercase())
        })
    }

    fn is_valid_path(&self, path: &Path, forbidden_paths: &[String]) -> bool {
        if !path.exists() {
            return false;
        }

        for forbidden in forbidden_paths {
            if forbidden.is_empty() {
                continue;
            }
            let forbidden_path = Path::new(forbidden);
            if path.starts_with(forbidden_path) {
                return false;
            }
        }
        true
    }

    // 判断是否需要处理符号链接
    fn should_process_symlink(
        &self,
        path: &Path,
        file_name: &str,
        symlink_mode: &SymlinkMode,
    ) -> bool {
        let is_explicit_symlink = file_name.ends_with(".symlink");
        match symlink_mode {
            SymlinkMode::ExplicitOnly => is_explicit_symlink,
            SymlinkMode::Auto => {
                if is_explicit_symlink {
                    return true;
                }
                match std::fs::symlink_metadata(path) {
                    Ok(metadata) => metadata.is_symlink(),
                    Err(_) => false,
                }
            }
        }
    }

    fn is_target_file(
        &self,
        path: &Path,
        checker: &PathChecker,
        symlink_mode: &SymlinkMode,
    ) -> bool {
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name,
            None => {
                warn!("无法获取文件名: {:?}", path);
                return false;
            }
        };

        let is_symlink = self.should_process_symlink(path, file_name, symlink_mode);

        if is_symlink && matches!(symlink_mode, SymlinkMode::Auto) {
            return true;
        }

        if !checker.is_match(file_name) {
            return false;
        }

        is_symlink || path.is_file()
    }

    fn recursive_visit_dir(
        &self,
        dir: &Path,
        depth: usize,
        checker: &PathChecker,
        symlink_mode: &SymlinkMode,
        forbidden_paths: &[String],
    ) -> Vec<String> {
        if !self.is_valid_path(dir, forbidden_paths) {
            return Vec::new();
        }

        WalkDir::new(dir)
            .max_depth(depth)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| self.is_valid_path(e.path(), forbidden_paths))
            .filter_map(|entry_result| match entry_result {
                Ok(entry) => Some(entry),
                Err(e) => {
                    debug!("Error reading directory entry: {}", e);
                    None
                }
            })
            .filter(|entry| self.is_target_file(entry.path(), checker, symlink_mode))
            .map(|entry| entry.path().to_string_lossy().into_owned())
            .collect()
    }

    fn parse_directory_configs(&self) -> Vec<DirectoryConfig> {
        let directories = self.settings.get("directories").and_then(|v| v.as_array());

        match directories {
            Some(arr) => arr
                .iter()
                .filter_map(|config| serde_json::from_value(config.clone()).ok())
                .collect(),
            None => Vec::new(),
        }
    }
}

impl Configurable for ProgramSource {
    fn component_id(&self) -> &str {
        "program-source"
    }

    fn component_name(&self) -> &str {
        "路径程序数据源"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::DataSource
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![SettingDefinition {
            field: FieldDefinition {
                key: "directories".to_string(),
                label: "扫描目录".to_string(),
                description: "配置要扫描的程序目录".to_string(),
                setting_type: SettingType::Array {
                    item: ArrayItem::Object(vec![
                        FieldDefinition {
                            key: "root_path".to_string(),
                            label: "目录路径".to_string(),
                            description: "要扫描的根目录".to_string(),
                            setting_type: SettingType::Path {
                                mode: crate::plugin_system::types::PathMode::Directory,
                            },
                            default_value: serde_json::json!(""),
                            visible: true,
                            editable: true,
                        },
                        FieldDefinition {
                            key: "max_depth".to_string(),
                            label: "扫描深度".to_string(),
                            description: "子目录递归深度".to_string(),
                            setting_type: SettingType::Number {
                                min: Some(1.0),
                                max: Some(10.0),
                                step: Some(1.0),
                            },
                            default_value: serde_json::json!(3),
                            visible: true,
                            editable: true,
                        },
                        FieldDefinition {
                            key: "pattern".to_string(),
                            label: "文件模式".to_string(),
                            description: "要匹配的文件类型，如 *.exe, *.lnk".to_string(),
                            setting_type: SettingType::Array {
                                item: ArrayItem::Primitive(PrimitiveType::Text),
                                min_items: Some(1),
                                max_items: None,
                                ui_hint: ArrayUiHint::Tags,
                            },
                            default_value: serde_json::json!(["*.exe", "*.lnk", "*.url"]),
                            visible: true,
                            editable: true,
                        },
                        FieldDefinition {
                            key: "pattern_type".to_string(),
                            label: "匹配方式".to_string(),
                            description: "通配符或正则表达式".to_string(),
                            setting_type: SettingType::Select {
                                options: vec!["Wildcard".to_string(), "Regex".to_string()],
                            },
                            default_value: serde_json::json!("Wildcard"),
                            visible: true,
                            editable: true,
                        },
                        FieldDefinition {
                            key: "excluded_keywords".to_string(),
                            label: "排除关键字".to_string(),
                            description: "包含这些关键字的路径将被忽略".to_string(),
                            setting_type: SettingType::Array {
                                item: ArrayItem::Primitive(PrimitiveType::Text),
                                min_items: None,
                                max_items: None,
                                ui_hint: ArrayUiHint::Tags,
                            },
                            default_value: serde_json::json!(["uninstall", "帮助", "help", "卸载"]),
                            visible: true,
                            editable: true,
                        },
                        FieldDefinition {
                            key: "forbidden_paths".to_string(),
                            label: "禁止路径".to_string(),
                            description: "这些路径及其子路径将不会被扫描".to_string(),
                            setting_type: SettingType::Array {
                                item: ArrayItem::Primitive(PrimitiveType::Text),
                                min_items: None,
                                max_items: None,
                                ui_hint: ArrayUiHint::Tags,
                            },
                            default_value: serde_json::json!([]),
                            visible: true,
                            editable: true,
                        },
                        FieldDefinition {
                            key: "symlink_mode".to_string(),
                            label: "符号链接模式".to_string(),
                            description: "如何处理符号链接".to_string(),
                            setting_type: SettingType::Select {
                                options: vec!["ExplicitOnly".to_string(), "Auto".to_string()],
                            },
                            default_value: serde_json::json!("ExplicitOnly"),
                            visible: true,
                            editable: true,
                        },
                    ]),
                    min_items: Some(1),
                    max_items: None,
                    ui_hint: ArrayUiHint::MasterDetail,
                },
                default_value: serde_json::json!([
                    {
                        "root_path": "C:\\ProgramData\\Microsoft\\Windows\\Start Menu",
                        "max_depth": 5,
                        "pattern": ["*.exe", "*.lnk", "*.url"],
                        "pattern_type": "Wildcard",
                        "excluded_keywords": ["uninstall", "帮助", "help", "卸载", "zerolaunch-rs"],
                        "forbidden_paths": [],
                        "symlink_mode": "ExplicitOnly"
                    },
                    {
                        "root_path": "C:\\Users\\用户名\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu",
                        "max_depth": 5,
                        "pattern": ["*.exe", "*.lnk", "*.url"],
                        "pattern_type": "Wildcard",
                        "excluded_keywords": ["uninstall", "帮助", "help", "卸载", "zerolaunch-rs"],
                        "forbidden_paths": [],
                        "symlink_mode": "ExplicitOnly"
                    },
                    {
                        "root_path": "C:\\Users\\用户名\\Desktop",
                        "max_depth": 3,
                        "pattern": ["*.exe", "*.lnk", "*.url"],
                        "pattern_type": "Wildcard",
                        "excluded_keywords": ["uninstall", "帮助", "help", "卸载", "zerolaunch-rs"],
                        "forbidden_paths": [],
                        "symlink_mode": "ExplicitOnly"
                    }
                ]),
                visible: true,
                editable: true,
            },
            group: Some("目录扫描".to_string()),
            order: 1,
        }]
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.clone()
    }

    fn apply_settings(&mut self, settings: serde_json::Value) -> Result<(), ConfigError> {
        self.settings = settings;
        Ok(())
    }

    fn on_settings_changed(&self) {}
}

impl DataSource for ProgramSource {
    fn fetch_candidates(&self) -> CachedCandidateData {
        let mut result = CachedCandidateData::new();
        let directory_configs = self.parse_directory_configs();

        for directory in directory_configs {
            let checker = match PathChecker::new(
                &directory.pattern,
                &directory.pattern_type,
                &directory.excluded_keywords,
            ) {
                Ok(c) => c,
                Err(message) => {
                    warn!("创建路径检查器失败: {}", message);
                    continue;
                }
            };

            let paths = self.recursive_visit_dir(
                Path::new(&directory.root_path),
                directory.max_depth as usize,
                &checker,
                &directory.symlink_mode,
                &directory.forbidden_paths,
            );

            debug!(
                "成功扫描目录: {}, 找到 {} 个程序",
                directory.root_path,
                paths.len()
            );

            let mut grouped_paths: HashMap<std::path::PathBuf, Vec<std::path::PathBuf>> =
                HashMap::new();
            for path_str in paths {
                let path = std::path::PathBuf::from(path_str);
                if let Some(parent) = path.parent() {
                    grouped_paths
                        .entry(parent.to_path_buf())
                        .or_default()
                        .push(path);
                }
            }

            for (dir_path, files_in_dir) in grouped_paths {
                let localized_names = parse_localized_names_from_dir(&dir_path);

                for target_path_buf in files_in_dir {
                    let target_path = target_path_buf.as_path();

                    let file_name = target_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .map(String::from)
                        .unwrap_or_default();

                    if self.contains_excluded_keywords(&file_name, &directory.excluded_keywords) {
                        debug!("文件名包含屏蔽字，跳过: {:?}", target_path);
                        continue;
                    }

                    let actual_path = target_path.to_path_buf();
                    let actual_path_str = target_path.to_string_lossy().to_string();

                    // 从实际的路径中提取文件名与显示名
                    // 这里为什么把链接与真实的文件都放在一起处理？
                    // 因为链接的文件名通常更具有可读性，且用户更习惯于看到链接的名字而非实际文件的名字
                    // 而且很有可能这个链接是用户自己创建名字，用于当成别名来处理
                    let file_name_lower = actual_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_lowercase())
                        .unwrap_or_default();

                    let show_name = actual_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(String::from)
                        .unwrap_or_default();

                    let launch_method = if let Some(ext) = actual_path.extension() {
                        if let Some(ext_str) = ext.to_str() {
                            if ["url"].contains(&ext_str) {
                                LaunchMethod::Url(actual_path_str.clone())
                            } else if ["lnk", "exe"].contains(&ext_str) {
                                LaunchMethod::Path(actual_path_str.clone())
                            } else {
                                LaunchMethod::File(actual_path_str.clone())
                            }
                        } else {
                            LaunchMethod::File(actual_path_str.clone())
                        }
                    } else {
                        LaunchMethod::File(actual_path_str.clone())
                    };

                    let localized_name = localized_names.get(&file_name_lower).cloned();

                    let final_show_name = localized_name.unwrap_or(show_name);

                    let icon = actual_path_str.clone();

                    let candidate = SearchCandidate {
                        id: 0, // 这个值由 CachedCandidateData 负责分配
                        name: final_show_name,
                        icon,
                        launch_method,
                        keywords: Vec::new(), //  这个值的内容由 KeywordOptimizer 负责填充
                        bias: 0.0,
                    };

                    result.add_candidate(candidate);
                }
            }
        }

        result
    }
}
