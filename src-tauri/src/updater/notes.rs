//! 版本更新内容：构建期内嵌的历史累积。
//!
//! `assets/updates.json` 由发布流水线在编译前生成（累积此前所有版本的结构化摘要），
//! 本次构建把它编译进二进制；运行时按「上次运行版本 → 当前版本」切片，零网络依赖。
//! 仓库内提交的是空种子，仅供本地开发编译；真实内容只存在于发布构建。

use serde::{Deserialize, Serialize};

/// 构建期内嵌的累积更新内容（JSON 文本）。发布流水线覆盖；本地开发为仓库种子。
const UPDATES_JSON: &str = include_str!("../../assets/updates.json");

/// 单个版本的结构化更新内容 —— 由发布流水线生成，经 `update_get_notes` 下发前端。
///
/// 仅在 `updater` 模块内解析、在 `commands::update` 中序列化下发；
/// 前端据此渲染「本次更新」页，不参与配置持久化。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionNotes {
    /// 版本号（不含 `v` 前缀），用于按 semver 切片。
    pub version: String,
    /// 发布日期（`YYYY-MM-DD`）；历史条目缺失时为空串。
    #[serde(default)]
    pub released_at: String,
    /// AI 摘要正文（Markdown 子集：段落、无序列表、引用、行内加粗与代码）。
    #[serde(default)]
    pub summary: String,
    /// 按类型分类的变更条目；无分类时为 empty。
    #[serde(default)]
    pub sections: Vec<NotesSection>,
    /// 与上一版本的 compare 链接；首版本或缺失时为空串。
    #[serde(default)]
    pub compare_url: String,
}

/// 一个变更分类 —— 发布流水线按 commit 前缀归类后的一段列表。
///
/// 仅作为 `VersionNotes` 的组成部分下发前端，不单独持久化。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotesSection {
    /// 分类标题（如 `✨ 新功能`），原样展示。
    pub title: String,
    /// 该分类下的条目文本（纯文本，可能含反引号与作者名）。
    #[serde(default)]
    pub items: Vec<String>,
}

/// 内嵌文件的顶层结构 —— 反序列化自 `assets/updates.json`。
///
/// 仅限本模块使用，不对外暴露。
#[derive(Debug, Deserialize)]
struct UpdatesFile {
    /// 按版本升序或乱序存放的条目；使用时统一排序。
    #[serde(default)]
    versions: Vec<VersionNotes>,
}

/// 一次「本次更新」查询的结果 —— `update_get_notes` 的返回体，经 IPC 原样下发。
///
/// 由前端启动流程消费：决定是否展示更新页，以及展示哪些版本区间。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotes {
    /// 当前运行版本（`CARGO_PKG_VERSION`，不含 `v` 前缀）。
    pub current_version: String,
    /// 上次运行版本；`None` 表示首次运行或记录值无法解析（此时不展示更新页）。
    pub last_version: Option<String>,
    /// `(last_version, current_version]` 区间内的条目，按版本升序；无内容时为 empty。
    pub versions: Vec<VersionNotes>,
}

/// 解析内嵌文件；损坏时返回空列表并告警，不阻断启动。
fn parse_embedded() -> Vec<VersionNotes> {
    match serde_json::from_str::<UpdatesFile>(UPDATES_JSON) {
        Ok(file) => file.versions,
        Err(e) => {
            tracing::warn!("解析内嵌更新内容失败: {e}");
            Vec::new()
        }
    }
}

/// 解析版本号，去掉可选的 `v` 前缀；非法版本返回 None。
fn parse_version(raw: &str) -> Option<semver::Version> {
    semver::Version::parse(raw.trim().trim_start_matches('v')).ok()
}

/// 计算需要展示的更新内容。
///
/// 参数：`last` —— 上次运行版本（`None` 表示首次运行）；`current` —— 当前版本。
/// 返回：区间内条目，按版本升序；首次运行、版本非法或区间内无条目时返回空列表。
pub fn collect(last: Option<&str>, current: &str) -> Vec<VersionNotes> {
    let Some(current) = parse_version(current) else {
        warn_invalid(current, "当前版本");
        return Vec::new();
    };
    let Some(last) = last.and_then(parse_version) else {
        return Vec::new();
    };

    let mut picked: Vec<(semver::Version, VersionNotes)> = parse_embedded()
        .into_iter()
        .filter_map(|item| {
            let version = parse_version(&item.version)?;
            (version > last && version <= current).then_some((version, item))
        })
        .collect();
    picked.sort_by(|a, b| a.0.cmp(&b.0));
    picked.into_iter().map(|(_, item)| item).collect()
}

/// 记录无法解析版本号的告警，避免静默丢弃。
fn warn_invalid(raw: &str, role: &str) {
    tracing::warn!(%role, %raw, "内嵌更新内容中的版本号无法解析，已跳过");
}
