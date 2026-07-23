use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::provider::{
    LanguageSupport, TranslateRequest, TranslationProvider, TranslationResult,
};
use super::providers::{mock_mirror_from, mock_placeholder_result, MOCK_PROVIDER_ID};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AggregateStatus {
    Ok,
    Partial,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AggregateResult {
    pub primary: Option<TranslationResult>,
    pub alternatives: Vec<TranslationResult>,
    pub status: AggregateStatus,
}

pub struct ProviderRegistry {
    providers: Vec<Arc<dyn TranslationProvider>>,
}

impl ProviderRegistry {
    pub fn new(providers: Vec<Arc<dyn TranslationProvider>>) -> Self {
        Self { providers }
    }

    /// 汇总已启用引擎在当前配置下的语言能力（并集）。
    pub fn language_support_for(&self, enabled: &[String]) -> LanguageSupport {
        let supports: Vec<_> = self
            .providers
            .iter()
            .filter(|p| enabled.iter().any(|id| id == p.id()))
            .map(|p| p.language_support())
            .collect();
        LanguageSupport::union(supports)
    }

    /// 对已启用引擎并行翻译，并按 **enabled 顺序**聚合主结果与备选。
    /// 不支持当前语言对的引擎会直接返回错误结果，不发起网络请求。
    /// 启用 mock 时：优先镜像其它引擎的成功结果，否则使用固定占位。
    pub async fn translate_all(
        &self,
        req: &TranslateRequest,
        enabled: &[String],
        primary_id: &str,
        timeout_ms: u64,
    ) -> AggregateResult {
        // 按用户启用顺序选取（拖拽调序依赖此顺序）
        let selected: Vec<_> = enabled
            .iter()
            .filter_map(|id| {
                self.providers
                    .iter()
                    .find(|p| p.id() == id)
                    .cloned()
            })
            .collect();
        let selected_ids: Vec<String> = selected.iter().map(|p| p.id().to_string()).collect();

        let mut set = tokio::task::JoinSet::new();
        for provider in selected {
            let req = req.clone();
            set.spawn(async move {
                let id = provider.id().to_string();
                let name = provider.name().to_string();
                let support = provider.language_support();
                if !support.supports_pair(&req.source, &req.target) {
                    return TranslationResult::err(
                        id,
                        name,
                        format!("不支持语言对 {}→{}", req.source, req.target),
                    );
                }
                match tokio::time::timeout(
                    std::time::Duration::from_millis(timeout_ms),
                    provider.translate(&req),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => TranslationResult::err(id, name, "超时"),
                }
            });
        }

        // 按 id 收集后，再按启用顺序重建，保证主引擎回退顺序稳定（不依赖 JoinSet 完成先后）
        let mut by_id: HashMap<String, TranslationResult> = HashMap::new();
        let mut orphans = Vec::new();
        while let Some(joined) = set.join_next().await {
            match joined {
                Ok(r) => {
                    by_id.insert(r.provider_id.clone(), r);
                }
                Err(e) => orphans.push(TranslationResult::err(
                    "unknown",
                    "未知",
                    e.to_string(),
                )),
            }
        }

        let mut results: Vec<TranslationResult> = selected_ids
            .iter()
            .filter_map(|id| by_id.remove(id))
            .collect();
        results.extend(by_id.into_values());
        results.extend(orphans);

        apply_mock_mirror(&mut results);

        let primary_idx = results
            .iter()
            .position(|r| r.provider_id == primary_id && r.is_success())
            .or_else(|| results.iter().position(|r| r.is_success()));

        let status = match (
            primary_idx.is_some(),
            results.iter().any(|r| !r.is_success()),
            results.iter().all(|r| !r.is_success()),
        ) {
            (_, _, true) => AggregateStatus::Error,
            (true, true, false) => AggregateStatus::Partial,
            (true, false, false) => AggregateStatus::Ok,
            (false, _, _) => AggregateStatus::Error,
        };

        let primary = primary_idx.map(|i| results[i].clone());
        let alternatives = results
            .into_iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != primary_idx)
            .map(|(_, r)| r)
            .collect();

        AggregateResult {
            primary,
            alternatives,
            status,
        }
    }
}

/// 若结果中含 mock：用其它引擎第一条成功结果覆盖；否则用固定占位。
fn apply_mock_mirror(results: &mut [TranslationResult]) {
    let mock_idx = match results
        .iter()
        .position(|r| r.provider_id == MOCK_PROVIDER_ID)
    {
        Some(i) => i,
        None => return,
    };

    let mirrored = results
        .iter()
        .find(|r| r.provider_id != MOCK_PROVIDER_ID && r.is_success())
        .map(mock_mirror_from)
        .unwrap_or_else(mock_placeholder_result);

    results[mock_idx] = mirrored;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SlowFailProvider;

    #[async_trait::async_trait]
    impl TranslationProvider for SlowFailProvider {
        fn id(&self) -> &str {
            "slow"
        }

        fn name(&self) -> &str {
            "Slow"
        }

        fn language_support(&self) -> LanguageSupport {
            LanguageSupport::bilingual(&["en", "zh"])
        }

        async fn translate(&self, _req: &TranslateRequest) -> TranslationResult {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            TranslationResult::err("slow", "Slow", "slow failure")
        }
    }

    struct OkProvider {
        id: &'static str,
        text: &'static str,
    }

    #[async_trait::async_trait]
    impl TranslationProvider for OkProvider {
        fn id(&self) -> &str {
            self.id
        }

        fn name(&self) -> &str {
            "Ok"
        }

        fn language_support(&self) -> LanguageSupport {
            LanguageSupport::bilingual(&["en", "zh"])
        }

        async fn translate(&self, _req: &TranslateRequest) -> TranslationResult {
            TranslationResult::ok(self.id, "Ok", self.text, None, None, vec![], None)
        }
    }

    struct FailProvider {
        id: &'static str,
    }

    #[async_trait::async_trait]
    impl TranslationProvider for FailProvider {
        fn id(&self) -> &str {
            self.id
        }

        fn name(&self) -> &str {
            "Fail"
        }

        fn language_support(&self) -> LanguageSupport {
            LanguageSupport::bilingual(&["en", "zh"])
        }

        async fn translate(&self, _req: &TranslateRequest) -> TranslationResult {
            TranslationResult::err(self.id, "Fail", "failed")
        }
    }

    fn sample_request() -> TranslateRequest {
        TranslateRequest {
            text: "hello".into(),
            source: "en".into(),
            target: "zh".into(),
        }
    }

    fn enabled(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_string()).collect()
    }

    #[tokio::test]
    async fn primary_success_status_ok() {
        let registry = ProviderRegistry::new(vec![
            Arc::new(OkProvider {
                id: "a",
                text: "primary-ok",
            }),
            Arc::new(OkProvider {
                id: "b",
                text: "alt-ok",
            }),
        ]);
        let req = sample_request();

        let result = registry
            .translate_all(&req, &enabled(&["a", "b"]), "a", 1000)
            .await;

        assert_eq!(result.status, AggregateStatus::Ok);
        let primary = result.primary.expect("应存在主结果");
        assert_eq!(primary.provider_id, "a");
        assert_eq!(primary.text, "primary-ok");
        assert_eq!(result.alternatives.len(), 1);
        assert_eq!(result.alternatives[0].provider_id, "b");
        assert!(result.alternatives[0].is_success());
    }

    #[tokio::test]
    async fn primary_fails_fallback_to_success() {
        let registry = ProviderRegistry::new(vec![
            Arc::new(FailProvider { id: "primary" }),
            Arc::new(OkProvider {
                id: "fallback",
                text: "ok",
            }),
        ]);
        let req = sample_request();

        let result = registry
            .translate_all(&req, &enabled(&["primary", "fallback"]), "primary", 1000)
            .await;

        assert_eq!(result.status, AggregateStatus::Partial);
        let primary = result.primary.expect("主引擎失败时应回退到成功结果");
        assert_eq!(primary.provider_id, "fallback");
        assert!(primary.is_success());
        assert_eq!(result.alternatives.len(), 1);
        assert_eq!(result.alternatives[0].provider_id, "primary");
        assert!(!result.alternatives[0].is_success());
    }

    #[tokio::test]
    async fn all_fail_status_error() {
        let registry = ProviderRegistry::new(vec![
            Arc::new(FailProvider { id: "a" }),
            Arc::new(FailProvider { id: "b" }),
        ]);
        let req = sample_request();

        let result = registry
            .translate_all(&req, &enabled(&["a", "b"]), "a", 1000)
            .await;

        assert_eq!(result.status, AggregateStatus::Error);
        assert!(result.primary.is_none());
        assert_eq!(result.alternatives.len(), 2);
        assert!(result.alternatives.iter().all(|r| !r.is_success()));
    }

    #[tokio::test]
    async fn slow_provider_times_out() {
        let registry = ProviderRegistry::new(vec![
            Arc::new(SlowFailProvider),
            Arc::new(OkProvider {
                id: "fast",
                text: "ok",
            }),
        ]);
        let req = sample_request();

        let result = registry
            .translate_all(&req, &enabled(&["slow", "fast"]), "slow", 1)
            .await;

        let slow = result
            .alternatives
            .iter()
            .chain(result.primary.iter())
            .find(|r| r.provider_id == "slow")
            .expect("应存在 slow 引擎的结果");
        assert_eq!(slow.error.as_deref(), Some("超时"));
    }

    #[tokio::test]
    async fn unsupported_pair_skips_translate() {
        let registry = ProviderRegistry::new(vec![Arc::new(OkProvider {
            id: "a",
            text: "should-not-appear",
        })]);
        let req = TranslateRequest {
            text: "hola".into(),
            source: "es".into(),
            target: "zh".into(),
        };

        let result = registry
            .translate_all(&req, &enabled(&["a"]), "a", 1000)
            .await;

        assert_eq!(result.status, AggregateStatus::Error);
        assert!(result.primary.is_none());
        assert_eq!(result.alternatives.len(), 1);
        let err = result.alternatives[0].error.as_deref().unwrap_or("");
        assert!(err.contains("不支持语言对"), "实际: {err}");
    }

    #[test]
    fn language_support_union_merges_enabled() {
        struct JaOnly;
        #[async_trait::async_trait]
        impl TranslationProvider for JaOnly {
            fn id(&self) -> &str {
                "ja"
            }
            fn name(&self) -> &str {
                "Ja"
            }
            fn language_support(&self) -> LanguageSupport {
                LanguageSupport::bilingual(&["ja", "en"])
            }
            async fn translate(&self, _req: &TranslateRequest) -> TranslationResult {
                TranslationResult::err("ja", "Ja", "n/a")
            }
        }

        let registry = ProviderRegistry::new(vec![
            Arc::new(OkProvider {
                id: "a",
                text: "x",
            }),
            Arc::new(JaOnly),
        ]);
        let support = registry.language_support_for(&enabled(&["a", "ja"]));
        assert!(support.supports_source("zh"));
        assert!(support.supports_source("ja"));
        assert!(support.supports_pair("ja", "en"));
    }

    #[tokio::test]
    async fn mock_mirrors_other_success() {
        use crate::builtin_plugin::triggerable::translator::providers::MockProvider;

        let registry = ProviderRegistry::new(vec![
            Arc::new(OkProvider {
                id: "a",
                text: "real-ok",
            }),
            Arc::new(MockProvider),
        ]);
        let req = sample_request();

        let result = registry
            .translate_all(&req, &enabled(&["a", MOCK_PROVIDER_ID]), "a", 1000)
            .await;

        assert_eq!(result.status, AggregateStatus::Ok);
        let primary = result.primary.expect("primary");
        assert_eq!(primary.provider_id, "a");
        assert_eq!(primary.text, "real-ok");

        let mock = result
            .alternatives
            .iter()
            .find(|r| r.provider_id == MOCK_PROVIDER_ID)
            .expect("mock alt");
        assert!(mock.is_success());
        assert_eq!(mock.text, "real-ok");
        assert_eq!(mock.provider_name, "模拟示例");
    }

    #[tokio::test]
    async fn mock_alone_uses_placeholder() {
        use crate::builtin_plugin::triggerable::translator::providers::MockProvider;

        let registry = ProviderRegistry::new(vec![Arc::new(MockProvider)]);
        let req = sample_request();

        let result = registry
            .translate_all(&req, &enabled(&[MOCK_PROVIDER_ID]), MOCK_PROVIDER_ID, 1000)
            .await;

        assert_eq!(result.status, AggregateStatus::Ok);
        let primary = result.primary.expect("primary");
        assert_eq!(primary.provider_id, MOCK_PROVIDER_ID);
        assert_eq!(primary.text, mock_placeholder_result().text);
    }

    #[tokio::test]
    async fn enabled_order_selects_primary_preference() {
        let registry = ProviderRegistry::new(vec![
            Arc::new(OkProvider {
                id: "a",
                text: "from-a",
            }),
            Arc::new(OkProvider {
                id: "b",
                text: "from-b",
            }),
        ]);
        let req = sample_request();

        let result = registry
            .translate_all(&req, &enabled(&["b", "a"]), "b", 1000)
            .await;

        let primary = result.primary.expect("primary");
        assert_eq!(primary.provider_id, "b");
        assert_eq!(primary.text, "from-b");
    }
}
