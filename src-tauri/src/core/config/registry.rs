use crate::core::types::{ComponentType, Configurable};
use dashmap::DashMap;
use std::sync::Arc;

/// 所有 Configurable 组件的注册中心。
/// 提供按 component_id 和 ComponentType 的查找能力。
pub struct ConfigurableRegistry {
    /// 组件注册表：component_id → Arc<dyn Configurable>
    components: DashMap<String, Arc<dyn Configurable>>,
    /// 类型索引：ComponentType → [component_id]
    type_index: DashMap<ComponentType, Vec<String>>,
}

impl ConfigurableRegistry {
    /// 创建空的注册中心
    pub fn new() -> Self {
        Self {
            components: DashMap::new(),
            type_index: DashMap::new(),
        }
    }

    /// 注册一个可配置组件。
    /// 同时将其信息写入类型索引。
    pub fn register(&self, component: Arc<dyn Configurable>) {
        let id = component.component_id().to_string();
        let component_type = component.component_type();

        // 维护类型索引
        self.type_index
            .entry(component_type)
            .or_default()
            .push(id.clone());

        self.components.insert(id, component);
    }

    /// 注销一个可配置组件。
    /// 同时从类型索引中移除。
    pub fn unregister(&self, component_id: &str) {
        if let Some((_, component)) = self.components.remove(component_id) {
            let component_type = component.component_type();
            if let Some(mut ids) = self.type_index.get_mut(&component_type) {
                ids.retain(|id| id != component_id);
            }
        }
    }

    /// 按 component_id 查找组件
    pub fn get(&self, component_id: &str) -> Option<Arc<dyn Configurable>> {
        self.components.get(component_id).map(|r| r.value().clone())
    }

    /// 按 ComponentType 查找所有组件
    pub fn get_by_type(&self, component_type: ComponentType) -> Vec<Arc<dyn Configurable>> {
        self.type_index
            .get(&component_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.components.get(id).map(|r| r.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取所有已注册组件
    pub fn get_all(&self) -> Vec<Arc<dyn Configurable>> {
        self.components.iter().map(|r| r.value().clone()).collect()
    }

    /// 获取已注册组件数量
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// 检查是否没有注册任何组件
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}

impl Default for ConfigurableRegistry {
    fn default() -> Self {
        Self::new()
    }
}
