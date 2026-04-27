#!/usr/bin/env bash
# build-guides.sh — Compile all IceLines guides from src/guides/ to docs/guides/

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_DIR="${REPO_ROOT}/src/guides"
OUT_DIR="${REPO_ROOT}/docs/guides"

WORKSPACE_TARGET="${REPO_ROOT}/../target"
PROOF="${WORKSPACE_TARGET}/debug/proof"
if [ ! -f "${PROOF}" ] && [ ! -f "${PROOF}.exe" ]; then
    PROOF="${REPO_ROOT}/../proof/target/debug/proof"
fi

CHECK_ONLY=false
FILTER=""

for arg in "$@"; do
    case "$arg" in
        --check) CHECK_ONLY=true ;;
        --help) echo "Usage: build-guides.sh [--check] [filter]"; exit 0 ;;
        *) FILTER="$arg" ;;
    esac
done

if [ ! -f "${PROOF}" ] && [ ! -f "${PROOF}.exe" ]; then
    echo "proof binary not found. Build from workspace root: cd C:/src && cargo build"
    exit 1
fi

mkdir -p "${OUT_DIR}"

echo ""
echo "icelines guide build"
echo "  source: ${SRC_DIR}"
echo "  output: ${OUT_DIR}"
echo ""

if [ -n "$FILTER" ]; then
    COMPILED=0; ERRORS=0
    while IFS= read -r src; do
        base="$(basename "$src")"
        [[ "$base" == *"${FILTER}"* ]] || continue
        echo "  compiling: ${base}"
        if $CHECK_ONLY; then
            "${PROOF}" compile --check --root "${REPO_ROOT}" "${src}" 2>&1 \
                && echo "    [ok]" || { echo "    [FAIL]"; ERRORS=$((ERRORS+1)); }
        else
            if "${PROOF}" compile --root "${REPO_ROOT}" --output-dir "${OUT_DIR}" "${src}" 2>&1; then
                COMPILED=$((COMPILED+1))
            else
                ERRORS=$((ERRORS+1))
            fi
        fi
    done < <(find "${SRC_DIR}" -name "*.source.md" | sort)
    echo ""
    $CHECK_ONLY && echo "check complete — ${ERRORS} errors" \
                || echo "compiled ${COMPILED} guides → ${OUT_DIR}"
    [ "${ERRORS}" -eq 0 ] || exit 1
else
    if $CHECK_ONLY; then
        "${PROOF}" compile --check --root "${REPO_ROOT}" "${SRC_DIR}" 2>&1
    else
        "${PROOF}" compile --root "${REPO_ROOT}" --output-dir "${OUT_DIR}" "${SRC_DIR}" 2>&1
        echo ""
        echo "compiled all guides → ${OUT_DIR}"
    fi
fi
