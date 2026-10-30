# Browser WASM pulse 02 — browser slice and bounded acquisition

Date: 2026-10-03
Parent: WP-BW-01 / REQ-BROWSER-001 / CHG-119.

## Authoritative implementation

The optimized Rust artifact exists (1,010,966 bytes before binding processing).
Matching wasm-bindgen 0.2.118 generated web bindings; the processed WASM file is
569,030 bytes. These are uncompressed file sizes, not peak-memory measurements.
TypeScript browser shell, dedicated worker, session package activation, shared
queries, player detail, exports and explicit IndexedDB save/remove are implemented.

Acquisition now enforces 20-second request/body timeouts, three retries, a shared
120-second refresh deadline, Retry-After, a 2 MiB source-page limit, and at most
two simultaneous report requests. Incomplete/changing pagination is rejected;
sibling requests abort after required-family failure. Download/import activation
checks its acquisition generation before replacing a newer selection. Query
failures clear prior visible/exportable results. Refresh failure and post-refresh
save failure have distinct recovery messages.

## Verification

`npm test` built TypeScript and passed 13 fixture acquisition tests: transient
HTTP/network failures, Retry-After/deadline, retry bound, terminal client/schema
failure, oversized stream, cancellation before/during body, request/body timeout,
date parsing, report concurrency, incomplete/changing pagination and invalid
season rejection. No automated test contacts live upstreams. A subsequent
`npm run check` passed after UI acquisition-generation changes. Final `npm test`
rebuilt the latest UI and passed all **18** tests: the 13 acquisition cases plus
five IndexedDB fixtures covering owned saved bytes, atomic active-pointer writes,
conflict rollback, competing writers, and independent removal behavior.
The pinned fake-indexeddb 6.2.5 dependency installed through the configured npm
proxy under network escalation. Sandbox proxy DNS and a public-registry TLS
attempt failed first; no TLS verification setting was weakened.

Earlier browser-preview evidence: the Rust engine displayed filtered leaders;
explicit saving and page reload restored the local dataset. Direct live NHL
refresh failed with Failed to fetch and retained the saved dataset. This is local
origin evidence only, not a deployed Pages/CORS acceptance check.

## Remaining gates

Full goal remains active. Hosted live-path disposition/proof, actual downloaded
release conversion, browser archive import, scores/schedule, automatic refresh
coordination, offline shell/update handling, lifecycle/quota/migration/crash
tests, sorting/rate-floor parity, measured resource budgets, screenshot review
and Pages build/deployment are unverified or incomplete. The passed Node
IndexedDB cases do not establish real-browser quota, denied storage, version
migration or cross-tab notification behavior. UI race/error changes have passed
TypeScript build but need browser interaction evidence. No shipped browser
status is promoted.
