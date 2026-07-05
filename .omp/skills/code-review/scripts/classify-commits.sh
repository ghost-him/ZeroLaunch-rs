#!/usr/bin/env bash
# -*- coding: UTF-8 -*-
# ============================================================================
# classify-commits.sh - Big commit identification script
#
# Usage: bash classify-commits.sh <git range>
#   e.g.: bash classify-commits.sh HEAD~5..HEAD
#         bash classify-commits.sh main..HEAD
#
# A commit is "big" if any of:
#   1. Changed files >= 8
#   2. Insertions + deletions >= 300
#   3. Crosses 2+ core subsystems
#
# Exit codes: 0=success
# ============================================================================
set -euo pipefail

export LANG="${LANG:-en_US.UTF-8}"
export LC_ALL="${LC_ALL:-en_US.UTF-8}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Shared classify_subsystem / is_core_subsystem (single source of truth)
# shellcheck source=lib.sh
. "$SCRIPT_DIR/lib.sh"

# Detect UTF-8 locale for safe character truncation.
ensure_utf8_locale

RANGE="${1:-}"
if [ -z "$RANGE" ]; then
    echo "ERROR: git range argument required" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# OPTIMIZATION: single `git log --numstat` call replaces 2×N `git show` calls.
#
# Output format (one git invocation for ALL commits):
#   __COMMIT__|<40-hex-sha>|<subject>
#   <added>\t<deleted>\t<filename>
#   ... (more numstat lines)
#   (blank line between commits)
#
# The SHA is always exactly 40 hex characters, so we parse it by position
# to avoid delimiter ambiguity with '|' in commit subjects.
# ---------------------------------------------------------------------------
OUTPUT=$(git log --no-merges --format='__COMMIT__|%H|%s' --numstat "$RANGE" 2>/dev/null || true)

if [ -z "$OUTPUT" ]; then
    echo "(no commits in range or git log failed)"
    exit 0
fi

echo "| Commit | Files | +/-Lines | Core Subsystems | Big |"
echo "|--------|-------|----------|-----------------|-----|"

BIG_COUNT=0
TOTAL_COUNT=0

# --- Accumulators for the current commit (reset per commit) ---------------
cur_sha=""
cur_subject=""
cur_files=0
cur_ins=0
cur_del=0
declare -A cur_core_seen=()
cur_core_list=""

# Emit one commit row — pure bash, no subprocess calls.
emit_commit() {
    local total_lines=$((cur_ins + cur_del))
    local n_core=${#cur_core_seen[@]}

    local is_big="No" reasons=""
    if [ "$cur_files" -ge 8 ]; then
        is_big="Yes"; reasons="files>=8"
    fi
    if [ "$total_lines" -ge 300 ]; then
        is_big="Yes"
        [ -n "$reasons" ] && reasons="$reasons, "
        reasons="${reasons}lines>=300"
    fi
    if [ "$n_core" -ge 2 ]; then
        is_big="Yes"
        [ -n "$reasons" ] && reasons="$reasons, "
        reasons="${reasons}cross-${n_core}-core-subs"
    fi

    # UTF-8-safe truncation — never splits a multi-byte character.
    # Fast path (no subshell) when UTF-8 locale is active.
    local short_sha="${cur_sha:0:8}"
    local short_subject
    if [ "${_UTF8_OK:-0}" = "1" ]; then
        short_subject="${cur_subject:0:40}"
    else
        short_subject=$(truncate_utf8 "$cur_subject" 40)
    fi

    if [ "$is_big" = "Yes" ]; then
        BIG_COUNT=$((BIG_COUNT + 1))
        echo "| \`$short_sha\` $short_subject | $cur_files | +$cur_ins/-$cur_del | $cur_core_list | **Yes** ($reasons) |"
    else
        echo "| \`$short_sha\` $short_subject | $cur_files | +$cur_ins/-$cur_del | $cur_core_list | No |"
    fi
}

# --- Single-pass parse of the combined git log output ---------------------
while IFS= read -r line; do
    # Skip blank separator lines between commits.
    [ -z "$line" ] && continue

    if [[ "$line" == __COMMIT__\|* ]]; then
        # Flush previous commit (if any).
        if [ -n "$cur_sha" ]; then
            emit_commit
        fi

        # Parse new commit header.
        # After stripping "__COMMIT__|", layout is: <40-hex-sha>|<subject>
        local_rest="${line#__COMMIT__|}"
        cur_sha="${local_rest:0:40}"
        cur_subject="${local_rest:41}"   # skip the '|' at index 40
        TOTAL_COUNT=$((TOTAL_COUNT + 1))

        # Reset accumulators.
        cur_files=0
        cur_ins=0
        cur_del=0
        cur_core_seen=()
        cur_core_list=""
        continue
    fi

    # Numstat line: <added>\t<deleted>\t<filename>
    cur_files=$((cur_files + 1))
    local_ins="${line%%$'\t'*}"
    local_rest="${line#*$'\t'}"
    local_del="${local_rest%%$'\t'*}"
    local_fname="${local_rest#*$'\t'}"

    [ "$local_ins" != "-" ] && cur_ins=$((cur_ins + local_ins))
    [ "$local_del" != "-" ] && cur_del=$((cur_del + local_del))

    # Subsystem classification + core dedup via associative array
    # (replaces `sort -u | grep -v | paste -sd,` pipeline).
    # Direct call sets CLASSIFY_SUB — no $(...) subshell (2163× faster on Windows).
    classify_subsystem "$local_fname"
    local_sub="$CLASSIFY_SUB"
    if is_core_subsystem "$local_sub"; then
        if [ -z "${cur_core_seen[$local_sub]:-}" ]; then
            cur_core_seen[$local_sub]=1
            [ -n "$cur_core_list" ] && cur_core_list="$cur_core_list,"
            cur_core_list="$cur_core_list$local_sub"
        fi
    fi
done <<< "$OUTPUT"

# Flush the last commit.
if [ -n "$cur_sha" ]; then
    emit_commit
fi

echo ""
echo "**Total**: $TOTAL_COUNT commits, $BIG_COUNT big commits need solo drill-down."
