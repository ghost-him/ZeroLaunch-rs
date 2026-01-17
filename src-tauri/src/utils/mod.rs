pub mod access_policy;
pub mod defer;
pub mod font_database;
pub mod i18n;
pub mod locale;
pub mod notify;
pub mod service_locator;
pub mod ui_controller;
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
