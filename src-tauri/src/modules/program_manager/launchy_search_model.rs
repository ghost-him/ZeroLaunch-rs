use std::fmt::Debug;
/// 这个文件是以LaunchyQT的搜索模型为基础进行的改造
/// 项目地址如下：https://github.com/samsonwang/LaunchyQt
/// 但是launchyqt是基于比较进行搜索的，而不是基于分数的
/// 所以我对这个搜索算法做了一些修改，从而可以适应当前的搜索框架

use std::sync::Arc;
use std::collections::HashMap;
use crate::program_manager::Program;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use crate::program_manager::Scorer;
/// `LaunchyScorer` 实现了模仿 LaunchyQT 搜索算法的评分策略。
///
/// 它将 Launchy 的多级比较规则（`CatLessPtr`）转化为一个数值分数，
/// 以便与现有的 Scorer-based 框架集成。
///
/// 评分优先级如下:
/// 1.  **精确匹配**: 获得最高的基础分。
/// 2.  **连续子串匹配**: 获得次高的基础分。
///     - 匹配位置越靠前，分数越高。
/// 3.  **子集匹配**: 如果用户输入的字符是程序名称的子集，则获得一个较低的基础分。
/// 4.  **名称长度惩罚**: 作为一个微小的调整项，名称越短，分数会略微高一点，用于打破平局。
///
/// 注意: Launchy 的 'usage' (使用频率) 动态权重部分没有在这里实现，
/// 因为框架已在外部处理了动态分数（`program_dynamic_value_based_launch_time`）。

pub struct LaunchyScorer {
}

impl Debug for LaunchyScorer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LaunchyScorer")
            .finish()
    }
}

impl LaunchyScorer {
    pub fn new() -> Self {
        LaunchyScorer {
        }
    }
}

impl Scorer for LaunchyScorer {
    fn calculate_score(&self, program: &Arc<Program>, user_input: &str) -> f64 {
        if user_input.is_empty() {
            // 如果没有输入，则不进行匹配，返回一个中性分数
            return 0.0;
        }

        let mut max_score = -1.0; // 使用一个负数作为未匹配的初始值

        for keyword in &program.search_keywords {
            let mut current_score = -1.0;

            // Step 1: 精确匹配 (最高优先级)
            if keyword.eq_ignore_ascii_case(user_input) {
                const EXACT_MATCH_BASE_SCORE: f64 = 100_000.0;
                current_score = EXACT_MATCH_BASE_SCORE;
            } else {
                // Step 2: 连续子串匹配 (次高优先级)
                if let Some(start_index) = keyword.to_lowercase().find(user_input) {
                    const CONTIGUOUS_MATCH_BASE_SCORE: f64 = 10_000.0;
                    // 匹配位置越靠前，分数越高。每个字符的偏移惩罚10分。
                    let position_penalty = (start_index as f64) * 10.0;
                    current_score = CONTIGUOUS_MATCH_BASE_SCORE - position_penalty;
                } else {
                    let mut compare_chars = HashMap::with_capacity(keyword.len());

                    // 统计 compare_name 中字符出现次数
                    for c in keyword.chars() {
                        *compare_chars.entry(c).or_insert(0) += 1;
                    }

                    // 计算匹配的字符数
                    let mut result = 0;
                    for c in user_input.chars() {
                        if let Some(count) = compare_chars.get_mut(&c) {
                            if *count > 0 {
                                result += 1;
                                *count -= 1;
                            }
                        }
                    }

                    if result == user_input.len() {
                        const SUBSET_MATCH_BASE_SCORE: f64 = 1_000.0;
                        current_score = SUBSET_MATCH_BASE_SCORE;
                    }
                }
            }

            if current_score > -1.0 {
                // Step 4: 名称长度惩罚 (用于打破平局)
                // 名称越长，加成越小。避免除以零。
                let length_bonus = 10.0 / (keyword.len() as f64 + 1.0);
                current_score += length_bonus;
            }

            if current_score > max_score {
                max_score = current_score;
            }
        }
        max_score
    }
}