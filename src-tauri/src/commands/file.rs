use crate::commands::utils::get_background_picture_path;
use crate::modules::config::config_manager::PartialConfig;
use crate::modules::storage::utils::read_or_create_bytes;
use crate::save_config_to_file;
use crate::AppState;
use image::codecs::png::PngEncoder;
use image::DynamicImage;
use image::ImageFormat;
use image::ImageReader;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Runtime;
use tracing::debug;
use tracing::warn;
// 将一个图片复制到指定的位置
pub fn copy_background_picture(file: String) -> Result<(), String> {
    let mut content: Vec<u8> = Vec::new();

    if !file.is_empty() {
        // 尝试打开图像文件
        let img_reader = match ImageReader::open(&file) {
            Ok(reader) => reader,
            Err(e) => {
                warn!("{}", e.to_string());
                return Err(e.to_string());
            }
        };

        // 尝试读取图像格式
        let format = match img_reader.format() {
            Some(fmt) => fmt,
            None => {
                warn!("读取图片格式失败");
                return Err("加载图片失败".to_string());
            }
        };

        // 尝试解码图像
        let mut img = match img_reader.decode() {
            Ok(image) => image,
            Err(e) => {
                warn!("{}", e.to_string());
                return Err(e.to_string());
            }
        };

        // 如果不是 PNG 格式，则转换为 RGBA8
        if format != ImageFormat::Png {
            img = DynamicImage::ImageRgba8(img.to_rgba8());
        }

        // 尝试使用 PNG 编码器将图像写入 Vec<u8>
        let encoder = PngEncoder::new(&mut content);
        if img.write_with_encoder(encoder).is_err() {
            // 编码失败，保持 content 为空
            content.clear();
        }
    }

    let target_path = get_background_picture_path();

    if let Ok(mut file) = File::create(target_path) {
        // 将所有字节写入文件
        let _ = file.write_all(&content);
    }

    Ok(())
}

/// 更新程序管理器的路径配置
#[tauri::command]
pub fn save_config<R: Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, Arc<AppState>>,
    partial_config: PartialConfig,
) -> Result<(), String> {
    let runtime_config = state.get_runtime_config().unwrap();
    debug!("{:?}", partial_config);
    runtime_config.update(partial_config);
    save_config_to_file(true);
    app.emit("update_search_bar_window", "").unwrap();
    Ok(())
}
