use crate::common::image_utils::ImageUtils;
use crate::host::cache_level::CacheLevel;
use crate::host::error::HostApiError;
use crate::services::icon::icon_cache::IconCacheService;
use crate::services::icon_request::IconRequest;
use async_trait::async_trait;

/// 图标提取器 trait，定义平台原语与跨平台业务默认实现。
/// 平台实现者只需实现 6 个原语方法，业务逻辑由默认实现提供。
/// 默认实现可按需覆盖。
#[async_trait]
pub trait IconExtractor: Send + Sync {
    // ===== 平台原语（各平台必须实现）=====

    /// 从本地文件路径提取图标，返回 PNG 格式字节数据。
    /// 参数：path - 文件路径（exe, lnk, url, ico, png 等）。
    /// 返回：PNG 格式图标字节数据，失败返回 HostApiError。
    async fn extract_from_path(&self, path: &str) -> Result<Vec<u8>, HostApiError>;

    /// 从网址提取图标（favicon），返回 PNG 格式字节数据。
    /// 参数：url - 网址。
    /// 返回：PNG 格式图标字节数据，失败返回 HostApiError。
    async fn extract_from_url(&self, url: &str) -> Result<Vec<u8>, HostApiError>;

    /// 从文件扩展名提取系统关联图标，返回 PNG 格式字节数据。
    /// 参数：ext - 文件扩展名（如 ".txt", ".doc"）。
    /// 返回：PNG 格式图标字节数据，失败返回 HostApiError。
    async fn extract_from_extension(&self, ext: &str) -> Result<Vec<u8>, HostApiError>;

    /// 获取默认应用图标的文件路径。
    /// 参数：无。
    /// 返回：默认应用图标路径字符串。
    fn default_app_icon_path(&self) -> &str;

    /// 获取默认网址图标的文件路径。
    /// 参数：无。
    /// 返回：默认网址图标路径字符串。
    fn default_web_icon_path(&self) -> &str;

    /// 检测当前平台网络是否可用。
    /// 参数：无。
    /// 返回：网络可用返回 true。
    fn is_network_available(&self) -> bool;

    // ===== 跨平台业务逻辑（默认实现）=====

    /// 根据 IconRequest 提取原始图标数据。
    /// 默认实现根据请求类型分发到对应的平台原语方法。
    /// 参数：request - 图标请求。
    /// 返回：PNG 格式图标字节数据，失败返回 HostApiError。
    async fn extract(&self, request: &IconRequest) -> Result<Vec<u8>, HostApiError> {
        match request {
            IconRequest::Path(p) => self.extract_from_path(p).await,
            IconRequest::Url(u) => self.extract_from_url(u).await,
            IconRequest::Extension(e) => self.extract_from_extension(e).await,
            IconRequest::Data(data) => decode_data_url(data),
        }
    }

    /// 提取图标并应用后处理：非位图先光栅化 → 裁剪白边 → 等比缩放到 128×128 上限 → WebP 无损编码（VP8L）。
    /// 参数：request - 图标请求。
    /// 返回：处理后的 WebP 格式图标字节数据（失败回退位图原始字节，消费方按字节头嗅探），提取失败返回 HostApiError。
    async fn extract_and_process(&self, request: &IconRequest) -> Result<Vec<u8>, HostApiError> {
        const MAX_ICON_SIZE: u32 = 128;
        let data = self.extract(request).await?;
        // 0. 非位图输入（插件图标常以 SVG 矢量提供）先光栅化为 PNG
        //    后续裁剪/缩放/编码都是位图链路，矢量字节会在每一步失败并原样透出，
        //    最终按位图 MIME 编码的 data URL 消费方无法解码（图标显示为破图）。
        let data = if is_raster_icon(&data) {
            data
        } else {
            ImageUtils::convert_image_to_png(data.clone())
                .await
                .unwrap_or(data)
        };
        // 1. 裁剪透明/白边（失败回退原数据）
        let trimmed = ImageUtils::trim_transparent_white_border(data.clone()).unwrap_or(data);
        // 2. 超过 128×128 时等比缩放到 128（失败回退裁剪产物）
        let resized = ImageUtils::resize_image(trimmed.clone(), MAX_ICON_SIZE, MAX_ICON_SIZE)
            .await
            .unwrap_or(trimmed);
        // 3. WebP 无损编码（失败回退原始字节，MIME 侧按字节头嗅探兜底）
        Ok(ImageUtils::to_webp(resized.clone()).unwrap_or(resized))
    }

    /// 加载默认图标。
    /// 默认实现：URL 类型加载默认网址图标，其他类型加载默认应用图标。
    /// 参数：request - 图标请求（用于判断类型）。
    /// 返回：默认图标的 WebP 字节数据，读取失败返回空 Vec。
    async fn load_default_icon(&self, request: &IconRequest) -> Vec<u8> {
        let default_path = match request {
            IconRequest::Url(_) => self.default_web_icon_path(),
            _ => self.default_app_icon_path(),
        };
        let png = tokio::fs::read(default_path).await.unwrap_or_default();
        if png.is_empty() {
            return png;
        }
        // 与提取路径一致编码为 WebP，统一返回字节格式
        ImageUtils::to_webp(png.clone()).unwrap_or(png)
    }

    /// 完整的图标获取流程，包含缓存策略。
    /// 默认实现：根据 CacheLevel 执行 L1 → L2 → 提取 → 写回缓存，提取失败返回默认图标。
    /// 缓存命中还会校验内容为可识别位图，历史版本写入的非位图条目按未命中处理并重新提取。
    /// 参数：cache - 图标缓存服务；request - 图标请求；level - 缓存等级。
    /// 返回：WebP 格式图标字节数据（回退路径为原始字节，消费方按字节头嗅探 MIME），失败返回 HostApiError。
    async fn get_icon(
        &self,
        cache: &IconCacheService,
        request: &IconRequest,
        level: CacheLevel,
    ) -> Result<Vec<u8>, HostApiError> {
        let hash_key = request.get_hash_string() + ".webp";

        // 1. 根据缓存等级查缓存
        if level != CacheLevel::SkipAll {
            // L1 查询
            if level == CacheLevel::Full {
                if let Some(data) = cache.get_l1(&hash_key) {
                    if is_raster_icon(&data) {
                        return Ok(data);
                    }
                }
            }

            // L2 查询
            if cache.contains_l2(&hash_key) {
                if let Some(data) = cache.get_l2(&hash_key).await {
                    if !is_raster_icon(&data) {
                        // 非位图内容视为未命中，走提取链路覆盖写回
                        return extract_and_cache(self, cache, request, level, &hash_key).await;
                    }
                    // L2 命中时回填 L1
                    if level == CacheLevel::Full {
                        cache.set_l1(&hash_key, data.clone());
                    }
                    return Ok(data);
                }
            }
        }

        extract_and_cache(self, cache, request, level, &hash_key).await
    }

    /// 强制从磁盘提取图标并更新缓存（跳过缓存读取）。
    /// 默认实现：直接提取 → 写回缓存。
    /// 参数：cache - 图标缓存服务；request - 图标请求；level - 缓存等级。
    /// 返回：WebP 格式图标字节数据，提取失败返回 HostApiError。
    async fn get_icon_and_update_cache(
        &self,
        cache: &IconCacheService,
        request: &IconRequest,
        level: CacheLevel,
    ) -> Result<Vec<u8>, HostApiError> {
        let hash_key = request.get_hash_string() + ".webp";
        let data = self.extract_and_process(request).await?;
        write_back_cache(cache, &hash_key, &data, level).await;
        Ok(data)
    }
}

/// 根据缓存等级将图标数据写回缓存。
/// Full: 写入 L1 + L2；SkipMemory: 只写 L2；SkipAll: 不写。
async fn write_back_cache(
    cache: &IconCacheService,
    hash_key: &str,
    icon_data: &[u8],
    level: CacheLevel,
) {
    if level == CacheLevel::Full {
        cache.set_l1(hash_key, icon_data.to_vec());
    }

    if level == CacheLevel::Full || level == CacheLevel::SkipMemory {
        cache.set_l2(hash_key, icon_data.to_vec()).await;
    }
}

/// 提取并后处理图标，成功后按缓存等级写回；失败或结果为空时回退默认图标。
/// 参数：extractor - 图标提取器；cache - 图标缓存服务；request - 图标请求；level - 缓存等级；hash_key - 缓存键。
/// 返回：图标字节数据（WebP，回退路径为位图原始字节）。
async fn extract_and_cache<E: IconExtractor + ?Sized>(
    extractor: &E,
    cache: &IconCacheService,
    request: &IconRequest,
    level: CacheLevel,
    hash_key: &str,
) -> Result<Vec<u8>, HostApiError> {
    let data = match extractor.extract_and_process(request).await {
        Ok(d) if !d.is_empty() => d,
        _ => return Ok(extractor.load_default_icon(request).await),
    };

    write_back_cache(cache, hash_key, &data, level).await;

    Ok(data)
}

/// 判断图标字节是否为可识别位图（image crate 的字节头嗅探）。
/// 缓存键固定以 .webp 结尾，但历史版本可能写入未光栅化的 SVG 等非位图字节，
/// 命中此类条目必须按未命中处理并重新提取。
/// 参数：data - 图标字节数据。
/// 返回：true 表示是可识别位图。
fn is_raster_icon(data: &[u8]) -> bool {
    image::guess_format(data).is_ok()
}

/// 解码 data URL 或纯 base64 为原始字节（IconRequest::Data 直通路径）。
/// 参数：data - "data:<mime>;base64,<payload>" 形式或纯 base64 字符串。
/// 返回：解码后的字节；格式不合法返回 HostApiError。
fn decode_data_url(data: &str) -> Result<Vec<u8>, HostApiError> {
    let payload = data.rsplit_once(";base64,").map(|(_, p)| p).unwrap_or(data);
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, payload).map_err(|e| {
        HostApiError::IconExtractionFailed {
            request: "data".to_string(),
            reason: format!("data URL 解码失败: {}", e),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 最小可渲染 SVG（第三方插件图标常见形态）。
    const SVG_SOURCE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#2f7fd8"/></svg>"##;

    /// 以 data URL 形式提供 SVG，与插件候选图标下发形态一致。
    fn svg_data_url() -> String {
        use base64::Engine;
        format!(
            "data:image/svg+xml;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(SVG_SOURCE)
        )
    }

    /// 最小提取器：平台原语不可用，图标只经 IconRequest::Data 直通。
    struct StubExtractor;

    #[async_trait]
    impl IconExtractor for StubExtractor {
        async fn extract_from_path(&self, _path: &str) -> Result<Vec<u8>, HostApiError> {
            Err(HostApiError::IconExtractionFailed {
                request: "path".to_string(),
                reason: "stub".to_string(),
            })
        }

        async fn extract_from_url(&self, _url: &str) -> Result<Vec<u8>, HostApiError> {
            Err(HostApiError::IconExtractionFailed {
                request: "url".to_string(),
                reason: "stub".to_string(),
            })
        }

        async fn extract_from_extension(&self, _ext: &str) -> Result<Vec<u8>, HostApiError> {
            Err(HostApiError::IconExtractionFailed {
                request: "extension".to_string(),
                reason: "stub".to_string(),
            })
        }

        fn default_app_icon_path(&self) -> &str {
            ""
        }

        fn default_web_icon_path(&self) -> &str {
            ""
        }

        fn is_network_available(&self) -> bool {
            false
        }
    }

    /// SVG 图标必须光栅化为位图，否则 data URL 只能按位图 MIME 标注 → 消费方解码失败（破图）。
    #[tokio::test]
    async fn svg_icon_is_rasterized_to_bitmap() {
        let processed = StubExtractor
            .extract_and_process(&IconRequest::Data(svg_data_url()))
            .await
            .expect("SVG 图标应能完成提取与后处理");

        let format = image::guess_format(&processed).expect("处理结果必须是可识别位图");
        assert!(
            matches!(format, image::ImageFormat::WebP | image::ImageFormat::Png),
            "SVG 应被光栅化，实际格式: {format:?}"
        );
    }

    /// 历史版本可能把非位图字节写进 .webp 缓存条目，命中后必须重新提取而非直接返回。
    #[tokio::test]
    async fn non_raster_cache_entry_is_re_extracted() {
        let dir = tempfile::tempdir().expect("创建临时缓存目录");
        let cache = IconCacheService::new(dir.path().to_string_lossy().into_owned());
        cache.init();

        let request = IconRequest::Data(svg_data_url());
        cache
            .set_l2(
                &(request.get_hash_string() + ".webp"),
                SVG_SOURCE.as_bytes().to_vec(),
            )
            .await;

        let data = StubExtractor
            .get_icon(&cache, &request, CacheLevel::Full)
            .await
            .expect("应忽略陈旧条目并重新提取");

        assert!(
            image::guess_format(&data).is_ok(),
            "返回内容必须是可识别位图，而不是缓存中的 SVG 文本"
        );
    }
}
