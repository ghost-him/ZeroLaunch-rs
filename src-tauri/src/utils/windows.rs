use std::os::windows::ffi::OsStrExt;
/// 存放与windows相关的工具类函数
use std::path::Path;

/// 将一个字符串转成windows的宽字符
pub fn get_u16_vec<P: AsRef<Path>>(path: P) -> Vec<u16> {
    path.as_ref()
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
