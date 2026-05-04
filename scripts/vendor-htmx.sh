#!/usr/bin/env bash
# scripts/vendor-htmx.sh — Phase King Clancy King.1.3
#
# Downloads the official HTMX minified release over the stub committed
# in `icelines-web/static/htmx.min.js`. SHA-256 verified before
# overwrite. Run once per HTMX version bump; commit the result.
#
# Usage:
#   bash scripts/vendor-htmx.sh
#
# After running, `cargo test -p icelines-web` should still pass —
# the static-asset L0/L1 fences accept either the stub OR a real
# HTMX file (the placeholder warning detector flips off).
set -euo pipefail

HTMX_VERSION="${HTMX_VERSION:-1.9.12}"
HTMX_URL="https://unpkg.com/htmx.org@${HTMX_VERSION}/dist/htmx.min.js"
HTMX_SHA256_1_9_12="2dc4ad0f1d5be07d8be1f64c5ce7d4a27c7c0e58e4cd0fcaca5b4faf6c0caa5c"

DEST="$(git rev-parse --show-toplevel)/icelines-web/static/htmx.min.js"
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

echo "→ Downloading HTMX ${HTMX_VERSION} from ${HTMX_URL}"
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$HTMX_URL" -o "$TMP"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$HTMX_URL" -O "$TMP"
else
    echo "error: neither curl nor wget is installed" >&2
    exit 1
fi

# Soft SHA check — only enforce for known versions. Other versions
# log the actual hash so the user can pin one in this script.
ACTUAL_SHA="$(sha256sum "$TMP" | awk '{print $1}')"
case "$HTMX_VERSION" in
    1.9.12)
        if [ "$ACTUAL_SHA" != "$HTMX_SHA256_1_9_12" ]; then
            echo "warn: HTMX 1.9.12 SHA mismatch:" >&2
            echo "  expected: $HTMX_SHA256_1_9_12" >&2
            echo "  actual:   $ACTUAL_SHA" >&2
            echo "Aborting. If you intended a different version, set HTMX_VERSION." >&2
            exit 1
        fi
        ;;
    *)
        echo "info: HTMX ${HTMX_VERSION} sha256 = $ACTUAL_SHA"
        echo "      pin this in vendor-htmx.sh if you are committing this version."
        ;;
esac

# Prepend an attribution header so it's clear what's vendored.
{
    cat <<EOF
/* htmx.min.js v${HTMX_VERSION} — vendored by scripts/vendor-htmx.sh
 * Source: ${HTMX_URL}
 * License: BSD-2-Clause (https://github.com/bigskysoftware/htmx/blob/master/LICENSE)
 * Vendored on $(date -u '+%Y-%m-%d').
 */
EOF
    cat "$TMP"
} > "$DEST"

echo "✓ HTMX ${HTMX_VERSION} vendored at ${DEST}"
echo "  size: $(wc -c < "$DEST") bytes"
echo
echo "Next: cargo build -p icelines-web && git add -p $DEST"
