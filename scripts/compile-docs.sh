#!/usr/bin/env bash
# Compile IceLines proof source documents to docs/ output directories.
#
# Uses [[compile]] entries in proof.toml to route each source directory:
#   src/guides/       → docs/guides/
#   src/presentations → docs/presentations/
#
# Run with no arguments to compile everything:
#   bash scripts/compile-docs.sh
#
# Then validate:
#   proof check docs/
#
# Requires proof binary in PATH or at C:/src/target/release/proof.exe

set -e

PROOF="${PROOF:-C:/src/target/release/proof.exe}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if ! command -v "$PROOF" &>/dev/null && ! [ -f "$PROOF" ]; then
    echo "proof not found. Set PROOF=/path/to/proof or build from C:/src/proof"
    exit 1
fi

echo "IceLines doc compile"
echo "Root: $ROOT"
echo ""

# proof compile with no path reads [[compile]] from proof.toml and
# routes each source_dir → output_dir automatically.
"$PROOF" compile --root "$ROOT"

echo ""
echo "Validating compiled output..."
"$PROOF" check "$ROOT/docs/guides/" "$ROOT/docs/presentations/" "$ROOT/docs/TUTORIAL.md" --no-fail
