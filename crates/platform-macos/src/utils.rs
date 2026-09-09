//! macOS 平台工具函数（与 `zerolaunch-platform-windows::utils` 对应的平台通用函数集合）。

use std::process::Command;

/// macOS 无 COM 概念，初始化为空操作。
/// 与 Windows `init_com()` 签名对齐，供宿主启动时统一调用。
pub fn init_com() {}

/// 获取 macOS 系统版本信息。
///
/// 经 `sw_vers` 命令读取（系统自带），拼装为
/// `macOS <productVersion> (<productName> <buildVersion>)` 形式；
/// 命令失败时回退到编译期架构名。
pub fn os_version() -> String {
    let product = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());
    let name = Command::new("sw_vers")
        .arg("-productName")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());
    let build = Command::new("sw_vers")
        .arg("-buildVersion")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    match (product, name, build) {
        (Some(version), Some(name), Some(build)) => {
            format!("macOS {} ({} {})", version, name, build)
        }
        (Some(version), _, _) => format!("macOS {}", version),
        _ => std::env::consts::OS.to_string(),
    }
}
