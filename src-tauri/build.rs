use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    generate_wiki_pages();
    tauri_build::build();
}

/// 扫描 `assets/wiki`，生成内嵌的使用手册索引模块。
///
/// 生成物写在 `OUT_DIR/wiki_pages.rs`，由 `core::agent::wiki` include；
/// 页面标题取首个一级标题，摘要取标题后的首段正文（跳过小标题与引用行）。
/// 缺少快照、标题为空或 id 重复时直接让构建失败，避免运行期出现空手册。
fn generate_wiki_pages() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("缺少 CARGO_MANIFEST_DIR 环境变量"));
    let wiki_dir = manifest_dir.join("assets/wiki");
    println!("cargo:rerun-if-changed={}", wiki_dir.display());
    assert!(
        wiki_dir.is_dir(),
        "缺少使用手册快照 {}：请在 xtask 目录运行 cargo run --bin xtask sync-wiki",
        wiki_dir.display()
    );

    let mut pages = Vec::new();
    collect_pages(&wiki_dir, &wiki_dir, &mut pages);
    assert!(
        !pages.is_empty(),
        "使用手册快照为空：请在 xtask 目录运行 cargo run --bin xtask sync-wiki"
    );
    pages.sort_by(|left, right| left.0.cmp(&right.0));

    let mut seen = HashSet::new();
    let mut generated = String::from("// 由 build.rs 依据 assets/wiki 生成，勿手改。\n");
    generated.push_str("static PAGES: &[WikiPage] = &[\n");
    for (id, path) in &pages {
        assert!(seen.insert(id.clone()), "使用手册页面 id 重复: {id}");
        let text = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("读取 {} 失败: {error}", path.display()));
        let (title, summary) = parse_page(&text, path);
        let rel = path
            .strip_prefix(&wiki_dir)
            .expect("页面路径必然位于 wiki 目录下")
            .to_string_lossy()
            .replace('\\', "/");
        generated.push_str(&format!(
            "    WikiPage {{ id: {id:?}, title: {title:?}, summary: {summary:?}, \
body: include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/wiki/{rel}\")) }},\n"
        ));
    }
    generated.push_str("];\n");

    let snapshot = fs::read_to_string(wiki_dir.join("SNAPSHOT")).unwrap_or_default();
    generated.push_str(&format!("static SNAPSHOT: &str = {snapshot:?};\n"));

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("缺少 OUT_DIR 环境变量"));
    fs::write(out_dir.join("wiki_pages.rs"), generated).expect("写入 wiki_pages.rs 失败");
}

/// 递归收集 Markdown 页面：返回（相对路径去扩展名、绝对路径）。
fn collect_pages(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| {
        panic!("读取目录 {} 失败: {error}", dir.display());
    });
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|error| panic!("读取 {} 的条目失败: {error}", dir.display()));
        let path = entry.path();
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_pages(root, &path, out);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let id = path
            .strip_prefix(root)
            .expect("页面路径必然位于 wiki 目录下")
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");
        out.push((id, path));
    }
}

/// 提取页面标题与摘要；一级标题缺失时让构建失败，摘要缺失时回退到标题。
fn parse_page(text: &str, path: &Path) -> (String, String) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let lines: Vec<&str> = text.lines().collect();
    let title_index = lines
        .iter()
        .position(|line| line.trim_start().starts_with("# "))
        .unwrap_or_else(|| panic!("使用手册页面缺少一级标题: {}", path.display()));
    let title = lines[title_index]
        .trim_start()
        .trim_start_matches("# ")
        .trim()
        .to_string();
    assert!(
        !title.is_empty(),
        "使用手册页面标题为空: {}",
        path.display()
    );

    let mut summary = lines[title_index + 1..]
        .iter()
        .map(|line| line.trim())
        .find(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('>'))
        .map(clean_inline)
        .unwrap_or_default();
    if summary.is_empty() {
        summary = lines[title_index + 1..]
            .iter()
            .map(|line| line.trim())
            .find(|line| line.starts_with('#'))
            .map(|line| line.trim_start_matches('#').trim().to_string())
            .unwrap_or_else(|| title.clone());
    }
    (title, truncate(&summary, 60))
}

/// 去掉摘要里的 Markdown 链接、强调与行内代码标记。
fn clean_inline(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '[' => {
                if let Some(close) = chars[index + 1..].iter().position(|ch| *ch == ']') {
                    let close = index + 1 + close;
                    if chars.get(close + 1) == Some(&'(') {
                        if let Some(end) = chars[close + 2..].iter().position(|ch| *ch == ')') {
                            out.extend(&chars[index + 1..close]);
                            index = close + 2 + end + 1;
                            continue;
                        }
                    }
                }
                out.push('[');
            }
            '*' | '`' => {}
            _ => out.push(chars[index]),
        }
        index += 1;
    }
    out.trim().to_string()
}

/// 按字符数截断，超长时补省略号。
fn truncate(text: &str, limit: usize) -> String {
    let mut out: String = text.chars().take(limit).collect();
    if text.chars().count() > limit {
        out.push('…');
    }
    out
}
