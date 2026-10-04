# Browser WASM pulse 19 — verified shell readiness and repair

Date: 2026-10-03. Scope: REQ-BROWSER-001 / WP-BW-01 foundation; selected
WP-BW-05 recovery evidence. The preceding planning turn made authoritative
documentation changes and is classified as progress. This pulse adds code and
runtime evidence; the overall goal remains active.

## Change

Offline startup checks every actual cached shell asset's size and SHA-256 before
reporting ready. Missing, changed, or evicted files report unavailable. Worker
messages are scoped to same-origin deployment clients; UI messages must come
from the registered worker URL.

The unavailable state exposes Repair offline files. The active worker downloads
only its fixed shell allowlist with no-store, omitted credentials, no redirects,
and a 120-second acquisition deadline. All downloads are verified in a staging
cache before any active-cache replacement. Network/integrity failure preserves
existing files. Concurrent requests share one acquisition and each requesting
tab receives its result. Staging is removed on completion or failure. Repairs
do not touch IndexedDB datasets, reload the app, or activate another build.
Cache API copying is not atomic: a cache-write failure can leave an incomplete
same-build cache, reports failure, and cannot justify readiness without recheck.

## Verification

Browser test suite: 108 tests pass after the final deadline regression was added.
Selected offline/UI tests cover readiness after eviction, missing and same-size
corruption, deployment scoping, explicit repair, failed late download/integrity,
two-tab repair sharing, bounded acquisition cancellation, retry UI, and no reload.

Distribution verifier: 20 shell assets and 75 public packages loaded and queried
with actual WASM; four hand-audited count sorts pass. Largest application plus
one package gzip sum: 398,014 bytes against the provisional 10 MiB budget.
No Rust source changes were made by this pulse.

## Actual browser evidence

`prepare-browser-offline-preview.mjs` copies the unchanged production
distribution into `target/browser-offline-preview` and adds a nonpublished test
page. Served on `http://localhost:8058/`, in-app browser tab 18:

1. Installed shell reported verified ready.
2. Remove cached stylesheet (test) deleted that fixture build's stylesheet.
3. UI reported missing/damaged cache and exposed Repair offline files.
4. Repair showed downloading/verifying, then verified ready and hid its button.
5. Repeated the same removal/repair flow while recording readable screenshots.

[Missing-cache state](browser-pulse-19-cache-missing.png).
[Repaired state](browser-pulse-19-cache-repaired.png).
Some screenshot API captures rendered inconsistently with DOM bounds; these
final captures use the documented screenshot method after an AX-state refresh.
This fixture validates recovery behavior, not production layout acceptance.

Tab 19 opened the unchanged application, loaded repository 2024-25 regular
season, saved it locally, and returned six rows for `p>=100`. The server's live
session was polled, then stopped with terminal exit 1; an independent HTTP
request confirmed port 8058 unreachable. Clicking the application's Reload
button led to a browser-generated data-URL error document, and application status
could no longer be located. Offline reopening therefore failed in this browser
environment. The observation does not distinguish application behavior from the
in-app browser's navigation/network handling; no bypass was attempted.

The server was restarted only after terminal shutdown was confirmed. Fresh tab
20 restored the saved repository 2024-25 regular dataset with 905 players,
demonstrating saved data survived; this was an online restore, not offline proof.

## Remaining release gates

Deployed live access or an allocated relay, server-stopped offline reopening,
actual browser file interchange, legacy migration runtime proof, representative
mobile/assistive-technology checks, full scoped native parity/resource measures,
and Pages deployment/rollback remain open. No publish, commit, or ship claim.
