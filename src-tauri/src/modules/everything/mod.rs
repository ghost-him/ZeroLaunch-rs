use crate::utils::defer::defer;
use everything_rs::{Everything, EverythingRequestFlags, EverythingSort};
use parking_lot::RwLock;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::warn;

pub mod config;
use config::EverythingConfig;

#[derive(Debug, Clone)]
pub struct EverythingSearchResult {
    pub id: u64,
    pub path: String,
}

/// 内部逻辑实现（非线程安全），包含真实字段与方法实现
#[derive(Debug)]
pub struct EverythingManagerInner {
    // 当前是不是正在搜索（防止出现多次快速搜索的情况）
    is_searching: AtomicBool,
    // 排序阈值
    pub sort_threshold: usize,
    // 排序方式
    pub sort_method: EverythingSort,
    // 结果限制
    pub result_limit: usize,
}

impl Default for EverythingManagerInner {
    fn default() -> Self {
        Self::new()
    }
}

impl EverythingManagerInner {
    pub fn new() -> Self {
        Self {
            is_searching: AtomicBool::new(false),
            sort_threshold: EverythingConfig::default().get_sort_threshold(),
            sort_method: EverythingConfig::default().get_sort_method().into(),
            result_limit: EverythingConfig::default().get_result_limit(),
        }
    }

    pub fn load_from_config(&mut self, config: Arc<EverythingConfig>) {
        self.sort_threshold = config.get_sort_threshold();
        self.sort_method = config.get_sort_method().into();
        self.result_limit = config.get_result_limit();
    }

    pub fn search(&self, query: &str) -> Result<Vec<EverythingSearchResult>, String> {
        if self
            .is_searching
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            panic!("Everything search is already in progress! Frontend should handle throttling.");
        }

        // Ensure is_searching is reset when this scope exits
        let is_searching_flag = &self.is_searching;
        let _guard = defer(move || {
            is_searching_flag.store(false, Ordering::SeqCst);
        });

        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let everything = Everything::new();
        everything.set_search(query);
        everything.set_request_flags(
            EverythingRequestFlags::FullPathAndFileName | EverythingRequestFlags::Extension,
        );
        everything.set_max_results(self.result_limit as u32);

        // Use sort_threshold from config
        if query.len() >= self.sort_threshold {
            everything.set_sort(self.sort_method);
        }

        everything
            .query()
            .map_err(|e| format!("Everything query failed: {:?}", e))?;

        let mut results = Vec::new();
        for (index, mut path) in everything.full_path_iter().flatten().enumerate() {
            let extension = everything
                .get_result_extension(index as u32)
                .unwrap_or_default();
            path.push_str(&extension);
            let mut hasher = DefaultHasher::new();
            path.hash(&mut hasher);
            let id: u64 = hasher.finish();
            results.push(EverythingSearchResult { id, path });
        }
        Ok(results)
    }

    pub fn launch(&self, path: &str) {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let result = std::process::Command::new("cmd")
            .args(["/C", "start", "", path])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();

        if let Err(e) = result {
            warn!("Everything 启动文件失败: {:?}, 路径: {}", e, path);
        }
    }
}

#[derive(Clone, Debug)]
pub struct EverythingManager {
    inner: Arc<RwLock<EverythingManagerInner>>,
}

impl Default for EverythingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EverythingManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(EverythingManagerInner::new())),
        }
    }

    pub fn load_from_config(&self, config: Arc<EverythingConfig>) {
        let mut inner = self.inner.write();
        inner.load_from_config(config);
    }

    pub fn search(&self, query: &str) -> Result<Vec<EverythingSearchResult>, String> {
        let inner = self.inner.read();
        inner.search(query)
    }

    pub fn launch(&self, path: &str) {
        let inner = self.inner.read();
        inner.launch(path);
    }
}
