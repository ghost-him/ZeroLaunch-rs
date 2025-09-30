use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
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
            build_installer_versions(arch).await?;
            build_portable_versions(arch).await?;
            println!("✅ 所有版本构建完成！");
        }
        Commands::BuildInstaller { arch } => {
            println!("🚀 开始构建安装包版本...");
            build_installer_versions(arch).await?;
            println!("✅ 安装包版本构建完成！");
        }
        Commands::BuildPortable { arch } => {
            println!("🚀 开始构建便携版本...");
            build_portable_versions(arch).await?;
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
async fn build_installer_versions(arch: &Architecture) -> Result<()> {
    match arch {
        Architecture::X64 | Architecture::All => {
            println!("📦 构建安装包 x64 版本...");
            run_command(&[
                "bun",
                "run",
                "tauri",
                "build",
                "--target",
                "x86_64-pc-windows-msvc",
            ])
            .await
            .context("构建安装包 x64 版本失败")?;
            move_installer_to_root("x86_64-pc-windows-msvc")?;
        }
        _ => {}
    }

    match arch {
        Architecture::Arm64 | Architecture::All => {
            println!("📦 构建安装包 ARM64 版本...");
            run_command(&[
                "bun",
                "run",
                "tauri",
                "build",
                "--target",
                "aarch64-pc-windows-msvc",
            ])
            .await
            .context("构建安装包 ARM64 版本失败")?;
            move_installer_to_root("aarch64-pc-windows-msvc")?;
        }
        _ => {}
    }

    Ok(())
}

/// 构建便携版本
async fn build_portable_versions(arch: &Architecture) -> Result<()> {
    match arch {
        Architecture::X64 | Architecture::All => {
            println!("📦 构建便携版 x64 版本...");
            run_command(&[
                "bun",
                "run",
                "tauri",
                "build",
                "--config",
                "src-tauri/tauri.conf.portable.json",
                "--target",
                "x86_64-pc-windows-msvc",
                "--",
                "--features",
                "portable",
            ])
            .await
            .context("构建便携版 x64 版本失败")?;
        }
        _ => {}
    }

    match arch {
        Architecture::Arm64 | Architecture::All => {
            println!("📦 构建便携版 ARM64 版本...");
            run_command(&[
                "bun",
                "run",
                "tauri",
                "build",
                "--config",
                "src-tauri/tauri.conf.portable.json",
                "--target",
                "aarch64-pc-windows-msvc",
                "--",
                "--features",
                "portable",
            ])
            .await
            .context("构建便携版 ARM64 版本失败")?;
        }
        _ => {}
    }

    // 打包便携版本
    println!("📦 打包便携版本...");
    package_portable_versions(arch).await?;

    Ok(())
}

fn move_installer_to_root(target: &str) -> Result<()> {
    let root_dir = env::current_dir()?;
    let bundle_dir = Path::new("src-tauri")
        .join("target")
        .join(target)
        .join("release")
        .join("bundle");

    if !bundle_dir.exists() {
        println!("⚠️  未找到 {} 的 bundle 目录，跳过移动安装包。", target);
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
                        let dest_path = root_dir.join(file_name);
                        fs::copy(&source_path, &dest_path).context(format!(
                            "无法将 {:?} 复制到 {:?}",
                            source_path, dest_path
                        ))?;
                        println!("✅ 已将安装包 {} 移动到根目录", file_name.to_string_lossy());
                    }
                }
            }
        }
    }

    Ok(())
}

/// 运行命令
async fn run_command(args: &[&str]) -> Result<()> {
    let mut cmd = Command::new(args[0]);
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
async fn package_portable_versions(arch: &Architecture) -> Result<()> {
    let target_dir = Path::new("src-tauri/target");
    let version = get_app_version()?;

    // 打包 x64 版本
    match arch {
        Architecture::X64 | Architecture::All => {
            if let Some(x64_exe) = find_portable_exe(target_dir, "x86_64-pc-windows-msvc")? {
                let zip_name = format!("ZeroLaunch-portable-{}-x64.zip", version);
                create_portable_zip(&x64_exe, &zip_name).await?;
                println!("✅ x64 便携版打包完成: {}", zip_name);
            }
        }
        _ => {}
    }

    // 打包 ARM64 版本
    match arch {
        Architecture::Arm64 | Architecture::All => {
            if let Some(arm64_exe) = find_portable_exe(target_dir, "aarch64-pc-windows-msvc")? {
                let zip_name = format!("ZeroLaunch-portable-{}-arm64.zip", version);
                create_portable_zip(&arm64_exe, &zip_name).await?;
                println!("✅ ARM64 便携版打包完成: {}", zip_name);
            }
        }
        _ => {}
    }

    Ok(())
}

/// 查找便携版可执行文件
fn find_portable_exe(target_dir: &Path, target: &str) -> Result<Option<PathBuf>> {
    let release_dir = target_dir.join(target).join("release");

    if !release_dir.exists() {
        println!("⚠️  未找到 {} 的构建目录", target);
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

    println!("⚠️  未找到 {} 的可执行文件", target);
    Ok(None)
}

/// 创建便携版 ZIP 包
async fn create_portable_zip(exe_path: &Path, zip_name: &str) -> Result<()> {
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
                                fs::remove_file(root_file_path).context(format!("删除根目录的 {:?} 失败", file_name))?;
                                println!("🧹 已清理根目录下的安装包: {}", file_name.to_string_lossy());
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
        let config: VersionConfig = serde_json::from_str(&config_content)
            .context("解析 src-tauri/tauri.conf.json 失败")?;
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
        let package: VersionConfig = serde_json::from_str(&package_content)
            .context("解析 package.json 失败")?;
        return Ok(package.version);
    }

    anyhow::bail!("未找到应用版本号，请确保配置文件中包含 version 字段");
}
