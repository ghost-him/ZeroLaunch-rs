//! Windows 平台工具函数。

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;

/// 当前进程可执行文件所在目录（便携数据目录与打包资源目录均以此为基准）。
///
/// 直接取系统给出的 exe 路径，不做 `canonicalize`：部分卷（映射盘、虚拟文件系统）
/// 不支持最终路径规范化查询，规范化失败会把可用目录误判为不可用。
/// 参数：无。
/// 返回：exe 同级目录；取不到 exe 路径或无父目录时返回 None。
pub fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
}

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
