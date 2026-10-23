use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
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

    // MSI 文件名由 tauri bundle 按 tauri.conf.json 的 productName 命名，不能凭空构造
    let product_name = match kind {
        BuildKind::Installer => Some(get_app_product_name()?),
        BuildKind::Portable => None,
    };

    println!("📋 将构建以下 {}:", kind.description());
    for target in targets {
        println!(
            "  ▶️ {} | 架构: {}",
            kind.item_label(),
            target.arch.display(),
        );

        match kind {
            BuildKind::Installer => {
                let msi_name = format!(
                    "{}_{}_{}_en-US.msi",
                    product_name
                        .as_deref()
                        .expect("Installer 分支已读取 productName"),
                    version,
                    target.arch.label()
                );
                let cli_name = format!("zerolaunch-cli_{}_{}.exe", version, target.arch.label());
                println!("      • {}", msi_name);
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
    /// 同步使用手册快照到 src-tauri/assets/wiki
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

    let args = vec![
        "bun".to_string(),
        "run".to_string(),
        "tauri".to_string(),
        "build".to_string(),
        "--target".to_string(),
        target.arch.triple().to_string(),
    ];

    run_command(args)
        .await
        .with_context(|| format!("构建安装包失败: 架构 {}", target.arch.display()))?;

    move_installer_to_root(target.arch)?;

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

    let args = vec![
        "bun".to_string(),
        "run".to_string(),
        "tauri".to_string(),
        "build".to_string(),
        "--config".to_string(),
        "src-tauri/tauri.conf.portable.json".to_string(),
        "--target".to_string(),
        target.arch.triple().to_string(),
        "--".to_string(),
        "--features".to_string(),
        "portable".to_string(),
    ];

    run_command(args)
        .await
        .with_context(|| format!("构建便携版失败: 架构 {}", target.arch.display()))?;

    package_portable_variant(target, version).await?;

    build_cli_binary(target.arch).await?;
    collect_cli_binary(target.arch, version)?;

    Ok(())
}

fn move_installer_to_root(target_arch: TargetArch) -> Result<()> {
    let root_dir = env::current_dir()?;
    // Cargo workspace 模式下 target 目录在项目根，而非 src-tauri/target。
    let bundle_dir = Path::new("target")
        .join(target_arch.triple())
        .join("release")
        .join("bundle");

    if !bundle_dir.exists() {
        println!(
            "⚠️  未找到 {} ({}) 的 bundle 目录，跳过移动安装包。",
            target_arch.triple(),
            target_arch.display()
        );
        return Ok(());
    }

    let installer_subdirs = ["msi"];

    for subdir_name in installer_subdirs {
        let subdir_path = bundle_dir.join(subdir_name);
        if subdir_path.is_dir() {
            for entry in fs::read_dir(&subdir_path)? {
                let entry = entry?;
                let source_path = entry.path();
                if source_path.is_file() {
                    if let Some(file_name) = source_path.file_name() {
                        let dest_name = OsString::from(&*file_name.to_string_lossy());
                        let dest_path = root_dir.join(&dest_name);
                        if dest_path.exists() {
                            fs::remove_file(&dest_path)
                                .context(format!("删除已存在的安装包 {:?} 失败", dest_path))?;
                        }

                        fs::copy(&source_path, &dest_path)
                            .context(format!("无法将 {:?} 复制到 {:?}", source_path, dest_path))?;
                        println!("✅ 已将安装包 {} 移动到根目录", dest_name.to_string_lossy());
                    }
                }
            }
        }
    }

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

/// 主程序可执行文件名（与 src-tauri/Cargo.toml 的 package name 一致）
const APP_EXE_NAME: &str = "zerolaunch-rs.exe";

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

    // 应用可执行文件名由 src-tauri 的 Cargo package name 决定（zerolaunch-rs），
    // tauri 构建不会按 tauri.conf.portable.json 的 productName 重命名 release 目录里的
    // 二进制（productName 只影响窗口标题、deep-link scheme 与 bundle 产物名）。
    // 必须精确匹配：release 目录里同时存在应用 zerolaunch-rs.exe 与 zerolaunch-cli.exe
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

    // 添加可执行文件：zip 内名称带架构后缀（如 zerolaunch-rs_x64.exe），
    // 与 zerolaunch-cli_{version}_{arch}.exe 的命名风格一致，便于区分 x64/arm64 产物。
    let exe_stem = APP_EXE_NAME
        .strip_suffix(".exe")
        .expect("APP_EXE_NAME 必须以 .exe 结尾");
    let exe_name = format!("{}_{}.exe", exe_stem, arch.label());
    zip.start_file(exe_name, options)?;
    let exe_data = fs::read(exe_path)?;
    std::io::copy(&mut exe_data.as_slice(), &mut zip)?;

    // 添加 icon 文件夹（如果存在）
    let icon_dir = Path::new("src-tauri/icons");
    if icon_dir.exists() {
        add_directory_to_zip(&mut zip, icon_dir, "icons", &options)?;
    }

    // 添加 locale 文件夹（如果存在）
    let locale_dir = Path::new("src-tauri/locales");
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
    // Cargo workspace 模式下 target 目录在项目根。
    let target_dir = Path::new("target");

    // 在删除 target 目录前，先清理根目录下的安装包副本
    let targets = ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"];
    let installer_subdirs = ["msi"];

    for target in targets {
        let bundle_dir = target_dir.join(target).join("release").join("bundle");
        for subdir_name in installer_subdirs {
            let subdir_path = bundle_dir.join(subdir_name);

            if subdir_path.is_dir() {
                if let Ok(entries) = fs::read_dir(subdir_path) {
                    for entry in entries.flatten() {
                        if let Some(file_name) = entry.path().file_name() {
                            let root_file_path = Path::new(file_name);
                            if root_file_path.exists() {
                                fs::remove_file(root_file_path)
                                    .context(format!("删除根目录的 {:?} 失败", file_name))?;
                                println!(
                                    "🧹 已清理根目录下的安装包: {}",
                                    file_name.to_string_lossy()
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    if target_dir.exists() {
        fs::remove_dir_all(target_dir).context("删除 target 目录失败")?;
        println!("🧹 已清理 {}", target_dir.display());
    }

    // 删除根目录下所有 zerolaunch-cli_* 文件
    let current_dir = env::current_dir()?;
    for entry in fs::read_dir(&current_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file() {
            let name = entry.file_name();
            if let Some(name_str) = name.to_str() {
                if name_str.starts_with("zerolaunch-cli_") && name_str.ends_with(".exe") {
                    fs::remove_file(entry.path()).context(format!("删除 {} 失败", name_str))?;
                    println!("🧹 已清理 {}", name_str);
                }
            }
        }
    }

    // 删除生成的 ZIP 文件
    let current_dir = env::current_dir()?;
    for entry in fs::read_dir(&current_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file() {
            let name = entry.file_name();
            if let Some(name_str) = name.to_str() {
                if name_str.starts_with("ZeroLaunch-portable-") && name_str.ends_with(".zip") {
                    fs::remove_file(entry.path()).context(format!("删除 {} 失败", name_str))?;
                    println!("🧹 已清理 {}", name_str);
                }
            }
        }
    }

    Ok(())
}

#[derive(Deserialize)]
struct VersionConfig {
    version: String,
}

/// 内部辅助类型：tauri.conf.json 的产品名数据模型。
///
/// 仅限本文件（xtask/main.rs）内使用，反序列化自 src-tauri/tauri.conf.json，
/// 用于准确展示 tauri bundle 生成的 MSI 安装包文件名。
#[derive(Deserialize)]
struct ProductConfig {
    /// tauri 产品名（如 "zerolaunch-rs"），tauri bundle 按它命名 MSI。
    /// None 表示配置文件中未声明 productName 字段。
    #[serde(rename = "productName")]
    product_name: Option<String>,
}

/// 从 tauri.conf.json 读取应用产品名（tauri bundle 按它命名 MSI 安装包）
fn get_app_product_name() -> Result<String> {
    let tauri_config_path = Path::new("src-tauri/tauri.conf.json");
    let config_content = fs::read_to_string(tauri_config_path)
        .with_context(|| format!("读取 {} 失败", tauri_config_path.display()))?;
    let config: ProductConfig =
        serde_json::from_str(&config_content).context("解析 src-tauri/tauri.conf.json 失败")?;
    config
        .product_name
        .context("src-tauri/tauri.conf.json 缺少 productName 字段")
}

fn get_app_version() -> Result<String> {
    let tauri_config_path = Path::new("src-tauri/tauri.conf.json");
    if tauri_config_path.exists() {
        let config_content = fs::read_to_string(tauri_config_path)
            .with_context(|| format!("读取 {} 失败", tauri_config_path.display()))?;
        let config: VersionConfig =
            serde_json::from_str(&config_content).context("解析 src-tauri/tauri.conf.json 失败")?;
        return Ok(config.version);
    }

    let portable_config_path = Path::new("src-tauri/tauri.conf.portable.json");
    if portable_config_path.exists() {
        let config_content = fs::read_to_string(portable_config_path)
            .with_context(|| format!("读取 {} 失败", portable_config_path.display()))?;
        let config: VersionConfig = serde_json::from_str(&config_content)
            .context("解析 src-tauri/tauri.conf.portable.json 失败")?;
        return Ok(config.version);
    }

    let package_json_path = Path::new("package.json");
    if package_json_path.exists() {
        let package_content = fs::read_to_string(package_json_path)
            .with_context(|| format!("读取 {} 失败", package_json_path.display()))?;
        let package: VersionConfig =
            serde_json::from_str(&package_content).context("解析 package.json 失败")?;
        return Ok(package.version);
    }

    anyhow::bail!("未找到应用版本号，请确保配置文件中包含 version 字段");
}

/// wiki 快照的输出目录（相对仓库根）。
const WIKI_ASSETS_DIR: &str = "src-tauri/assets/wiki";
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

/// 解析 wiki 源目录：显式参数优先，其次同级工作区，最后克隆/更新缓存。
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

    let cache = repo_root.join("target/wiki-cache");
    let refreshed = cache.join(".git").is_dir() && git(&cache, &["pull", "--ff-only"]).is_ok();
    if !refreshed {
        if cache.exists() {
            fs::remove_dir_all(&cache)
                .with_context(|| format!("清理 wiki 缓存 {} 失败", cache.display()))?;
        }
        if let Some(parent) = cache.parent() {
            fs::create_dir_all(parent)?;
        }
        let target = cache.to_string_lossy().to_string();
        git(repo_root, &["clone", "--depth", "1", WIKI_REMOTE, &target])?;
    }
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
            "zerolaunch-rs.exe".to_string(),
        ];
        let picked = pick_portable_exe(&names, APP_EXE_NAME).expect("应找到应用 exe");
        assert_eq!(picked.as_str(), "zerolaunch-rs.exe");
    }

    #[test]
    fn pick_portable_exe_应用缺失时返回_none() {
        let names = vec!["zerolaunch-cli.exe".to_string()];
        assert!(pick_portable_exe(&names, APP_EXE_NAME).is_none());
    }
}
