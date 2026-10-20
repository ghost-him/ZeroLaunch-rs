//! 设置助手系统提示词：把「说明书 + 输出格式约定」拼成模型的系统消息。
//!
//! 提示词正文在同目录的 `prompt.md`，`{REFERENCES_MARKER}` 与 `{manual}` 是占位符；
//! 本文件只保留常量与装配逻辑，改文案不必动 Rust 代码。
//! 说明书由 `SettingsManual::render` 现场生成（schema + 译文 + 前端下发的设置页路径 + 命令层投影的触发方式），
//! 提示词只负责不可由数据推导的部分：角色与语气、工具用法、回答次序、引用行格式。
//!
//! 引用为何用「正文标记行」而非工具调用：标记行能与终答同轮返回，无需为收集引用多跑一轮，
//! 也不依赖各提供方工具调用的稳定性；代价是这段格式约定必须被模型原样遵守（解析见
//! `super::split_references`）。

/// 「相关设置」引用行的行首标记；宿主按此前缀解析模型答复。
pub const REFERENCES_MARKER: &str = "相关设置";

/// 系统提示词模板：`{REFERENCES_MARKER}` 为引用行标记，`{manual}` 为说明书正文。
const TEMPLATE: &str = include_str!("./prompt.md");

/// 生成系统提示词。
///
/// 参数：manual - 说明书文本（`SettingsManual::render` 的产物）；
/// wiki_index - 使用手册目录（标题、id、摘要，`wiki::index_text` 的产物）。
/// 返回：模板占位符替换后的完整系统提示词。
pub fn system_prompt(manual: &str, wiki_index: &str) -> String {
    TEMPLATE
        .replace("{REFERENCES_MARKER}", REFERENCES_MARKER)
        .replace("{wiki_index}", wiki_index)
        .replace("{manual}", manual)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 模板占位符必须全部被替换，手册与说明书都要进入最终提示词。
    #[test]
    fn system_prompt_replaces_manual_and_wiki_index() {
        let prompt = system_prompt("MANUAL_BODY", "WIKI_INDEX");
        assert!(prompt.contains("MANUAL_BODY"));
        assert!(prompt.contains("WIKI_INDEX"));
        assert!(!prompt.contains("{manual}"));
        assert!(!prompt.contains("{wiki_index}"));
        assert!(!prompt.contains("{REFERENCES_MARKER}"));
        assert!(prompt.contains("相关设置:"));
    }
}
