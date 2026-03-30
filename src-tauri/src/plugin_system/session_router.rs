use super::cached_candidate::CachedCandidateData;
use super::candidate_pipeline::CandidatePipeline;
use super::launcher_registry::LauncherRegistry;
use super::search_pipeline::SearchPipeline;
use super::service::PluginService;
use super::types::*;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMode {
    None,
    Plugin(Option<String>),
    Search,
}

pub struct SessionRouter {
    plugin_service: Arc<PluginService>,
    search_pipeline: Arc<RwLock<SearchPipeline>>,
    candidate_pipeline: Arc<RwLock<CandidatePipeline>>,
    cached_candidates: RwLock<CachedCandidateData>,
    current_mode: RwLock<SessionMode>,
    launcher_registry: RwLock<LauncherRegistry>,
}

impl SessionRouter {
    pub fn new(plugin_service: Arc<PluginService>) -> Self {
        Self {
            plugin_service,
            search_pipeline: Arc::new(RwLock::new(SearchPipeline::new(None, Vec::new(), 3))),
            candidate_pipeline: Arc::new(RwLock::new(CandidatePipeline::new())),
            cached_candidates: RwLock::new(CachedCandidateData::new()),
            current_mode: RwLock::new(SessionMode::None),
            launcher_registry: RwLock::new(LauncherRegistry::new()),
        }
    }

    /// 注册一个启动器
    pub fn register_launcher(&self, launcher: Arc<dyn Launcher>) {
        self.launcher_registry.write().register(launcher);
    }

    pub fn refresh_candidates(&self) {
        let candidates = self.candidate_pipeline.read().collect();
        *self.cached_candidates.write() = candidates;
    }

    pub async fn route_query(&self, trace_id: &str, query: &Query) -> QueryResponse {
        // 生成一个上下文
        let mut ctx = PluginContext::new(trace_id);
        ctx.with_query(query.raw_query.clone());

        let results = self.plugin_service.query(&ctx, query).await;

        // 如果被插件触发了，那么就要进入插件模式，同时记录一下是哪个插件触发了
        if let Some(results) = results {
            // 插件被触发时，会写到上下文中，从而让该函数能被调用
            let plugin_id = ctx.plugin_id.clone();
            *self.current_mode.write() = SessionMode::Plugin(plugin_id.clone());
            return results;
        }
        // 如果插件没有被触发，那么说明当前还是要进入搜索模式

        *self.current_mode.write() = SessionMode::Search;

        let cached_candidate = self.cached_candidates.read();

        let scored_candidates = self
            .search_pipeline
            .as_ref()
            .read()
            .search(&cached_candidate, &query.search_term);

        let results = scored_candidates
            .into_iter()
            .map(|candidate| {
                let search_candidate = cached_candidate
                    .get_candidate(candidate.candidate_id)
                    .unwrap();

                let actions = vec![ResultAction {
                    id: "launch".to_string(),
                    label: "打开应用".to_string(),
                    icon: "default_icon".to_string(),
                    is_default: true,
                }];

                ListItem {
                    id: search_candidate.id,
                    title: search_candidate.name.clone(),
                    subtitle: search_candidate.name.clone(),
                    icon: search_candidate.icon.clone(),
                    score: candidate.score,
                    actions,
                }
            })
            .collect();
        QueryResponse::List { results }
    }

    // 执行一个动作
    // action_id 是用户触发的动作的ID，这个id就是指actions的id，在查询时会被传递到前端，当用户触发一个动作时，前端会把这个id传回来的
    pub async fn route_confirm(
        &self,
        trace_id: &str,
        action_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        let mode = self.current_mode.read().clone();
        let mut ctx = PluginContext::new(trace_id);

        match mode {
            SessionMode::Plugin(plugin_id) => {
                let plugin_id =
                    plugin_id.ok_or_else(|| "No plugin in current session".to_string())?;

                ctx.with_plugin_id(plugin_id.clone());
                self.plugin_service
                    .execute_action(&ctx, &plugin_id, action_id, payload)
                    .await
                    .map_err(|e| e.to_string())
            }
            SessionMode::Search => {
                let cached_candidate = self.cached_candidates.read();
                let candidate = cached_candidate
                    .get_candidate(payload["candidate_id"].as_u64().unwrap_or(0) as CandidateId)
                    .ok_or_else(|| "Candidate not found".to_string())?;

                self.launcher_registry
                    .read()
                    .launch(&candidate.launch_method)
                    .map_err(|e| e.to_string())
            }
            SessionMode::None => Err("No active session".to_string()),
        }
    }

    pub fn reset_session(&self) {
        *self.current_mode.write() = SessionMode::None;
    }

    pub fn current_mode(&self) -> SessionMode {
        self.current_mode.read().clone()
    }
}
