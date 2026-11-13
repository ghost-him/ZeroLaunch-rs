//! 宿主 embedding 缓存：按 {文本, 供应商, 模型, 维度} 的 SHA-256 键缓存向量化结果。
//!
//! 两层结构：L1 为进程内 LRU（热命中零 IO），L2 为经 PluginHandle::cache_* 落盘的
//! 分片文件（<domain>/<sha前2位>/<sha>.bin）。缓存只存 provider 原始输出向量，
//! 不改变归一化等向量语义；L2 文件损坏/版本不符时删除并视为未命中（自愈）。

use std::sync::Arc;

use lru::LruCache;
use parking_lot::Mutex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::model::ModelEmbeddingRequest;

/// 缓存域（PluginHandle::cache_* 的 domain 参数）。
pub const CACHE_DOMAIN: &str = "model-embedding";
/// L1 内存缓存条数上限。
const L1_CAPACITY: usize = 4096;
/// L2 磁盘缓存条数上限（必须大于 L1：L1 是热数据 LRU，L2 是全量落盘）。
const L2_MAX_ENTRIES: usize = 4096 * 4;
/// L2 文件 magic（ZLEB + version 1，小端 f32）。
const L2_MAGIC: &[u8; 4] = b"ZLEB";
const L2_VERSION: u8 = 1;

/// 缓存键载荷：提供方身份 + 单条输入请求（全字段参与键）。
///
/// 仅限本文件内使用；序列化整个单元素请求，未来新增请求字段自动参与键，
/// 无需在键计算处同步维护字段清单。
#[derive(Serialize)]
struct CacheKeyPayload<'a> {
    /// 提供方身份（如 "openai"），与 model_id 前缀独立：不同提供方的相同 model_id 相互隔离。
    provider: &'a str,
    /// 单条输入请求（input 仅含该条文本及其对应的 task_type/dimensions）。
    request: &'a ModelEmbeddingRequest,
}

/// 计算缓存键：`sha256(serde_json(provider ‖ 单条输入请求))`。
/// 提供方/模型/文本/任务类型/维度任一不同即不同条目；
/// 组装由提供方内部完成，键直接覆盖请求参数全集，无需感知最终输入文本。
pub fn compute_key(provider: &str, single: &ModelEmbeddingRequest) -> [u8; 32] {
    let payload = CacheKeyPayload {
        provider,
        request: single,
    };
    let bytes = serde_json::to_vec(&payload).expect("缓存键载荷序列化不应失败");
    Sha256::digest(&bytes).into()
}

/// 缓存键对应的分片路径（<sha前2位>/<sha>.bin）。
fn l2_rel_key(key: &[u8; 32]) -> String {
    let hex = hex_bytes(key);
    format!("{}/{}.bin", &hex[..2], hex)
}

fn hex_bytes(key: &[u8; 32]) -> String {
    key.iter().map(|b| format!("{b:02x}")).collect()
}

/// 向量序列化：magic + version + dimensions(u32 LE) + f32s(LE)。
fn encode_vector(dimensions: u32, vec: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(9 + vec.len() * 4);
    out.extend_from_slice(L2_MAGIC);
    out.push(L2_VERSION);
    out.extend_from_slice(&dimensions.to_le_bytes());
    for v in vec {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// 向量反序列化；格式不符返回 None（调用方视为未命中并删除）。
fn decode_vector(data: &[u8]) -> Option<(u32, Vec<f32>)> {
    if data.len() < 9 || &data[..4] != L2_MAGIC || data[4] != L2_VERSION {
        return None;
    }
    let dimensions = u32::from_le_bytes(data[5..9].try_into().ok()?);
    let rest = &data[9..];
    if !rest.len().is_multiple_of(4) {
        return None;
    }
    let vec: Vec<f32> = rest
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect();
    if vec.len() != dimensions as usize {
        return None;
    }
    Some((dimensions, vec))
}

/// embedding 向量缓存：L1（LRU）+ L2（PluginHandle::cache_* 落盘）。
pub struct EmbeddingCache {
    handle: Arc<PluginHandle>,
    l1: Mutex<LruCache<[u8; 32], Arc<Vec<f32>>>>,
}

impl EmbeddingCache {
    /// 创建缓存实例；base_dir 提供缓存服务（经 core PluginHandle 的 cache_* 存取）。
    pub fn new(handle: Arc<PluginHandle>) -> Self {
        Self {
            handle,
            l1: Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(L1_CAPACITY).unwrap(),
            )),
        }
    }

    /// 读缓存：L1 命中即返回；L2 命中进 L1；损坏时删除并视为未命中。
    pub async fn get(&self, key: &[u8; 32]) -> Option<Arc<Vec<f32>>> {
        if let Some(vec) = self.l1.lock().get(key) {
            return Some(vec.clone());
        }
        let rel = l2_rel_key(key);
        let data = self.handle.cache_get(CACHE_DOMAIN, &rel).await.ok()??;
        match decode_vector(&data) {
            Some((_dims, vec)) => {
                let arc = Arc::new(vec);
                self.l1.lock().put(*key, arc.clone());
                Some(arc)
            }
            None => {
                let _ = self.handle.cache_delete(CACHE_DOMAIN, &rel).await;
                None
            }
        }
    }

    /// 写缓存：进 L1 并异步落盘（失败仅告警，不影响调用方）。
    pub async fn put(&self, key: &[u8; 32], dimensions: u32, vec: Vec<f32>) {
        let data = encode_vector(dimensions, &vec);
        self.l1.lock().put(*key, Arc::new(vec));
        let handle = self.handle.clone();
        let rel = l2_rel_key(key);
        crate::runtime::spawn(async move {
            if let Err(e) = handle.cache_put(CACHE_DOMAIN, &rel, &data).await {
                tracing::warn!("embedding 缓存落盘失败: {}", e);
                return;
            }
            // 容量控制：L2 超过上限时删除最旧条目（按修改时间），防磁盘无限膨胀。
            if let Err(e) = handle.cache_cleanup(CACHE_DOMAIN, L2_MAX_ENTRIES).await {
                tracing::debug!("embedding L2 容量清理失败: {}", e);
            }
        });
    }
}

/// 扫描 L2 缓存目录，超过 L2_MAX_ENTRIES 时按修改时间删除最旧条目。
/// 目录结构：<cache_root>/<plugin_id>/<domain>/<sha前2>/<sha>.bin。
/// 失败仅告警（容量控制是尽力而为，不影响缓存写入）。
#[cfg(test)]
mod tests {
    use super::*;
    use zerolaunch_plugin_api::services::model::SemanticTask;

    /// 构造单条输入请求（task_type 恒空，单独用例覆盖）。
    fn single_request(
        model_id: &str,
        text: &str,
        dimensions: Option<u32>,
    ) -> ModelEmbeddingRequest {
        ModelEmbeddingRequest {
            model_id: model_id.to_string(),
            input: vec![text.to_string()],
            template_args: None,
            task_type: SemanticTask::RetrievalDocument,
            dimensions,
        }
    }

    #[test]
    fn compute_key_is_sensitive_to_all_segments() {
        let a = compute_key("openai", &single_request("openai/m", "text", Some(256)));
        assert_eq!(
            a,
            compute_key("openai", &single_request("openai/m", "text", Some(256)))
        );
        // 任意一段不同 → 键不同
        assert_ne!(
            a,
            compute_key("openai", &single_request("openai/m", "text2", Some(256)))
        );
        // 不同 provider 的相同 model_id 相互隔离
        assert_ne!(
            a,
            compute_key("ollama", &single_request("openai/m", "text", Some(256)))
        );
        assert_ne!(
            a,
            compute_key("openai", &single_request("openai/m2", "text", Some(256)))
        );
        assert_ne!(
            a,
            compute_key("openai", &single_request("openai/m", "text", None))
        );
        assert_ne!(
            a,
            compute_key("openai", &single_request("openai/m", "text", Some(512)))
        );
        // task_type 参与键
        let mut with_task = single_request("openai/m", "text", Some(256));
        with_task.task_type = SemanticTask::RetrievalQuery;
        assert_ne!(a, compute_key("openai", &with_task));
    }

    #[test]
    fn vector_encode_decode_roundtrip() {
        let vec = vec![0.1_f32, -0.2, 3.3, 4.4];
        let data = encode_vector(4, &vec);
        let (dims, decoded) = decode_vector(&data).unwrap();
        assert_eq!(dims, 4);
        assert_eq!(decoded, vec);
    }

    #[test]
    fn decode_rejects_corrupted_payload() {
        let vec = vec![0.1_f32];
        let mut data = encode_vector(1, &vec);
        data[0] = b'X'; // 破坏 magic
        assert!(decode_vector(&data).is_none());

        let mut data = encode_vector(2, &[0.1, 0.2]);
        data[5] = 99; // 破坏版本
        assert!(decode_vector(&data).is_none());

        // 仅含部分维度头时应视为损坏，不能触发切片 panic。
        assert!(decode_vector(b"ZLEB\x01\x00\x00\x00").is_none());
    }

    #[test]
    fn l2_rel_key_uses_two_level_sharding() {
        let key = compute_key("openai", &single_request("openai/m", "text", Some(256)));
        let rel = l2_rel_key(&key);
        assert_eq!(rel.len(), 2 + 1 + 64 + 4); // "ab/"+hex(64)+".bin"
        assert_eq!(rel.as_bytes()[2], b'/');
        assert!(rel.ends_with(".bin"));
    }
}
