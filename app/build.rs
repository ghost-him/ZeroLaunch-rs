//! 将内置语言包与图标复制到可执行文件目录，保证 GPUI 宿主启动时可直接加载。

use std::path::{Path, PathBuf};

fn main() {
    // QuickJS 在深调用链上需要比 Windows 默认 1MB 更大的主线程栈。
    println!("cargo:rustc-link-arg=/STACK:16777216");
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("target profile directory")
        .to_path_buf();

    copy_json_locales(
        &manifest.join("../crates/zerolaunch-rs/locales"),
        &profile_dir.join("locales"),
    );
    copy_directory(&manifest.join("icons"), &profile_dir.join("icons"));
}

/// 复制三种界面语言到目标目录。
///
/// 参数：source_dir - 语言包目录；destination - 目标 locales 目录。
/// 返回：无；源文件缺失或写入失败时让构建失败。
fn copy_json_locales(source_dir: &Path, destination: &Path) {
    for language in ["zh-Hans.json", "zh-Hant.json", "en.json"] {
        let source = source_dir.join(language);
        println!("cargo:rerun-if-changed={}", source.display());
        if let Err(error) = std::fs::create_dir_all(destination) {
            panic!("无法创建语言包目录 {}: {error}", destination.display());
        }
        let target = destination.join(language);
        if let Err(error) = std::fs::copy(&source, &target) {
            panic!(
                "复制语言包 {} -> {} 失败: {error}",
                source.display(),
                target.display()
            );
        }
    }
}

/// 递归复制图标目录。
///
/// 参数：source_dir - 源图标目录；destination - 目标 icons 目录。
/// 返回：无；源目录缺失或读写失败时让构建失败。
fn copy_directory(source_dir: &Path, destination: &Path) {
    println!("cargo:rerun-if-changed={}", source_dir.display());
    let entries = std::fs::read_dir(source_dir)
        .unwrap_or_else(|error| panic!("无法读取图标目录 {}: {error}", source_dir.display()));
    if let Err(error) = std::fs::create_dir_all(destination) {
        panic!("无法创建图标目录 {}: {error}", destination.display());
    }
    for entry in entries {
        let entry = entry.expect("读取图标目录失败");
        let source = entry.path();
        let target = destination.join(entry.file_name());
        if source.is_dir() {
            copy_directory(&source, &target);
        } else if let Err(error) = std::fs::copy(&source, &target) {
            panic!(
                "复制图标 {} -> {} 失败: {error}",
                source.display(),
                target.display()
            );
        }
    }
}
