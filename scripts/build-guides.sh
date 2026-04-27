#!/usr/bin/env bash
# build-guides.sh — Compile all IceLines guides from src/guides/ to docs/guides/
#
# Usage:
#   scripts/build-guides.sh           # compile all guides
#   scripts/build-guides.sh --check   # validate without writing
#   scripts/build-guides.sh [filter]  # compile only guides matching filter
#
# Requires proof CLI — built from C:\src (workspace root, siblings: proof/ + mdpath/)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_DIR="${REPO_ROOT}/src/guides"
OUT_DIR="${REPO_ROOT}/docs/guides"

# Locate proof binary — workspace target takes priority
WORKSPACE_TARGET="${REPO_ROOT}/../target"
PROOF="${WORKSPACE_TARGET}/debug/proof"
if [ ! -f "${PROOF}" ] && [ ! -f "${PROOF}.exe" ]; then
    PROOF="${REPO_ROOT}/../proof/target/debug/proof"
fi
if [ ! -f "${PROOF}" ] && [ ! -f "${PROOF}.exe" ]; then
    echo "proof binary not found. Build from the workspace root:"
    echo "  cd C:/src && cargo build"
    exit 1
fi

CHECK_ONLY=false
FILTER=""

for arg in "$@"; do
    case "$arg" in
        --check) CHECK_ONLY=true ;;
        --help)
            echo "Usage: build-guides.sh [--check] [filter]"
            exit 0 ;;
        *) FILTER="$arg" ;;
    esac
done

mkdir -p "${OUT_DIR}"

COMPILED=0
ERRORS=0

compile_one() {
    local src="$1"
    local base
    base="$(basename "$src")"

    local out_name
    if [[ "$base" == *.slides.source.md ]]; then
        out_name="${base%.slides.source.md}.slides.md"
    elif [[ "$base" == *.dashboard.source.md ]]; then
        out_name="${base%.dashboard.source.md}.dashboard.md"
    else
        out_name="${base%.source.md}.md"
    fi

    local out="${OUT_DIR}/${out_name}"
    echo "  compiling: ${base}"

    if $CHECK_ONLY; then
        if "${PROOF}" compile --check --root "${REPO_ROOT}" "${src}" 2>&1; then
            echo "    [ok] ${base}"
        else
            echo "    [FAIL] ${base}"
            ERRORS=$((ERRORS + 1))
        fi
    else
        if "${PROOF}" compile --root "${REPO_ROOT}" -o "${out}" "${src}" 2>&1; then
            echo "    → ${out_name}"
            COMPILED=$((COMPILED + 1))
        else
            echo "    [FAIL] ${base}"
            ERRORS=$((ERRORS + 1))
        fi
    fi
}

echo ""
echo "icelines guide build"
echo "  source: ${SRC_DIR}"
echo "  output: ${OUT_DIR}"
echo ""

while IFS= read -r src; do
    base="$(basename "$src")"
    if [ -n "$FILTER" ] && [[ "$base" != *"${FILTER}"* ]]; then
        continue
    fi
    compile_one "$src"
done < <(find "${SRC_DIR}" -name "*.source.md" | sort)

echo ""
if $CHECK_ONLY; then
    echo "check complete — ${ERRORS} errors"
else
    echo "compiled ${COMPILED} guides → ${OUT_DIR}"
    [ "${ERRORS}" -gt 0 ] && echo "  WARNING: ${ERRORS} guides failed"
fi

[ "${ERRORS}" -eq 0 ] || exit 1
