#!/usr/bin/env bash
# -*- coding: UTF-8 -*-
# ============================================================================
# lib.sh - Shared helpers for code-review scripts
#
# Sourced by collect-context.sh / classify-commits.sh.
# Single source of truth for subsystem classification so the two scripts
# can never drift apart.
#
# NOTE on directory names: the `refactor/plugin-system` branch renamed
#   src-tauri/src/plugin_system  -> plugin_framework
#   src-tauri/src/plugin         -> builtin_plugin
#   src-tauri/src/sdk/           -> sdk.rs   (dir collapsed to a single file)
#   src-tauri/src/logging/       -> logging.rs
#   src-tauri/src/bootstrap/     -> bootstrap.rs
# Legacy names are kept below so the skill still works on older branches.
# ============================================================================

# --- UTF-8 locale detection (called once at startup) ----------------------
# Tries multiple locale names; sets _UTF8_OK=1 if bash character slicing
# works correctly, _UTF8_OK=0 if byte-safe fallback is needed.
_UTF8_OK=0
ensure_utf8_locale() {
    # Quick bash-level test: create a 2-byte UTF-8 char (Æ = 0xC3 0x86).
    # In UTF-8 locale: ${#s} = 1;  in C locale: ${#s} = 2.
    local test_bytes
    test_bytes=$(printf '\xc3\x86')
    if [ "${#test_bytes}" = "1" ]; then
        _UTF8_OK=1
        return 0
    fi
    # Current locale is not UTF-8 — try common names.
    for _loc in C.UTF-8 C.utf8 en_US.UTF-8 en_US.utf8 zh_CN.UTF-8 zh_CN.utf8; do
        if LC_ALL="$_loc" bash -c 'test "${#$(printf "\xc3\x86")}" = "1"' 2>/dev/null; then
            export LC_ALL="$_loc" LANG="$_loc"
            _UTF8_OK=1
            return 0
        fi
    done
    # No UTF-8 locale available — caller should use byte-safe truncation.
    return 1
}

# --- UTF-8-safe string truncation ------------------------------------------
# Truncates $1 to at most $2 characters (default 40), never splitting a
# multi-byte UTF-8 sequence.  Uses fast bash slicing when _UTF8_OK=1,
# falls back to od-based byte parsing otherwise.
truncate_utf8() {
    local s="$1"
    local max="${2:-40}"
    if [ "${_UTF8_OK:-0}" = "1" ]; then
        printf '%s' "${s:0:$max}"
        return
    fi
    # Byte-safe fallback: convert to hex via od (1 subprocess), then
    # walk UTF-8 sequences in pure bash, converting back with printf %b.
    local hex result_hex count i total b seglen
    hex=$(printf '%s' "$s" | od -An -tx1 | tr -d ' \n')
    result_hex=""
    count=0
    i=0
    total=${#hex}
    while [ "$i" -lt "$total" ] && [ "$count" -lt "$max" ]; do
        b=$((16#${hex:i:2}))
        seglen=2            # hex digits per character (1 byte = 2 hex)
        if   [ "$b" -ge 240 ]; then seglen=8  # 4-byte UTF-8
        elif [ "$b" -ge 224 ]; then seglen=6  # 3-byte (CJK)
        elif [ "$b" -ge 192 ]; then seglen=4  # 2-byte
        fi
        result_hex+="${hex:i:seglen}"
        i=$((i + seglen))
        count=$((count + 1))
    done
    # Convert hex pairs back to \xHH escapes, then let printf %b decode.
    local escaped="" j
    j=0
    while [ "$j" -lt "${#result_hex}" ]; do
        escaped+="\\x${result_hex:j:2}"
        j=$((j + 2))
    done
    printf '%b' "$escaped"
}

# Canonical list of "core" subsystems. Touching >=2 of these signals extra
# architecture-boundary attention. Legacy names retained for cross-branch use.
CORE_SUBSYSTEMS="plugin-api plugin-protocol plugin-host plugin-sdk-rust platform-windows commands plugin_framework builtin_plugin plugin_system plugin core sdk cli_server bridge"

# Map a repo-relative path to a subsystem label.
# Sets the global variable CLASSIFY_SUB (no subshell — safe for tight loops).
classify_subsystem() {
    case "$1" in
        # --- workspace crates -------------------------------------------------
        crates/plugin-api/*)           CLASSIFY_SUB="plugin-api" ;;
        crates/plugin-protocol/*)      CLASSIFY_SUB="plugin-protocol" ;;
        crates/plugin-host/*)          CLASSIFY_SUB="plugin-host" ;;
        crates/plugin-sdk-rust/*)      CLASSIFY_SUB="plugin-sdk-rust" ;;
        crates/platform-windows/*)     CLASSIFY_SUB="platform-windows" ;;
        zerolaunch-cli/*)              CLASSIFY_SUB="zerolaunch-cli" ;;
        plugin-template/*)             CLASSIFY_SUB="plugin-template" ;;
        xtask/*)                       CLASSIFY_SUB="xtask" ;;
        # --- src-tauri backend (current names) --------------------------------
        src-tauri/src/commands/*)          CLASSIFY_SUB="commands" ;;
        src-tauri/src/plugin_framework/*)  CLASSIFY_SUB="plugin_framework" ;;
        src-tauri/src/builtin_plugin/*)    CLASSIFY_SUB="builtin_plugin" ;;
        src-tauri/src/core/*)              CLASSIFY_SUB="core" ;;
        src-tauri/src/cli_server/*)        CLASSIFY_SUB="cli_server" ;;
        src-tauri/src/state/*)             CLASSIFY_SUB="state" ;;
        src-tauri/src/window/*)            CLASSIFY_SUB="window" ;;
        src-tauri/src/tray/*)              CLASSIFY_SUB="tray" ;;
        src-tauri/src/utils/*)             CLASSIFY_SUB="utils" ;;
        src-tauri/src/sdk.rs|src-tauri/src/sdk/*)           CLASSIFY_SUB="sdk" ;;
        src-tauri/src/logging.rs|src-tauri/src/logging/*)   CLASSIFY_SUB="logging" ;;
        src-tauri/src/bootstrap.rs|src-tauri/src/bootstrap/*) CLASSIFY_SUB="bootstrap" ;;
        src-tauri/src/lib.rs|src-tauri/src/main.rs)         CLASSIFY_SUB="app-entry" ;;
        # --- src-tauri backend (legacy names, pre-refactor branches) ----------
        src-tauri/src/plugin_system/*)     CLASSIFY_SUB="plugin_system" ;;
        src-tauri/src/plugin/*)            CLASSIFY_SUB="plugin" ;;
        # --- frontend ---------------------------------------------------------
        src-ui/bridge/*)               CLASSIFY_SUB="bridge" ;;
        src-ui/stores/*)               CLASSIFY_SUB="stores" ;;
        src-ui/composables/*)          CLASSIFY_SUB="composables" ;;
        src-ui/router/*)               CLASSIFY_SUB="router" ;;
        src-ui/views/*)                CLASSIFY_SUB="views" ;;
        src-ui/components/*)           CLASSIFY_SUB="components" ;;
        src-ui/plugins/*)              CLASSIFY_SUB="frontend-plugins" ;;
        src-ui/utils/*)                CLASSIFY_SUB="frontend-utils" ;;
        src-ui/i18n/*)                 CLASSIFY_SUB="i18n" ;;
        src-ui/styles/*)               CLASSIFY_SUB="styles" ;;
        # --- config / meta ----------------------------------------------------
        src-tauri/Cargo.toml|Cargo.toml) CLASSIFY_SUB="workspace-config" ;;
        .claude/rules/*)               CLASSIFY_SUB="rules" ;;
        *)                             CLASSIFY_SUB="other" ;;
    esac
}

# Return 0 if the given subsystem label is a "core" subsystem.
is_core_subsystem() {
    local sub="$1"
    for s in $CORE_SUBSYSTEMS; do
        [ "$sub" = "$s" ] && return 0
    done
    return 1
}

# Map a repo-relative path to the .claude/rules/*.md files that govern it.
# Sets the global variable MAP_RULES (space-separated filenames, no subshell).
map_rules_for_path() {
    case "$1" in
        src-tauri/src/builtin_plugin/*|src-tauri/src/plugin_framework/*|\
        src-tauri/src/plugin/*|src-tauri/src/plugin_system/*)
            MAP_RULES="plugin-system.md data-flow.md general.md" ;;
        src-tauri/src/commands/*|src-ui/bridge/*|src-tauri/src/cli_server/*)
            MAP_RULES="commands.md data-flow.md general.md" ;;
        src-ui/*)
            MAP_RULES="frontend.md general.md" ;;
        crates/plugin-api/*|crates/plugin-protocol/*|crates/plugin-host/*|crates/plugin-sdk-rust/*)
            MAP_RULES="sdk.md directory-map.md" ;;
        crates/platform-windows/*)
            MAP_RULES="directory-map.md" ;;
        src-tauri/src/core/*)
            MAP_RULES="config.md data-flow.md general.md" ;;
        plugin-template/*)
            MAP_RULES="third-party-plugin.md sdk.md" ;;
        *)
            MAP_RULES="general.md" ;;
    esac
}
