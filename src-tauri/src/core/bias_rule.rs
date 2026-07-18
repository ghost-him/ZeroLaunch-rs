/// 固定偏移量规则，按 target 精确匹配（target 已归一化为 lowercase）。
#[derive(Debug, Clone)]
pub struct BiasRule {
    pub target: String,
    pub bias: f64,
}
