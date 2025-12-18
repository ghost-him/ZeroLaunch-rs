use crate::program_manager::search_model::Scorer;
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
            let input_len = user_input.chars().count();
            let target_len = names.chars().count();

            // 条件性容错：短关键字（<=2字符）严格匹配，长关键字允许多打1字符
            let tolerance = if target_len <= 2 { 0 } else { 1 };
            if target_len + tolerance < input_len {
                continue;
            }

            let mut score: f64 = shortest_edit_dis(names, user_input);

            // 计算长度比率
            let input_len_f = input_len as f64;
            let target_len_f = target_len as f64;

            // 1. 限制比率加成：如果输入比目标长，比率锁定为 1.0，避免"越长分越高"的逻辑谬误
            let ratio = if input_len > target_len {
                1.0
            } else {
                input_len_f / target_len_f
            };
            score *= adjust_score_log2(ratio);

            // 2. 动态溢出惩罚：根据溢出比例动态调整惩罚
            // 溢出越多惩罚越重，对长词更宽容，对短词更严格
            if input_len > target_len {
                let overflow_ratio = (input_len_f - target_len_f) / target_len_f;
                // 惩罚因子：溢出比例 * 0.3，最低 0.7
                let penalty = (1.0 - overflow_ratio * 0.3).max(0.7);
                score *= penalty;
            }

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

impl Default for StandardScorer {
    fn default() -> Self {
        Self::new()
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
    // 初始化为最大可能距离（即完全插入），确保包含dp[0][n]的情况
    let mut min_operations = n as i32;

    // 初始化prev数组（对应i=0）
    for (j, value) in prev.iter_mut().enumerate() {
        *value = j as i32;
    }

    for i in 1..=m {
        current[0] = 0; // dp[i][0] = 0，允许从compare的任意位置开始匹配
        for j in 1..=n {
            let cost = if compare_chars[i - 1] == input_chars[j - 1] {
                0
            } else {
                1
            };
            current[j] = (prev[j - 1] + cost) // 替换/匹配
                .min(prev[j] + 1) // 删除 (compare中有，input中无)
                .min(current[j - 1] + 1); // 插入 (compare中无，input中有)
        }
        // 记录dp[i][n]，即input完全匹配到compare[..i]的某个后缀的代价
        if current[n] < min_operations {
            min_operations = current[n];
        }
        // 交换prev和current
        std::mem::swap(&mut prev, &mut current);
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
