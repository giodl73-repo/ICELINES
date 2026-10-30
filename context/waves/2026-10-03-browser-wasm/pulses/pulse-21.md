# Browser WASM pulse 21 — bounded package transport

Date: 2026-10-03. REQ-BROWSER-001 / WP-BW-01 foundation, selected
WP-BW-03/05 reliability evidence. The preceding planning recap did not change
implementation state; this continuation revalidated the worktree and resumed
the pending transport verification. Goal remains active.

## Change

Published season downloads use the shared safe-read transport instead of an
unbounded fetch/body read. Each attempt covers headers and body with a 20-second
timeout, permits at most three retries, respects Retry-After, and shares a
120-second foreground deadline. Advertised package size limits are enforced on
Content-Length and streamed bytes. Credentials, redirects and caching remain
disabled. Length, SHA-256, source and context checks still precede activation;
shared Rust owns normalization.

Cancel is visible during season acquisition. Aborting releases the reader,
suppresses retries, hides Cancel and preserves previous active data. Generation
checks prevent obsolete activation. A regression test exposed cancellation
between headers and reader setup; that path now cancels the body explicitly.

## Verification

- `npm test`: 118 passed, zero failures. Five package-read cases cover exact
  bytes/safe retries, header and stream overflow, stalled-body timeout and
  four-attempt ceiling, header-to-body cancellation, and expired deadlines or
  invalid budgets before transport.
- `npm run check`: TypeScript passed.
- `node scripts/verify-browser-dist.mjs`: 21 shell assets, all 75 public
  packages loaded and queried through actual WASM, four hand-audited count
  sorts. Largest application plus one-package gzip sum: 399,568 bytes against
  the provisional 10 MiB budget. No Rust changes this pulse.

## Actual browser evidence

`scripts/prepare-browser-package-preview.mjs` copies the built application to
`target/browser-package-preview` and adds visible disposable transport controls.
It intercepts only same-origin content-addressed package reads. Catalog,
worker, WASM, package bytes and IndexedDB remain real. The modified shell fails
offline integrity intentionally; this fixture cannot prove offline readiness.

In-app browser tab 22, `http://127.0.0.1:8060/`:

1. Loaded and saved 2024-25 regular; queried `p >= 100`: six players,
   Kucherov first with 121 points.
2. Selected 2023-24 and stalled its body. Header remained 2024-25;
   status showed loading and Cancel was visible.
3. Cancelled: one request and one cancelled body. Status reported retained
   previous data; Cancel was hidden. Saved 2024-25 state and the identical
   six-row query remained available.
4. Enabled two synthetic HTTP 503 responses and loaded again. Third request
   succeeded with real bytes. Active context became 2023-24, state In memory,
   and the retained filter returned nine players, Kucherov first with 144.
   The new season was not automatically saved.

[Cancelled download](../screenshots/pulse-21-package-cancelled.png).
[Successful bounded retry](../screenshots/pulse-21-package-retry.png).

## Remaining gates

This proves selected fixture transport and UI behavior, not deployed live CORS,
server-stopped offline reopening, actual downloaded-file round trips, physical
mobile/accessibility breadth, migration runtime, complete native parity/resource
coverage, or Pages publish/update/rollback. Those release requirements remain
open. No publication or release approval is claimed.
