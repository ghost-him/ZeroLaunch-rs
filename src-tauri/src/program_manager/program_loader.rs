/// 这个类用于加载电脑上程序，通过扫描路径或使用系统调用接口
///
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use super::{
    config::{ProgramLauncherConfig, ProgramLoaderConfig},
    Program,
};
use std::sync::Arc;

pub struct ProgramLoader {
    /// 要扫描的路径
    target_path: Vec<String>,
    /// 不扫描的路径
    forbidden_path: Vec<String>,
    /// 禁止的程序关键字（当程序的名字中有与其完全一致的子字符串时，不注册）
    forbidden_program_key: Vec<String>,
    /// 设置程序的固定权重偏移（当程序的名字中有与其完全一致的子字符串时，才会添加）
    program_bias: HashMap<String, f64>,
}

impl ProgramLoader {
    /// 创建
    pub fn new() -> Self {
        ProgramLoader {
            target_path: Vec::new(),
            forbidden_path: Vec::new(),
            forbidden_program_key: Vec::new(),
            program_bias: HashMap::new(),
        }
    }
    /// 使用配置文件初始化
    pub fn load_from_config(&self, config: &ProgramLoaderConfig) {}
    /// 添加目标路径
    pub fn add_target_path(&mut self, path: String) {
        self.target_path.push(path);
    }
    /// 添加不扫描的路径
    pub fn add_forbidden_path(&mut self, path: String) {
        self.forbidden_path.push(path);
    }
    /// 添加禁止的程序关键字
    pub fn add_forbidden_program_key(&mut self, key: String) {
        self.forbidden_program_key.insert(key);
    }
    /// 设置程序的固定权重偏移
    pub fn add_program_bias(&mut self, key: String, value: f64) {
        self.program_bias.insert(key, value);
    }
    /// 获取所有的程序
    pub fn load_program(&self) -> Vec<Arc<Program>> {
        Vec::new()
    }

    /// 判断是不是一个有效的路径
    /// 1. 路径本身有效
    /// 2. 没有被屏蔽
    fn is_valid_path(&self, path: &Path) -> bool {
        if !path.exists() {
            return false;
        }

        for str in &self.forbidden_path {
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
    pub fn recursive_visit_dir(&self, dir: &Path, depth: usize) -> Vec<String> {
        if depth == 0 {
            return Vec::new();
        }

        if !self.is_valid_path(&dir) {
            return Vec::new();
        }

        let mut result: Vec<String> = Vec::new();
        if dir.is_dir() {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    let sub_result = self.recursive_visit_dir(path.as_path(), depth - 1);
                    result.extend(sub_result);
                } else {
                    if self.is_target_file(path.as_path()) {
                        result.push(path.to_str().unwrap().to_string());
                    }
                }
            }
        } else {
            if self.is_valid_path(dir) {
                result.push(dir.to_str().unwrap().to_string());
            }
        }
        result
    }
}
