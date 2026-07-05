#!/usr/bin/env bash
# ============================================================================
# summarize-changes 安全检查脚本
# 输出工作区暂存区 / 未暂存 / 未跟踪文件的完整状态概览，减少误会。
# ============================================================================
set -euo pipefail

# --- 收集三类变更 -----------------------------------------------------------
staged=$(git diff --cached --name-only 2>/dev/null || true)
unstaged=$(git diff --name-only 2>/dev/null || true)
untracked=$(git ls-files --others --exclude-standard 2>/dev/null || true)

# --- 工具函数：安全计数（空字符串返回 0）------------------------------------
count_lines() {
    if [ -z "$1" ]; then
        echo 0
    else
        echo "$1" | wc -l | tr -d ' '
    fi
}

# --- 输出报告 ---------------------------------------------------------------
echo "=== 工作区变更状态 ==="
echo ""

# 暂存区
n=$(count_lines "$staged")
if [ "$n" -gt 0 ]; then
    echo "📦 暂存区（staged）：${n} 个文件"
    echo "$staged" | sed 's/^/   - /'
else
    echo "📦 暂存区（staged）：无"
fi
echo ""

# 未暂存（已跟踪文件的修改）
n=$(count_lines "$unstaged")
if [ "$n" -gt 0 ]; then
    echo "📝 未暂存（unstaged）：${n} 个文件"
    echo "$unstaged" | sed 's/^/   - /'
else
    echo "📝 未暂存（unstaged）：无"
fi
echo ""

# 未跟踪（新文件）
n=$(count_lines "$untracked")
if [ "$n" -gt 0 ]; then
    echo "❓ 未跟踪（untracked）：${n} 个文件"
    echo "$untracked" | sed 's/^/   - /'
else
    echo "❓ 未跟踪（untracked）：无"
fi
echo ""

# --- 汇总判断 ---------------------------------------------------------------
n_staged=$(count_lines "$staged")
n_unstaged=$(count_lines "$unstaged")
n_untracked=$(count_lines "$untracked")

if [ "$n_staged" -eq 0 ] && [ "$n_unstaged" -eq 0 ] && [ "$n_untracked" -eq 0 ]; then
    echo "✅ 工作区干净，无任何变更。"
    exit 0
elif [ "$n_unstaged" -eq 0 ] && [ "$n_untracked" -eq 0 ]; then
    echo "✅ 所有变更均已暂存。默认模式等价于 --staged 模式。"
    exit 0
else
    echo "⚠️ 存在未暂存或未跟踪的文件。默认模式将汇总【所有本地变更】。"
    echo "   若仅需汇总暂存区，请使用 /summarize-changes --staged。"
    exit 1
fi
