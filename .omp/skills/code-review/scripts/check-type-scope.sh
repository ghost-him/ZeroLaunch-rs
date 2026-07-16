#!/usr/bin/env bash
# -*- coding: UTF-8 -*-
# ============================================================================
# check-type-scope.sh - Boundary type leak checker
#
# Usage: bash check-type-scope.sh
#
# Enforces architecture-principles.md P2: a type's definition location encodes
# its responsibility, and its responsibility determines its usage scope.
# Boundary types (IPC DTOs, including BridgeError which now lives in commands/)
# must NOT leak into internal modules.
#
# Two levels of checking:
#   HARD  — internal module imports from commands/ (definite)
#   SOFT  — IPC DTO type names appear in internal modules (suspected, LLM review)
#
# Exit code: always 0 (informational; caller decides).
# ============================================================================
set -euo pipefail

export LANG="${LANG:-en_US.UTF-8}"
export LC_ALL="${LC_ALL:-en_US.UTF-8}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

cd "$PROJECT_ROOT"

SRC_DIR="src-tauri/src"
CMD_DIR="$SRC_DIR/commands"
# Internal modules where boundary types must NOT appear.
INTERNAL_DIRS="core plugin_framework builtin_plugin state tray window"

if [ ! -d "$CMD_DIR" ]; then
    echo "(type-scope check skipped: commands/ not found)"
    exit 0
fi

echo "=== Boundary Type Scope Check ==="
echo ""

# --- 1. Extract IPC DTO type names from commands/*.rs -----------------------
IPC_DTOS=$(grep -hoE 'pub struct [A-Za-z_]+' "$CMD_DIR"/*.rs 2>/dev/null \
    | awk '{print $3}' | sort -u || true)

# --- 2. HARD check: internal modules importing from entry layer -------------
HARD_VIOLATIONS=0
echo "--- Hard check: internal modules importing from entry layer ---"

for dir in $INTERNAL_DIRS; do
    dirpath="$SRC_DIR/$dir"
    [ ! -d "$dirpath" ] && continue

    # use crate::commands::...  (importing IPC DTOs or BridgeError from entry layer.
    # BridgeError now lives in commands/bridge_error.rs, so this single check
    # covers both IPC DTOs and the boundary error type.)
    hits=$(grep -rn 'use crate::commands::' "$dirpath/" 2>/dev/null || true)
    if [ -n "$hits" ]; then
        while IFS= read -r line; do
            HARD_VIOLATIONS=$((HARD_VIOLATIONS + 1))
            echo "[FAIL] $dir/ imports from commands/ (entry-layer types leaking inward)"
            echo "       $line"
        done <<< "$hits"
    fi
done

if [ "$HARD_VIOLATIONS" -eq 0 ]; then
    echo "[OK] No internal module imports from commands/"
fi

# --- 3. SOFT check: IPC DTO type names in internal modules ------------------
echo ""
echo "--- Soft check: IPC DTO type names in internal modules (LLM review) ---"
SOFT_HITS=0

for type in $IPC_DTOS; do
    [ -z "$type" ] && continue
    for dir in $INTERNAL_DIRS; do
        dirpath="$SRC_DIR/$dir"
        [ ! -d "$dirpath" ] && continue

        # Search for the type name, excluding comment lines.
        # grep -rn output format is "filepath:linenum:content", so we match
        # the ":linenum:" boundary to reliably detect comment prefixes.
        hits=$(grep -rn "$type" "$dirpath/" 2>/dev/null \
            | grep -vE ':[0-9]+:\s*//' \
            | grep -vE ':[0-9]+:\s*\*' \
            || true)
        if [ -n "$hits" ]; then
            SOFT_HITS=$((SOFT_HITS + 1))
            echo "[SUSPECT] IPC DTO '$type' found in $dir/ — verify this is not a boundary leak"
            while IFS= read -r line; do
                echo "          $line"
            done <<< "$hits"
        fi
    done
done

if [ "$SOFT_HITS" -eq 0 ]; then
    echo "[OK] No IPC DTO type name found in internal modules"
fi

# --- Summary ----------------------------------------------------------------
echo ""
echo "---"
echo "Boundary type scope: $HARD_VIOLATIONS hard violation(s), $SOFT_HITS suspected leak(s)"
echo "IPC DTOs monitored: $(printf '%s\n' "$IPC_DTOS" | grep -c . || true)"
exit 0
