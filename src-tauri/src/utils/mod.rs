pub mod access_policy;
pub mod font_database;
pub mod locale;
pub mod trace_id;
pub mod waiting_hashmap;
pub mod windows;
use chrono::{Local, NaiveDate};
use time::OffsetDateTime;
/// 生成当前日期的函数
pub fn generate_current_date() -> String {
    let current_date = Local::now().date_naive();
    current_date.format("%Y-%m-%d").to_string()
}

/// 生成当前的时间
pub fn get_current_time() -> i64 {
    let now = OffsetDateTime::now_utc();
    now.unix_timestamp()
}

/// 折叠字符串中的连续空格为单个空格，并去除首尾空格
///
/// 与 legacy 版本的输入预处理保持一致，供评分输入归一化与关键词优化器复用。
///
/// # Arguments
/// * `input_text` - 原始输入字符串
///
/// # Returns
/// * 折叠后的字符串（可能等于原字符串）
pub fn collapse_repeated_spaces(input_text: &str) -> String {
    let mut result = String::new();
    let mut is_space = false;

    for c in input_text.chars() {
        if c != ' ' {
            result.push(c);
            is_space = false;
        } else if !is_space && !result.is_empty() {
            result.push(c);
            is_space = true;
        } else {
            is_space = true;
        }
    }

    if result.ends_with(' ') {
        result.pop();
    }

    result
}

/// 比较日期字符串与当前日期的函数
pub fn is_date_current(date_str: &str) -> bool {
    // 解析输入的日期字符串
    let input_date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        Ok(date) => date,
        Err(e) => {
            tracing::warn!("Failed to parse date string '{}': {}", date_str, e);
            return false; // 如果解析失败,返回false
        }
    };

    // 获取当前日期
    let current_date = Local::now().date_naive();

    // 比较两个日期
    input_date == current_date
}
