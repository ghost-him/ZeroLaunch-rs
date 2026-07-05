#!/usr/bin/env bash
# -*- coding: UTF-8 -*-
# ============================================================================
# collect-context.sh - Code review context collection script
#
# Usage: bash collect-context.sh <mode> [range_or_n]
#   mode:
#     working-tree  - Review working tree changes (git diff HEAD)
#     staged        - Review staged changes (git diff --cached)
#     branch        - Review all changes in current branch vs default branch
#     range         - Review specified git range (requires range_or_n)
#     commits       - Review last N commits (requires range_or_n=N)
#
# Output: Structured context report for main agent and sub agents
# Exit codes: 0=success, 1=arg error, 2=git command failed
# ============================================================================
set -euo pipefail

MODE="${1:-working-tree}"
ARG="${2:-}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

export LANG="${LANG:-en_US.UTF-8}"
export LC_ALL="${LC_ALL:-en_US.UTF-8}"

# Shared classification / rule-mapping helpers (single source of truth)
# shellcheck source=lib.sh
. "$SCRIPT_DIR/lib.sh"

# Detect UTF-8 locale for safe character truncation.
ensure_utf8_locale

# --- Resolve diff range -----------------------------------------------------
resolve_range() {
    case "$MODE" in
        working-tree)
            echo "HEAD"
            ;;
        staged)
            echo "--cached"
            ;;
        branch)
            # Resolve default branch locally (no network round-trip).
            local default_branch=""
            default_branch=$(git -C "$PROJECT_ROOT" symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null \
                | sed 's#^origin/##' || true)
            local merge_base=""
            # Try, in order: origin/<detected>, origin/main, origin/master, local main/master
            for ref in "origin/$default_branch" "origin/main" "origin/master" "main" "master"; do
                [ -z "$ref" ] && continue
                [ "$ref" = "origin/" ] && continue
                if git -C "$PROJECT_ROOT" rev-parse --verify --quiet "$ref" >/dev/null 2>&1; then
                    merge_base=$(git -C "$PROJECT_ROOT" merge-base HEAD "$ref" 2>/dev/null || true)
                    [ -n "$merge_base" ] && break
                fi
            done
            if [ -z "$merge_base" ]; then
                echo "ERROR" >&2
                exit 2
            fi
            echo "${merge_base}..HEAD"
            ;;
        range)
            if [ -z "$ARG" ]; then
                echo "ERROR" >&2
                exit 1
            fi
            echo "$ARG"
            ;;
        commits)
            if [ -z "$ARG" ]; then
                echo "ERROR" >&2
                exit 1
            fi
            echo "HEAD~${ARG}..HEAD"
            ;;
        *)
            echo "ERROR" >&2
            exit 1
            ;;
    esac
}

count_lines() {
    if [ -z "$1" ]; then echo 0; else echo "$1" | wc -l | tr -d ' '; fi
}

# --- Main -------------------------------------------------------------------
RANGE=$(resolve_range)
if [ "$RANGE" = "ERROR" ]; then
    echo "[ERROR] Failed to resolve diff range" >&2
    exit 2
fi

cd "$PROJECT_ROOT"

# Helper: run the right git diff command based on range type
run_diff_stat() {
    if [ "$RANGE" = "--cached" ]; then
        git diff --cached --stat 2>/dev/null
    elif [ "$RANGE" = "HEAD" ]; then
        git diff HEAD --stat 2>/dev/null
    else
        git diff "$RANGE" --stat 2>/dev/null
    fi
}

run_diff_names() {
    if [ "$RANGE" = "--cached" ]; then
        git diff --cached --name-only 2>/dev/null
    elif [ "$RANGE" = "HEAD" ]; then
        git diff HEAD --name-only 2>/dev/null
    else
        git diff "$RANGE" --name-only 2>/dev/null
    fi
}

# Determine range label
if [ "$RANGE" = "--cached" ]; then
    RANGE_LABEL="staged"
elif [ "$RANGE" = "HEAD" ]; then
    RANGE_LABEL="working-tree (vs HEAD)"
else
    RANGE_LABEL="$RANGE"
fi

echo "============================================================"
echo "  Code Review Context Report"
echo "  Range: $RANGE_LABEL"
echo "  Time:  $(date '+%Y-%m-%d %H:%M:%S')"
echo "============================================================"
echo ""

# --- 1. Diff Stat -----------------------------------------------------------
echo "## 1. Diff Stat"
echo '```'
run_diff_stat || echo "(failed to get stat)"
echo '```'
echo ""

# --- Pre-compute all classification data in a single pass -------------------
# Replaces 3 separate echo|while-read loops + 6 grep -c calls with one pass.
CHANGED_FILES=$(run_diff_names || true)
N_FILES=$(count_lines "$CHANGED_FILES")

SUB_FILE_MAP=""               # "subsystem|file" per line (section 2)
declare -A _SUB_SEEN=()
SUB_LIST=""                   # unique subsystems, newline-separated (section 3)
declare -A _RULE_SEEN=()
RULE_LIST=""                  # unique rule files, newline-separated (section 4)
# Boolean flags (sections 5-7) — replaces 6 `echo|grep -c` subprocess calls
HAS_RUST=0; NEEDS_CARGO=0; HAS_RUST_IPC=0; HAS_HANDLER=0; HAS_FE_BRIDGE=0; HAS_TS_CONTRACT=0

while IFS= read -r f; do
    [ -z "$f" ] && continue

    # Subsystem classification (sections 2 & 3)
    # Direct call sets CLASSIFY_SUB — no $(...) subshell.
    classify_subsystem "$f"
    sub="$CLASSIFY_SUB"
    SUB_FILE_MAP="${SUB_FILE_MAP}${sub}|${f}"$'\n'
    if [ -z "${_SUB_SEEN[$sub]:-}" ]; then
        _SUB_SEEN[$sub]=1
        SUB_LIST="${SUB_LIST}${sub}"$'\n'
    fi

    # Rule mapping (section 4)
    # Direct call sets MAP_RULES (space-separated) — no $(...) subshell.
    map_rules_for_path "$f"
    for r in $MAP_RULES; do
        [ -z "$r" ] && continue
        if [ -z "${_RULE_SEEN[$r]:-}" ]; then
            _RULE_SEEN[$r]=1
            RULE_LIST="${RULE_LIST}${r}"$'\n'
        fi
    done

    # Boolean flags (replaces 6 `echo "$CHANGED_FILES" | grep -cE` calls)
    case "$f" in
        *.rs|*.toml) HAS_RUST=1 ;;
    esac
    case "$f" in
        *.rs) NEEDS_CARGO=1 ;;
    esac
    case "$f" in
        src-tauri/src/commands/*) HAS_RUST_IPC=1 ;;
    esac
    case "$f" in
        src-tauri/src/lib.rs) HAS_HANDLER=1 ;;
    esac
    case "$f" in
        src-ui/bridge/*) HAS_FE_BRIDGE=1 ;;
    esac
    case "$f" in
        src-ui/bridge/contract.ts) HAS_TS_CONTRACT=1 ;;
    esac
done <<< "$CHANGED_FILES"

# --- 2. Changed files by subsystem ------------------------------------------
echo "## 2. Changed Files ($N_FILES files)"
echo ""
printf '%s' "$SUB_FILE_MAP" | sort | awk -F'|' '
{
    if ($1 != current) {
        if (current != "") print ""
        print "### " $1
        current = $1
    }
    print "  - " $2
}'
echo ""

# --- 3. Subsystem crossing stats --------------------------------------------
echo "## 3. Subsystem Crossing"
N_SUBSYSTEMS=${#_SUB_SEEN[@]}
SUBSYSTEMS=$(printf '%s' "$SUB_LIST" | sort)
echo "Total subsystems touched: $N_SUBSYSTEMS"
echo "Subsystems: $(printf '%s' "$SUBSYSTEMS" | tr '\n' ' ')"
echo ""

CORE_SUBSYSTEMS_HIT=""
N_CORE=0
while IFS= read -r s; do
    [ -z "$s" ] && continue
    if is_core_subsystem "$s"; then
        N_CORE=$((N_CORE + 1))
        CORE_SUBSYSTEMS_HIT="${CORE_SUBSYSTEMS_HIT}${s} "
    fi
done <<< "$SUBSYSTEMS"
echo "Core subsystems touched: $N_CORE ($CORE_SUBSYSTEMS_HIT)"
if [ "$N_CORE" -ge 2 ]; then
    echo "[WARN] Crosses 2+ core subsystems - pay extra attention to architecture boundaries"
fi
echo ""

# --- 4. Rule files to load --------------------------------------------------
echo "## 4. Rule Files to Load"
printf '%s' "$RULE_LIST" | sort | sed 's/^/- /'
echo ""

# --- 5. Dependency direction check (if Rust files changed) ------------------
# HAS_RUST pre-computed in the single-pass loop above.
if [ "$HAS_RUST" -gt 0 ]; then
    echo "## 5. Workspace Dependency Direction Check"
    if [ -f "$SCRIPT_DIR/check-deps-direction.sh" ]; then
        bash "$SCRIPT_DIR/check-deps-direction.sh" 2>/dev/null || echo "(dep direction check failed)"
    else
        echo "(check-deps-direction.sh not found, skipped)"
    fi
    echo ""
fi

# --- 6. Build check suggestion ----------------------------------------------
# Any Rust source change can break compilation; suggest a build check.
# NEEDS_CARGO pre-computed in the single-pass loop above.
if [ "$NEEDS_CARGO" -gt 0 ]; then
    echo "## 6. Build Check Suggestion"
    echo "[WARN] Rust sources changed - recommend a build + lint pass"
    echo "       Build:  cd src-tauri && cargo check 2>&1 | tail -20"
    echo "       Lint :  cd src-tauri && cargo clippy 2>&1 | tail -40"
    echo "               (clippy::await_holding_lock deterministically flags RwLock/Mutex guards held across .await —"
    echo "                a core project rule; prefer this over eyeballing the diff)"
    echo ""
fi

# --- 7. IPC command consistency (deterministic cross-check) -----------------
# All HAS_* flags pre-computed in the single-pass loop above.
if [ "$HAS_RUST_IPC" -gt 0 ] || [ "$HAS_HANDLER" -gt 0 ] || [ "$HAS_FE_BRIDGE" -gt 0 ]; then
    echo "## 7. IPC Command Consistency"
    if [ -f "$SCRIPT_DIR/check-ipc-commands.sh" ]; then
        bash "$SCRIPT_DIR/check-ipc-commands.sh" 2>/dev/null || echo "(check-ipc-commands.sh failed)"
    else
        echo "(check-ipc-commands.sh not found, skipped)"
    fi
    # Presence-based hint on top of the deterministic set check above.
    if [ "$HAS_RUST_IPC" -gt 0 ] && [ "$HAS_TS_CONTRACT" -eq 0 ]; then
        echo "[HINT] commands/ changed but bridge/contract.ts not touched - verify IPC types need sync"
    elif [ "$HAS_RUST_IPC" -eq 0 ] && [ "$HAS_TS_CONTRACT" -gt 0 ]; then
        echo "[HINT] bridge/contract.ts changed but commands/ not touched - verify backend needs sync"
    fi
    echo ""
fi

# --- 8. Big commit classification (for multi-commit ranges) ------------------
if [ "$MODE" = "commits" ] || [ "$MODE" = "range" ] || [ "$MODE" = "branch" ]; then
    if [ "$RANGE" != "HEAD" ] && [ "$RANGE" != "--cached" ]; then
        echo "## 8. Big Commit Classification"
        if [ -f "$SCRIPT_DIR/classify-commits.sh" ]; then
            bash "$SCRIPT_DIR/classify-commits.sh" "$RANGE" 2>/dev/null || echo "(classify-commits.sh failed)"
        else
            echo "(classify-commits.sh not found, skipped)"
        fi
        echo ""
    fi
fi

echo "============================================================"
echo "  Context collection complete"
echo "============================================================"
