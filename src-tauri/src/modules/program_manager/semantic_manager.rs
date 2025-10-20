use crate::error::AppResult;
use crate::modules::program_manager::unit::EmbeddingVec;
use crate::program_manager::LaunchMethod;
use crate::program_manager::SemanticStoreItem;
use bincode::Decode;
use bincode::Encode;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

/// 提供语义 embedding 的运行时后端抽象
pub trait EmbeddingBackend: Send + Sync {
    fn generate_embedding_for_loader(
        &self,
        show_name: &str,
        search_keywords: &str,
        launch_method: &LaunchMethod,
        description: &str,
    ) -> AppResult<EmbeddingVec>;

    fn generate_embedding_for_manager(&self, user_input: &str) -> AppResult<EmbeddingVec>;

    fn compute_similarity(&self, embedding1: &EmbeddingVec, embedding2: &EmbeddingVec) -> f32;

    fn release_resources(&self) {}

    /// 后端是否就绪（例如模型文件是否存在）。默认认为就绪。
    fn is_ready(&self) -> bool;
}

pub struct SemanticManager {
    semantic_store: Arc<RwLock<HashMap<String, SemanticStoreItem>>>,
    embedding_backend: Option<Arc<dyn EmbeddingBackend>>,
    program_embedding_cache: Arc<RwLock<HashMap<LaunchMethod, CachedEntry>>>,
}

impl std::fmt::Debug for SemanticManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticManager")
            .field("semantic_store_len", &self.semantic_store.read().len())
            .field("has_backend", &self.has_backend())
            .field("cache_len", &self.program_embedding_cache.read().len())
            .finish()
    }
}

#[derive(Debug, Clone, Encode, Decode)]
struct CachedEntry {
    embedding: EmbeddingVec,
}

#[derive(Encode, Decode)]
struct SerEntry {
    key: LaunchMethod,
    embedding: Vec<f32>,
}

impl SemanticManager {
    pub fn new(
        embedding_backend: Option<Arc<dyn EmbeddingBackend>>,
        semantic_store: HashMap<String, SemanticStoreItem>,
    ) -> Self {
        Self {
            semantic_store: Arc::new(RwLock::new(semantic_store)),
            embedding_backend,
            program_embedding_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn has_backend(&self) -> bool {
        self.embedding_backend.is_some()
    }

    /// 检查后端是否可用（例如模型文件是否齐全）
    pub fn is_backend_ready(&self) -> bool {
        match &self.embedding_backend {
            Some(b) => b.is_ready(),
            None => false,
        }
    }

    pub fn get_semantic_descriptions(&self, launch_method: &LaunchMethod) -> String {
        let key = launch_method.get_text();
        self.semantic_store
            .read()
            .get(&key)
            .map(|item| item.description.clone())
            .unwrap_or_default()
    }

    pub fn update_semantic_store(&self, new_store: HashMap<String, SemanticStoreItem>) {
        let mut store = self.semantic_store.write();
        *store = new_store;
    }

    pub fn get_runtime_data(&self) -> HashMap<String, SemanticStoreItem> {
        self.semantic_store.read().clone()
    }

    pub fn generate_embedding_for_loader(
        &self,
        show_name: &str,
        search_keywords: &str,
        launch_method: &LaunchMethod,
        description: &str,
    ) -> AppResult<EmbeddingVec> {
        if let Some(backend) = &self.embedding_backend {
            debug!(
                "生成embedding的文本: title: {} | text: 软件名字:{}，也叫做:{}，启动地址或uwp包族名:{}，描述信息:{}",
                show_name,
                show_name,
                search_keywords,
                launch_method.get_text(),
                description
            );
            backend.generate_embedding_for_loader(
                show_name,
                search_keywords,
                launch_method,
                description,
            )
        } else {
            Ok(Vec::new())
        }
    }

    pub fn generate_embedding_for_manager(&self, user_input: &str) -> AppResult<EmbeddingVec> {
        if let Some(backend) = &self.embedding_backend {
            debug!("用户输入: {}", user_input);
            backend.generate_embedding_for_manager(user_input)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn compute_similarity(&self, embedding1: &EmbeddingVec, embedding2: &EmbeddingVec) -> f32 {
        if let Some(backend) = &self.embedding_backend {
            backend.compute_similarity(embedding1, embedding2)
        } else {
            0.0
        }
    }

    pub fn get_cached_embedding(&self, key: &LaunchMethod) -> Option<EmbeddingVec> {
        let cache = self.program_embedding_cache.read();
        cache.get(key).map(|entry| entry.embedding.clone())
    }

    pub fn put_cached_embedding(&self, key: &LaunchMethod, embedding: &EmbeddingVec) {
        if self.embedding_backend.is_none() {
            return;
        }
        let mut cache = self.program_embedding_cache.write();
        cache.insert(
            key.clone(),
            CachedEntry {
                embedding: embedding.clone(),
            },
        );
    }

    pub fn release_backend_resources(&self) {
        if self.embedding_backend.is_none() {
            return;
        }

        if let Some(backend) = &self.embedding_backend {
            debug!("Releasing embedding backend resources while retaining cached embeddings");
            backend.release_resources();
        }
    }

    pub fn export_embeddings_cache_to_bytes(&self) -> Vec<u8> {
        if self.embedding_backend.is_none() {
            return Vec::new();
        }

        let cache = self.program_embedding_cache.read();
        let mut list: Vec<SerEntry> = Vec::with_capacity(cache.len());
        for (key, value) in cache.iter() {
            list.push(SerEntry {
                key: key.clone(),
                embedding: value.embedding.clone(),
            });
        }

        bincode::encode_to_vec(&list, bincode::config::standard()).unwrap_or_default()
    }

    pub fn load_embeddings_cache_from_bytes(&self, bytes: Option<&[u8]>) -> bool {
        if self.embedding_backend.is_none() {
            return false;
        }

        let Some(data) = bytes else {
            return false;
        };

        let list: Vec<SerEntry> =
            match bincode::decode_from_slice(data, bincode::config::standard()) {
                Ok((v, _)) => v,
                Err(_) => return false,
            };

        let mut map: HashMap<LaunchMethod, CachedEntry> = HashMap::with_capacity(list.len());
        for item in list.into_iter() {
            map.insert(
                item.key,
                CachedEntry {
                    embedding: item.embedding,
                },
            );
        }
        *self.program_embedding_cache.write() = map;
        true
    }
}
