#!/usr/bin/env bash
# Compile IceLines proof source documents to docs/ output directories.
# Usage: bash scripts/compile-docs.sh
# Requires proof binary at C:/src/target/release/proof.exe or proof in PATH.

set -e

PROOF="${PROOF:-C:/src/target/release/proof.exe}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if ! command -v "$PROOF" &>/dev/null && ! [ -f "$PROOF" ]; then
    echo "proof binary not found. Set PROOF=/path/to/proof or build from C:/src/proof"
    exit 1
fi

echo "Compiling IceLines docs..."
echo "  Root: $ROOT"
echo "  proof: $PROOF"
echo ""

# Guides: src/guides/*.source.md → docs/guides/*.md
mkdir -p "$ROOT/docs/guides"
for src in "$ROOT/src/guides"/*.source.md; do
    base=$(basename "$src" .source.md)
    out="$ROOT/docs/guides/${base}.md"
    "$PROOF" compile "$src" -o "$out" --root "$ROOT"
done

# Presentations: src/presentations/*.source.md → docs/presentations/*.md
mkdir -p "$ROOT/docs/presentations"
for src in "$ROOT/src/presentations"/*.source.md; do
    base=$(basename "$src" .source.md)
    out="$ROOT/docs/presentations/${base}.md"
    "$PROOF" compile "$src" -o "$out" --root "$ROOT"
done

echo ""
echo "Done. Compiled guides:"
ls "$ROOT/docs/guides/"
echo ""
echo "Compiled presentations:"
ls "$ROOT/docs/presentations/"
