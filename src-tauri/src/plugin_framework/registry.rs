use dashmap::DashMap;
use std::sync::Arc;
use zerolaunch_plugin_api::{Plugin, PluginMetadata};

pub struct PluginRegistry {
    // 当前已经注册的插件列表，key是插件ID，value是插件实例
    plugins: DashMap<String, Arc<dyn Plugin>>,
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

    /// 注册一个插件。
    /// 参数：plugin - 要注册的插件实例。
    /// 返回：无。
    pub fn register(&self, plugin: Arc<dyn Plugin>) {
        let id = plugin.metadata().id.clone();
        self.plugins.insert(id, plugin);
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
        self.plugins.get(plugin_id).map(|e| e.value().clone())
    }

    /// 获取当前注册的所有插件实例。
    /// 参数：无。
    /// 返回：插件实例列表。
    pub fn get_all(&self) -> Vec<Arc<dyn Plugin>> {
        self.plugins.iter().map(|e| e.value().clone()).collect()
    }

    /// 获取所有插件的元数据。
    /// 参数：无。
    /// 返回：插件元数据列表。
    pub fn get_all_metadata(&self) -> Vec<PluginMetadata> {
        self.plugins
            .iter()
            .map(|e| e.value().metadata().clone())
            .collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
