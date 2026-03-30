use super::types::{Plugin, PluginMetadata};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

pub struct PluginRegistry {
    // 当前已经注册的插件列表，key是插件ID，value是插件实例
    plugins: DashMap<String, Arc<dyn Plugin>>,
    // 插件元数据缓存，key是插件ID，value是插件元数据
    metadata_cache: RwLock<HashMap<String, PluginMetadata>>,
    // 触发词到插件ID的映射，key是触发词，value是插件ID
    trigger_map: DashMap<String, String>,
}

impl PluginRegistry {
    /// 创建一个新的插件注册中心。
    /// 参数：无。
    /// 返回：初始化后的 PluginRegistry。
    pub fn new() -> Self {
        Self {
            plugins: DashMap::new(),
            metadata_cache: RwLock::new(HashMap::new()),
            trigger_map: DashMap::new(),
        }
    }

    /// 注册一个插件，并缓存其元数据和触发词映射。
    /// 参数：plugin - 要注册的插件实例。
    /// 返回：无。
    pub fn register(&self, plugin: Arc<dyn Plugin>) {
        let metadata = plugin.metadata();
        let id = metadata.id.clone();

        for keyword in &metadata.trigger_keywords {
            self.trigger_map.insert(keyword.clone(), id.clone());
        }

        self.metadata_cache
            .write()
            .insert(id.clone(), metadata.clone());
        self.plugins.insert(id, plugin);
    }

    /// 注销指定插件，并移除相关缓存和触发词映射。
    /// 参数：plugin_id - 要移除的插件 ID。
    /// 返回：无。
    pub fn unregister(&self, plugin_id: &str) {
        if let Some((_, plugin)) = self.plugins.remove(plugin_id) {
            let metadata = plugin.metadata();
            for keyword in &metadata.trigger_keywords {
                self.trigger_map.remove(keyword);
            }
            self.metadata_cache.write().remove(plugin_id);
        }
    }

    /// 根据插件 ID 获取插件实例。
    /// 参数：plugin_id - 插件 ID。
    /// 返回：找到则返回插件实例，找不到则返回 None。
    pub fn get(&self, plugin_id: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.get(plugin_id).map(|e| e.value().clone())
    }

    /// 获取当前注册的所有插件实例。
    /// 参数：无。
    /// 返回：插件实例列表。
    pub fn get_all(&self) -> Vec<Arc<dyn Plugin>> {
        self.plugins.iter().map(|e| e.value().clone()).collect()
    }

    /// 根据触发词获取对应插件。
    /// 参数：keyword - 触发词。
    /// 返回：找到则返回插件实例，找不到则返回 None。
    pub fn get_by_trigger(&self, keyword: &str) -> Option<Arc<dyn Plugin>> {
        self.trigger_map
            .get(keyword)
            .and_then(|plugin_id| self.get(plugin_id.value()))
    }

    /// 获取所有插件的元数据。
    /// 参数：无。
    /// 返回：插件元数据列表。
    pub fn get_all_metadata(&self) -> Vec<PluginMetadata> {
        self.metadata_cache.read().values().cloned().collect()
    }

    /// 从查询字符串中解析触发词和搜索内容。
    /// 参数：query - 原始查询字符串。
    /// 返回：(触发词, 剩余查询内容)。没有触发词时返回 (None, 原字符串)。
    pub fn parse_trigger<'a>(&self, query: &'a str) -> (Option<String>, &'a str) {
        let parts: Vec<&str> = query.splitn(2, ' ').collect();
        if parts.len() > 1 && self.trigger_map.contains_key(parts[0]) {
            return (Some(parts[0].to_string()), parts[1]);
        }
        (None, query)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
