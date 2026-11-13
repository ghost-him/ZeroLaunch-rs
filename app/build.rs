//! 将内置语言包复制到可执行文件目录，保证 GPUI 宿主启动时能加载界面文案。

use std::path::PathBuf;

fn main() {
    // QuickJS 在深调用链上需要比 Windows 默认 1MB 更大的主线程栈。
    println!("cargo:rustc-link-arg=/STACK:16777216");
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let source_dir = manifest.join("../src-ui/i18n/locales");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("target profile directory")
        .to_path_buf();
    let destination = profile_dir.join("locales");

    for language in ["zh-Hans.json", "zh-Hant.json", "en.json"] {
        let source = source_dir.join(language);
        println!("cargo:rerun-if-changed={}", source.display());
        if let Err(error) = std::fs::create_dir_all(&destination) {
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
