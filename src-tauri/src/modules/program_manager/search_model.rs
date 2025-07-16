use super::Program;
use core::f64;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
/// SearchModel 表示一个综合的搜索模型
///
/// Preprocessor 表示一个预处理函数，会在加载程序，和预处理用户输入时使用。
///
/// SearchAlgorithm 定义了搜索算法所需具备的核心功能和行为
///
/// ScoreAdjuster 代表一个函数 y = f(x)，通常用于调整权重或将一个值映射到另一个域
use std::{fmt::Debug, sync::Arc};

/// 实现一个评分策略
pub trait Scorer: Send + Sync + std::fmt::Debug {
    /// 计算给定程序和用户输入的匹配分数。
    ///
    /// # Arguments
    /// * `program` - 需要评分的程序。
    /// * `user_input` - 用户输入的搜索字符串。
    ///
    /// # Returns
    /// * 一个 f64 类型的分数，分数越高表示匹配度越高。
    fn calculate_score(&self, program: &Arc<Program>, user_input: &str) -> f64;
}

////////////////////////////////////////
///
/// 这些是用第三方库提供的匹配算法，所以就不单开一个文件了
///
////////////////////////////////////////



pub struct SkimScorer {
    matcher: Box<dyn FuzzyMatcher>,
}

impl Scorer for SkimScorer {
    fn calculate_score(&self, program: &Arc<Program>, user_input: &str) -> f64 {
        let mut ret: f64 = -10000.0;
        for name in &program.search_keywords {
            if name.chars().count() < user_input.chars().count() {
                continue;
            }
            let score = self.matcher.fuzzy_match(name, user_input);
            if let Some(s) = score {
                ret = f64::max(ret, s as f64);
            }
        }
        ret
    }
}

impl Debug for SkimScorer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkimScorer").finish()
    }
}

impl SkimScorer {
    pub fn new() -> Self {
        SkimScorer {
            matcher: Box::new(SkimMatcherV2::default()),
        }
    }
}

/////////////////////////////////////////////
///
/// 以下是用来预处理的函数
///
/////////////////////////////////////////////


/// 去除一个程序名中的版本号
pub fn remove_version_number(input_text: &str) -> String {
    let mut ret = String::new();
    let mut s = 0;
    let mut in_version = false;
    let chars: Vec<char> = input_text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        // 处理括号
        if ch == '(' {
            s += 1;
            in_version = true; // 假设版本信息在 '(' 之后开始
        } else if ch == ')' {
            if s > 0 {
                s -= 1;
            }
            in_version = false; // 假设版本信息在 ')' 结束
        } else if s == 0 && !in_version {
            // 检查当前字符是否是版本号的一部分
            if ch.is_ascii_digit() || ch == '.' {
                // 检查前一个字符是否是空格（以正确识别版本号）
                if i > 0 && chars[i - 1] == ' ' {
                    // 跳过整个版本号
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                        i += 1;
                    }
                    // 跳过任何后续的空格
                    while i < chars.len() && chars[i] == ' ' {
                        i += 1;
                    }
                    // 减少 i 以抵消下一次迭代的增加
                    i = i.saturating_sub(1);
                    i += 1;
                    continue;
                }
            }
            ret.push(ch);
        } else {
            // 如果在版本信息中或括号内，不做任何处理
        }

        i += 1;
    }

    // 去除结尾的空格
    while ret.ends_with(' ') {
        ret.pop();
    }

    ret
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
