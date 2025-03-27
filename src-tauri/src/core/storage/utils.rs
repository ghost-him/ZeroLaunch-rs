use std::fs;
use std::fs::ReadDir;
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
pub fn get_lnk_target_path(lnk_path: &str) -> Option<String> {
    // 尝试打开.lnk文件
    let shell_link = match lnk::ShellLink::open(lnk_path) {
        Ok(link) => link,
        Err(e) => {
            warn!("无法打开lnk文件: {:?}", e);
            return None;
        }
    };

    // 获取link_info，如果不存在则返回None
    let link_info = match shell_link.link_info() {
        Some(info) => info,
        None => {
            warn!("无法获取link_info, path: {}", lnk_path);
            return None;
        }
    };

    // 获取本地基本路径，如果不存在则返回None
    match link_info.local_base_path() {
        Some(path) => Some(path.clone()),
        None => {
            warn!("无法获取基本路径, path: {}", lnk_path);
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
