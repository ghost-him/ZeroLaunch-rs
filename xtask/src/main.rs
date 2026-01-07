use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
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

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum AiMode {
    /// 启用 AI 特性（完全体）
    Enabled,
    /// 禁用 AI 特性（精简版）
    Disabled,
}

impl AiMode {
    fn is_enabled(self) -> bool {
        matches!(self, AiMode::Enabled)
    }

    fn display(self) -> &'static str {
        match self {
            AiMode::Enabled => "启用 AI",
            AiMode::Disabled => "关闭 AI",
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum AiProfile {
    /// 仅构建启用 AI 的完全体
    Enabled,
    /// 仅构建关闭 AI 的精简版
    Disabled,
    /// 同时构建启用与关闭 AI 的版本
    Both,
}

impl AiProfile {
    fn modes(self) -> Vec<AiMode> {
        match self {
            AiProfile::Enabled => vec![AiMode::Enabled],
            AiProfile::Disabled => vec![AiMode::Disabled],
            AiProfile::Both => vec![AiMode::Disabled, AiMode::Enabled],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BuildTarget {
    arch: TargetArch,
    ai_mode: AiMode,
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

fn collect_build_targets(arch: &Architecture, ai_modes: &[AiMode]) -> Vec<BuildTarget> {
    let mut targets = Vec::new();
    for target_arch in expand_architecture(arch) {
        for &ai_mode in ai_modes {
            targets.push(BuildTarget {
                arch: target_arch,
                ai_mode,
            });
        }
    }
    targets
}

fn print_build_plan(kind: BuildKind, targets: &[BuildTarget], version: &str) {
    if targets.is_empty() {
        println!("⚠️ 当前命令未匹配到任何 {} 构建目标。", kind.description());
        return;
    }

    println!("📋 将构建以下 {}:", kind.description());
    for target in targets {
        println!(
            "  ▶️ {} | 架构: {} | 模式: {}",
            kind.item_label(),
            target.arch.display(),
            target.ai_mode.display()
        );

        match kind {
            BuildKind::Installer => {
                let base_nsis = format!(
                    "zerolaunch-rs_{}_{}-setup.exe",
                    version,
                    target.arch.label()
                );
                let base_msi = format!("ZeroLaunch_{}_{}_en-US.msi", version, target.arch.label());
                let final_nsis = generate_installer_name(&base_nsis, version, target.ai_mode);
                let final_msi = generate_installer_name(&base_msi, version, target.ai_mode);
                println!("      • {}", final_nsis);
                println!("      • {}", final_msi);
            }
            BuildKind::Portable => {
                let suffix = if target.ai_mode.is_enabled() {
                    ""
                } else {
                    "-lite"
                };
                let zip_name = format!(
                    "ZeroLaunch-portable{}-{}-{}.zip",
                    suffix,
                    version,
                    target.arch.label()
                );
                println!("      • {}", zip_name);
            }
        }
    }
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
        /// 是否启用 AI 特性
        #[arg(long, value_enum, default_value_t = AiProfile::Both)]
        ai: AiProfile,
    },
    /// 只构建安装包版本
    BuildInstaller {
        /// 指定构建架构
        #[arg(short, long, value_enum, default_value_t = Architecture::All)]
        arch: Architecture,
        /// 是否启用 AI 特性
        #[arg(long, value_enum, default_value_t = AiMode::Enabled)]
        ai: AiMode,
    },
    /// 只构建便携版本
    BuildPortable {
        /// 指定构建架构
        #[arg(short, long, value_enum, default_value_t = Architecture::All)]
        arch: Architecture,
        /// 是否启用 AI 特性
        #[arg(long, value_enum, default_value_t = AiMode::Enabled)]
        ai: AiMode,
    },
    /// 清理构建产物
    Clean,
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
        Commands::BuildAll { arch, ai } => {
            println!("🚀 开始构建所有版本...");
            let version = get_app_version()?;
            let ai_modes = ai.modes();
            build_installer_versions(arch, &ai_modes, &version).await?;
            build_portable_versions(arch, &ai_modes, &version).await?;
            println!("✅ 所有版本构建完成！");
        }
        Commands::BuildInstaller { arch, ai } => {
            println!("🚀 开始构建安装包版本...");
            let version = get_app_version()?;
            let ai_modes = vec![*ai];
            build_installer_versions(arch, &ai_modes, &version).await?;
            println!("✅ 安装包版本构建完成！");
        }
        Commands::BuildPortable { arch, ai } => {
            println!("🚀 开始构建便携版本...");
            let version = get_app_version()?;
            let ai_modes = vec![*ai];
            build_portable_versions(arch, &ai_modes, &version).await?;
            println!("✅ 便携版本构建完成！");
        }
        Commands::Clean => {
            println!("🧹 清理构建产物...");
            clean_build_artifacts()?;
            println!("✅ 清理完成！");
        }
    }

    Ok(())
}

/// 构建安装包版本
async fn build_installer_versions(
    arch: &Architecture,
    ai_modes: &[AiMode],
    version: &str,
) -> Result<()> {
    let targets = collect_build_targets(arch, ai_modes);
    print_build_plan(BuildKind::Installer, &targets, version);

    for target in targets {
        build_single_installer(target, version).await?;
    }

    Ok(())
}

async fn build_single_installer(target: BuildTarget, version: &str) -> Result<()> {
    println!(
        "📦 构建安装包 -> 架构: {} | 模式: {}",
        target.arch.display(),
        target.ai_mode.display()
    );

    let mut args = vec![
        "bun".to_string(),
        "run".to_string(),
        "tauri".to_string(),
        "build".to_string(),
        "--target".to_string(),
        target.arch.triple().to_string(),
    ];

    if target.ai_mode.is_enabled() {
        args.push("--".to_string());
        args.push("--features".to_string());
        args.push("ai".to_string());
    }

    run_command(args).await.with_context(|| {
        format!(
            "构建安装包失败: 架构 {} | 模式 {}",
            target.arch.display(),
            target.ai_mode.display()
        )
    })?;

    move_installer_to_root(target.arch, version, target.ai_mode)?;

    Ok(())
}

/// 构建便携版本
async fn build_portable_versions(
    arch: &Architecture,
    ai_modes: &[AiMode],
    version: &str,
) -> Result<()> {
    let targets = collect_build_targets(arch, ai_modes);
    print_build_plan(BuildKind::Portable, &targets, version);

    for target in targets {
        build_single_portable(target, version).await?;
    }

    Ok(())
}

async fn build_single_portable(target: BuildTarget, version: &str) -> Result<()> {
    println!(
        "📦 构建便携版 -> 架构: {} | 模式: {}",
        target.arch.display(),
        target.ai_mode.display()
    );

    let mut args = vec![
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
    ];

    let features = if target.ai_mode.is_enabled() {
        "portable,ai".to_string()
    } else {
        "portable".to_string()
    };
    args.push(features);

    run_command(args).await.with_context(|| {
        format!(
            "构建便携版失败: 架构 {} | 模式 {}",
            target.arch.display(),
            target.ai_mode.display()
        )
    })?;

    package_portable_variant(target, version).await?;

    Ok(())
}

fn move_installer_to_root(target_arch: TargetArch, version: &str, ai_mode: AiMode) -> Result<()> {
    let root_dir = env::current_dir()?;
    let bundle_dir = Path::new("src-tauri")
        .join("target")
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

    // 需要检查的子目录
    let installer_subdirs = ["msi", "nsis"];

    for subdir_name in installer_subdirs {
        let subdir_path = bundle_dir.join(subdir_name);
        if subdir_path.is_dir() {
            // 遍历子目录中的文件
            for entry in fs::read_dir(&subdir_path)? {
                let entry = entry?;
                let source_path = entry.path();
                if source_path.is_file() {
                    if let Some(file_name) = source_path.file_name() {
                        let file_name_str = file_name.to_string_lossy();
                        let dest_name = if ai_mode.is_enabled() {
                            OsString::from(&*file_name_str)
                        } else {
                            OsString::from(generate_installer_name(
                                &file_name_str,
                                version,
                                ai_mode,
                            ))
                        };
                        let dest_path = root_dir.join(&dest_name);
                        if dest_path.exists() {
                            fs::remove_file(&dest_path)
                                .context(format!("删除已存在的安装包 {:?} 失败", dest_path))?;
                        }
                        // 如果拷贝出的是精简版，顺便清理 root 下可能残留的完全体安装包
                        if !ai_mode.is_enabled() {
                            let original_path = root_dir.join(file_name);
                            if original_path.exists() {
                                fs::remove_file(&original_path).context(format!(
                                    "删除残留的安装包 {:?} 失败",
                                    original_path
                                ))?;
                            }
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

fn generate_installer_name(original: &str, version: &str, ai_mode: AiMode) -> String {
    if ai_mode.is_enabled() || original.contains("_lite") {
        return original.to_string();
    }

    let version_marker = format!("_{}", version);
    if let Some(idx) = original.find(&version_marker) {
        let mut renamed = String::with_capacity(original.len() + 6);
        renamed.push_str(&original[..idx]);
        renamed.push_str("_lite");
        renamed.push_str(&original[idx..]);
        return renamed;
    }

    if let Some(dot_idx) = original.rfind('.') {
        let mut renamed = String::with_capacity(original.len() + 6);
        renamed.push_str(&original[..dot_idx]);
        renamed.push_str("_lite");
        renamed.push_str(&original[dot_idx..]);
        return renamed;
    }

    format!("{}_lite", original)
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

/// 打包便携版本
async fn package_portable_variant(target: BuildTarget, version: &str) -> Result<()> {
    let target_dir = Path::new("src-tauri/target");
    let suffix = if target.ai_mode.is_enabled() {
        ""
    } else {
        "-lite"
    };
    let zip_name = format!(
        "ZeroLaunch-portable{}-{}-{}.zip",
        suffix,
        version,
        target.arch.label()
    );

    if let Some(exe_path) = find_portable_exe(target_dir, target.arch)? {
        println!(
            "📦 打包便携版 -> 架构: {} | 模式: {} => {}",
            target.arch.display(),
            target.ai_mode.display(),
            zip_name
        );
        create_portable_zip(&exe_path, &zip_name, target.arch).await?;
        println!("✅ 便携版打包完成: {}", zip_name);
    } else {
        println!(
            "⚠️ 未找到 {} ({}) 的便携版可执行文件，跳过打包。",
            target.arch.triple(),
            target.arch.display()
        );
    }

    Ok(())
}

/// 查找便携版可执行文件
fn find_portable_exe(target_dir: &Path, arch: TargetArch) -> Result<Option<PathBuf>> {
    let release_dir = target_dir.join(arch.triple()).join("release");

    if !release_dir.exists() {
        println!(
            "⚠️  未找到 {} ({}) 的构建目录",
            arch.triple(),
            arch.display()
        );
        return Ok(None);
    }

    // 查找 .exe 文件
    for entry in fs::read_dir(&release_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("exe") {
            // 排除依赖文件，只要主程序
            let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if file_name.contains("zero") || file_name.contains("launch") || file_name == "app" {
                return Ok(Some(path));
            }
        }
    }

    println!(
        "⚠️  未找到 {} ({}) 的可执行文件",
        arch.triple(),
        arch.display()
    );
    Ok(None)
}

/// 创建便携版 ZIP 包
async fn create_portable_zip(exe_path: &Path, zip_name: &str, arch: TargetArch) -> Result<()> {
    let zip_path = Path::new(zip_name);
    let file = fs::File::create(zip_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 添加可执行文件
    let exe_name = exe_path.file_name().unwrap().to_str().unwrap();
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

    // 添加 Everything64.dll（仅限 x64 架构，因为 everything-rs 不支持 ARM64）
    if arch == TargetArch::X86_64 {
        let dll_path = Path::new("src-tauri/Everything64.dll");
        if dll_path.exists() {
            zip.start_file("Everything64.dll", options)?;
            let dll_data = fs::read(dll_path)?;
            std::io::copy(&mut dll_data.as_slice(), &mut zip)?;
        }
    }

    // 添加 models/readme.md（如果存在）
    let models_readme_path = Path::new("src-tauri/models/readme.md");
    if models_readme_path.exists() {
        // 首先确保 models 目录在 zip 中存在
        zip.add_directory("models", options)?;
        zip.start_file("models/readme.md", options)?;
        let readme_data = fs::read(models_readme_path)?;
        std::io::copy(&mut readme_data.as_slice(), &mut zip)?;
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
    // ... 函数体保持不变
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap().to_str().unwrap();
        let zip_path = format!("{}/{}", zip_dir_name, name);

        if path.is_file() {
            // 注意：因为 FileOptions<()> 实现了 Copy trait，所以 *options 是有效的
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
    let target_dir = Path::new("src-tauri/target");
    let version = get_app_version().ok();

    // 在删除 target 目录前，先清理根目录下的安装包副本
    let targets = ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"];
    let installer_subdirs = ["msi", "nsis"];

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

                            if let (Some(version), Some(name_str)) =
                                (version.as_ref(), file_name.to_str())
                            {
                                let no_ai_name =
                                    generate_installer_name(name_str, version, AiMode::Disabled);
                                if no_ai_name != name_str {
                                    let no_ai_path = Path::new(&no_ai_name);
                                    if no_ai_path.exists() {
                                        fs::remove_file(no_ai_path).context(format!(
                                            "删除根目录的 {:?} 失败",
                                            no_ai_name
                                        ))?;
                                        println!("🧹 已清理根目录下的安装包: {}", no_ai_name);
                                    }
                                }
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
