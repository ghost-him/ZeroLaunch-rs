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
        }
    }

    /// 提取图标并应用后处理（裁剪白边）。
    /// 默认实现：提取 → 裁剪透明白边，裁剪失败时返回原始数据。
    /// 参数：request - 图标请求。
    /// 返回：处理后的 PNG 格式图标字节数据，提取失败返回 HostApiError。
    async fn extract_and_process(&self, request: &IconRequest) -> Result<Vec<u8>, HostApiError> {
        let data = self.extract(request).await?;
        Ok(ImageUtils::trim_transparent_white_border(data.clone()).unwrap_or(data))
    }

    /// 加载默认图标。
    /// 默认实现：URL 类型加载默认网址图标，其他类型加载默认应用图标。
    /// 参数：request - 图标请求（用于判断类型）。
    /// 返回：默认图标的 PNG 字节数据，读取失败返回空 Vec。
    async fn load_default_icon(&self, request: &IconRequest) -> Vec<u8> {
        let default_path = match request {
            IconRequest::Url(_) => self.default_web_icon_path(),
            _ => self.default_app_icon_path(),
        };
        tokio::fs::read(default_path).await.unwrap_or_default()
    }

    /// 完整的图标获取流程，包含缓存策略。
    /// 默认实现：根据 CacheLevel 执行 L1 → L2 → 提取 → 写回缓存，提取失败返回默认图标。
    /// 参数：cache - 图标缓存服务；request - 图标请求；level - 缓存等级。
    /// 返回：PNG 格式图标字节数据，失败返回 HostApiError。
    async fn get_icon(
        &self,
        cache: &IconCacheService,
        request: &IconRequest,
        level: CacheLevel,
    ) -> Result<Vec<u8>, HostApiError> {
        let hash_key = request.get_hash_string() + ".png";

        // 1. 根据缓存等级查缓存
        if level != CacheLevel::SkipAll {
            // L1 查询
            if level == CacheLevel::Full {
                if let Some(data) = cache.get_l1(&hash_key) {
                    return Ok(data);
                }
            }

            // L2 查询
            if cache.contains_l2(&hash_key) {
                if let Some(data) = cache.get_l2(&hash_key).await {
                    // L2 命中时回填 L1
                    if level == CacheLevel::Full {
                        cache.set_l1(&hash_key, data.clone());
                    }
                    return Ok(data);
                }
            }
        }

        // 2. 缓存未命中 → 提取 + 后处理
        let data = match self.extract_and_process(request).await {
            Ok(d) if !d.is_empty() => d,
            _ => {
                // 3. 提取失败 → 默认图标
                return Ok(self.load_default_icon(request).await);
            }
        };

        // 4. 写回缓存
        write_back_cache(cache, &hash_key, &data, level).await;

        Ok(data)
    }

    /// 强制从磁盘提取图标并更新缓存（跳过缓存读取）。
    /// 默认实现：直接提取 → 写回缓存。
    /// 参数：cache - 图标缓存服务；request - 图标请求；level - 缓存等级。
    /// 返回：PNG 格式图标字节数据，提取失败返回 HostApiError。
    async fn get_icon_and_update_cache(
        &self,
        cache: &IconCacheService,
        request: &IconRequest,
        level: CacheLevel,
    ) -> Result<Vec<u8>, HostApiError> {
        let hash_key = request.get_hash_string() + ".png";
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
