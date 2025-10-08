use crate::core::storage::windows_utils::{get_desktop_path, get_start_menu_paths};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialProgramLoaderConfig {
    pub target_paths: Option<Vec<DirectoryConfig>>,
    pub program_bias: Option<HashMap<String, (f64, String)>>,
    pub is_scan_uwp_programs: Option<bool>,
    pub index_web_pages: Option<Vec<(String, String)>>,
    pub custom_command: Option<Vec<(String, String)>>,
    pub forbidden_paths: Option<Vec<String>>,
    pub program_alias: Option<HashMap<String, Vec<String>>>,
    pub semantic_descriptions: Option<HashMap<String, String>>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DirectoryConfig {
    /// 当前的根目录
    pub root_path: String,
    /// 当前这个文件夹遍历的深度
    pub max_depth: u32,
    /// 当前这个文件夹要索引的文件类型
    pub pattern: Vec<String>,
    /// 使用的索引表达方式：是文件通配符表示还是使用正则表示(Wildcard, Regex)
    pub pattern_type: String,
    /// 要禁止的程序关键字
    pub excluded_keywords: Vec<String>,
}

impl DirectoryConfig {
    pub fn new(target_path: String, depth: u32) -> DirectoryConfig {
        DirectoryConfig {
            root_path: target_path,
            max_depth: depth,
            pattern: vec![
                "*.url".to_string(),
                "*.exe".to_string(),
                "*.lnk".to_string(),
            ],
            pattern_type: "Wildcard".to_string(),
            excluded_keywords: vec![
                "帮助".to_string(),
                "help".to_string(),
                "uninstall".to_string(),
                "卸载".to_string(),
                "zerolaunch-rs".to_string(),
            ],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ProgramLoaderConfigInner {
    /// 要索引的文件夹(目标路径)
    #[serde(default = "ProgramLoaderConfigInner::default_target_paths")]
    pub target_paths: Vec<DirectoryConfig>,
    /// 设置程序的固定权重偏移 (key) => (bias, note)
    #[serde(default = "ProgramLoaderConfigInner::default_program_bias")]
    pub program_bias: HashMap<String, (f64, String)>,
    /// 是不是要遍历uwp应用
    #[serde(default = "ProgramLoaderConfigInner::default_is_scan_uwp_programs")]
    pub is_scan_uwp_programs: bool,
    /// 索引的网页(关键字，网址)
    #[serde(default = "ProgramLoaderConfigInner::default_index_web_pages")]
    pub index_web_pages: Vec<(String, String)>,
    /// 自定义添加的命令(关键字，命令)
    pub custom_command: Vec<(String, String)>,
    /// 禁止的地址
    #[serde(default = "ProgramLoaderConfigInner::default_forbidden_paths")]
    pub forbidden_paths: Vec<String>,
    /// 给程序的别名，将程序的地址(LaunchMethod)当成key (key)=>([alias])
    #[serde(default = "ProgramLoaderConfigInner::default_program_alias")]
    pub program_alias: HashMap<String, Vec<String>>,
    /// 程序的语义性描述信息 (launch_method) => (description)
    #[serde(default = "ProgramLoaderConfigInner::default_semantic_descriptions")]
    pub semantic_descriptions: HashMap<String, String>,
}

impl Default for ProgramLoaderConfigInner {
    fn default() -> Self {
        Self {
            target_paths: Self::default_target_paths(),
            program_bias: Self::default_program_bias(),
            is_scan_uwp_programs: Self::default_is_scan_uwp_programs(),
            index_web_pages: Self::default_index_web_pages(),
            custom_command: Self::default_custom_command(),
            forbidden_paths: Self::default_forbidden_paths(),
            program_alias: Self::default_program_alias(),
            semantic_descriptions: Self::default_semantic_descriptions(),
        }
    }
}

impl ProgramLoaderConfigInner {
    pub(crate) fn default_program_alias() -> HashMap<String, Vec<String>> {
        HashMap::new()
    }

    pub(crate) fn default_target_paths() -> Vec<DirectoryConfig> {
        let (common, user) =
            get_start_menu_paths().unwrap_or_else(|_| (String::new(), String::new()));
        let desktop_path = get_desktop_path().unwrap_or_else(|_| String::new());
        vec![
            DirectoryConfig::new(common, 5),
            DirectoryConfig::new(user, 5),
            DirectoryConfig::new(desktop_path, 3),
        ]
    }

    pub(crate) fn default_forbidden_paths() -> Vec<String> {
        Vec::new()
    }

    pub(crate) fn default_program_bias() -> HashMap<String, (f64, String)> {
        HashMap::new()
    }

    pub(crate) fn default_is_scan_uwp_programs() -> bool {
        true
    }

    pub(crate) fn default_index_web_pages() -> Vec<(String, String)> {
        vec![
            // 预置 Bing 搜索，占位符 {} 会在运行时被替换为用户输入
            ("bing 搜索".to_string(), "https://www.bing.com/search?q={}".to_string()),
        ]
    }

    pub(crate) fn default_custom_command() -> Vec<(String, String)> {
        vec![
            // 常用系统命令预置
            ("关机".to_string(), "shutdown /s /t 0".to_string()),
            ("重启".to_string(), "shutdown /r /t 0".to_string()),
            ("锁屏".to_string(), "rundll32.exe user32.dll,LockWorkStation".to_string()),
        ]
    }

    pub(crate) fn default_semantic_descriptions() -> HashMap<String, String> {
        HashMap::new()
    }
}

impl ProgramLoaderConfigInner {
    pub fn to_partial(&self) -> PartialProgramLoaderConfig {
        PartialProgramLoaderConfig {
            target_paths: Some(self.target_paths.clone()),
            program_bias: Some(self.program_bias.clone()),
            is_scan_uwp_programs: Some(self.is_scan_uwp_programs),
            index_web_pages: Some(self.index_web_pages.clone()),
            custom_command: Some(self.custom_command.clone()),
            forbidden_paths: Some(self.forbidden_paths.clone()),
            program_alias: Some(self.program_alias.clone()),
            semantic_descriptions: Some(self.semantic_descriptions.clone()),
        }
    }

    pub fn update(&mut self, partial_config: PartialProgramLoaderConfig) {
        if let Some(partial_target_paths) = partial_config.target_paths {
            self.target_paths = partial_target_paths;
        }
        if let Some(partial_program_bias) = partial_config.program_bias {
            self.program_bias = partial_program_bias;
        }
        if let Some(partial_is_scan_uwp_programs) = partial_config.is_scan_uwp_programs {
            self.is_scan_uwp_programs = partial_is_scan_uwp_programs;
        }
        if let Some(partial_index_web_pages) = partial_config.index_web_pages {
            self.index_web_pages = partial_index_web_pages;
        }
        if let Some(partial_custom_command) = partial_config.custom_command {
            self.custom_command = partial_custom_command;
        }
        if let Some(partial_forbidden_paths) = partial_config.forbidden_paths {
            self.forbidden_paths = partial_forbidden_paths;
        }
        if let Some(partial_program_alias) = partial_config.program_alias {
            self.program_alias = partial_program_alias;
        }
        if let Some(partial_semantic_descriptions) = partial_config.semantic_descriptions {
            self.semantic_descriptions = partial_semantic_descriptions;
        }
    }
}
#[derive(Debug)]
pub struct ProgramLoaderConfig {
    inner: RwLock<ProgramLoaderConfigInner>,
}

impl Default for ProgramLoaderConfig {
    fn default() -> Self {
        ProgramLoaderConfig {
            inner: RwLock::new(ProgramLoaderConfigInner::default()),
        }
    }
}

impl ProgramLoaderConfig {
    pub fn to_partial(&self) -> PartialProgramLoaderConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }

    pub fn get_target_paths(&self) -> Vec<DirectoryConfig> {
        self.inner.read().target_paths.clone()
    }

    pub fn get_program_bias(&self) -> HashMap<String, (f64, String)> {
        self.inner.read().program_bias.clone()
    }

    pub fn get_is_scan_uwp_programs(&self) -> bool {
        self.inner.read().is_scan_uwp_programs
    }

    pub fn get_index_web_pages(&self) -> Vec<(String, String)> {
        self.inner.read().index_web_pages.clone()
    }

    pub fn get_custom_command(&self) -> Vec<(String, String)> {
        self.inner.read().custom_command.clone()
    }

    pub fn update(&self, partial_config: PartialProgramLoaderConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }
    pub fn get_forbidden_paths(&self) -> Vec<String> {
        self.inner.read().forbidden_paths.clone()
    }
    pub fn get_program_alias(&self) -> HashMap<String, Vec<String>> {
        self.inner.read().program_alias.clone()
    }
    pub fn get_semantic_descriptions(&self) -> HashMap<String, String> {
        self.inner.read().semantic_descriptions.clone()
    }
}
