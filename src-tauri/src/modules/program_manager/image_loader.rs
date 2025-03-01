use crate::core::image_processor::ImageProcessor;

/// 这个类主要用于加载程序的图片，支持并发查询
use image::codecs::png::PngEncoder;
use image::DynamicImage;
use image::ImageFormat;
use image::ImageReader;

use std::path::Path;
use tracing::{debug, warn};

#[derive(Debug)]
pub struct ImageLoader {
    default_app_icon_path: String,
}

impl ImageLoader {
    /// 新建一个
    pub fn new(default_icon_path: String) -> ImageLoader {
        ImageLoader {
            default_app_icon_path: default_icon_path,
        }
    }
    /// 加载一个图片
    pub fn load_image(&self, icon_path: &str) -> Vec<u8> {
        let mut pic_bytes: Vec<u8> = self.load_image_from_path(icon_path);
        if pic_bytes.is_empty() {
            pic_bytes = self.load_image_from_path(&self.default_app_icon_path)
        }
        pic_bytes
    }
    /// 使用路径加载一个图片
    fn load_image_from_path(&self, icon_path: &str) -> Vec<u8> {
        ImageProcessor::load_image_from_path(icon_path)
    }
}
