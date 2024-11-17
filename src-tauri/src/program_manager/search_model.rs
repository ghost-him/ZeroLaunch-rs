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

/// 调用函数指针利用不同公式进行权重计算
/// operation：目标权重公式
pub fn calculate_weight(program_name: &str, input_name: &str, operation: SearchAlgorithmFn) -> f64{
    operation(program_name,input_name)
}

/// 调用函数调整权重得分
/// operation: 权重调整公式
pub fn score_adjust(score: f64, operation: ScoreAdjusterFn) -> f64{
    operation(score)
}

/// 得分权重调整公式log2
pub fn adjust_score_log2(origin_score : f64) -> f64{
    3.0 * (origin_score + 1.0).log2()
}

/// 权重计算最短编辑距离
pub fn shortest_edit_dis(compare_name: &str,input_name: &str) -> f64{
    let input_length = input_name.len();
    let compare_length = compare_name.len();
    let mut dp = vec![vec![256;input_length + 1];compare_length + 1];
    for i in 0..=compare_length {
        dp[i][0] = 0;
    }
    for i in 1..=input_length {
        dp[0][i] = i;
    }
    for i in 1..=compare_length{
        for j in 1..=input_length{
            if compare_name.chars().nth(i-1) == input_name.chars().nth(j-1){
                dp[i][j] = std::cmp::min(dp[i][j], dp[i-1][j-1]);
            }
            else{
                dp[i][j] = std::cmp::min(dp[i-1][j-1]+1, dp[i-1][j]+1);
            }
        }
    }
    let mut  min_operations = dp[compare_length][input_length];
    if input_length < compare_length{
        for i in input_length..=compare_length{
            min_operations = std::cmp::min(min_operations, dp[i][input_length]);
        }
    }
    let mut value : f64= 0.0;
    if input_length != 0{
        value = (1 - min_operations/input_length) as f64;
    }
    score_adjust(input_length as f64, adjust_score_log2) * f64::exp(3.0 * value - 2.0)
}

/// 权重计算KMP
pub fn KMP(compare_name: &str,input_name: &str) -> f64{
    let mut end_pos : f64 = 0.0;
    for i in 0..std::cmp::min(compare_name.len(), input_name.len()){
        if compare_name.chars().nth(i) == input_name.chars().nth(i){
            end_pos +=1.0;
        }else{
            break;
        }
    }
    if let Some(pos) = compare_name.find(input_name) {
        end_pos = end_pos + pos as f64;
    } 
    else{}
    end_pos
}

pub fn StandardSearchFn(program: Arc<Program>, user_input: &str) -> f64 {
    // todo: 完成这个实现，如果使用到了什么子算法，用上面的模块实现出来再完成这个就可以了
    // program中的字符串与user_input都已经是预处理过了，不再需要预处理了
    let mut score : f64 = calculate_weight(&program.show_name,user_input,shortest_edit_dis);
    score = score * score_adjust((user_input.len() as f64)/(program.show_name.len() as f64), adjust_score_log2);
    score = score + calculate_weight(&program.show_name,user_input, KMP);
    score
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
