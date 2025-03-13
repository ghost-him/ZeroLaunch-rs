use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

/// 一个支持异步等待的 HashMap
#[derive(Debug)]
pub struct AsyncWaitingHashMap<K, V> {
    data: Arc<Mutex<HashMap<K, V>>>,
    notifiers: Arc<Mutex<HashMap<K, Arc<Notify>>>>,
}

impl<K, V> AsyncWaitingHashMap<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// 创建一个新的 AsyncWaitingHashMap
    pub fn new() -> Self {
        AsyncWaitingHashMap {
            data: Arc::new(Mutex::new(HashMap::new())),
            notifiers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 异步插入键值对，如果有等待该键的任务，则通知它们
    pub async fn insert(&self, key: K, value: V) -> Option<V> {
        let mut data = self.data.lock().await;
        let result = data.insert(key.clone(), value);

        // 通知所有等待这个键的任务
        let notifiers = self.notifiers.lock().await;
        if let Some(notifier) = notifiers.get(&key) {
            notifier.notify_waiters();
        }

        result
    }

    /// 异步获取键对应的值，如果不存在则等待
    pub async fn get_or_wait(&self, key: K) -> V {
        // 首先尝试获取值
        {
            let data = self.data.lock().await;
            if let Some(value) = data.get(&key) {
                return value.clone();
            }
        }

        // 如果值不存在，获取或创建一个通知器
        let notifier = {
            let mut notifiers = self.notifiers.lock().await;
            match notifiers.entry(key.clone()) {
                Entry::Occupied(entry) => entry.get().clone(),
                Entry::Vacant(entry) => {
                    let notify = Arc::new(Notify::new());
                    entry.insert(notify.clone());
                    notify
                }
            }
        };

        // 循环等待值出现
        loop {
            // 再次检查值是否已经存在
            {
                let data = self.data.lock().await;
                if let Some(value) = data.get(&key) {
                    return value.clone();
                }
            }

            // 等待通知
            notifier.notified().await;

            // 被通知后再次检查值
            let data = self.data.lock().await;
            if let Some(value) = data.get(&key) {
                // 清理不再需要的通知器
                let mut notifiers = self.notifiers.lock().await;
                notifiers.remove(&key);

                return value.clone();
            }
        }
    }

    /// 异步尝试获取值，但不等待（返回 Option）
    pub async fn get(&self, key: &K) -> Option<V> {
        let data = self.data.lock().await;
        data.get(key).cloned()
    }

    /// 异步删除键值对
    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut data = self.data.lock().await;
        data.remove(key)
    }

    /// 异步检查键是否存在
    pub async fn contains_key(&self, key: &K) -> bool {
        let data = self.data.lock().await;
        data.contains_key(key)
    }

    /// 异步获取当前 HashMap 中的键值对数量
    pub async fn len(&self) -> usize {
        let data = self.data.lock().await;
        data.len()
    }

    /// 异步检查 HashMap 是否为空
    pub async fn is_empty(&self) -> bool {
        let data = self.data.lock().await;
        data.is_empty()
    }

    /// 异步清空 HashMap
    pub async fn clear(&self) {
        let mut data = self.data.lock().await;
        data.clear();
    }
}

impl<K, V> Default for AsyncWaitingHashMap<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}
