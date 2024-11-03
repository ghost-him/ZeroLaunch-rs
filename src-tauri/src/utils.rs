use crate::config::RuntimeConfig;
use crate::singleton::Singleton;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub fn read_or_create(path: &str, content: Option<String>) -> Result<String, String> {
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

#[tauri::command]
pub fn get_item_size() -> Vec<usize> {
    let (item_width, item_height) = RuntimeConfig::instance().lock().unwrap().get_item_size();
    println!("item {} {}", item_width, item_height);
    vec![item_width, item_height]
}

#[tauri::command]
pub fn get_window_size() -> Vec<usize> {
    let (window_width, window_height) = RuntimeConfig::instance().lock().unwrap().get_window_size();
    println!("window {} {}", window_width, window_height);
    vec![window_width, window_height]
}

#[tauri::command]
pub fn get_window_scale_factor() -> f64 {
    let result = RuntimeConfig::instance()
        .lock()
        .unwrap()
        .get_window_scale_factor();
    println!("scale factor: {}", result);
    result
}
