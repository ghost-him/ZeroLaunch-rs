use crate::utils::defer::defer;
use everything_rs::{Everything, EverythingRequestFlags, EverythingSort};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::warn;

#[derive(Debug)]
pub struct EverythingManager {
    is_searching: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct EverythingSearchResult {
    pub id: u64,
    pub path: String,
}

impl Default for EverythingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EverythingManager {
    pub fn new() -> Self {
        Self {
            is_searching: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<EverythingSearchResult>, String> {
        if self
            .is_searching
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            panic!("Everything search is already in progress! Frontend should handle throttling.");
        }

        // Ensure is_searching is reset when this scope exits
        let is_searching_guard = self.is_searching.clone();
        let _guard = defer(move || {
            is_searching_guard.store(false, Ordering::SeqCst);
        });

        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let everything = Everything::new();
        everything.set_search(query);
        everything.set_request_flags(EverythingRequestFlags::FullPathAndFileName);
        if query.len() > 4 {
            everything.set_sort(EverythingSort::NameAscending);
        }

        everything
            .query()
            .map_err(|e| format!("Everything query failed: {:?}", e))?;

        let mut results = Vec::new();
        for path in everything.full_path_iter().flatten().take(limit) {
            let mut hasher = DefaultHasher::new();
            path.hash(&mut hasher);
            let id = hasher.finish();
            results.push(EverythingSearchResult { id, path });
        }
        Ok(results)
    }

    pub fn launch(&self, path: &str) {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // 使用 cmd /C start "" "path" 来启动文件，这会使用系统默认关联的程序打开文件
        // 这种方式与 ProgramLauncher 中的 LaunchMethod::File 逻辑一致
        let result = std::process::Command::new("cmd")
            .args(["/C", "start", "", path])
            .creation_flags(CREATE_NO_WINDOW) // 隐藏命令窗口
            .spawn();

        if let Err(e) = result {
            warn!("Everything 启动文件失败: {:?}, 路径: {}", e, path);
        }
    }
}
