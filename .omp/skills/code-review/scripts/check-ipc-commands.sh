#!/usr/bin/env bash
# -*- coding: UTF-8 -*-
# ============================================================================
# check-ipc-commands.sh - Deterministic IPC command consistency check
#
# Usage: bash check-ipc-commands.sh
#
# Cross-checks the THREE places a Tauri IPC command name must line up, and
# reports any drift deterministically (no LLM judgment needed):
#
#   1. DEFINED    - #[tauri::command] fns in src-tauri/src/commands/*.rs
#   2. REGISTERED - entries in generate_handler![ ... ] in src-tauri/src/lib.rs
#   3. INVOKED    - invoke()/invokeCommand() call sites in src-ui/bridge/commands.ts
#
# Findings:
#   [FAIL] frontend invokes a command that the backend does not register
#   [FAIL] a command is registered but has no #[tauri::command] definition
#   [WARN] a command is defined but never registered (dead / forgotten)
#   [INFO] a command is registered but the frontend bridge never calls it
#          (may be CLI-only / event-driven — verify, not necessarily a bug)
#
# Exit code: always 0 (informational; caller decides). Findings are printed.
# ============================================================================
set -euo pipefail

export LANG="${LANG:-en_US.UTF-8}"
export LC_ALL="${LC_ALL:-en_US.UTF-8}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
cd "$PROJECT_ROOT"

CMD_DIR="src-tauri/src/commands"
LIB_RS="src-tauri/src/lib.rs"
FE_BRIDGE="src-ui/bridge/commands.ts"

if [ ! -d "$CMD_DIR" ] || [ ! -f "$LIB_RS" ]; then
    echo "(IPC check skipped: expected paths not found)"
    exit 0
fi

# 1. DEFINED — fn name on the line following a #[tauri::command...] attribute
DEFINED=$(awk '
    /#\[tauri::command/ { pending=1; next }
    pending==1 {
        if (match($0, /fn[ \t]+[A-Za-z_][A-Za-z0-9_]*/)) {
            s=substr($0, RSTART, RLENGTH)
            sub(/^fn[ \t]+/, "", s)
            print s
            pending=0
        }
    }
' "$CMD_DIR"/*.rs 2>/dev/null | sort -u)

# 2. REGISTERED — last path segment of crate::commands::<mod>::<name> in lib.rs
REGISTERED=$(grep -oE 'crate::commands::[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*' "$LIB_RS" 2>/dev/null \
    | awk -F'::' '{print $NF}' | sort -u)

# 3. INVOKED — first string literal arg of invoke()/invokeCommand() in the FE bridge
#    (normalize " -> ' first so both quote styles are handled)
if [ -f "$FE_BRIDGE" ]; then
    INVOKED=$(tr '"' "'" < "$FE_BRIDGE" \
        | grep -oE "invoke[A-Za-z]*(<[^>]*>)?\([[:space:]]*'[A-Za-z_][A-Za-z0-9_]*'" 2>/dev/null \
        | grep -oE "'[A-Za-z_][A-Za-z0-9_]*'" \
        | tr -d "'" | sort -u)
else
    INVOKED=""
fi

N_DEFINED=$(printf '%s\n' "$DEFINED"    | grep -c . || true)
N_REG=$(printf '%s\n' "$REGISTERED"     | grep -c . || true)
N_INVOKED=$(printf '%s\n' "$INVOKED"    | grep -c . || true)

echo "Commands: defined=$N_DEFINED, registered=$N_REG, frontend-invoked=$N_INVOKED"

FINDINGS=0

# frontend invokes but not registered -> hard error (call will fail at runtime)
FE_NOT_REG=$(comm -23 <(printf '%s\n' "$INVOKED" | grep -v '^$') \
                      <(printf '%s\n' "$REGISTERED" | grep -v '^$') || true)
if [ -n "$FE_NOT_REG" ]; then
    while read -r c; do
        [ -z "$c" ] && continue
        echo "[FAIL] frontend invokes '$c' but backend does not register it (generate_handler!)"
        FINDINGS=$((FINDINGS + 1))
    done <<< "$FE_NOT_REG"
fi

# registered but not defined -> would not compile / stale handler entry
REG_NOT_DEF=$(comm -13 <(printf '%s\n' "$DEFINED" | grep -v '^$') \
                      <(printf '%s\n' "$REGISTERED" | grep -v '^$') || true)
if [ -n "$REG_NOT_DEF" ]; then
    while read -r c; do
        [ -z "$c" ] && continue
        echo "[FAIL] '$c' is registered in generate_handler! but has no #[tauri::command] definition in commands/"
        FINDINGS=$((FINDINGS + 1))
    done <<< "$REG_NOT_DEF"
fi

# defined but not registered -> dead command / forgotten registration
DEF_NOT_REG=$(comm -23 <(printf '%s\n' "$DEFINED" | grep -v '^$') \
                      <(printf '%s\n' "$REGISTERED" | grep -v '^$') || true)
if [ -n "$DEF_NOT_REG" ]; then
    while read -r c; do
        [ -z "$c" ] && continue
        echo "[WARN] '$c' has a #[tauri::command] definition but is not registered in generate_handler!"
        FINDINGS=$((FINDINGS + 1))
    done <<< "$DEF_NOT_REG"
fi

# registered but frontend never calls -> informational (CLI/event-driven?)
if [ -n "$INVOKED" ]; then
    REG_NOT_FE=$(comm -13 <(printf '%s\n' "$INVOKED" | grep -v '^$') \
                          <(printf '%s\n' "$REGISTERED" | grep -v '^$') || true)
    if [ -n "$REG_NOT_FE" ]; then
        while read -r c; do
            [ -z "$c" ] && continue
            echo "[INFO] '$c' is registered but the frontend bridge never invokes it (CLI/event-driven? verify)"
        done <<< "$REG_NOT_FE"
    fi
fi

if [ "$FINDINGS" -eq 0 ]; then
    echo "[OK] defined / registered command sets are consistent"
fi
exit 0
