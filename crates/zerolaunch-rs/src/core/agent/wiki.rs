//! 内置使用手册：构建期由 `assets/wiki` 生成，运行时只读、不访问文件系统。
//!
//! 页面 id 是相对路径去扩展名（如 `核心搜索体验/搜索窗口交互`）；
//! 目录文本按一级分类分组并带全局编号，供模型挑页后调用 `read_wiki`。

/// 一页使用手册。
pub struct WikiPage {
    /// 页面 id（相对路径去 `.md`，统一 `/` 分隔）。
    pub id: &'static str,
    /// 一级标题。
    pub title: &'static str,
    /// 标题后的首段摘要（构建期截断）。
    pub summary: &'static str,
    /// 页面正文（构建期 `include_str!` 内嵌）。
    pub body: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/wiki_pages.rs"));

/// 手册目录文本：按一级分类分组，每页带全局编号、id 与摘要。
pub(crate) fn index_text() -> String {
    let mut out = format!("## 使用手册目录\n{}\n", snapshot_line());
    let mut category = "";
    for (index, page) in PAGES.iter().enumerate() {
        let current = page.id.split_once('/').map(|(name, _)| name).unwrap_or("");
        if current != category {
            category = current;
            if !category.is_empty() {
                out.push_str(&format!("\n### {category}\n"));
            }
        }
        out.push_str(&format!(
            "{}. {}（id：{}）— {}\n",
            index + 1,
            page.title,
            page.id,
            page.summary
        ));
    }
    out
}

/// 按 id 查找页面；找不到时再按一级标题匹配，并忽略误带的 `.md` 后缀。
pub(crate) fn find(query: &str) -> Option<&'static WikiPage> {
    let query = query.trim().trim_end_matches(".md").trim();
    PAGES
        .iter()
        .find(|page| page.id == query)
        .or_else(|| PAGES.iter().find(|page| page.title == query))
}

/// 读取整页正文，剥离已漂移的 `> 代码依据` 脚注；未知 id 返回可行动的提示。
pub(crate) fn read(id: &str) -> Result<String, String> {
    let Some(page) = find(id) else {
        return Err(format!(
            "未找到手册页面 {id}；参数取系统消息「使用手册目录」里标注的 id"
        ));
    };
    Ok(strip_code_references(page.body))
}

/// 去掉每页末尾的代码依据行：行号随代码演进失效，也不该出现在用户答复里。
fn strip_code_references(body: &str) -> String {
    body.lines()
        .filter(|line| !line.trim_start().starts_with("> 代码依据"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 把快照元数据压成一行：日期、提交与是否有未提交修改。
fn snapshot_line() -> String {
    let mut commit = "";
    let mut date = "";
    let mut dirty = false;
    for line in SNAPSHOT.lines() {
        if let Some(value) = line.strip_prefix("commit=") {
            commit = value.trim();
        } else if let Some(value) = line.strip_prefix("date=") {
            date = value.trim();
        } else if let Some(value) = line.strip_prefix("dirty=") {
            dirty = value.trim() == "true";
        }
    }
    if date.is_empty() || date == "unknown" {
        return "手册快照：未知".to_string();
    }
    let date = date.get(..10).unwrap_or(date);
    if commit.is_empty() || commit == "unknown" {
        return format!("手册快照：{date}");
    }
    if dirty {
        format!("手册快照：{date}（提交 {commit}，含未提交修改）")
    } else {
        format!("手册快照：{date}（提交 {commit}）")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// 页面 id 唯一，且标题、摘要都不为空。
    #[test]
    fn pages_have_unique_ids_and_metadata() {
        assert!(!PAGES.is_empty(), "手册快照不能为空");
        let ids: HashSet<&str> = PAGES.iter().map(|page| page.id).collect();
        assert_eq!(ids.len(), PAGES.len(), "页面 id 必须唯一");
        for page in PAGES {
            assert!(!page.title.trim().is_empty(), "{} 缺少标题", page.id);
            assert!(!page.summary.trim().is_empty(), "{} 缺少摘要", page.id);
        }
    }

    /// 目录覆盖全部页面，并带快照信息与编号。
    #[test]
    fn index_lists_every_page() {
        let index = index_text();
        assert!(index.contains("手册快照："));
        for page in PAGES {
            assert!(index.contains(page.id), "目录缺少 {}", page.id);
            assert!(index.contains(page.title), "目录缺少标题 {}", page.title);
        }
        assert!(index.contains("1. "), "目录应带编号");
    }

    /// 读取正文时剥离代码依据行；标题也能定位到同一页。
    #[test]
    fn read_strips_code_references_and_accepts_title() {
        for page in PAGES {
            let body = read(page.id).expect("已存在的 id 必须能读取");
            assert!(!body
                .lines()
                .any(|line| line.trim_start().starts_with("> 代码依据")));
            assert_eq!(find(page.title).map(|found| found.id), Some(page.id));
        }
    }

    /// 未知 id 返回提示而不是空文本。
    #[test]
    fn unknown_id_reports_missing() {
        let error = read("不存在的页面").expect_err("未知 id 应当报错");
        assert!(error.contains("未找到手册页面"));
    }
}
