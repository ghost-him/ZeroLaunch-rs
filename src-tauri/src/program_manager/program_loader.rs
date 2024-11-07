use super::pinyin_mapper::PinyinMapper;
use super::search_model::*;
use super::LaunchMethod;
/// 这个类用于加载电脑上程序，通过扫描路径或使用系统调用接口
///
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::Hash;
use std::io;
use std::path::Path;

use super::{
    config::{ProgramLauncherConfig, ProgramLoaderConfig},
    Program,
};
use std::sync::Arc;

struct GuidGenerator {
    next_id: u64,
}

impl GuidGenerator {
    pub fn new() -> Self {
        GuidGenerator { next_id: 1 }
    }
    pub fn get_guid(&mut self) -> u64 {
        let ret = self.next_id;
        self.next_id += 1;
        ret
    }
}

pub struct ProgramLoader {
    /// 要扫描的路径
    target_paths: Vec<String>,
    /// 不扫描的路径
    forbidden_paths: Vec<String>,
    /// 禁止的程序关键字（当程序的名字中有与其完全一致的子字符串时，不注册）
    forbidden_program_key: Vec<String>,
    /// 设置程序的固定权重偏移（当程序的名字中有与其完全一致的子字符串时，才会添加）
    program_bias: HashMap<String, f64>,
    /// guid生成器
    guid_generator: GuidGenerator,
    /// 判断一个程序有没有被添加
    program_name_hash: HashSet<String>,
    /// 拼音转换器
    pinyin_mapper: PinyinMapper,
}

impl ProgramLoader {
    /// 创建
    pub fn new() -> Self {
        ProgramLoader {
            target_paths: Vec::new(),
            forbidden_paths: Vec::new(),
            forbidden_program_key: Vec::new(),
            program_bias: HashMap::new(),
            guid_generator: GuidGenerator::new(),
            program_name_hash: HashSet::new(),
            pinyin_mapper: PinyinMapper::new(),
        }
    }

    pub fn save_to_config(&self) -> ProgramLoaderConfig {
        ProgramLoaderConfig {
            target_paths: self.target_paths.clone(),
            forbidden_paths: self.forbidden_paths.clone(),
            forbidden_program_key: self.forbidden_program_key.clone(),
            program_bias: self.program_bias.clone(),
        }
    }

    /// 使用配置文件初始化
    pub fn load_from_config(&mut self, config: &ProgramLoaderConfig) {
        self.target_paths = config.target_paths.clone();
        self.forbidden_paths = config.forbidden_paths.clone();
        self.forbidden_program_key = config.forbidden_program_key.clone();
        self.program_bias = config.program_bias.clone();
        self.guid_generator = GuidGenerator::new();
        self.program_name_hash = HashSet::new();
    }
    /// 添加目标路径
    pub fn add_target_path(&mut self, path: String) {
        self.target_paths.push(path);
    }
    /// 添加不扫描的路径
    pub fn add_forbidden_path(&mut self, path: String) {
        self.forbidden_paths.push(path);
    }
    /// 添加禁止的程序关键字
    pub fn add_forbidden_program_key(&mut self, key: String) {
        self.forbidden_program_key.push(key);
    }
    /// 设置程序的固定权重偏移
    pub fn add_program_bias(&mut self, key: &str, value: f64) {
        self.program_bias.insert(key.to_string(), value);
    }
    /// 获得程序的固定权重偏移
    pub fn get_program_bias(&self, key: &str) -> f64 {
        let mut result: f64 = 0.0;
        for item in &self.program_bias {
            if key.contains(item.0) {
                result += item.1;
            }
        }
        result
    }
    /// 获取所有的程序
    pub fn load_program(&mut self) -> Vec<Arc<Program>> {
        // todo完成程序的加载
        // 遍历所有的目标路径
        let mut program_path: Vec<String> = Vec::new();
        for path_str in &self.target_paths {
            let path = Path::new(&path_str);
            program_path.extend(self.recursive_visit_dir(path, 3).unwrap());
        }
        let mut result: Vec<Arc<Program>> = Vec::new();

        // 添加通过地址找到的文件
        for path_str in program_path {
            let path = Path::new(&path_str);

            let show_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)
                .unwrap_or_default();
            // 用于判断的名字
            let unique_name = show_name.to_lowercase();
            // 判断当前的程序有没有被添加过
            if self.is_exist(&unique_name) {
                // 已经存在
                continue;
            }

            let removed_version_name = remove_version_number(&show_name);
            // 经过过滤的名字
            let filtered_name = remove_repeated_space(&removed_version_name);

            // 以大写首字母开头的名字
            let uppercase_name = get_upper_case_latter(&filtered_name).to_lowercase();

            // 小写名字
            let lower_name = filtered_name.to_lowercase();

            // 无空格的名字
            let no_space_name = remove_string_space(&lower_name);

            // 拼音字母
            let pinyin_name = self.pinyin_mapper.convert(&lower_name);

            let guid = self.guid_generator.get_guid();

            let alias: Vec<String> = vec![
                filtered_name,
                uppercase_name,
                lower_name,
                no_space_name,
                pinyin_name,
            ];
            let stable_bias = self.get_program_bias(&unique_name);
            let program = Arc::new(Program {
                program_guid: guid,
                show_name: show_name,
                launch_method: LaunchMethod::Path(path_str),
                alias: alias,
                stable_bias: stable_bias,
            });
            println!("{:?}", program.as_ref());
            result.push(program);
        }
        // 添加通过uwp找到的文件
        result
    }

    /// 判断是不是一个有效的路径
    /// 1. 路径本身有效
    /// 2. 没有被屏蔽
    fn is_valid_path(&self, path: &Path) -> bool {
        if !path.exists() {
            return false;
        }

        for str in &self.forbidden_paths {
            let temp = Path::new(&str);
            // 如果当前的路径以禁止路径开头
            if path.starts_with(temp) {
                return false;
            }
        }
        true
    }

    /// 判断一个目标文件是不是想要的
    fn is_target_file(&self, path: &Path) -> bool {
        if !path.is_file() && !path.is_symlink() {
            return false;
        }

        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        if !["url", "exe", "lnk"].contains(&extension) {
            return false;
        }

        path.file_stem()
            .and_then(|name| name.to_str())
            .map(|name| {
                !self
                    .forbidden_program_key
                    .iter()
                    .any(|key| name.contains(key))
            })
            .unwrap_or(false)
    }

    /// 递归遍历一个文件夹
    /// 会自动跳过不可遍历的文件夹
    /// 返回文件夹中所有的文件
    pub fn recursive_visit_dir(&self, dir: &Path, depth: usize) -> io::Result<Vec<String>> {
        if depth == 0 || !self.is_valid_path(dir) {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();

        if dir.is_dir() {
            match fs::read_dir(dir) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(entry) => {
                                let path = entry.path();
                                if path.is_dir() {
                                    match self.recursive_visit_dir(&path, depth - 1) {
                                        Ok(sub_result) => result.extend(sub_result),
                                        Err(e) => eprintln!(
                                            "Error accessing directory {}: {}",
                                            path.display(),
                                            e
                                        ),
                                    }
                                } else if self.is_target_file(&path) {
                                    if let Some(path_str) = path.to_str() {
                                        result.push(path_str.to_string());
                                    }
                                }
                            }
                            Err(e) => eprintln!("Error reading directory entry: {}", e),
                        }
                    }
                }
                Err(e) => eprintln!("Error reading directory {}: {}", dir.display(), e),
            }
        } else if self.is_valid_path(dir) {
            if let Some(dir_str) = dir.to_str() {
                result.push(dir_str.to_string());
            }
        }

        Ok(result)
    }

    /// 判断一个程序有没有被添加
    fn is_exist(&mut self, program_name: &str) -> bool {
        if self.program_name_hash.contains(&program_name.to_string()) {
            return true;
        }
        self.program_name_hash.insert(program_name.to_string());
        return false;
    }
}
