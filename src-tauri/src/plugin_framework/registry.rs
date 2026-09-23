use dashmap::DashMap;
use std::sync::Arc;
use zerolaunch_plugin_api::{Plugin, PluginMetadata};

/// 注册中心条目：插件实例与其插件级元数据成对存放。
pub struct RegistryEntry {
    pub plugin: Arc<dyn Plugin>,
    pub metadata: Arc<PluginMetadata>,
}

pub struct PluginRegistry {
    // 当前已经注册的插件列表，key是插件ID，value是插件实例与元数据
    plugins: DashMap<String, RegistryEntry>,
}

impl PluginRegistry {
    /// 创建一个新的插件注册中心。
    /// 参数：无。
    /// 返回：初始化后的 PluginRegistry。
    pub fn new() -> Self {
        Self {
            plugins: DashMap::new(),
        }
    }

    /// 注册一个插件（键取元数据中的插件 ID）。
    /// 参数：plugin - 要注册的插件实例；metadata - 该插件的插件级元数据。
    /// 返回：无。
    pub fn register(&self, plugin: Arc<dyn Plugin>, metadata: Arc<PluginMetadata>) {
        let id = metadata.id.clone();
        self.plugins.insert(id, RegistryEntry { plugin, metadata });
    }

    /// 注销指定插件。
    /// 参数：plugin_id - 插件 ID。
    /// 返回：无。
    pub fn unregister(&self, plugin_id: &str) {
        self.plugins.remove(plugin_id);
    }

    /// 根据插件 ID 获取插件实例。
    /// 参数：plugin_id - 插件 ID。
    /// 返回：找到则返回插件实例，找不到则返回 None。
    pub fn get(&self, plugin_id: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins
            .get(plugin_id)
            .map(|e| e.value().plugin.clone())
    }

    /// 根据插件 ID 获取插件级元数据。
    /// 参数：plugin_id - 插件 ID。
    /// 返回：找到则返回元数据，找不到则返回 None。
    pub fn get_metadata(&self, plugin_id: &str) -> Option<Arc<PluginMetadata>> {
        self.plugins
            .get(plugin_id)
            .map(|e| e.value().metadata.clone())
    }

    /// 获取当前注册的所有插件实例。
    /// 参数：无。
    /// 返回：插件实例列表。
    pub fn get_all(&self) -> Vec<Arc<dyn Plugin>> {
        self.plugins
            .iter()
            .map(|e| e.value().plugin.clone())
            .collect()
    }

    /// 获取当前注册的所有插件实例与其元数据（成对遍历，无需二次查表）。
    /// 参数：无。
    /// 返回：(插件实例, 元数据) 列表。
    pub fn get_all_with_metadata(&self) -> Vec<(Arc<dyn Plugin>, Arc<PluginMetadata>)> {
        self.plugins
            .iter()
            .map(|e| (e.value().plugin.clone(), e.value().metadata.clone()))
            .collect()
    }

    /// 获取所有插件的元数据。
    /// 参数：无。
    /// 返回：插件元数据列表。
    pub fn get_all_metadata(&self) -> Vec<PluginMetadata> {
        self.plugins
            .iter()
            .map(|e| e.value().metadata.as_ref().clone())
            .collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
