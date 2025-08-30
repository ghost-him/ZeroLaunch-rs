use encoding_rs;
use std::fs;
/// 存放通用工具函数
use std::io;
use std::io::Error;
use std::path::Path;
use tracing::warn;

/// 读取一个文件，如果没有这个文件，则返回错误
/// 返回一个字符串
pub fn read_str(path: &str) -> Result<String, Error> {
    fs::read_to_string(path)
}

pub fn create_str(path: &str, content: &str) -> Result<(), String> {
    if let Some(parent) = Path::new(path).parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return Err(format!("无法创建文件夹: {}", e));
        }
    }
    match fs::write(path, content) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("无法写入文件: {}，错误: {}", path, e)),
    }
}

/// 从一个文件中读取数据，如果没有这个文件，则创建一个新的文件，并写入初始内容
/// 返回一个字符串
pub fn read_or_create_str(path: &str, content: Option<String>) -> Result<String, String> {
    match read_str(path) {
        Ok(data) => Ok(data),
        Err(error) => {
            if error.kind() == io::ErrorKind::NotFound {
                let initial_content = content.unwrap_or_default();
                match create_str(path, &initial_content) {
                    Ok(_) => Ok(initial_content),
                    Err(write_err) => Err(format!("无法写入文件: {}", write_err)),
                }
            } else {
                Err(format!("无法读取文件: {}", error))
            }
        }
    }
}

/// 读取一个文件，如果没有这个文件，则返回错误
/// 返回一个字节数组
pub fn read_bytes(path: &str) -> Result<Vec<u8>, Error> {
    fs::read(path)
}

/// 创建一个文件，并写入字节内容。如果文件已存在则覆盖。
/// 写入前会确保父目录存在。
pub fn create_bytes(path: &str, content: &[u8]) -> Result<(), String> {
    if let Some(parent) = Path::new(path).parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return Err(format!("无法创建文件夹: {}", e));
        }
    }
    match fs::write(path, content) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("无法写入文件: {}，错误: {}", path, e)),
    }
}

/// 从一个文件中读取数据，如果没有这个文件，则创建一个新的文件，并写入初始内容
/// 返回一个字节数组
pub fn read_or_create_bytes(path: &str, content: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
    match read_bytes(path) {
        // 使用新的 read_bytes 函数
        Ok(data) => Ok(data),
        Err(error) => {
            if error.kind() == io::ErrorKind::NotFound {
                let initial_content = content.unwrap_or_default(); // 如果 content 为 None，则使用空 Vec<u8>
                match create_bytes(path, &initial_content) {
                    // 使用新的 create_bytes 函数
                    Ok(_) => Ok(initial_content),
                    Err(write_err) => Err(format!("无法写入文件: {}", write_err)),
                }
            } else {
                Err(format!("无法读取文件: {}", error))
            }
        }
    }
}

/// 将lnk解析为绝对路径
/// 优先使用本地的编码，如果失败，则使用utf16编码
pub fn get_lnk_target_path(lnk_path: &str) -> Option<String> {
    let shell_link_result = lnk::ShellLink::open(lnk_path, encoding_rs::GB18030);

    let shell_link = match shell_link_result {
        Ok(link) => link,
        Err(_) => {
            // 2. 二次尝试：如果首次尝试失败，则使用 UTF-16LE 编码
            match lnk::ShellLink::open(lnk_path, encoding_rs::UTF_16LE) {
                Ok(link) => {
                    warn!(
                        "在主要编码尝试失败后，成功使用 UTF-16LE 编码打开 LNK 文件: {}",
                        lnk_path
                    );
                    link
                }
                Err(e_utf16) => {
                    warn!(
                        "尝试使用 UTF-16LE 编码打开 LNK 文件 '{}' 再次失败: {:?}",
                        lnk_path, e_utf16
                    );
                    return None;
                }
            }
        }
    };

    // 从成功打开的 shell_link 中提取路径信息
    let link_info = match shell_link.link_info() {
        Some(info) => info,
        None => {
            warn!("无法从 LNK 文件 '{}' 获取 link_info。", lnk_path);
            return None;
        }
    };

    match link_info.local_base_path() {
        Some(path) => Some(path.to_string()),
        None => {
            warn!(
                "无法从 LNK 文件 '{}' 获取基本路径 (local_base_path)。",
                lnk_path
            );
            None
        }
    }
}

/// 读取一个目标目录，如果读到了，则返回数据，如果没有这个目录，则新建这个目录
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
