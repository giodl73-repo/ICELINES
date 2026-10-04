# Browser WASM pulse 25 — composed subpath runtime and full check

Date: 2026-10-04. REQ-BROWSER-001 / WP-BW-05/06 selected evidence.
Previous turn was progress: preserved-site publication composition. Goal active.

## Full checked build

`node scripts/build-browser.mjs --offline --check` completed with exit 0.
Log: `target/browser-pulse-25-build.txt`.

- Actual WASM: 21 shell assets, 75 public packages, four hand-audited count
  sorts; largest application plus one package gzip sum 399,568 bytes.
- Native-loader/distributed-WASM: 270 cases, 33,613 full rows, 90 empty results.
- Rust: schedule 5; core stats catalog 88; portable data 2; WASM 8;
  native/browser loader parity 2; native stats loader 42; web leaders 6.
- Python: catalog 7 and Pages composition 4.
- Browser Node: 118, no failures.

Incremental-cache hard-link fallback warnings appeared; checks completed normally.
This is the local Windows pipeline, not remote GitHub Linux execution.

## Project-subpath observation

Composed the actual gh-pages documentation and browser distribution under
`target/browser-pages-preview-25/ICELINES`. The staged shell-manifest digest was
checked against the current built distribution before serving. A local server
at port 8061 served the exact `/ICELINES/workbench/` path; no fault adapter or
modified application shell was used.

In-app browser tab 23:

1. Opened `/ICELINES/workbench/`; all 75 catalog choices were available.
2. Loaded 2024-25 regular via the worker/WASM. Queried `p >= 100`, producing
   six players; saved it explicitly.
3. Generated a shared link with explicit public-filter inclusion:
   `#/leaders?season=20242025&type=regular&kind=skaters&sort=points&gp=&filter=p+%3E%3D+100`.
4. Navigated to that link and clicked production Reload application. After
   initialization, status was Loaded your saved public season, state Saved
   locally, filter `p >= 100`, six rows, Kucherov first with 121 points.

## Offline observation and recovery

Before the test, shell integrity status was available offline. Stopped the
confirmed server handle 33992 (terminal exit 1), then independently confirmed
HTTP connection refusal. Clicking the app's own reload button reached an
in-app-browser connection-refused error document; browser-use URL policy blocked
subsequent interaction with that data-URL page. No policy bypass was attempted.
Offline reopening failed in this environment. The observation does not identify
an application versus host-navigation cause, and cannot close the offline gate.

Restarted only after server terminal state was established (new handle 80377),
confirmed HTTP 200, and opened a fresh allowed HTTP tab 24 with the bookmark.
It restored the saved season and six-row query online. Root `/ICELINES/` in tab
25 rendered the preserved IceLines tracker, Material navigation and rankings.
This verifies root rendering, not every legacy link or legacy hockey calculation.

[Online restored subpath](../screenshots/pulse-25-subpath-restored.png).

## Remaining gates

Representative local subpath loading/bookmark/saved restore now have direct
browser evidence. Exact deployed HTTPS behavior, live CORS or an allocated
relay, offline reopen, physical mobile/accessibility/resource breadth, migration
runtime, Pages source migration, remote workflow and deploy/rollback remain open.
No publication or ship claim. Avoid another identical in-app offline test until
a diagnostic change or different authorized browser environment can yield new
evidence.
