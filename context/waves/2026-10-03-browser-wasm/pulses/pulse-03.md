# Browser WASM pulse 03 — offline shell and update coordination

Date: 2026-10-03. Parent: WP-BW-01 / REQ-BROWSER-001 / CHG-119.

## Implemented

Build-time shell manifest pins SHA-256 hashes of index/style, compiled UI/worker
modules, generated WASM/bindings and public catalog. Service-worker installation
verifies hashes and removes an incomplete cache on failure. Shell cache names
include deployment base path plus build fingerprint. Fetch interception accepts
only exact allowlisted GET URLs; live endpoints, dataset files, query-bearing
URLs and other deployments pass through without Cache API persistence.
Missing cached engine files produce a visible 503 rather than a mixed network
build. Activation retires only the current deployment's old shell caches.

An explicit update action requires consent from all open app tabs before
skipWaiting; the UI warns to save/export memory-only work before reloading.
Application reload is also available directly. No dataset persistence policy is
changed by the service worker.

## Verification and limits

TypeScript build and all **25 Node tests** passed after initial offline changes.
Seven additional tests cover complete shell install, corrupt-asset rollback,
Pages project-root resolution, dataset/private-query/non-GET exclusions, refusing
mixed-build fallback, cache namespace isolation, and all-tab update consent.
The final reload-control addition passed a TypeScript build and the repeated
`npm test` run passed all 25 tests. Current summed gzip
asset size is **233,021 bytes**; this excludes SW/manifest headers, requested season
packages and runtime memory. It is not a whole deployment/download measurement.

Embedded browser preview: `http://127.0.0.1:8048/ICELINES/` loaded the Rust worker,
installed the shell, filtered p>=100 to eight rows and saved the selected dataset.
The waiting app update activated via the UI and restored that saved dataset;
the query again returned eight rows. Both browser-tool reload and in-app reload
after stopping the known server process failed to reopen the app. The preview
showed ERR_CONNECTION_REFUSED; inspection was then blocked on its generated
data-URL error page by browser tool policy. No offline capability is accepted
from the installed-state message alone, and no policy bypass was attempted.
Further normal/deployed browser offline evidence is required; root cause is
not proven to be application code versus the embedded navigation environment.

Preview restored after the test: loopback server exec session **61659** at port
8048, serving `target/browser-preview`; browser tab **5** marked for continuation.
Working slice screenshot: [browser-pulse-03.png](browser-pulse-03.png).

## Remaining scope

Full goal active. Offline reopen, normal/deployed browser upgrades and cross-tab
consent, real live path/relay host, release archive conversion/download, browser
archive import, scores/schedule, refresh coordination, worker recovery, storage
quota/migrations, sort/rate-floor parity, resource budgets and Pages deployment
remain incomplete or unverified. No shipped capability is promoted.
