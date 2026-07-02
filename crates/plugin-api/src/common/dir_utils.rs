use std::fs;
use std::io;
use std::path::Path;

/// 跨平台目录工具函数。
/// SDK 内部自足，不依赖 crate::core。
pub struct DirUtils;

impl DirUtils {
    /// 读取一个目标目录，如果读到了，则返回数据，如果没有这个目录，则新建这个目录。
    /// 参数：path - 目录路径。
    /// 返回：目录条目迭代器，失败返回错误信息字符串。
    pub fn read_dir_or_create<P: AsRef<Path>>(path: P) -> Result<fs::ReadDir, String> {
        match fs::read_dir(&path) {
            Ok(dir) => Ok(dir),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                fs::create_dir_all(&path).map_err(|e| format!("无法创建目录: {}", e))?;
                fs::read_dir(path).map_err(|e| format!("无法读取新创建的目录: {}", e))
            }
            Err(e) => Err(format!("无法读取目录: {}", e)),
        }
    }
}
