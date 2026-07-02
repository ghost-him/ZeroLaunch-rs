use crate::common::dir_utils::DirUtils;
use dashmap::{DashMap, DashSet};
use parking_lot::RwLock;
use std::path::Path;
use tracing::warn;

/// 图标缓存服务，提供 L1 内存缓存 + L2 文件缓存的原子操作原语。
/// 本服务是纯缓存工具，不知道图标如何提取，也不知道 IconRequest/CacheLevel 等业务语义。
/// 业务逻辑由 IconExtractor trait 的默认实现提供，本服务仅提供存取能力。
pub struct IconCacheService {
    /// L1: 内存缓存，key 为文件名（如 "abc123.png"），value 为 PNG 字节数据
    memory_cache: DashMap<String, Vec<u8>>,
    /// L2: 文件缓存目录路径
    cache_dir: RwLock<String>,
    /// L2: 已缓存的文件名集合（启动时扫描填充，运行时追加）
    cached_file_hashes: DashSet<String>,
}

impl IconCacheService {
    /// 创建图标缓存服务实例。
    /// 参数：cache_dir - L2 文件缓存目录路径。
    /// 返回：初始化后的 IconCacheService（尚未扫描 L2 缓存，需调用 init()）。
    pub fn new(cache_dir: String) -> Self {
        Self {
            memory_cache: DashMap::new(),
            cache_dir: RwLock::new(cache_dir),
            cached_file_hashes: DashSet::new(),
        }
    }

    /// 初始化缓存服务，扫描 L2 文件缓存目录填充 cached_file_hashes。
    /// 参数：无。
    /// 返回：无。
    /// 特性：必须在 new() 之后、首次缓存操作之前调用。
    pub fn init(&self) {
        let cache_dir = self.cache_dir.read().clone();
        let hashes = Self::scan_cached_icons(&cache_dir);
        for hash in hashes {
            self.cached_file_hashes.insert(hash);
        }
    }

    // ===== L1 内存缓存操作 =====

    /// 从 L1 内存缓存获取数据。
    /// 参数：key - 缓存键（文件名，如 "abc123.png"）。
    /// 返回：命中返回 Some(数据)，未命中返回 None。
    pub fn get_l1(&self, key: &str) -> Option<Vec<u8>> {
        self.memory_cache
            .get(key)
            .map(|entry| entry.value().clone())
    }

    /// 写入 L1 内存缓存。
    /// 参数：key - 缓存键；data - 图标 PNG 数据。
    /// 返回：无。
    pub fn set_l1(&self, key: &str, data: Vec<u8>) {
        self.memory_cache.insert(key.to_string(), data);
    }

    /// 清空 L1 内存缓存。
    /// 参数：无。
    /// 返回：无。
    pub fn clear_l1(&self) {
        self.memory_cache.clear();
    }

    // ===== L2 文件缓存操作 =====

    /// 检查 L2 文件缓存中是否包含指定键。
    /// 参数：key - 缓存键（文件名）。
    /// 返回：存在返回 true。
    pub fn contains_l2(&self, key: &str) -> bool {
        self.cached_file_hashes.contains(key)
    }

    /// 从 L2 文件缓存读取数据。
    /// 参数：key - 缓存键（文件名）。
    /// 返回：成功返回 Some(数据)，失败或不存在返回 None。
    pub async fn get_l2(&self, key: &str) -> Option<Vec<u8>> {
        let cache_dir = self.cache_dir.read().clone();
        let icon_path = Path::new(&cache_dir).join(key);
        tokio::fs::read(&icon_path).await.ok()
    }

    /// 异步写入 L2 文件缓存。
    /// 参数：key - 缓存键（文件名）；data - 图标 PNG 数据。
    /// 返回：无。
    /// 特性：写入操作在后台线程执行，不阻塞调用方；同时更新 cached_file_hashes。
    pub async fn set_l2(&self, key: &str, data: Vec<u8>) {
        let cache_dir = self.cache_dir.read().clone();
        let icon_path = Path::new(&cache_dir).join(key);
        let key_owned = key.to_string();

        if let Err(e) = tokio::fs::write(&icon_path, data).await {
            warn!("写入图标缓存失败 {:?}: {}", icon_path, e);
        } else {
            self.cached_file_hashes.insert(key_owned);
        }
    }

    // ===== 管理操作 =====

    /// 更新图标缓存目录路径，同时清空 L1 内存缓存并重新扫描 L2。
    /// 参数：new_cache_dir - 新的图标文件缓存目录路径。
    /// 返回：无。
    pub fn update_cache_dir(&self, new_cache_dir: &str) {
        self.memory_cache.clear();

        {
            let mut cache_dir = self.cache_dir.write();
            *cache_dir = new_cache_dir.to_string();
        }

        self.cached_file_hashes.clear();
        let hashes = Self::scan_cached_icons(new_cache_dir);
        for hash in hashes {
            self.cached_file_hashes.insert(hash);
        }
    }

    /// 扫描缓存目录，获取所有已缓存图标的文件名集合。
    /// 参数：cache_dir - 缓存目录路径。
    /// 返回：文件名集合。
    fn scan_cached_icons(cache_dir: &str) -> DashSet<String> {
        let result = DashSet::new();
        match DirUtils::read_dir_or_create(cache_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let file_name = file_name.to_string_lossy();
                    result.insert(file_name.into_owned());
                }
            }
            Err(e) => warn!("扫描图标缓存目录失败: {}", e),
        }
        result
    }
}
