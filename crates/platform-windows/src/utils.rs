//! Windows 平台工具函数。

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;

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
