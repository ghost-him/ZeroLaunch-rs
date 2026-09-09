//! Windows 平台工具函数。

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

/// 将一个字符串转成windows的宽字符
pub fn get_u16_vec<P: AsRef<Path>>(path: P) -> Vec<u16> {
    path.as_ref()
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// 使用 Windows API 展开环境变量
pub fn expand_environment_variables(input: &str) -> Option<String> {
    unsafe {
        // 转换为 UTF-16
        let wide_input: Vec<u16> = OsStr::new(input)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // 首先获取需要的缓冲区大小
        let required_size = ExpandEnvironmentStringsW(PCWSTR::from_raw(wide_input.as_ptr()), None);

        if required_size == 0 {
            return None;
        }

        // 分配缓冲区并展开
        let mut buffer: Vec<u16> = vec![0; required_size as usize];
        let result =
            ExpandEnvironmentStringsW(PCWSTR::from_raw(wide_input.as_ptr()), Some(&mut buffer));

        if result > 0 && result <= required_size {
            // 移除末尾的 null 终止符
            if let Some(&0) = buffer.last() {
                buffer.pop();
            }
            Some(String::from_utf16_lossy(&buffer))
        } else {
            None
        }
    }
}

/// 获取 Windows 详细版本信息（从注册表读取产品名与构建号）。
/// 构建号 >= 22000 时归一化为 Windows 11 显示名；读取失败返回兜底字符串。
pub fn os_version() -> String {
    let hk_common = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(key) = hk_common.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion") {
        let product_name: String = key
            .get_value("ProductName")
            .unwrap_or_else(|_| "Windows".to_string());
        let current_build: String = key
            .get_value("CurrentBuild")
            .unwrap_or_else(|_| "".to_string());
        let display_version: String = key
            .get_value("DisplayVersion")
            .unwrap_or_else(|_| "".to_string());

        let build_num: u32 = current_build.parse().unwrap_or(0);
        let mut os_name = product_name;

        if build_num >= 22000 {
            if os_name.contains("Windows 10") {
                os_name = os_name.replace("Windows 10", "Windows 11");
            } else if !os_name.contains("Windows 11") {
                os_name = format!("Windows 11 ({})", os_name);
            }
        }

        if !display_version.is_empty() {
            format!("{} {} (Build {})", os_name, display_version, current_build)
        } else {
            format!("{} (Build {})", os_name, current_build)
        }
    } else {
        "Windows (Unknown)".to_string()
    }
}
