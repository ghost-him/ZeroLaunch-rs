use crate::plugin_framework::SessionRouter;
use crate::sdk::HostApi;
use async_trait::async_trait;
use base64::Engine;
use serde::Serialize;
use std::sync::Arc;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigActionDef, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::host::PluginHandle;
// ============================================================================
// CandidateSummary — 返回给前端的搜索结果结构
// ============================================================================

/// 搜索候选项的摘要信息，包含 base64 图标数据 URL。
#[derive(Debug, Clone, Serialize)]
struct CandidateSummary {
    /// 程序名称
    #[serde(rename = "name")]
    name: String,
    /// 目标路径/URL
    #[serde(rename = "target")]
    target: String,
    /// 目标类型（Path, App 等）
    #[serde(rename = "targetType")]
    target_type: String,
    /// 图标 base64 data URL（如 "data:image/png;base64,..."）
    #[serde(rename = "icon")]
    icon: String,
}

// ============================================================================
// CandidateRegistryConfig — Core Configurable 组件
// ============================================================================

/// 候选项注册表配置组件，提供 `search_candidates` ConfigAction。
///
/// 唯一职责是供设置页面的 SearchTable 字段调用，返回已索引程序的搜索列表。
/// 不出现在设置页的 Tab 列表中（由 settingsSidebar.ts 排除）。
pub struct CandidateRegistryConfig {
    core: ComponentCore,
    session_router: Arc<SessionRouter>,
    _host_api: Arc<HostApi>,
    plugin_handle: Arc<PluginHandle>,
}

impl CandidateRegistryConfig {
    /// 创建 CandidateRegistryConfig 实例。
    ///
    /// plugin_handle 通过 host_api.register("candidate-registry") 创建，
    /// 用于异步提取图标。
    pub fn new(session_router: Arc<SessionRouter>, host_api: Arc<HostApi>) -> Self {
        let plugin_handle = host_api.register("candidate-registry", Default::default());
        Self {
            core: ComponentCore::new(
                "candidate-registry".to_string(),
                "候选项注册表".to_string(),
                "提供已索引程序的搜索服务，供设置页面使用".to_string(),
                ComponentType::Core,
                5,
            ),
            session_router,
            _host_api: host_api,
            plugin_handle,
        }
    }
}

#[async_trait]
impl Configurable for CandidateRegistryConfig {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    /// 空 Schema — 无用户可配置项
    fn setting_schema(&self) -> Vec<SettingDefinition> {
        Vec::new()
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn apply_settings(
        &self,
        _settings: serde_json::Value,
    ) -> Result<(), zerolaunch_plugin_api::config::ConfigError> {
        Ok(())
    }

    fn config_actions(&self) -> Vec<ConfigActionDef> {
        vec![ConfigActionDef {
            action: "search_candidates".to_string(),
            label: "搜索程序".to_string(),
            description: "按名称或路径搜索已索引的程序".to_string(),
        }]
    }

    async fn execute_config_action(
        &self,
        action: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match action {
            "search_candidates" => {
                let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let candidates = self.session_router.get_cached_candidates();
                let query_lower = query.to_lowercase();
                let plugin_handle = self.plugin_handle.clone();

                // 现在已是 async 上下文，直接 await
                let mut results: Vec<CandidateSummary> = Vec::new();
                for c in candidates
                    .into_iter()
                    .filter(|c| {
                        if query.is_empty() {
                            true
                        } else {
                            c.name.to_lowercase().contains(&query_lower)
                                || c.target.payload().to_lowercase().contains(&query_lower)
                        }
                    })
                    .take(50)
                {
                    let icon_data = plugin_handle.get_icon_or_default(c.icon.clone()).await;
                    let icon_url = if icon_data.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "data:image/png;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(&icon_data)
                        )
                    };
                    results.push(CandidateSummary {
                        name: c.name,
                        target: c.target.payload().to_string(),
                        target_type: c.target.target_type().as_str().to_string(),
                        icon: icon_url,
                    });
                }

                serde_json::to_value(results).map_err(|e| e.to_string())
            }
            _ => Err(format!("未知动作: {}", action)),
        }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

// ============================================================================
// 注册到 inventory（作为 ConfigEntry）
// ============================================================================

use crate::plugin_framework::builtin_registry::{ConfigEntry, InventoryContext};

fn build_candidate_registry(ctx: &InventoryContext) -> Arc<dyn Configurable> {
    Arc::new(CandidateRegistryConfig::new(
        ctx.session_router().clone(),
        ctx.host_api().clone(),
    ))
}

inventory::submit! {
    ConfigEntry {
        component_id: "candidate-registry",
        priority: 5,
        factory: build_candidate_registry,
    }
}
