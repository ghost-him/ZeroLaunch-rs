use crate::modules::config::default::PINYIN_CONTENT_JS;
use serde::{Deserialize, Serialize};
/// 这个类用于将中文名字转换成拼音名字
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
struct Item {
    pinyin: String,
    word: String,
}
#[derive(Debug)]
pub struct PinyinMapper {
    pinyin: HashMap<String, String>,
}

impl Default for PinyinMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl PinyinMapper {
    pub fn new() -> Self {
        let items: Vec<Item> =
            serde_json::from_str(PINYIN_CONTENT_JS).expect("Failed to parse PINYIN_CONTENT_JS");

        let mut word_to_pinyin: HashMap<String, String> = HashMap::new();
        for item in items {
            word_to_pinyin.insert(item.word, item.pinyin);
        }
        PinyinMapper {
            pinyin: word_to_pinyin,
        }
    }

    pub fn convert(&self, word: &str) -> String {
        let mut result = String::new();
        let mut prev_is_han = false; // 用于跟踪前一个字符是否为 ASCII

        for c in word.chars() {
            if let Some(pinyin) = self.pinyin.get(&c.to_string()) {
                // 如果前一个字符是 ASCII，且结果字符串不为空，插入一个空格
                if !prev_is_han && !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(pinyin);
                result.push(' ');
                prev_is_han = true; // 当前字符是中文，设置标志位为 false
            } else {
                // 如果当前字符是 ASCII，直接添加
                result.push(c);
                prev_is_han = false; // 设置标志位为 true
            }
        }

        result.trim_end().to_string()
    }
}
