use std::fs;
/// 存放通用工具函数
use std::io;
use std::path::Path;

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

/// 判断目标路径是不是一个可写路径
pub fn is_writable_directory(path: &str) -> bool {
    let path = Path::new(path);

    // 检查路径是否存在且是一个目录
    if !path.is_dir() {
        return false;
    }

    // 尝试在目录中创建一个临时文件
    let temp_file_path = path.join("temp_test_file.txt");
    match fs::write(&temp_file_path, "test content") {
        Ok(_) => {
            // 如果成功创建文件，尝试修改它
            match fs::write(&temp_file_path, "modified content") {
                Ok(_) => {
                    // 清理：删除临时文件
                    if let Err(e) = fs::remove_file(&temp_file_path) {
                        eprintln!("警告：无法删除临时文件: {}", e);
                    }
                    true
                }
                Err(_) => {
                    // 清理：尝试删除临时文件
                    let _ = fs::remove_file(&temp_file_path);
                    false
                }
            }
        }
        Err(_) => false,
    }
}
