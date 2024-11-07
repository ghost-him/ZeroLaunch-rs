use super::Program;
/// SearchModel 表示一个综合的搜索模型
///
/// Preprocessor 表示一个预处理函数，会在加载程序，和预处理用户输入时使用。
///
/// SearchAlgorithm 定义了搜索算法所需具备的核心功能和行为
///
/// ScoreAdjuster 代表一个函数 y = f(x)，通常用于调整权重或将一个值映射到另一个域
use std::sync::Arc;

/// 预处理一个函数
/// input: 要预处理的字符串
/// 返回值：经过预处理后的字符串
pub type PreprocessorFn = fn(&str) -> String;

/// 计算两个字符串之间的权重
/// source: 目标字符串
/// user_input: 用户输入的字符串
/// 返回：两个字符串的匹配值
pub type SearchAlgorithmFn = fn(&str, &str) -> f64;
/// 将一个值映射
/// x: 要映射的
/// 返回值：映射后的结果
pub type ScoreAdjusterFn = fn(f64) -> f64;

/// 表示一个综合的，集成多种子算法的搜索算法
/// source: 目标程序
/// user_input: 用户输入的字符串
pub type SearchModelFn = fn(Arc<Program>, &str) -> f64;

pub fn StandardSearchFn(program: Arc<Program>, user_input: &str) -> f64 {
    // todo: 完成这个实现，如果使用到了什么子算法，用上面的模块实现出来再完成这个就可以了
    // program中的字符串与user_input都已经是预处理过了，不再需要预处理了
    0.0
}

/// 去除一个程序名中的版本号
pub fn remove_version_number(input_text: &str) -> String {
    input_text
        .split_whitespace()
        .next()
        .unwrap_or(input_text)
        .to_string()
}

/// 去除一个字符串中多余的空格
pub fn remove_repeated_space(input_text: &str) -> String {
    let mut result = String::new();
    let mut is_space = false;

    for c in input_text.chars() {
        if c != ' ' {
            result.push(c);
            is_space = false;
        } else {
            if !is_space && !result.is_empty() {
                result.push(c);
            }
            is_space = true;
        }
    }

    // Remove trailing space if exists
    if result.ends_with(' ') {
        result.pop();
    }

    result
}

/// 将一个字符串中所有的空格都删去
pub fn remove_string_space(input_text: &str) -> String {
    input_text.chars().filter(|&c| c != ' ').collect()
}

/// 获取字符串中所有的大写ascii字母
/// "HelloWorld" => "HW", "PowerPoint" => "PP"
pub fn get_upper_case_latter(input_text: &str) -> String {
    let mut result = String::new();

    for c in input_text.chars() {
        if c.is_ascii_uppercase() {
            result.push(c);
        }
        if !c.is_ascii() {
            result.clear();
            break;
        }
    }

    result
}

/// 获取字符串中的首字母
pub fn get_first_letters(s: &str) -> String {
    s.split_whitespace()
        .filter_map(|word| word.chars().next())
        .collect()
}
