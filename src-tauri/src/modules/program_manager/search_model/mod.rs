pub mod ai_fuzzy_search_model;
pub mod launchy_search_model;
pub mod skim_search_model;
pub mod standard_search_model;
use crate::program_manager::search_model::launchy_search_model::LaunchyScorer;
use crate::program_manager::search_model::skim_search_model::SkimScorer;
use crate::program_manager::search_model::standard_search_model::StandardScorer;
use crate::program_manager::Program;
use core::f64;
use serde::{Deserialize, Serialize};
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

/// 搜索模型的配置信息（用于序列化/反序列化）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SearchModelConfig {
    #[serde(rename = "skim")]
    Skim,
    #[serde(rename = "standard")]
    #[default]
    Standard,
    #[serde(rename = "launchy")]
    Launchy,
    #[serde(rename = "semantic")]
    Semantic,
}

impl SearchModelConfig {
    /// 判断当前配置是否属于传统搜索模型
    pub fn is_traditional_search(&self) -> bool {
        matches!(
            self,
            SearchModelConfig::Skim | SearchModelConfig::Standard | SearchModelConfig::Launchy
        )
    }
}

/// 搜索模型工厂
pub struct SearchModelFactory;
impl SearchModelFactory {
    /// 根据配置创建具体的 Scorer 实例
    pub fn create_scorer(config: Arc<SearchModelConfig>) -> SearchModel {
        let scorer: Arc<dyn Scorer> = match config.as_ref() {
            SearchModelConfig::Skim => Arc::new(SkimScorer::new()),
            SearchModelConfig::Standard => Arc::new(StandardScorer::new()),
            SearchModelConfig::Launchy => Arc::new(LaunchyScorer::new()),
            // SearchModelConfig::Semantic => {
            //     let semantic_manager = semantic_manager
            //         .expect_programming("semantic搜索模型需要语义管理器");
            //     Arc::new(SemanticScorer::new(semantic_manager))
            // }
            _ => {
                panic!("当前代码不应该执行到这里！！！");
            }
        };

    SearchModel::new(scorer)
    }
}

pub struct SearchModel {
    scorer: Arc<dyn Scorer>,
}

impl SearchModel {
    /// 创建一个新的 SearchModel 实例
    pub fn new(scorer: Arc<dyn Scorer>) -> Self {
        SearchModel { scorer }
    }
}

impl Default for SearchModel {
    fn default() -> Self {
        SearchModel::new(Arc::new(StandardScorer::new()))
    }
}

impl Scorer for SearchModel {
    fn calculate_score(&self, program: &Arc<Program>, user_input: &str) -> f64 {
        self.scorer.calculate_score(program, user_input)
    }
}

impl std::fmt::Debug for SearchModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchModel")
            .field("scorer", &"Arc<dyn Scorer>")
            .finish()
    }
}

// //////////////////////////////////////////
//
// 以下是用来预处理的函数
//
// //////////////////////////////////////////

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
