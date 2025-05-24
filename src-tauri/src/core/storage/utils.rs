use encoding_rs;
use std::fs;
/// 存放通用工具函数
use std::io;
use std::path::Path;
use tracing::warn;
pub fn read_or_create_str(path: &str, content: Option<String>) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(data) => Ok(data),
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                if let Some(parent) = Path::new(path).parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        return Err(format!("无法创建文件夹: {}", e));
                    }
                }
                let initial_content = content.unwrap_or("".to_string());
                match fs::write(path, initial_content.clone()) {
                    Ok(_) => Ok(initial_content),
                    Err(write_err) => Err(format!("无法写入文件: {}", write_err)),
                }
            } else {
                Err(format!("无法读取： {}", e))
            }
        }
    }
}

pub fn read_or_create_bytes(path: &str, content: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
    match fs::read(path) {
        Ok(data) => Ok(data),
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                if let Some(parent) = Path::new(path).parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        return Err(format!("无法创建文件夹: {}", e));
                    }
                }
                let initial_content = content.unwrap_or_else(Vec::new);
                match fs::write(path, &initial_content) {
                    Ok(_) => Ok(initial_content),
                    Err(write_err) => Err(format!("无法写入文件: {}", write_err)),
                }
            } else {
                Err(format!("无法读取文件: {}", e))
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
            return None;
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
