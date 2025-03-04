use crate::modules::storage::windows_utils::get_start_menu_paths;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialProgramLoaderConfig {
    pub target_paths: Option<Vec<(String, u32)>>,
    pub forbidden_paths: Option<Vec<String>>,
    pub forbidden_program_key: Option<Vec<String>>,
    pub program_bias: Option<HashMap<String, (f64, String)>>,
    pub is_scan_uwp_programs: Option<bool>,
    pub index_file_paths: Option<Vec<String>>,
    pub index_web_pages: Option<Vec<(String, String)>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ProgramLoaderConfigInner {
    /// 保存的要启动的地址(目标路径，递归的深度)
    #[serde(default = "ProgramLoaderConfigInner::default_target_paths")]
    pub target_paths: Vec<(String, u32)>,
    /// 禁止的地址
    #[serde(default = "ProgramLoaderConfigInner::default_forbidden_paths")]
    pub forbidden_paths: Vec<String>,
    /// 禁止的程序关键字
    #[serde(default = "ProgramLoaderConfigInner::default_forbidden_program_key")]
    pub forbidden_program_key: Vec<String>,
    /// 设置程序的固定权重偏移 (key) => (bias, note)
    #[serde(default = "ProgramLoaderConfigInner::default_program_bias")]
    pub program_bias: HashMap<String, (f64, String)>,
    /// 是不是要遍历uwp应用
    #[serde(default = "ProgramLoaderConfigInner::default_is_scan_uwp_programs")]
    pub is_scan_uwp_programs: bool,
    /// 索引的单体文件（路径）
    #[serde(default = "ProgramLoaderConfigInner::default_index_file_paths")]
    pub index_file_paths: Vec<String>,
    /// 索引的网页(关键字，网址)
    #[serde(default = "ProgramLoaderConfigInner::default_index_web_pages")]
    pub index_web_pages: Vec<(String, String)>,
}

impl Default for ProgramLoaderConfigInner {
    fn default() -> Self {
        Self {
            target_paths: Self::default_target_paths(),
            forbidden_paths: Self::default_forbidden_paths(),
            forbidden_program_key: Self::default_forbidden_program_key(),
            program_bias: Self::default_program_bias(),
            is_scan_uwp_programs: Self::default_is_scan_uwp_programs(),
            index_file_paths: Self::default_index_file_paths(),
            index_web_pages: Self::default_index_web_pages(),
        }
    }
}

impl ProgramLoaderConfigInner {
    pub(crate) fn default_target_paths() -> Vec<(String, u32)> {
        let (common, user) =
            get_start_menu_paths().unwrap_or_else(|_| (String::new(), String::new()));
        vec![(common, 5), (user, 5)]
    }

    pub(crate) fn default_forbidden_paths() -> Vec<String> {
        Vec::new()
    }

    pub(crate) fn default_forbidden_program_key() -> Vec<String> {
        vec![
            "帮助".to_string(),
            "help".to_string(),
            "uninstall".to_string(),
            "卸载".to_string(),
            "zerolaunch-rs".to_string(),
        ]
    }

    pub(crate) fn default_program_bias() -> HashMap<String, (f64, String)> {
        HashMap::new()
    }

    pub(crate) fn default_is_scan_uwp_programs() -> bool {
        true
    }

    pub(crate) fn default_index_file_paths() -> Vec<String> {
        Vec::new()
    }

    pub(crate) fn default_index_web_pages() -> Vec<(String, String)> {
        Vec::new()
    }
}

impl ProgramLoaderConfigInner {
    pub fn to_partial(&self) -> PartialProgramLoaderConfig {
        PartialProgramLoaderConfig {
            target_paths: Some(self.target_paths.clone()),
            forbidden_paths: Some(self.forbidden_paths.clone()),
            forbidden_program_key: Some(self.forbidden_program_key.clone()),
            program_bias: Some(self.program_bias.clone()),
            is_scan_uwp_programs: Some(self.is_scan_uwp_programs),
            index_file_paths: Some(self.index_file_paths.clone()),
            index_web_pages: Some(self.index_web_pages.clone()),
        }
    }

    pub fn update(&mut self, partial_config: PartialProgramLoaderConfig) {
        if let Some(partial_target_paths) = partial_config.target_paths {
            self.target_paths = partial_target_paths;
        }
        if let Some(partial_forbidden_paths) = partial_config.forbidden_paths {
            self.forbidden_paths = partial_forbidden_paths;
        }
        if let Some(partial_forbidden_program_key) = partial_config.forbidden_program_key {
            self.forbidden_program_key = partial_forbidden_program_key;
        }
        if let Some(partial_program_bias) = partial_config.program_bias {
            self.program_bias = partial_program_bias;
        }
        if let Some(partial_is_scan_uwp_programs) = partial_config.is_scan_uwp_programs {
            self.is_scan_uwp_programs = partial_is_scan_uwp_programs;
        }
        if let Some(partial_index_file_paths) = partial_config.index_file_paths {
            self.index_file_paths = partial_index_file_paths;
        }
        if let Some(partial_index_web_pages) = partial_config.index_web_pages {
            self.index_web_pages = partial_index_web_pages;
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

    pub fn get_target_paths(&self) -> Vec<(String, u32)> {
        self.inner.read().target_paths.clone()
    }

    pub fn get_forbidden_paths(&self) -> Vec<String> {
        self.inner.read().forbidden_paths.clone()
    }

    pub fn get_forbidden_program_key(&self) -> Vec<String> {
        self.inner.read().forbidden_program_key.clone()
    }

    pub fn get_program_bias(&self) -> HashMap<String, (f64, String)> {
        self.inner.read().program_bias.clone()
    }

    pub fn get_is_scan_uwp_programs(&self) -> bool {
        self.inner.read().is_scan_uwp_programs
    }

    pub fn get_index_file_paths(&self) -> Vec<String> {
        self.inner.read().index_file_paths.clone()
    }

    pub fn get_index_web_pages(&self) -> Vec<(String, String)> {
        self.inner.read().index_web_pages.clone()
    }

    pub fn update(&self, partial_config: PartialProgramLoaderConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }
}
