use super::config::PINYIN_CONTENT_JS;
use serde::{Deserialize, Serialize};
/// 这个类用于将中文名字转换成拼音名字
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
struct Item {
    pinyin: String,
    word: String,
}

pub struct PinyinMapper {
    pinyin: HashMap<String, String>,
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
        for c in word.chars() {
            if let Some(pinyin) = self.pinyin.get(&c.to_string()) {
                result.push_str(pinyin);
                result.push(' ');
            } else if c.is_ascii() {
                result.push(c);
            }
        }
        result.trim_end().to_string()
    }
}
