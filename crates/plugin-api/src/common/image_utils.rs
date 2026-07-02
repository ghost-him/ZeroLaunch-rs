use image::GenericImageView;
use image::ImageBuffer;
use image::ImageEncoder;
use image::ImageFormat;
use image::Rgba;
use image::RgbaImage;
use kmeans_colors::get_kmeans;
use palette::{IntoColor, Lab, Srgb};
use rand::RngExt;
use rayon::prelude::*;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use tracing::info;

/// 跨平台图片处理工具的错误类型。
/// 不依赖 crate::error，SDK 内部自足。
#[derive(Debug, thiserror::Error)]
pub enum ImageUtilsError {
    #[error("图片处理错误: {0}")]
    ProcessingError(String),

    #[error("任务执行错误: {0}")]
    TaskJoinError(String),
}

/// 跨平台图片处理工具函数集合。
/// 从 core::image_processor 中提取的平台无关逻辑，不依赖任何 Windows API。
pub struct ImageUtils;

impl ImageUtils {
    /// 将图片数据转换为 PNG 格式。
    /// 支持 SVG、BMP、JPEG、GIF、WebP 等格式自动检测并转换。
    /// 参数：image_data - 原始图片字节数据。
    /// 返回：PNG 格式字节数据，失败返回 ImageUtilsError。
    pub async fn convert_image_to_png(image_data: Vec<u8>) -> Result<Vec<u8>, ImageUtilsError> {
        if image_data.is_empty() {
            return Err(ImageUtilsError::ProcessingError(
                "Input image data is empty".to_string(),
            ));
        }

        if Self::is_html_content(&image_data) {
            return Err(ImageUtilsError::ProcessingError(
                "Downloaded content appears to be HTML, not an image".to_string(),
            ));
        }

        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, ImageUtilsError> {
            match usvg::Tree::from_data(&image_data, &usvg::Options::default()) {
                Ok(tree) => {
                    let pixmap_size = tree.size().to_int_size();
                    if pixmap_size.width() == 0 || pixmap_size.height() == 0 {
                        return Err(ImageUtilsError::ProcessingError(format!(
                            "Invalid SVG dimensions (width: {}px, height: {}px)",
                            pixmap_size.width(),
                            pixmap_size.height()
                        )));
                    }

                    let mut pixmap =
                        tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
                            .ok_or_else(|| {
                                ImageUtilsError::ProcessingError(
                                    "Failed to create Pixmap for SVG rendering".to_string(),
                                )
                            })?;

                    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

                    let png_data = pixmap.encode_png().map_err(|e| {
                        ImageUtilsError::ProcessingError(format!(
                            "Failed to encode SVG as PNG: {}",
                            e
                        ))
                    })?;
                    Ok(png_data)
                }
                Err(_) => {
                    let img_reader = image::ImageReader::new(Cursor::new(image_data))
                        .with_guessed_format()
                        .map_err(|e| {
                            ImageUtilsError::ProcessingError(format!(
                                "Failed to create image reader: {}",
                                e
                            ))
                        })?;

                    let format = img_reader.format().ok_or_else(|| {
                        ImageUtilsError::ProcessingError(
                            "Unable to detect image format".to_string(),
                        )
                    })?;

                    let mut img = img_reader.decode().map_err(|e| {
                        ImageUtilsError::ProcessingError(format!("Failed to decode image: {}", e))
                    })?;

                    if format != ImageFormat::Png {
                        img = image::DynamicImage::ImageRgba8(img.to_rgba8());
                    }

                    let mut png_data = Vec::new();
                    let encoder = image::codecs::png::PngEncoder::new(&mut png_data);
                    img.write_with_encoder(encoder).map_err(|e| {
                        ImageUtilsError::ProcessingError(format!(
                            "Failed to encode image as PNG: {}",
                            e
                        ))
                    })?;
                    Ok(png_data)
                }
            }
        })
        .await
        .map_err(|e| ImageUtilsError::TaskJoinError(format!("Task join error: {}", e)))?
    }

    /// 判断数据是否像是 HTML 内容。
    /// 参数：data - 待检测的字节数据。
    /// 返回：true 表示是 HTML 内容。
    pub fn is_html_content(data: &[u8]) -> bool {
        if let Ok(s) = std::str::from_utf8(data) {
            let s_trimmed = s.trim_start();
            if s_trimmed.eq_ignore_ascii_case("<!DOCTYPE html")
                || s_trimmed.starts_with("<!DOCTYPE html")
                || s_trimmed.starts_with("<html")
                || s_trimmed.starts_with("<HTML")
            {
                return true;
            }
        }
        false
    }

    /// 调整图片大小，如果超过指定尺寸则等比缩放。
    /// 参数：data - PNG 图片字节数据；max_width - 最大宽度；max_height - 最大高度。
    /// 返回：调整大小后的 PNG 字节数据，失败返回 ImageUtilsError。
    pub async fn resize_image(
        data: Vec<u8>,
        max_width: u32,
        max_height: u32,
    ) -> Result<Vec<u8>, ImageUtilsError> {
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, ImageUtilsError> {
            let img = image::load_from_memory(&data).map_err(|e| {
                ImageUtilsError::ProcessingError(format!(
                    "Failed to load image for resizing: {}",
                    e
                ))
            })?;

            if img.width() <= max_width && img.height() <= max_height {
                return Ok(data);
            }

            let resized = img.thumbnail(max_width, max_height);

            let mut png_data = Vec::new();
            let encoder = image::codecs::png::PngEncoder::new(&mut png_data);
            resized.write_with_encoder(encoder).map_err(|e| {
                ImageUtilsError::ProcessingError(format!("Failed to encode resized image: {}", e))
            })?;

            Ok(png_data)
        })
        .await
        .map_err(|e| ImageUtilsError::TaskJoinError(format!("Task join error: {}", e)))?
    }

    /// 从 PNG 图像数据中裁剪掉外围的白色或透明像素。
    /// 参数：png_data - PNG 格式图片字节数据（须为正方形）。
    /// 返回：裁剪后的 PNG 字节数据，失败返回 ImageUtilsError。
    pub fn trim_transparent_white_border(png_data: Vec<u8>) -> Result<Vec<u8>, ImageUtilsError> {
        let img = image::load_from_memory(&png_data).map_err(|e| {
            ImageUtilsError::ProcessingError(format!("Failed to load image from memory: {}", e))
        })?;

        let width = img.width();
        let height = img.height();

        if width != height {
            return Err(ImageUtilsError::ProcessingError(format!(
                "Input image is not square: {}x{}",
                width, height
            )));
        }

        let mut border_width = 0;
        let size = width;

        'outer: for layer in 0..size / 2 {
            for x in layer..size - layer {
                let pixel = img.get_pixel(x, layer);
                if !Self::is_white_or_transparent(pixel) {
                    break 'outer;
                }
            }

            for y in layer..size - layer {
                let pixel = img.get_pixel(size - 1 - layer, y);
                if !Self::is_white_or_transparent(pixel) {
                    break 'outer;
                }
            }

            for x in layer..size - layer {
                let pixel = img.get_pixel(x, size - 1 - layer);
                if !Self::is_white_or_transparent(pixel) {
                    break 'outer;
                }
            }

            for y in layer..size - layer {
                let pixel = img.get_pixel(layer, y);
                if !Self::is_white_or_transparent(pixel) {
                    break 'outer;
                }
            }

            border_width = layer + 1;
        }

        if border_width >= size / 2 {
            return Ok(png_data);
        }

        let new_size = size - 2 * border_width;
        let mut new_img = ImageBuffer::new(new_size, new_size);

        for y in 0..new_size {
            for x in 0..new_size {
                let pixel = img.get_pixel(x + border_width, y + border_width);
                new_img.put_pixel(x, y, pixel);
            }
        }

        let mut output = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut output);
        encoder
            .write_image(
                &new_img.into_raw(),
                new_size,
                new_size,
                image::ColorType::Rgba8.into(),
            )
            .map_err(|e| {
                ImageUtilsError::ProcessingError(format!("Failed to encode trimmed image: {}", e))
            })?;

        Ok(output)
    }

    /// 判断像素是否为白色或透明。
    fn is_white_or_transparent(pixel: Rgba<u8>) -> bool {
        pixel[3] < 10 || (pixel[0] > 245 && pixel[1] > 245 && pixel[2] > 245)
    }

    /// 将 RGBA 图像数据编码为 PNG 格式。
    /// 参数：rgba_image - RGBA 格式图像缓冲区。
    /// 返回：PNG 字节数据，失败返回 ImageUtilsError。
    pub fn rgba_image_to_png(rgba_image: &RgbaImage) -> Result<Vec<u8>, ImageUtilsError> {
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        rgba_image
            .write_to(&mut cursor, ImageFormat::Png)
            .map_err(|e| {
                ImageUtilsError::ProcessingError(format!(
                    "Failed to encode RGBA image as PNG: {}",
                    e
                ))
            })?;
        Ok(buffer)
    }

    /// 获取图片的主色调。
    /// 使用 K-Means 聚类算法分析图片中非透明像素的颜色分布。
    /// 参数：image_data - PNG 格式图片字节数据。
    /// 返回：(R, G, B) 主色调元组，失败返回 ImageUtilsError。
    pub async fn get_dominant_color(image_data: Vec<u8>) -> Result<(u8, u8, u8), ImageUtilsError> {
        tokio::task::spawn_blocking(move || -> Result<(u8, u8, u8), ImageUtilsError> {
            let img = image::load_from_memory(&image_data).map_err(|e| {
                ImageUtilsError::ProcessingError(format!(
                    "Failed to load image for color analysis: {}",
                    e
                ))
            })?;
            let rgba_img = img.to_rgba8();

            let pixels: Vec<[u8; 3]> = rgba_img
                .pixels()
                .par_bridge()
                .filter_map(|pixel| {
                    if pixel[3] != 0 {
                        Some([pixel[0], pixel[1], pixel[2]])
                    } else {
                        None
                    }
                })
                .collect();

            if pixels.is_empty() {
                return Err(ImageUtilsError::ProcessingError(
                    "No visible pixels found in image for color analysis".to_string(),
                ));
            }

            let lab_samples: Vec<Lab> = pixels
                .par_iter()
                .map(|&rgb| Srgb::from(rgb).into_format::<f32>().into_color())
                .collect();

            let cluster_count = 5;
            let max_iterations = 20;
            let tolerance = 1.0;
            let runs = 3;
            let verbose = false;

            let lab_samples_arc = Arc::new(lab_samples);
            let best_result = (0..runs)
                .into_par_iter()
                .map(|_| {
                    let seed = rand::rng().random::<u64>();
                    let samples = Arc::clone(&lab_samples_arc);
                    get_kmeans(
                        cluster_count,
                        max_iterations,
                        tolerance,
                        verbose,
                        &samples,
                        seed,
                    )
                })
                .min_by(|a, b| {
                    a.score
                        .partial_cmp(&b.score)
                        .expect("Score comparison should not fail")
                })
                .expect("Should have at least one clustering result");

            let cluster_counts = {
                let counts = vec![0; cluster_count];
                let counts_mutex = Arc::new(Mutex::new(counts));

                best_result.indices.par_iter().for_each(|&i| {
                    let mut counts = counts_mutex.lock().expect("Mutex should not be poisoned");
                    counts[i as usize] += 1;
                });

                Arc::try_unwrap(counts_mutex)
                    .expect("Arc should have only one reference")
                    .into_inner()
                    .expect("Mutex should not be poisoned")
            };

            let (dominant_idx, _) = cluster_counts
                .iter()
                .enumerate()
                .max_by_key(|&(_, count)| count)
                .expect("Should have at least one cluster count");

            let dominant_lab = best_result.centroids[dominant_idx];
            let srgb: Srgb = dominant_lab.into_color();
            let rgb = srgb.into_format::<u8>();
            let (r, g, b) = rgb.into_components();

            info!("Dominant color analysis complete: RGB({}, {}, {})", r, g, b);
            Ok((r, g, b))
        })
        .await
        .map_err(|e| ImageUtilsError::TaskJoinError(format!("Task join error: {}", e)))?
    }
}
