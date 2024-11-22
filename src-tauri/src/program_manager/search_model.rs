use super::Program;
use core::f64;
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

/// 调用函数指针利用不同公式进行权重计算
/// operation：目标权重公式
pub fn calculate_weight(program_name: &str, input_name: &str, operation: SearchAlgorithmFn) -> f64 {
    operation(program_name, input_name)
}

/// 调用函数调整权重得分
/// operation: 权重调整公式
pub fn score_adjust(score: f64, operation: ScoreAdjusterFn) -> f64 {
    operation(score)
}

/// 得分权重调整公式log2
pub fn adjust_score_log2(origin_score: f64) -> f64 {
    3.0 * ((origin_score + 1.0).log2())
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
    score_adjust(n as f64, adjust_score_log2) * (3.0 * value - 2.0).exp()
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

pub fn standard_search_fn(program: Arc<Program>, user_input: &str) -> f64 {
    // todo: 完成这个实现，如果使用到了什么子算法，用上面的模块实现出来再完成这个就可以了
    // program中的字符串与user_input都已经是预处理过了，不再需要预处理了
    let mut ret: f64 = -10000.0;
    for names in &program.alias {
        if names.chars().count() < user_input.chars().count() {
            continue;
        }
        let mut score: f64 = calculate_weight(names, user_input, shortest_edit_dis);
        score *= score_adjust(
            (user_input.chars().count() as f64) / (names.chars().count() as f64),
            adjust_score_log2,
        );
        score += calculate_weight(names, user_input, kmp);
        ret = f64::max(ret, score);
    }
    ret
}

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
