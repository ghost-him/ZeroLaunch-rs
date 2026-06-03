use crate::core::config::setting_builders::SchemaBuilder;
use crate::plugin_system::types::{
    ComponentType, ConfigError, Configurable, KeywordOptimizer, SettingDefinition,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionNumberRemoverSettings {
    #[serde(rename = "priority", default = "default_priority_10")]
    priority: i32,
    #[serde(rename = "uses_context", default = "default_uses_context_true")]
    uses_context: bool,
}

fn default_priority_10() -> i32 {
    10
}

fn default_uses_context_true() -> bool {
    true
}

impl Default for VersionNumberRemoverSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionNumberRemoverSettings {
    fn new() -> Self {
        Self {
            priority: 10,
            uses_context: true,
        }
    }

    /// 从输入文本中移除版本号（括号内容及空格后的数字.数字模式）。
    /// 返回清理后的字符串。
    fn remove_version_number(&self, input_text: &str) -> String {
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

    /// 对关键词执行版本号移除优化。
    /// 若结果与原文相同则返回空 Vec。
    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = self.remove_version_number(keyword);
        if result.is_empty() || result == keyword {
            Vec::new()
        } else {
            vec![result]
        }
    }
}

pub struct VersionNumberRemover {
    inner: RwLock<VersionNumberRemoverSettings>,
}

impl Default for VersionNumberRemover {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionNumberRemover {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(VersionNumberRemoverSettings::new()),
        }
    }
}

impl Configurable for VersionNumberRemover {
    fn component_id(&self) -> &str {
        "version-number-remover"
    }

    fn component_name(&self) -> &str {
        "版本号移除器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::KeywordOptimizer
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number("priority", "优先级", "优化器执行优先级，数值越小越先执行")
                .order(0)
                .default(10.0)
                .min(1.0)
                .max(100.0)
                .step(1.0)
                .build(),
            SchemaBuilder::boolean(
                "uses_context",
                "使用上下文",
                "是否对所有已累积的关键词进行优化，而非仅对原始名称优化",
            )
            .order(1)
            .default(true)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: VersionNumberRemoverSettings =
            serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }
}

impl KeywordOptimizer for VersionNumberRemover {
    fn optimize(&self, keyword: &str) -> Vec<String> {
        self.inner.read().optimize(keyword)
    }

    fn uses_context(&self) -> bool {
        self.inner.read().uses_context
    }

    fn get_priority(&self) -> i32 {
        self.inner.read().priority
    }
}

use crate::plugin_system::builtin_registry::KeywordOptimizerEntry;
use std::sync::Arc;

pub(crate) fn build_version_number_remover() -> (Arc<dyn Configurable>, Arc<dyn KeywordOptimizer>) {
    let opt: Arc<dyn KeywordOptimizer> = Arc::new(VersionNumberRemover::new());
    let configurable: Arc<dyn Configurable> = opt.clone();
    (configurable, opt)
}

::inventory::submit! {
    KeywordOptimizerEntry {
        component_id: "version-number-remover",
        priority: 0,
        factory: build_version_number_remover,
    }
}
