use crate::core::bias_rule::BiasRule;
use crate::utils::collapse_repeated_spaces;
use std::collections::HashMap;
use std::sync::Arc;
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_api::{
    CachedCandidateData, DataSource, KeywordInjector, KeywordInputSource, KeywordOptimizer,
    SearchCandidate,
};

pub struct CandidatePipeline {
    data_sources: Vec<Arc<dyn DataSource>>,
    keyword_optimizers: Vec<Arc<dyn KeywordOptimizer>>,
    keyword_injectors: Vec<Arc<dyn KeywordInjector>>,
    bias_rules: HashMap<String, f64>,
}

impl CandidatePipeline {
    pub fn new() -> Self {
        Self {
            data_sources: Vec::new(),
            keyword_optimizers: Vec::new(),
            keyword_injectors: Vec::new(),
            bias_rules: HashMap::new(),
        }
    }

    /// 设置固定偏移量规则列表，内部转换为 HashMap 以支持 O(1) 查找。
    /// 规则按 target 精确匹配（target 已预归一化为 lowercase）。
    pub fn set_bias_rules(&mut self, rules: Vec<BiasRule>) {
        self.bias_rules = rules.into_iter().map(|r| (r.target, r.bias)).collect();
    }

    pub fn add_source(&mut self, source: Arc<dyn DataSource>) {
        self.data_sources.push(source);
    }

    pub fn remove_source(&mut self, component_id: &str) {
        self.data_sources
            .retain(|s| s.component_id() != component_id);
    }

    pub fn add_keyword_optimizer(&mut self, optimizer: Arc<dyn KeywordOptimizer>) {
        self.keyword_optimizers.push(optimizer);
    }

    pub fn remove_keyword_optimizer(&mut self, component_id: &str) {
        self.keyword_optimizers
            .retain(|op| op.component_id() != component_id);
    }

    pub fn add_keyword_injector(&mut self, injector: Arc<dyn KeywordInjector>) {
        self.keyword_injectors.push(injector);
    }

    pub fn remove_keyword_injector(&mut self, component_id: &str) {
        self.keyword_injectors
            .retain(|inj| inj.component_id() != component_id);
    }

    /// 收集数据源候选项并统一过关键字处理流水线（优化器只排序一次、候选只遍历一次）。
    /// 沉浸式插件候选不在此收集，由调用方经 CachedCandidateData::add_plugin_candidate
    /// 单独并入缓存。
    pub async fn collect(&self) -> CachedCandidateData {
        let mut raw: Vec<SearchCandidate> = Vec::new();
        for source in &self.data_sources {
            raw.extend(
                source
                    .fetch_candidates()
                    .await
                    .get_candidates()
                    .iter()
                    .cloned(),
            );
        }

        // 优化器按 priority 升序（一次构建，供全部候选复用）
        let mut sorted: Vec<&dyn KeywordOptimizer> =
            self.keyword_optimizers.iter().map(|a| a.as_ref()).collect();
        sorted.sort_by_key(|op| op.get_priority());

        // 注入器无需排序
        let injectors: Vec<&dyn KeywordInjector> =
            self.keyword_injectors.iter().map(|a| a.as_ref()).collect();

        let mut processed = Vec::with_capacity(raw.len());
        for c in raw {
            processed.push(self.process_candidate(c, &sorted, &injectors).await);
        }

        // 统一去重 + 分配 id（重建索引）
        let mut candidates = CachedCandidateData::new();
        for c in processed {
            candidates.add_candidate(c);
        }
        candidates
    }

    /// 对单个候选运行完整关键字处理流水线（纯函数，值进值出）：
    /// 保留候选自带 keywords → 名称派生（优化器链）→ 注入器 → 去重 → 固定偏置。
    /// `sorted` / `injectors` 由调用方一次性构建传入。
    pub async fn process_candidate(
        &self,
        mut candidate: SearchCandidate,
        sorted: &[&dyn KeywordOptimizer],
        injectors: &[&dyn KeywordInjector],
    ) -> SearchCandidate {
        let mut keywords = std::mem::take(&mut candidate.keywords);
        keywords.extend(Self::apply_keyword_optimizers(&candidate.name, sorted).await);
        for injector in injectors {
            keywords.extend(injector.inject_keywords(&candidate).await);
        }
        candidate.keywords = Self::deduplicate_keywords(keywords);
        let target = candidate.target.payload().to_ascii_lowercase();
        if let Some(bias) = self.bias_rules.get(&target) {
            candidate.bias += bias;
        }
        candidate
    }

    // 对单个名称运行受控 DAG 关键词优化流水线（DAG-lite），返回去重后的关键词列表。
    //
    // 模型（替代旧「uses_context 布尔 × 单一累积池」的线性链）：
    // 1. 归一化小写基（小写 + 折叠空格）恒进最终关键词池（原始名不进池）；
    // 2. 逐优化器按 `get_priority()` 升序执行；输入由 `input_source()` 声明：
    //    - `OriginalName`：吃原始展示名（驼峰缩写）；产物进池；
    //    - `NormalizedBase`：吃归一化基（一次）；
    //    - `Refined`：吃当前最终池全体（幂等精化）；
    //    - `OptimizerOutput { producer_id }`：吃另一已注册优化器的**登记输出**——
    //      每个优化器执行完以自己的 component_id 将产物登记到输出注册表，
    //      消费者按其声明的 producer_id 精确取用（pinyin-converter 只是普通
    //      被引用生产者，无任何组件名硬编码）；
    // 3. 依赖执行序约束：producer 的 priority 必须小于消费者（升序遍历保证
    //    producer 先运行）；违反该约束的配置在管道构建期被拒绝。
    // 产物永不回流到已执行层，从结构上杜绝「缩写器反复作用于派生词」的污染。
    // 参数 `sorted` 必须已按 `get_priority()` 升序排列（调用方负责排序一次复用）。
    async fn apply_keyword_optimizers(name: &str, sorted: &[&dyn KeywordOptimizer]) -> Vec<String> {
        // 归一化小写基：所有派生词的公共起点（对齐 legacy original_lower 语义）。
        let base = collapse_repeated_spaces(&name.to_lowercase());
        // 最终关键词池（会随各层产物增长；作为精化层消费的累积上下文）。
        let mut final_pool: Vec<String> = vec![base.clone()];
        // 各优化器的登记输出：component_id -> 该优化器产出的关键词。
        // OptimizerOutput 消费者按 producer_id 精确取用。
        let mut outputs: HashMap<String, Vec<String>> = HashMap::new();
        // 归一化基与原始名作为可被引用的内建产物预登记。
        outputs.insert("__base__".to_string(), vec![base.clone()]);
        outputs.insert("__original__".to_string(), vec![name.to_string()]);

        for optimizer in sorted {
            // 判定输入：按声明的输入来源取词。
            let produced: Vec<String> = match optimizer.input_source() {
                KeywordInputSource::OriginalName => {
                    // 原始展示名层：只跑一次原始名（不读累积池）。
                    let out = optimizer.optimize(name).await;
                    final_pool.extend(out.iter().cloned());
                    out
                }
                KeywordInputSource::NormalizedBase => {
                    // 归一化基层：只处理一次归一化基（幂等精化/拼音转换）。
                    let out = optimizer.optimize(&base).await;
                    final_pool.extend(out.iter().cloned());
                    out
                }
                KeywordInputSource::Refined => {
                    // 精化层：对当前最终池全体逐词变换（幂等精化器）。
                    // 先快照当前池，避免 extend 与迭代借用冲突（优化器输出也入池）。
                    let snapshot: Vec<String> = final_pool.clone();
                    let mut out = Vec::new();
                    for src in snapshot.iter() {
                        out.extend(optimizer.optimize(src).await);
                    }
                    final_pool.extend(out.iter().cloned());
                    out
                }
                KeywordInputSource::OptimizerOutput { producer_id } => {
                    // 引用层：吃指定生产者优化器登记的输出。
                    // 依赖序由 producer 先于消费者执行保证（priority 升序），
                    // 缺产物视为空输入（producer 未产出或未注册）。
                    let srcs = outputs.get(&producer_id).cloned().unwrap_or_default();
                    let mut out = Vec::new();
                    for src in srcs.iter() {
                        out.extend(optimizer.optimize(src).await);
                    }
                    final_pool.extend(out.iter().cloned());
                    out
                }
            };
            // 以本优化器 component_id 登记输出，供后续 OptimizerOutput 消费者引用。
            outputs.insert(optimizer.component_id().to_string(), produced);
        }
        Self::deduplicate_keywords(final_pool)
    }

    fn deduplicate_keywords(keywords: Vec<String>) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        keywords
            .into_iter()
            .filter(|k| seen.insert(k.clone()))
            .collect()
    }

    /// 调试用：对单个名称运行关键字优化器链，返回所有生成的关键字。
    /// 不修改候选项缓存。内部自行排序后调用共享逻辑。
    pub async fn generate_keywords_for_name(&self, name: &str) -> Vec<String> {
        let mut sorted: Vec<&dyn KeywordOptimizer> =
            self.keyword_optimizers.iter().map(|a| a.as_ref()).collect();
        sorted.sort_by_key(|op| op.get_priority());
        Self::apply_keyword_optimizers(name, &sorted).await
    }

    /// 根据 component_id 查找已注册的 Configurable 组件。
    /// 参数：component_id - 组件标识符。
    /// 返回：找到则返回组件引用，否则返回 None。
    pub fn find_configurable(&self, component_id: &str) -> Option<Arc<dyn Configurable>> {
        // 先从数据源中查找
        if let Some(found) = self
            .data_sources
            .iter()
            .find(|s| s.component_id() == component_id)
            .map(|s| s.clone() as Arc<dyn Configurable>)
        {
            return Some(found);
        }
        // 再从关键词优化器中查找
        if let Some(found) = self
            .keyword_optimizers
            .iter()
            .find(|op| op.component_id() == component_id)
            .map(|op| op.clone() as Arc<dyn Configurable>)
        {
            return Some(found);
        }
        // 最后从关键词注入器中查找
        self.keyword_injectors
            .iter()
            .find(|inj| inj.component_id() == component_id)
            .map(|inj| inj.clone() as Arc<dyn Configurable>)
    }
}

impl Default for CandidatePipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use zerolaunch_plugin_api::config::{ComponentCore, ComponentType, SettingDefinition};
    use zerolaunch_plugin_api::KeywordInputSource;

    #[async_trait]
    impl Configurable for MarkerOptimizer {
        fn core(&self) -> &ComponentCore {
            // 组件 ID 经 core() 读取（登记输出以 component_id 为 key），
            // 桩必须提供真实 core 而非 panic。
            &self.component_id_core
        }
        fn setting_schema(&self) -> Vec<SettingDefinition> {
            vec![]
        }
    }

    struct MarkerOptimizer {
        component_id_core: ComponentCore,
        priority: u32,
        source: KeywordInputSource,
        marker: &'static str,
    }

    fn marker(
        id: &'static str,
        priority: u32,
        source: KeywordInputSource,
        marker: &'static str,
    ) -> MarkerOptimizer {
        MarkerOptimizer {
            component_id_core: ComponentCore::new(
                id.to_string(),
                id.to_string(),
                String::new(),
                ComponentType::KeywordOptimizer,
                priority,
            ),
            priority,
            source,
            marker,
        }
    }

    fn marker_output(
        id: &'static str,
        priority: u32,
        producer_id: &'static str,
        marker: &'static str,
    ) -> MarkerOptimizer {
        MarkerOptimizer {
            component_id_core: ComponentCore::new(
                id.to_string(),
                id.to_string(),
                String::new(),
                ComponentType::KeywordOptimizer,
                priority,
            ),
            priority,
            source: KeywordInputSource::OptimizerOutput {
                producer_id: producer_id.to_string(),
            },
            marker,
        }
    }

    #[async_trait]
    impl KeywordOptimizer for MarkerOptimizer {
        async fn optimize(&self, keyword: &str) -> Vec<String> {
            let out = format!("{keyword}{}", self.marker);
            if out == keyword {
                Vec::new()
            } else {
                vec![out]
            }
        }
        fn input_source(&self) -> KeywordInputSource {
            self.source.clone()
        }
        fn get_priority(&self) -> u32 {
            self.priority
        }
    }

    /// 产物登记 + 跨优化器引用正确性：
    /// - `NormalizedBase` 优化器（生产者 b）的输出以自身 component_id 登记；
    /// - `OptimizerOutput` 消费者（c）只吃 b 的登记产物，不吃归一化基或原始名
    ///   派生词（杜绝 QQ yin le→QQ→qq 式跨层污染）。
    #[tokio::test]
    async fn optimizer_output_consumes_only_producer_output() {
        let opts: Vec<MarkerOptimizer> = vec![
            marker("a-orig", 10, KeywordInputSource::OriginalName, "-orig"),
            marker("b-prod", 20, KeywordInputSource::NormalizedBase, "-prod"),
            marker_output("c-cons", 30, "b-prod", "-cons"),
            marker("d-refined", 40, KeywordInputSource::Refined, "-ref"),
        ];
        let sorted: Vec<&dyn KeywordOptimizer> =
            opts.iter().map(|o| o as &dyn KeywordOptimizer).collect();
        let out = CandidatePipeline::apply_keyword_optimizers("QQ音乐", &sorted).await;

        // 归一化基恒在：qq音乐
        assert!(out.contains(&"qq音乐".to_string()), "缺归一化基: {out:?}");
        // OriginalName 层：只吃原始名一次 → QQ音乐-orig
        assert!(
            out.contains(&"QQ音乐-orig".to_string()),
            "OriginalName 应作用于原始名: {out:?}"
        );
        // 生产者 b 吃归一化基 → qq音乐-prod
        assert!(
            out.contains(&"qq音乐-prod".to_string()),
            "NormalizedBase 生产者应作用于归一化基: {out:?}"
        );
        // 消费者 c 只吃 b 的产物 → qq音乐-prod-cons
        assert!(
            out.contains(&"qq音乐-prod-cons".to_string()),
            "OptimizerOutput 应消费生产者输出: {out:?}"
        );
        // 防污染断言：消费者只应吃生产者输出（-cons 只能出现在 -prod-cons 中，
        // 不允许 -orig-cons / 单独 -cons 挂在基或其他产物上）。
        assert!(
            !out.iter()
                .filter(|k| k.ends_with("-cons"))
                .any(|k| !k.ends_with("-prod-cons")),
            "消费者只应消费生产者输出: {out:?}"
        );
        // Refined 层吃最终池全体 → 应有各 -ref 变体；但 Refined 在 c 之后跑，见 refined 测试。
        assert!(
            out.iter().any(|k| k.ends_with("-ref")),
            "Refined 应消费最终池: {out:?}"
        );
    }

    /// Refined 层消费当前最终池全体（幂等精化器语义），输出进入后续精化输入。
    #[tokio::test]
    async fn refined_consumes_full_pool_in_priority_order() {
        let opts: Vec<MarkerOptimizer> = vec![
            marker("x", 10, KeywordInputSource::Refined, "-x"),
            marker("y", 20, KeywordInputSource::Refined, "-y"),
        ];
        let sorted: Vec<&dyn KeywordOptimizer> =
            opts.iter().map(|o| o as &dyn KeywordOptimizer).collect();
        let out = CandidatePipeline::apply_keyword_optimizers("abc", &sorted).await;
        // x 吃 [abc] → abc-x；y 吃 [abc, abc-x]（快照含 x 产物）→ abc-y, abc-x-y。
        assert!(out.contains(&"abc-x".to_string()), "缺 x 产物: {out:?}");
        assert!(out.contains(&"abc-y".to_string()), "缺 y 产物: {out:?}");
        assert!(
            out.contains(&"abc-x-y".to_string()),
            "Refined 应按优先级吃到前序精化产物: {out:?}"
        );
        // 自身输出不回流到自身（x 不会产出 abc-x-x）。
        assert!(
            !out.iter().any(|k| k.contains("-x-x")),
            "Refined 产物不应回流给自身: {out:?}"
        );
    }
}
