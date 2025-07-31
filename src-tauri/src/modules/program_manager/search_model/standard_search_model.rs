use crate::program_manager::search_model::search_model::Scorer;
use crate::program_manager::Program;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
#[derive(Serialize, Deserialize)]
pub struct StandardScorer;

impl Scorer for StandardScorer {
    fn calculate_score(&self, program: &Arc<Program>, user_input: &str) -> f64 {
        // todo: 完成这个实现，如果使用到了什么子算法，用上面的模块实现出来再完成这个就可以了
        // program中的字符串与user_input都已经是预处理过了，不再需要预处理了
        let mut ret: f64 = -10000.0;
        for names in &program.search_keywords {
            if names.chars().count() < user_input.chars().count() {
                continue;
            }
            let mut score: f64 = shortest_edit_dis(names, user_input);
            score *= adjust_score_log2(
                (user_input.chars().count() as f64) / (names.chars().count() as f64),
            );
            score += subset_dis(names, user_input);
            score += kmp(names, user_input);
            ret = f64::max(ret, score);
        }
        ret
    }
}

impl Debug for StandardScorer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StandardScorer").finish()
    }
}

impl StandardScorer {
    pub fn new() -> Self {
        StandardScorer
    }
}

/// 得分权重调整公式log2
pub fn adjust_score_log2(origin_score: f64) -> f64 {
    3.0 * ((origin_score + 1.0).log2())
}

/// 子集匹配算法
pub fn subset_dis(compare_name: &str, input_name: &str) -> f64 {
    let mut compare_chars = HashMap::with_capacity(compare_name.len());

    // 统计 compare_name 中字符出现次数
    for c in compare_name.chars() {
        *compare_chars.entry(c).or_insert(0) += 1;
    }

    // 计算匹配的字符数
    let mut result = 0;
    for c in input_name.chars() {
        if let Some(count) = compare_chars.get_mut(&c) {
            if *count > 0 {
                result += 1;
                *count -= 1;
            }
        }
    }

    result as f64
}

/// 权重计算最短编辑距离
pub fn shortest_edit_dis(compare_name: &str, input_name: &str) -> f64 {
    let compare_chars: Vec<char> = compare_name.chars().collect();
    let input_chars: Vec<char> = input_name.chars().collect();
    let m = compare_chars.len();
    let n = input_chars.len();

    if n == 0 {
        return 1.0;
    }

    let mut prev = vec![0i32; n + 1];
    let mut current = vec![0i32; n + 1];
    let mut min_operations = i32::MAX;

    // 初始化prev数组（对应i=0）
    for j in 0..=n {
        prev[j] = j as i32;
    }

    for i in 1..=m {
        current[0] = 0; // dp[i][0] = 0
        for j in 1..=n {
            if compare_chars[i - 1] == input_chars[j - 1] {
                current[j] = prev[j - 1];
            } else {
                current[j] = std::cmp::min(prev[j - 1] + 1, prev[j] + 1);
            }
        }
        // 记录dp[i][n]
        if i >= n && current[n] < min_operations {
            min_operations = current[n];
        }
        // 交换prev和current
        std::mem::swap(&mut prev, &mut current);
    }

    // 确保min_operations包含dp[m][n]
    if m >= n && prev[n] < min_operations {
        min_operations = prev[n];
    }

    // 计算最终得分
    let value = 1.0 - (min_operations as f64 / n as f64);
    adjust_score_log2(n as f64) * (3.0 * value - 2.0).exp()
}

/// 权重计算KMP
pub fn kmp(compare_name: &str, input_name: &str) -> f64 {
    let mut ret: f64 = 0.0;

    // 首字符串匹配
    for (c1, c2) in compare_name.chars().zip(input_name.chars()) {
        if c1 == c2 {
            ret += 1.0;
        } else {
            break;
        }
    }

    // 子字符串匹配
    if compare_name.contains(input_name) {
        ret += input_name.chars().count() as f64;
    }

    ret
}
