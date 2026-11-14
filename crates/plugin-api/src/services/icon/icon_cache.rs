use crate::common::dir_utils::DirUtils;
use dashmap::{DashMap, DashSet};
use parking_lot::RwLock;
use std::path::Path;
use tracing::warn;

/// 失败记录文件名（位于 L2 图标缓存目录内）。
const FAILURES_FILE: &str = "failures.json";

/// 图标缓存服务，提供 L1 内存缓存 + L2 文件缓存的原子操作原语。
/// 本服务是纯缓存工具，不知道图标如何提取，也不知道 IconRequest/CacheLevel 等业务语义。
/// 业务逻辑由 IconExtractor trait 的默认实现提供，本服务仅提供存取能力。
pub struct IconCacheService {
    /// L1: 内存缓存，key 为文件名（如 "abc123.webp"），value 为 WebP 字节数据
    memory_cache: DashMap<String, Vec<u8>>,
    /// L2: 文件缓存目录路径
    cache_dir: RwLock<String>,
    /// L2: 已缓存的文件名集合（启动时扫描填充，运行时追加）
    cached_file_hashes: DashSet<String>,
    /// 提取失败记录（键同缓存键）：命中即跳过联网抓取，直到手动重试。
    /// 与图标字节同目录持久化为 `failures.json`，重启后仍生效。
    failures: DashSet<String>,
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
            failures: DashSet::new(),
        }
    }

    /// 初始化缓存服务，扫描 L2 文件缓存目录填充 cached_file_hashes，并载入失败记录。
    /// 参数：无。
    /// 返回：无。
    /// 特性：必须在 new() 之后、首次缓存操作之前调用。
    pub fn init(&self) {
        let cache_dir = self.cache_dir.read().clone();
        let hashes = Self::scan_cached_icons(&cache_dir);
        for hash in hashes {
            self.cached_file_hashes.insert(hash);
        }
        self.load_failures();
    }

    /// 预载 L2 文件缓存到 L1 内存。
    /// 参数：无。
    /// 返回：无。
    /// 特性：并行读取，全部完成后返回；失败条目静默跳过（查询路径会回退到按需读 L2）。
    pub async fn preload_l1(&self) {
        let cache_dir = self.cache_dir.read().clone();
        let keys: Vec<String> = self
            .cached_file_hashes
            .iter()
            .map(|entry| entry.clone())
            .collect();
        let mut handles = Vec::with_capacity(keys.len());
        for key in keys {
            let dir = cache_dir.clone();
            let key_clone = key.clone();
            handles.push(tokio::spawn(async move {
                let data = tokio::fs::read(Path::new(&dir).join(&key_clone)).await.ok();
                (key, data)
            }));
        }
        for handle in handles {
            if let Ok((key, Some(data))) = handle.await {
                self.memory_cache.insert(key, data);
            }
        }
        let (count, bytes): (usize, usize) = self
            .memory_cache
            .iter()
            .map(|e| (1, e.value().len()))
            .fold((0, 0), |(c, b), (cc, bb)| (c + cc, b + bb));
        tracing::info!(
            "图标缓存预载完成: {} 个, {:.1} KB",
            count,
            bytes as f64 / 1024.0
        );
    }

    // ===== L1 内存缓存操作 =====

    /// 从 L1 内存缓存获取数据。
    /// 参数：key - 缓存键（文件名，如 "abc123.webp"）。
    /// 返回：命中返回 Some(数据)，未命中返回 None。
    pub fn get_l1(&self, key: &str) -> Option<Vec<u8>> {
        self.memory_cache
            .get(key)
            .map(|entry| entry.value().clone())
    }

    /// 写入 L1 内存缓存。
    /// 参数：key - 缓存键；data - 图标 WebP 数据。
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

    /// 移除 L1 内存缓存中的指定键（用于图标覆盖后强制刷新缓存）。
    /// 参数：key - 缓存键（文件名，如 "abc123.webp"）。
    /// 返回：无。
    pub fn remove_l1(&self, key: &str) {
        self.memory_cache.remove(key);
    }

    // ===== L2 文件缓存操作 =====

    /// 检查 L2 文件缓存中是否包含指定键。
    /// 参数：key - 缓存键（文件名）。
    /// 返回：存在返回 true。
    pub fn contains_l2(&self, key: &str) -> bool {
        self.cached_file_hashes.contains(key)
    }

    // ===== 失败记录（网络类提取失败）=====

    /// 从 L2 目录读取失败记录填充内存表。
    /// 参数：无。
    /// 返回：无。
    /// 特性：文件不存在或解析失败时按空表处理。
    fn load_failures(&self) {
        self.failures.clear();
        let path = Path::new(&*self.cache_dir.read()).join(FAILURES_FILE);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(e) => {
                warn!("读取图标失败记录失败: {}", e);
                return;
            }
        };
        match serde_json::from_slice::<Vec<String>>(&bytes) {
            Ok(keys) => {
                for key in keys {
                    self.failures.insert(key);
                }
            }
            Err(e) => warn!("解析图标失败记录失败: {}", e),
        }
    }

    /// 将失败记录写入 L2 目录；仅在集合变化时调用，文件很小。
    /// 参数：无。
    /// 返回：无。
    fn save_failures(&self) {
        let path = Path::new(&*self.cache_dir.read()).join(FAILURES_FILE);
        let mut keys: Vec<String> = self.failures.iter().map(|entry| entry.clone()).collect();
        keys.sort();
        match serde_json::to_vec(&keys) {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(&path, bytes) {
                    warn!("写入图标失败记录失败: {}", e);
                }
            }
            Err(e) => warn!("序列化图标失败记录失败: {}", e),
        }
    }

    /// 判断该键是否已记录失败。
    /// 参数：key - 缓存键。
    /// 返回：true 表示命中失败记录，调用方应直接回退默认图标、不再联网。
    pub fn should_skip(&self, key: &str) -> bool {
        self.failures.contains(key)
    }

    /// 记录一次提取失败并立即落盘。
    /// 参数：key - 缓存键。
    /// 返回：无。
    pub fn mark_failed(&self, key: &str) {
        if self.failures.insert(key.to_string()) {
            self.save_failures();
        }
    }

    /// 清除单条失败记录；记录不存在时不写盘。
    /// 参数：key - 缓存键。
    /// 返回：无。
    pub fn clear_failed(&self, key: &str) {
        if self.failures.remove(key).is_some() {
            self.save_failures();
        }
    }

    /// 清空全部失败记录（设置页整体重试前调用）；表为空时不写盘。
    /// 参数：无。
    /// 返回：无。
    pub fn clear_all_failed(&self) {
        if self.failures.is_empty() {
            return;
        }
        self.failures.clear();
        self.save_failures();
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
    /// 参数：key - 缓存键（文件名）；data - 图标 WebP 数据。
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
        self.load_failures();
    }

    /// 扫描缓存目录，收集当前格式（.webp）文件名集合。
    /// 参数：cache_dir - 缓存目录路径。
    /// 返回：当前格式（.webp）文件名集合。
    fn scan_cached_icons(cache_dir: &str) -> DashSet<String> {
        let result = DashSet::new();
        match DirUtils::read_dir_or_create(cache_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let file_name = file_name.to_string_lossy();
                    if !file_name.ends_with(".webp") {
                        continue;
                    }
                    result.insert(file_name.into_owned());
                }
            }
            Err(e) => warn!("扫描图标缓存目录失败: {}", e),
        }
        result
    }
}
