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
    echo "[FAIL] Found $VIOLATIONS dependency direction violation(s)"
    exit 1
else
    echo "[OK] All dependency directions are compliant"
    exit 0
fi
