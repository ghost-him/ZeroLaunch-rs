#!/usr/bin/env bash
# -*- coding: UTF-8 -*-
# ============================================================================
# check-deps-direction.sh - Workspace dependency direction checker
#
# Usage: bash check-deps-direction.sh
#
# Checks that Cargo workspace crates follow the allowed dependency direction:
#   plugin-api (L1) <- plugin-protocol (L2) <- plugin-host (L3) <- src-tauri (L4)
#   plugin-api (L1) <- platform-windows (L2) <- src-tauri (L4)
#   plugin-api (L1) <- plugin-protocol (L2) <- plugin-sdk-rust (L3)
#     (the third-party Rust SDK runs as a subprocess and legitimately needs the
#      JSON-RPC protocol crate, so it sits at L3 alongside plugin-host)
#
# A lower-level crate depending on a higher-level crate is a violation.
#
# Exit codes: 0=no violations, 1=violations found
# ============================================================================
set -euo pipefail

export LANG="${LANG:-en_US.UTF-8}"
export LC_ALL="${LC_ALL:-en_US.UTF-8}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

cd "$PROJECT_ROOT"

# Crate hierarchy (lower number = lower level)
get_crate_level() {
    case "$1" in
        zerolaunch-plugin-api)        echo 1 ;;
        zerolaunch-plugin-protocol)   echo 2 ;;
        zerolaunch-platform-windows)  echo 2 ;;
        zerolaunch-plugin-host)       echo 3 ;;
        zerolaunch-plugin-sdk-rust)   echo 3 ;;
        zerolaunch-rs)                echo 4 ;;
        zerolaunch-cli)               echo 4 ;;
        *)                            echo 99 ;;
    esac
}

extract_deps() {
    local toml_file="$1"
    awk '
        /^\[dependencies\]/ || /^\[dev-dependencies\]/ { in_deps=1; next }
        /^\[/ { in_deps=0; next }
        in_deps && /zerolaunch-/ {
            match($0, /zerolaunch-[a-z-]+/)
            if (RSTART > 0) {
                dep = substr($0, RSTART, RLENGTH)
                print dep
            }
        }
    ' "$toml_file" | sort -u
}

# Internal module layer mapping (src-tauri/src/).
# Levels derived from the actual `use crate::` dependency graph.
# See references/architecture-principles.md P3 for the full table and rationale.
get_internal_level() {
    case "$1" in
        utils|logging)                    echo 0 ;;
        sdk)                              echo 1 ;;
        core)                             echo 2 ;;
        plugin_framework|tray|window)     echo 3 ;;
        builtin_plugin|state)             echo 4 ;;
        commands|cli_server|bridge_error) echo 5 ;;
        bootstrap|lib|main)               echo 6 ;;
        *)                                echo "" ;;
    esac
}

VIOLATIONS=0
CHECKED=0

echo "=== Workspace Dependency Direction Check ==="
echo ""

# Find all Cargo.toml files (excluding workspace root, target, node_modules)
TOML_FILES=$(find . -name "Cargo.toml" -not -path "./Cargo.toml" -not -path "*/target/*" -not -path "*/node_modules/*" 2>/dev/null || true)

for toml_file in $TOML_FILES; do
    crate_name=$(grep -m1 '^name =' "$toml_file" | sed 's/^name *= *"\([^"]*\)".*/\1/' || true)
    [ -z "$crate_name" ] && continue

    crate_level=$(get_crate_level "$crate_name")
    if [ "$crate_level" -ge 99 ]; then
        continue
    fi

    CHECKED=$((CHECKED + 1))
    deps=$(extract_deps "$toml_file")

    if [ -z "$deps" ]; then
        continue
    fi

    for dep in $deps; do
        dep_level=$(get_crate_level "$dep")

        if [ "$dep_level" -ge 99 ]; then
            continue
        fi

        if [ "$dep_level" -gt "$crate_level" ]; then
            VIOLATIONS=$((VIOLATIONS + 1))
            echo "[FAIL] $crate_name (L$crate_level) -> $dep (L$dep_level)"
            echo "       File: $toml_file"
            echo "       Reason: lower-level crate must not depend on higher-level crate"
            echo ""
        fi
    done
done

echo "---"
echo "Checked $CHECKED workspace crates"

if [ "$VIOLATIONS" -gt 0 ]; then
    echo "[FAIL] Found $VIOLATIONS workspace dependency direction violation(s)"
else
    echo "[OK] All workspace dependency directions are compliant"
fi

# --- Internal Module Layer Check (src-tauri/src/) ---------------------------
# Checks that `use crate::X` follows the internal layering: high → low only.
# See references/architecture-principles.md P3 for the layer table.
INTERNAL_VIOLATIONS=0
INTERNAL_CHECKED=0

echo ""
echo "=== Internal Module Layer Check (src-tauri/src/) ==="
echo ""

SRC_DIR="src-tauri/src"
if [ -d "$SRC_DIR" ]; then
    while IFS= read -r f; do
        # Determine the top-level module of this file from its path.
        rel="${f#$SRC_DIR/}"
        if [[ "$rel" == */* ]]; then
            caller_mod="${rel%%/*}"
        else
            caller_mod="${rel%.rs}"
        fi

        caller_level=$(get_internal_level "$caller_mod")
        [ -z "$caller_level" ] && continue
        INTERNAL_CHECKED=$((INTERNAL_CHECKED + 1))

        # Extract all crate::X first-segment references (use + fully-qualified).
        # LIMITATION: only `crate::X` patterns are checked. `super::` imports
        # (e.g. `use super::super::commands::BridgeError`) are NOT detected.
        # This is acceptable because super:: is typically used for sibling
        # modules within the same layer; cross-layer super:: chains are rare
        # and would be caught by the type-scope check (check-type-scope.sh).
        deps=$(grep -oE 'crate::[a-z_]+' "$f" 2>/dev/null | sed 's/crate:://' | sort -u || true)

        for dep in $deps; do
            # Skip self-references (module referencing its own sub-modules).
            [ "$dep" = "$caller_mod" ] && continue

            dep_level=$(get_internal_level "$dep")
            [ -z "$dep_level" ] && continue  # unknown top-level module

            if [ "$dep_level" -gt "$caller_level" ]; then
                INTERNAL_VIOLATIONS=$((INTERNAL_VIOLATIONS + 1))
                echo "[FAIL] $caller_mod (L$caller_level) -> $dep (L$dep_level) — reverse dependency"
                echo "       File: $f"
            fi
        done
    done < <(find "$SRC_DIR" -name '*.rs' -type f 2>/dev/null)
fi

echo "---"
echo "Checked $INTERNAL_CHECKED internal source files"

if [ "$INTERNAL_VIOLATIONS" -gt 0 ]; then
    echo "[FAIL] Found $INTERNAL_VIOLATIONS internal module layer violation(s)"
else
    echo "[OK] All internal module layers are compliant"
fi

# --- Combined exit code -----------------------------------------------------
TOTAL_VIOLATIONS=$((VIOLATIONS + INTERNAL_VIOLATIONS))
echo ""
if [ "$TOTAL_VIOLATIONS" -gt 0 ]; then
    echo "[FAIL] Total: $VIOLATIONS workspace + $INTERNAL_VIOLATIONS internal = $TOTAL_VIOLATIONS violation(s)"
    exit 1
else
    echo "[OK] All dependency directions (workspace + internal) are compliant"
    exit 0
fi
