//! 结果排除（exclusion）的运行时类型 —— 归一化后的规则值集合。
//!
//! 与 `bias_rule` 同层（P1：运行时类型属于 `core/`，不含 serde、不引用 ConfigManager）：
//! 存储类型 `ExclusionSettings`（`core/config/exclusion_settings.rs`）经转换函数
//! `exclusion_settings_to_rules` 生成本类型，供 `plugin_framework::candidate_filter`
//! 在候选采集末端直接比较。数据源与配置组件不得持有本类型（它们各自只读自己的 Settings）。

/// 结果排除运行时类型 —— 归一化后的规则值集合（已去重，顺序不影响语义）。
///
/// 仅在 `plugin_framework::candidate_filter` 的候选过滤阶段使用；
/// 由 `core::config::exclusion_settings::exclusion_settings_to_rules` 从持久化配置编译，
/// 不在采集/搜索热路径之外的地方流转。
#[derive(Debug, Clone, Default)]
pub struct ExclusionRules {
    /// 精确匹配值（小写）：与候选执行目标全等即排除，覆盖所有目标类型
    /// （路径、应用、URL、命令等）。空串值不参与匹配。
    pub exact: Vec<String>,
    /// 精确匹配值（路径归一化）：仅对文件/路径类型目标生效，
    /// 容忍 `/`↔`\` 与尾部分隔符差异。
    pub exact_path: Vec<String>,
    /// 目录前缀（路径归一化，不含尾部分隔符）：候选路径等于该前缀，
    /// 或以 `\` 续接其后（避免 `C:\foo` 误伤 `C:\foobar`）。仅对文件/路径目标生效。
    pub path_prefixes: Vec<String>,
    /// 关键字列表（小写）：候选执行目标或显示名的子串命中任一关键字即排除。
    pub contains: Vec<String>,
}

impl ExclusionRules {
    /// 是否不含任何规则。
    /// 返回：四种规则全空时返回 true（调用方据此短路，避免无谓遍历并保持缓存原样）。
    pub fn is_empty(&self) -> bool {
        self.exact.is_empty()
            && self.exact_path.is_empty()
            && self.path_prefixes.is_empty()
            && self.contains.is_empty()
    }
}

/// 归一化路径文本：ASCII 小写 + `/` 折叠为 `\` + 去尾部分隔符。
/// 参数：value - 原始路径文本（用户输入或候选项执行目标）。
/// 返回：用于精确/前缀比较的归一化路径（如 `C:\Foo\` → `c:\foo`）。
/// 说明：保留盘符形态（`c:`），前缀匹配时由调用方补分隔符比较。
pub fn normalize_path(value: &str) -> String {
    let mut normalized = value.to_ascii_lowercase().replace('/', "\\");
    while normalized.len() > 1 && normalized.ends_with('\\') {
        normalized.pop();
    }
    normalized
}

/// 归一化文本：仅做 ASCII 小写（与 `BiasRule` 的 target 归一化口径一致）。
/// 参数：value - 原始文本。
/// 返回：ASCII 小写文本。
pub fn normalize_text(value: &str) -> String {
    value.to_ascii_lowercase()
}
