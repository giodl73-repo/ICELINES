# Browser WASM pulse 27 — offline fragment-route correction

Date: 2026-10-04. REQ-BROWSER-001 / WP-BW-06. Goal remains active.

The worker compared fragment-bearing request URLs directly with the shell
allowlist. Requests retain fragments in the tested Request implementation;
bookmarked client routes therefore missed the cached HTML. The prior test named
hash reload used a fragment-free URL and did not cover this boundary.

The fetch handler now clears the fragment before matching the deployment root
and shell assets. Query-bearing requests, datasets, other origins and parent
paths remain excluded. A regression test exercises root/index bookmarks and an
asset fragment under the composed `/ICELINES/workbench/` path, including refusal
cases. It failed against the previous worker and passed after the correction.

## Evidence

- Initial failing test: `target/browser-pulse-27-before.txt`.
- `npm test`: 119 passed, zero failed; `target/browser-pulse-27-tests.txt`.
- Actual distribution verification: 21 shell assets, 75 WASM packages, four
  hand goldens; compressed application size 399,663 bytes.
- Shell build: `eb2d4aa41a2dd1a3b515fbc5a95efd946decc476b5092d322afccedba5fa6c41`.
- Worker template SHA-256 (manifest `worker_sha256`): `0cf24ac769be0a1e0dfe20dc2b1069b3a23f1fb5596610dbc4debd87363e89b9`.
- Current distribution composed with the actual documentation baseline into
  `target/browser-pages-preview-27/ICELINES`; preservation inventory is
  `target/browser-pages-preservation-27.json`.

In the actual in-app browser at
`http://127.0.0.1:8062/ICELINES/workbench/`, loaded the real 2024–25 regular
package, ran `p >= 100` (six rows), and explicitly saved it. Stopped server
session 40049 with Ctrl+C (exit 1), then independently confirmed HTTP connection
refusal. The production Reload application action reopened the cached shell.
WASM initialized and the saved public season restored all 905 skaters. Private
filters are omitted from the default bookmark, so that unfiltered restoration
was expected. Entering `p >= 100` again ran a fresh offline query with six rows;
Nikita Kucherov led with 121 points. The server remained stopped throughout.

![Fresh offline query](../screenshots/pulse-27-offline-query.png)

## Remaining gates

This supersedes the earlier local offline failures in pulses 19 and 25 for the
selected corrected build/browser/path. It does not establish deployed HTTPS,
all devices, full update lifecycle, or retained-artifact rollback. Pages source
migration/deployment, live endpoint access and representative mobile/resource
acceptance remain open. Relay hosting stays deferred until deployed-origin
testing, as the user requested. No remote settings or deployment changed.
