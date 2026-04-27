#!/usr/bin/env bash
# Compile IceLines proof source documents to docs/ output directories.
#
# Uses proof.toml [compile] config for guides (source_dir → output_dir).
# Presentations use explicit -o flag.
#
# Usage:
#   bash scripts/compile-docs.sh           # compile all
#   bash scripts/compile-docs.sh guides    # guides only
#   bash scripts/compile-docs.sh pres      # presentations only

set -e

PROOF="${PROOF:-C:/src/target/release/proof.exe}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="${1:-all}"

if ! command -v "$PROOF" &>/dev/null && ! [ -f "$PROOF" ]; then
    echo "proof not found. Set PROOF=/path/to/proof"
    exit 1
fi

echo "IceLines doc compile — proof $("$PROOF" --version 2>&1 | head -1)"
echo "Root: $ROOT"
echo ""

if [[ "$TARGET" == "all" || "$TARGET" == "guides" ]]; then
    echo "Compiling guides → docs/guides/"
    # proof.toml [compile] routes src/guides → docs/guides automatically
    "$PROOF" compile src/guides --root "$ROOT" -c "$ROOT/proof.toml"
fi

if [[ "$TARGET" == "all" || "$TARGET" == "pres" ]]; then
    echo "Compiling presentations → docs/presentations/"
    mkdir -p "$ROOT/docs/presentations"
    for src in "$ROOT/src/presentations"/*.source.md; do
        base=$(basename "$src" .source.md)
        "$PROOF" compile "$src" -o "$ROOT/docs/presentations/${base}.md" --root "$ROOT"
    done
fi

echo ""
echo "Done."
