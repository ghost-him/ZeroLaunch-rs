use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::ZipWriter;
use zip::write::FileOptions;

#[derive(Clone, Debug, ValueEnum)]
enum Architecture {
    /// x86_64 架构
    X64,
    /// ARM64 架构
    Arm64,
    /// 所有架构
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetArch {
    X86_64,
    AArch64,
}

impl TargetArch {
    fn triple(self) -> &'static str {
        match self {
            TargetArch::X86_64 => "x86_64-pc-windows-msvc",
            TargetArch::AArch64 => "aarch64-pc-windows-msvc",
        }
    }

    fn label(self) -> &'static str {
        match self {
            TargetArch::X86_64 => "x64",
            TargetArch::AArch64 => "arm64",
        }
    }

    fn display(self) -> &'static str {
        match self {
            TargetArch::X86_64 => "x64",
            TargetArch::AArch64 => "ARM64",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BuildTarget {
    arch: TargetArch,
}

#[derive(Clone, Copy, Debug)]
enum BuildKind {
    Installer,
    Portable,
}

impl BuildKind {
    fn description(self) -> &'static str {
        match self {
            BuildKind::Installer => "安装包",
            BuildKind::Portable => "便携版",
        }
    }

    fn item_label(self) -> &'static str {
        match self {
            BuildKind::Installer => "安装包",
            BuildKind::Portable => "便携包",
        }
    }
}

fn expand_architecture(arch: &Architecture) -> Vec<TargetArch> {
    match arch {
        Architecture::X64 => vec![TargetArch::X86_64],
        Architecture::Arm64 => vec![TargetArch::AArch64],
        Architecture::All => vec![TargetArch::X86_64, TargetArch::AArch64],
    }
}

fn collect_build_targets(arch: &Architecture) -> Vec<BuildTarget> {
    expand_architecture(arch)
        .into_iter()
        .map(|target_arch| BuildTarget { arch: target_arch })
        .collect()
}

fn print_build_plan(kind: BuildKind, targets: &[BuildTarget], version: &str) -> Result<()> {
    if targets.is_empty() {
        println!("⚠️ 当前命令未匹配到任何 {} 构建目标。", kind.description());
        return Ok(());
    }

    // 安装包由 Inno Setup 生成，文件名与脚本定义保持一致。
    println!("📋 将构建以下 {}:", kind.description());
    for target in targets {
        println!(
            "  ▶️ {} | 架构: {}",
            kind.item_label(),
            target.arch.display(),
        );

        match kind {
            BuildKind::Installer => {
                let setup_name = format!(
                    "ZeroLaunch-{}-{}-setup.exe",
                    version,
                    target.arch.label()
                );
                let cli_name = format!("zerolaunch-cli_{}_{}.exe", version, target.arch.label());
                println!("      • {}", setup_name);
                println!("      • {}", cli_name);
            }
            BuildKind::Portable => {
                let zip_name = format!(
                    "ZeroLaunch-portable-{}-{}.zip",
                    version,
                    target.arch.label()
                );
                println!("      • {}", zip_name);
            }
        }
    }

    Ok(())
}

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "ZeroLaunch-rs 自动化构建工具")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 构建所有版本
    BuildAll {
        /// 指定构建架构
        #[arg(short, long, value_enum, default_value_t = Architecture::All)]
        arch: Architecture,
    },
    /// 只构建安装包版本
    BuildInstaller {
        /// 指定构建架构
        #[arg(short, long, value_enum, default_value_t = Architecture::All)]
        arch: Architecture,
    },
    /// 只构建便携版本
    BuildPortable {
        /// 指定构建架构
        #[arg(short, long, value_enum, default_value_t = Architecture::All)]
        arch: Architecture,
    },
    /// 清理构建产物
    Clean,
    /// 同步使用手册快照到 crates/zerolaunch-rs/assets/wiki
    SyncWiki {
        /// wiki 源目录；默认 ../ZeroLaunch-rs.wiki，不存在时克隆到 target/wiki-cache
        #[arg(long)]
        source: Option<PathBuf>,
        /// 只校验快照是否与源一致，不写入
        #[arg(long)]
        check: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    //  切换工作目录
    let current_dir = env::current_dir()?;
    println!("当前工作目录是: {}", current_dir.display());
    let parent_dir = current_dir
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "无法获取父目录，可能已在根目录"))?;
    env::set_current_dir(parent_dir)?;
    println!("成功切换到父目录。");
    let new_current_dir = env::current_dir()?;
    println!("新的当前工作目录: {}", new_current_dir.display());

    println!("ZeroLaunch开启了lto优化，所以编译时间会长达数分钟，请耐心等待...");

    let cli = Cli::parse();

    match &cli.command {
        Commands::BuildAll { arch } => {
            println!("🚀 开始构建所有版本...");
            let version = get_app_version()?;
            build_installer_versions(arch, &version).await?;
            build_portable_versions(arch, &version).await?;
            println!("✅ 所有版本构建完成！");
        }
        Commands::BuildInstaller { arch } => {
            println!("🚀 开始构建安装包版本...");
            let version = get_app_version()?;
            build_installer_versions(arch, &version).await?;
            println!("✅ 安装包版本构建完成！");
        }
        Commands::BuildPortable { arch } => {
            println!("🚀 开始构建便携版本...");
            let version = get_app_version()?;
            build_portable_versions(arch, &version).await?;
            println!("✅ 便携版本构建完成！");
        }
        Commands::Clean => {
            println!("🧹 清理构建产物...");
            clean_build_artifacts()?;
            println!("✅ 清理完成！");
        }
        Commands::SyncWiki { source, check } => {
            sync_wiki(source.as_deref(), *check)?;
        }
    }

    Ok(())
}

/// 构建安装包版本
async fn build_installer_versions(arch: &Architecture, version: &str) -> Result<()> {
    let targets = collect_build_targets(arch);
    print_build_plan(BuildKind::Installer, &targets, version)?;

    for target in targets {
        build_single_installer(target, version).await?;
    }

    Ok(())
}

async fn build_single_installer(target: BuildTarget, version: &str) -> Result<()> {
    println!("📦 构建安装包 -> 架构: {}", target.arch.display());

    run_command(vec![
        "cargo".to_string(),
        "build".to_string(),
        "-p".to_string(),
        "zerolaunch-gpui".to_string(),
        "--release".to_string(),
        "--target".to_string(),
        target.arch.triple().to_string(),
    ])
    .await
    .with_context(|| format!("构建安装包失败: 架构 {}", target.arch.display()))?;

    run_command(vec![
        find_iscc()?,
        format!("/DAppVersion={version}"),
        format!("/DTargetTriple={}", target.arch.triple()),
        format!("/DArchLabel={}", target.arch.label()),
        format!("/DRepoRoot={}", env::current_dir()?.display()),
        "installer/zerolaunch.iss".to_string(),
    ])
    .await
    .context("Inno Setup 打包失败")?;

    let setup_name = format!("ZeroLaunch-{}-{}-setup.exe", version, target.arch.label());
    anyhow::ensure!(
        Path::new(&setup_name).is_file(),
        "Inno Setup 未生成安装包 {setup_name}"
    );
    println!("✅ 已生成安装包 {setup_name}");

    build_cli_binary(target.arch).await?;
    collect_cli_binary(target.arch, version)?;

    Ok(())
}

/// 收集 zerolaunch-cli 二进制文件到项目根目录（带版本和架构信息）
fn collect_cli_binary(target_arch: TargetArch, version: &str) -> Result<()> {
    let cli_path = Path::new("target")
        .join(target_arch.triple())
        .join("release")
        .join("zerolaunch-cli.exe");

    if cli_path.exists() {
        let dest_name = format!("zerolaunch-cli_{}_{}.exe", version, target_arch.label());
        let root_dir = env::current_dir()?;
        let dest_path = root_dir.join(&dest_name);
        if dest_path.exists() {
            fs::remove_file(&dest_path).context(format!("删除已存在的 {} 失败", dest_name))?;
        }
        fs::copy(&cli_path, &dest_path).context(format!("无法将 {:?} 复制到根目录", cli_path))?;
        println!("✅ 已将 {} 移动到根目录", dest_name);
    } else {
        println!(
            "⚠️  未找到 {} ({}) 的 zerolaunch-cli.exe",
            target_arch.triple(),
            target_arch.display()
        );
    }

    Ok(())
}

/// 构建 zerolaunch-cli 二进制文件
async fn build_cli_binary(target_arch: TargetArch) -> Result<()> {
    println!("🔨 构建 zerolaunch-cli -> 架构: {}", target_arch.display());

    let args = vec![
        "cargo".to_string(),
        "build".to_string(),
        "-p".to_string(),
        "zerolaunch-cli".to_string(),
        "--release".to_string(),
        "--target".to_string(),
        target_arch.triple().to_string(),
    ];

    run_command(args)
        .await
        .with_context(|| format!("构建 zerolaunch-cli 失败: 架构 {}", target_arch.display()))?;

    println!("✅ zerolaunch-cli 构建完成: {}", target_arch.display());
    Ok(())
}

/// 构建便携版本
async fn build_portable_versions(arch: &Architecture, version: &str) -> Result<()> {
    let targets = collect_build_targets(arch);
    print_build_plan(BuildKind::Portable, &targets, version)?;

    for target in targets {
        build_single_portable(target, version).await?;
    }

    Ok(())
}

async fn build_single_portable(target: BuildTarget, version: &str) -> Result<()> {
    println!("📦 构建便携版 -> 架构: {}", target.arch.display());

    run_command(vec![
        "cargo".to_string(),
        "build".to_string(),
        "-p".to_string(),
        "zerolaunch-gpui".to_string(),
        "--release".to_string(),
        "--target".to_string(),
        target.arch.triple().to_string(),
        "--features".to_string(),
        "portable".to_string(),
    ])
    .await
    .with_context(|| format!("构建便携版失败: 架构 {}", target.arch.display()))?;

    package_portable_variant(target, version).await?;

    build_cli_binary(target.arch).await?;
    collect_cli_binary(target.arch, version)?;

    Ok(())
}

/// 运行命令
async fn run_command(args: Vec<String>) -> Result<()> {
    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..]);

    let output = cmd.output().context("执行命令失败")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("命令执行失败: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        println!("{}", stdout);
    }

    Ok(())
}

/// 主程序可执行文件名（与 app/Cargo.toml 的 bin 名一致）
const APP_EXE_NAME: &str = "zerolaunch-gpui.exe";

/// 定位 Inno Setup 编译器：优先 PATH，其次常见安装目录。
fn find_iscc() -> Result<String> {
    if let Some(path) = env::var_os("PATH") {
        for dir in env::split_paths(&path) {
            let candidate = dir.join("ISCC.exe");
            if candidate.is_file() {
                return Ok(candidate.to_string_lossy().to_string());
            }
        }
    }
    for candidate in [
        r"C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
        r"C:\Program Files\Inno Setup 6\ISCC.exe",
        r"C:\Program Files\Inno Setup 7\ISCC.exe",
    ] {
        if Path::new(candidate).is_file() {
            return Ok(candidate.to_string());
        }
    }
    anyhow::bail!("未找到 Inno Setup 编译器 ISCC.exe，请安装 Inno Setup 6 并加入 PATH")
}

/// 打包便携版本
async fn package_portable_variant(target: BuildTarget, version: &str) -> Result<()> {
    // Cargo workspace 模式下 target 目录在项目根。
    let target_dir = Path::new("target");
    let zip_name = format!(
        "ZeroLaunch-portable-{}-{}.zip",
        version,
        target.arch.label()
    );

    let exe_path = find_portable_exe(target_dir, target.arch)?;
    println!(
        "📦 打包便携版 -> 架构: {} => {}",
        target.arch.display(),
        zip_name
    );
    create_portable_zip(&exe_path, &zip_name, target.arch).await?;
    println!("✅ 便携版打包完成: {}", zip_name);

    Ok(())
}

/// 查找便携版可执行文件
fn find_portable_exe(target_dir: &Path, arch: TargetArch) -> Result<PathBuf> {
    let release_dir = target_dir.join(arch.triple()).join("release");

    if !release_dir.exists() {
        anyhow::bail!(
            "未找到 {} ({}) 的构建目录 {:?}",
            arch.triple(),
            arch.display(),
            release_dir
        );
    }

    // 应用可执行文件名由 app/Cargo.toml 的 bin 名决定（zerolaunch-gpui）。
    // 必须精确匹配：release 目录里同时存在应用 zerolaunch-gpui.exe 与 zerolaunch-cli.exe
    // （CLI 工具），模糊匹配会按不确定的 read_dir 顺序误选其一，曾导致便携包打进 CLI
    // （issue #80）。
    let exe_names: Vec<String> = fs::read_dir(&release_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("exe"))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();

    let portable_exe = pick_portable_exe(&exe_names, APP_EXE_NAME)
        .map(|name| release_dir.join(name))
        .with_context(|| {
            format!(
                "未找到 {} ({}) 的便携版可执行文件 {}，release 目录内 exe 列表: {:?}",
                arch.triple(),
                arch.display(),
                APP_EXE_NAME,
                exe_names
            )
        })?;

    Ok(portable_exe)
}

/// 从 release 目录的 exe 文件名中精确选出应用可执行文件（排除 zerolaunch-cli.exe）
fn pick_portable_exe<'a>(exe_names: &'a [String], app_exe_name: &str) -> Option<&'a String> {
    exe_names.iter().find(|name| name.as_str() == app_exe_name)
}

/// 创建便携版 ZIP 包
async fn create_portable_zip(exe_path: &Path, zip_name: &str, arch: TargetArch) -> Result<()> {
    let zip_path = Path::new(zip_name);
    let file = fs::File::create(zip_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 添加可执行文件：便携包内统一命名为 ZeroLaunch_{arch}.exe，便于区分架构。
    let exe_name = format!("ZeroLaunch_{}.exe", arch.label());
    zip.start_file(exe_name, options)?;
    let exe_data = fs::read(exe_path)?;
    std::io::copy(&mut exe_data.as_slice(), &mut zip)?;

    // 添加 icon 文件夹（如果存在）
    let icon_dir = Path::new("app/icons");
    if icon_dir.exists() {
        add_directory_to_zip(&mut zip, icon_dir, "icons", &options)?;
    }

    // 添加 locale 文件夹（如果存在）
    let locale_dir = Path::new("crates/zerolaunch-rs/locales");
    if locale_dir.exists() {
        add_directory_to_zip(&mut zip, locale_dir, "locales", &options)?;
    }

    zip.finish()?;
    Ok(())
}

/// 将目录添加到 ZIP
fn add_directory_to_zip(
    zip: &mut ZipWriter<fs::File>,
    dir_path: &Path,
    zip_dir_name: &str,
    options: &FileOptions<()>,
) -> Result<()> {
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap().to_str().unwrap();
        let zip_path = format!("{}/{}", zip_dir_name, name);

        if path.is_file() {
            zip.start_file(&zip_path, *options)?;
            let mut file = fs::File::open(&path)?;
            std::io::copy(&mut file, zip)?;
        } else if path.is_dir() {
            add_directory_to_zip(zip, &path, &zip_path, options)?;
        }
    }
    Ok(())
}

/// 清理构建产物
fn clean_build_artifacts() -> Result<()> {
    let target_dir = Path::new("target");
    if target_dir.exists() {
        fs::remove_dir_all(target_dir).context("删除 target 目录失败")?;
        println!("🧹 已清理 {}", target_dir.display());
    }

    for entry in fs::read_dir(env::current_dir()?)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let is_artifact = (name.starts_with("ZeroLaunch-") && name.ends_with(".exe"))
            || (name.starts_with("ZeroLaunch-portable-") && name.ends_with(".zip"))
            || (name.starts_with("zerolaunch-cli_") && name.ends_with(".exe"));
        if is_artifact {
            fs::remove_file(entry.path()).context(format!("删除 {name} 失败"))?;
            println!("🧹 已清理 {name}");
        }
    }

    Ok(())
}

/// 从 Cargo workspace 读取应用版本号。
fn get_app_version() -> Result<String> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .context("执行 cargo metadata 失败")?;
    anyhow::ensure!(
        output.status.success(),
        "cargo metadata 失败: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("解析 cargo metadata 失败")?;
    let version = metadata["packages"]
        .as_array()
        .and_then(|packages| {
            packages
                .iter()
                .find(|package| package["name"] == "zerolaunch-gpui")
        })
        .and_then(|package| package["version"].as_str())
        .context("cargo metadata 中缺少 zerolaunch-gpui 版本号")?;
    Ok(version.to_string())
}

/// wiki 快照的输出目录（相对仓库根）。
const WIKI_ASSETS_DIR: &str = "crates/zerolaunch-rs/assets/wiki";
/// 远端 wiki 仓库地址：本地源目录不存在时使用。
const WIKI_REMOTE: &str = "https://github.com/ghost-him/ZeroLaunch-rs.wiki.git";

/// 同步或校验使用手册快照。
///
/// 参数：source - 源目录（相对仓库根或绝对路径）；None 时优先用同级的 wiki 工作区，
/// 再退到 target/wiki-cache 的克隆；check - 只比对不写入。
/// 返回：校验发现差异、源目录缺失或复制失败时报错。
fn sync_wiki(source: Option<&Path>, check: bool) -> Result<()> {
    let repo_root = env::current_dir()?;
    let source = resolve_wiki_source(&repo_root, source)?;
    let source_pages = collect_wiki_pages(&source)?;
    let target = repo_root.join(WIKI_ASSETS_DIR);

    println!("📚 wiki 源目录: {}", source.display());
    println!("📄 页面数量: {}", source_pages.len());
    anyhow::ensure!(!source_pages.is_empty(), "wiki 源目录中没有 Markdown 页面");

    if check {
        let target_pages = collect_wiki_pages(&target)?;
        let mut problems = Vec::new();
        for (rel, source_path) in &source_pages {
            match target_pages.get(rel) {
                None => problems.push(format!("缺少 {rel}")),
                Some(target_path) => {
                    if fs::read(source_path)? != fs::read(target_path)? {
                        problems.push(format!("内容不一致 {rel}"));
                    }
                }
            }
        }
        for rel in target_pages.keys() {
            if !source_pages.contains_key(rel) {
                problems.push(format!("源中已不存在 {rel}"));
            }
        }
        anyhow::ensure!(
            problems.is_empty(),
            "使用手册快照与源不一致（{} 处），请运行 cargo run --bin xtask sync-wiki 更新:\n{}",
            problems.len(),
            problems.join("\n")
        );
        println!("✅ 使用手册快照与源一致。");
        return Ok(());
    }

    if target.exists() {
        fs::remove_dir_all(&target)
            .with_context(|| format!("清理旧快照 {} 失败", target.display()))?;
    }
    for (rel, source_path) in &source_pages {
        let dest = target.join(format!("{rel}.md"));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source_path, &dest)
            .with_context(|| format!("复制 {} 失败", source_path.display()))?;
    }
    fs::write(target.join("SNAPSHOT"), wiki_snapshot(&source))?;
    println!(
        "✅ 已同步 {} 个页面到 {}",
        source_pages.len(),
        target.display()
    );
    Ok(())
}

/// 解析 wiki 源目录：显式参数优先，其次同级工作区，最后重新克隆到 target/wiki-cache。
fn resolve_wiki_source(repo_root: &Path, source: Option<&Path>) -> Result<PathBuf> {
    if let Some(source) = source {
        let source = if source.is_absolute() {
            source.to_path_buf()
        } else {
            repo_root.join(source)
        };
        anyhow::ensure!(source.is_dir(), "wiki 源目录不存在: {}", source.display());
        return Ok(source);
    }

    let local = repo_root.join("../ZeroLaunch-rs.wiki");
    if local.is_dir() {
        return Ok(local);
    }

    // 每次重新克隆，不复用上次的目录：target/ 会被 CI 缓存整体带走，
    // 复用一个不完整的克隆时 git 仍会报 "Already up to date"，导致快照为空。
    let cache = repo_root.join("target/wiki-cache");
    if cache.exists() {
        fs::remove_dir_all(&cache)
            .with_context(|| format!("清理 wiki 缓存 {} 失败", cache.display()))?;
    }
    if let Some(parent) = cache.parent() {
        fs::create_dir_all(parent)?;
    }
    let target = cache.to_string_lossy().to_string();
    git(repo_root, &["clone", "--depth", "1", WIKI_REMOTE, &target])?;
    Ok(cache)
}

/// 递归收集 Markdown 页面，键为相对路径去扩展名并统一用 `/` 分隔。
fn collect_wiki_pages(root: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let mut pages = BTreeMap::new();
    if root.is_dir() {
        collect_wiki_pages_into(root, root, &mut pages)?;
    }
    Ok(pages)
}

fn collect_wiki_pages_into(
    root: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        // 跳过隐藏条目与 GitHub wiki 元页面（_Sidebar/_Footer/_Header），它们不是手册正文。
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.starts_with('_') {
            continue;
        }
        if path.is_dir() {
            // archive/ 放的是旧版 wiki 页面，不能当现行文档收进快照。
            if entry.file_name().to_string_lossy() == "archive" {
                continue;
            }
            collect_wiki_pages_into(root, &path, out)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let rel = path.strip_prefix(root)?;
        let key = rel.with_extension("").to_string_lossy().replace('\\', "/");
        out.insert(key, path);
    }
    Ok(())
}

/// 记录快照来源：提交、提交时间与工作区是否有未提交的 Markdown 修改。
fn wiki_snapshot(source: &Path) -> String {
    let commit =
        git(source, &["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|_| "unknown".to_string());
    let date =
        git(source, &["log", "-1", "--format=%cI"]).unwrap_or_else(|_| "unknown".to_string());
    let dirty = git(source, &["status", "--porcelain", "--", "*.md"])
        .map(|text| !text.is_empty())
        .unwrap_or(false);
    format!(
        "source={}\ncommit={}\ndate={}\ndirty={}\n",
        source.display(),
        commit,
        date,
        dirty
    )
}

/// 在指定目录执行 git 命令，返回裁剪后的标准输出。
fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .context("执行 git 失败")?;
    anyhow::ensure!(
        output.status.success(),
        "git {} 失败: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_portable_exe_从混合目录中精确选中应用() {
        // release 目录同时存在应用 exe 与 CLI exe，必须精确选中应用 exe，
        // 这是 issue #80（便携包打进 CLI）的回归测试。
        let names = vec![
            "zerolaunch-cli.exe".to_string(),
            "zerolaunch-gpui.exe".to_string(),
        ];
        let picked = pick_portable_exe(&names, APP_EXE_NAME).expect("应找到应用 exe");
        assert_eq!(picked.as_str(), "zerolaunch-gpui.exe");
    }

    #[test]
    fn pick_portable_exe_应用缺失时返回_none() {
        let names = vec!["zerolaunch-cli.exe".to_string()];
        assert!(pick_portable_exe(&names, APP_EXE_NAME).is_none());
    }
}
