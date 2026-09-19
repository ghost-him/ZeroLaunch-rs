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

/// 移除名称中的版本号（括号内容及空格后的「数字.数字」模式）。
///
/// 与 legacy 版本的关键词预处理一致：`"PowerPoint 2024" → "PowerPoint"`、
/// `"Mozilla Firefox 115.0" → "Mozilla Firefox"`、括号内容整体移除。
/// 供候选管道归一化基与 version-number-remover 优化器复用。
///
/// # Arguments
/// * `input_text` - 原始输入字符串（可为任意大小写）
///
/// # Returns
/// * 清理版本号后的字符串（可能等于原字符串）
pub fn remove_version_number(input_text: &str) -> String {
    let mut ret = String::new();
    let mut s = 0;
    let mut in_version = false;
    let chars: Vec<char> = input_text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '(' {
            s += 1;
            in_version = true;
        } else if ch == ')' {
            if s > 0 {
                s -= 1;
            }
            in_version = false;
        } else if s == 0 && !in_version {
            // 空格后紧跟数字/点视为版本号起始，整体跳过（如 " 2024"、" 115.0"）。
            if (ch.is_ascii_digit() || ch == '.') && i > 0 && chars[i - 1] == ' ' {
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                while i < chars.len() && chars[i] == ' ' {
                    i += 1;
                }
                i = i.saturating_sub(1);
                i += 1;
                continue;
            }
            ret.push(ch);
        }

        i += 1;
    }

    while ret.ends_with(' ') {
        ret.pop();
    }

    ret
}

/// 计算两个 `%Y-%m-%d` 日期相差的天数。
/// 参数：from - 起始日期；to - 结束日期。
/// 返回：相差天数；任一日期解析失败或 `to` 早于 `from`（时钟回拨）时返回 None。
pub fn days_between(from: &str, to: &str) -> Option<usize> {
    let from = NaiveDate::parse_from_str(from, "%Y-%m-%d").ok()?;
    let to = NaiveDate::parse_from_str(to, "%Y-%m-%d").ok()?;
    usize::try_from((to - from).num_days()).ok()
}
