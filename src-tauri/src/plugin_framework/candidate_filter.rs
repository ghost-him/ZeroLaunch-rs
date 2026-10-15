//! 候选结果排除过滤 —— 采集末端对全部数据源与插件候选统一生效的过滤阶段。
//!
//! 与 `CandidatePipeline` 的 bias 注入同属“配置驱动的候选修正”，区别是本阶段发生在
//! `SessionDispatcher` 全量重建候选项的末尾，因而覆盖尚未进入候选流水线的插件候选项，
//! 解决“单数据源自带的 forbidden_paths 只能剪枝自己的扫描、挡不住其他数据源产出同一目标”
//! 的覆盖缺口。过滤过程无状态、无副作用，输入输出均为 `CachedCandidateData`。

use zerolaunch_plugin_api::{CachedCandidateData, ExecutionTarget, SearchCandidate};

use crate::core::exclusion_rule::{normalize_path, normalize_text, ExclusionRules};

/// 判断候选项是否命中排除规则。
/// 参数：candidate - 待判定候选项（执行目标与显示名参与匹配）；rules - 归一化规则集。
/// 返回：命中任一规则返回 true（调用方据此丢弃该候选项）。
fn is_excluded(candidate: &SearchCandidate, rules: &ExclusionRules) -> bool {
    let payload = normalize_text(candidate.target.payload());
    if rules.exact.contains(&payload) {
        return true;
    }

    // 路径类目标：目录前缀与路径归一化精确匹配（`/`↔`\` 与尾部分隔符差异视为同一条目）。
    // 非路径目标（App/Url/Command/BuiltinCommand/Plugin）不参与目录规则，
    // 避免把 URL 或命令文本误当路径比较。
    if matches!(
        candidate.target,
        ExecutionTarget::Path(_) | ExecutionTarget::File(_)
    ) {
        let path = normalize_path(&payload);
        if rules.exact_path.contains(&path) {
            return true;
        }
        if rules
            .path_prefixes
            .iter()
            .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}\\")))
        {
            return true;
        }
    }

    if !rules.contains.is_empty() {
        let name = normalize_text(&candidate.name);
        if rules
            .contains
            .iter()
            .any(|keyword| payload.contains(keyword) || name.contains(keyword))
        {
            return true;
        }
    }

    false
}

/// 对候选缓存整表应用结果排除过滤。
///
/// 无规则时原样返回（零开销短路，不重建缓存）；有规则时按原顺序重建：
/// 插件候选（`ExecutionTarget::Plugin`）经 `add_plugin_candidate` 重新并入，
/// 保持“仅按目标去重、可与同名数据源候选共存”的既有合并语义；
/// 其余候选项经 `add_candidate` 并入，沿用既有的目标/显示名去重规则。
/// 参数：data - 已合并数据源与插件候选的缓存；rules - 归一化规则集。
/// 返回：过滤后的缓存（候选项 id 按保留顺序重新分配）。
pub(crate) fn filter_candidates(
    mut data: CachedCandidateData,
    rules: &ExclusionRules,
) -> CachedCandidateData {
    if rules.is_empty() {
        return data;
    }

    let source = std::mem::take(data.get_candidates_mut());
    let mut result = CachedCandidateData::new();
    for candidate in source {
        if is_excluded(&candidate, rules) {
            continue;
        }
        if matches!(candidate.target, ExecutionTarget::Plugin(_)) {
            result.add_plugin_candidate(candidate);
        } else {
            result.add_candidate(candidate);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::exclusion_settings::{
        exclusion_settings_to_rules, ExclusionKind, ExclusionRuleEntry, ExclusionSettings,
    };
    use zerolaunch_plugin_api::services::IconRequest;

    /// 构造测试候选项：只关心显示名与执行目标，其余字段取占位值。
    fn candidate(name: &str, target: ExecutionTarget) -> SearchCandidate {
        SearchCandidate {
            id: 0,
            name: name.to_string(),
            icon: IconRequest::Path(String::new()),
            target,
            keywords: Vec::new(),
            bias: 0.0,
            trigger_keywords: Vec::new(),
        }
    }

    /// 由 (匹配方式, 规则值) 列表编译运行时规则集（走真实转换函数，不绕过存储层）。
    fn rules(entries: &[(ExclusionKind, &str)]) -> ExclusionRules {
        let settings = ExclusionSettings {
            rules: entries
                .iter()
                .map(|(kind, value)| ExclusionRuleEntry {
                    kind: *kind,
                    target: (*value).to_string(),
                    note: String::new(),
                })
                .collect(),
        };
        exclusion_settings_to_rules(&settings)
    }

    fn path_candidate(name: &str, path: &str) -> SearchCandidate {
        candidate(name, ExecutionTarget::Path(path.to_string()))
    }

    /// 精确规则只挡住目标全等的候选项，不误伤同目录下的其他程序。
    #[test]
    fn exact_rule_blocks_only_matching_target() {
        let rules = rules(&[(ExclusionKind::Exact, r"C:\Tools\updater\fix.exe")]);
        assert!(is_excluded(
            &path_candidate("fix", r"C:\Tools\updater\fix.exe"),
            &rules
        ));
        assert!(!is_excluded(
            &path_candidate("xjc", r"C:\Tools\updater\xjc.exe"),
            &rules
        ));
    }

    /// 精确规则对非路径目标（应用/URL/命令）同样生效。
    #[test]
    fn exact_rule_covers_non_path_targets() {
        let rules = rules(&[(ExclusionKind::Exact, "mailto:help@example.com")]);
        assert!(is_excluded(
            &candidate(
                "邮件",
                ExecutionTarget::Url("mailto:help@example.com".to_string())
            ),
            &rules
        ));
        assert!(!is_excluded(
            &candidate(
                "邮件2",
                ExecutionTarget::Url("mailto:other@example.com".to_string())
            ),
            &rules
        ));
    }

    /// 目录规则排除子目录内容，但不误伤名字以该目录为前缀的兄弟目录。
    #[test]
    fn path_prefix_blocks_subtree_without_prefix_text_collision() {
        let rules = rules(&[(ExclusionKind::PathPrefix, r"C:\Tools\updater")]);
        assert!(is_excluded(
            &path_candidate("fix", r"C:\Tools\updater\fix.exe"),
            &rules
        ));
        assert!(is_excluded(
            &path_candidate("fix", r"C:\Tools\updater\nested\fix.exe"),
            &rules
        ));
        // 目录自身作为候选（例如 .lnk 指向目录）也应被排除
        assert!(is_excluded(
            &path_candidate("updater", r"C:\Tools\updater"),
            &rules
        ));
        // 文本前缀相同但不是子目录：不得排除
        assert!(!is_excluded(
            &path_candidate("updater-helper", r"C:\Tools\updater-helper\xjc.exe"),
            &rules
        ));
    }

    /// 路径规则大小写不敏感，且 `/` 与 `\`、尾部分隔符差异不影响判定。
    #[test]
    fn path_rules_tolerate_case_and_separators() {
        let rules = rules(&[
            (ExclusionKind::PathPrefix, "c:/tools/updater/"),
            (ExclusionKind::Exact, r"C:\Tools\bin\xjc.exe"),
        ]);
        assert!(is_excluded(
            &path_candidate("fix", r"C:\TOOLS\UPDATER\fix.exe"),
            &rules
        ));
        assert!(is_excluded(
            &path_candidate("xjc", r"c:/tools/bin/xjc.exe"),
            &rules
        ));
    }

    /// 目录规则只作用于文件/路径目标：应用（UWP）与命令目标不参与目录比较。
    #[test]
    fn path_rules_ignore_non_path_targets() {
        let rules = rules(&[(ExclusionKind::PathPrefix, r"C:\Tools\updater")]);
        assert!(!is_excluded(
            &candidate(
                "计算器",
                ExecutionTarget::App("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".to_string())
            ),
            &rules
        ));
        assert!(!is_excluded(
            &candidate(
                "关机",
                ExecutionTarget::Command(r"C:\Tools\updater\fix.exe /s".to_string())
            ),
            &rules
        ));
    }

    /// 关键字规则同时匹配执行目标与显示名。
    #[test]
    fn contains_rule_matches_payload_or_name() {
        let by_payload = rules(&[(ExclusionKind::Contains, "uninstall")]);
        assert!(is_excluded(
            &path_candidate("卸载工具", r"C:\Tools\uninstall.exe"),
            &by_payload
        ));

        let by_name = rules(&[(ExclusionKind::Contains, "崩溃上报")]);
        assert!(is_excluded(
            &path_candidate("崩溃上报器", r"C:\Tools\crashpad.exe"),
            &by_name
        ));
        assert!(!is_excluded(
            &path_candidate("主程序", r"C:\Tools\main.exe"),
            &by_name
        ));
    }

    /// 空串与纯空白规则值被丢弃，且不产生任何过滤效果。
    #[test]
    fn blank_rule_values_are_ignored() {
        let rules = rules(&[
            (ExclusionKind::Exact, ""),
            (ExclusionKind::PathPrefix, "   "),
            (ExclusionKind::Contains, ""),
        ]);
        assert!(rules.is_empty());
        assert!(!is_excluded(
            &path_candidate("main", r"C:\Tools\main.exe"),
            &rules
        ));
    }

    /// 有规则时缓存重建保持原顺序，且插件候选与同名数据源候选仍可共存
    /// （回归保护：不得因重建而把插件候选按显示名去重掉）。
    #[test]
    fn rebuild_keeps_order_and_plugin_candidate_coexistence() {
        let mut data = CachedCandidateData::new();
        data.add_candidate(path_candidate("同名", r"C:\Tools\main.exe"));
        data.add_candidate(path_candidate("drop", r"C:\Tools\updater\fix.exe"));
        data.add_plugin_candidate(candidate(
            "同名",
            ExecutionTarget::Plugin("demo-plugin".to_string()),
        ));

        let rules = rules(&[(ExclusionKind::PathPrefix, r"C:\Tools\updater")]);
        let filtered = filter_candidates(data, &rules);

        let names: Vec<&str> = filtered
            .get_candidates()
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names, vec!["同名", "同名"], "保留原顺序且两类同名候选共存");
        assert!(matches!(
            filtered.get_candidates()[1].target,
            ExecutionTarget::Plugin(_)
        ));
        // id 按保留顺序重排（刷新流程会随后递增世代，前端不以旧 id 复用）
        assert_eq!(filtered.get_candidates()[0].id, 1);
        assert_eq!(filtered.get_candidates()[1].id, 2);
    }

    /// 无规则时不重建缓存（短路返回原数据，避免每次刷新都克隆整表）。
    #[test]
    fn no_rules_returns_input_untouched() {
        let mut data = CachedCandidateData::new();
        data.add_candidate(path_candidate("main", r"C:\Tools\main.exe"));
        data.add_candidate(path_candidate("drop", r"C:\Tools\updater\fix.exe"));

        let filtered = filter_candidates(data, &ExclusionRules::default());
        assert_eq!(filtered.get_candidates().len(), 2);
    }
}
