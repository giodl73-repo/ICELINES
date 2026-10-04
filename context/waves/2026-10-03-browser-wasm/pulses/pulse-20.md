# Browser WASM pulse 20 — catalog failure recovery

Date: 2026-10-03. REQ-BROWSER-001 / WP-BW-01 foundation; selected WP-BW-03/05
recovery evidence. Previous turn classified as progress: implemented cache
repair, passing regression checks, and browser observations. Goal remains active.

## Change and purpose

Startup previously threw on a failed catalog fetch before checking saved season
data. It now handles catalog acquisition/validation failure separately and
continues engine, season-library and schedule restoration. Catalog status has a
visible retry control. An absent catalog disables only public season loading;
saved data selection and imports remain available. Retrying catalog access
updates choices while preserving the active bytes and completed result.

The catalog parser requires schema 1, at most 2048 entries, unique IDs and
season/type contexts, valid consecutive season years, supported season types,
bounded source/identity strings, integer sizes/counts and lowercase SHA-256.
Every runtime package URL must equal `data/<advertised-sha256>.json`; arbitrary
origins, traversal, query strings and fragments are refused. Additional published
provenance fields are permitted; only validated runtime fields are projected.
Catalog acquisition uses the shared reader's 2 MiB limit, safe GET retries,
omitted credentials/no-store/no redirects, and 20-second total deadline.

Package acquisition checks length, SHA-256 and source/season/type before engine
activation and refuses redirects. Shared Rust still owns domain normalization
and row validation. Missing catalogs do not authorize private saved-data
substitution into incoming public routes. Root startup can restore the selected
saved dataset; without catalog identity it produces local-only view links.

## Verification

- `npm test`: 113 passing tests. New tests validate the actual 75-package catalog,
  hostile paths, bad metadata/schema, duplicate identities/contexts, and safe
  acquisition options plus terminal errors. Existing storage/worker/offline/
  schedule/query regressions also pass.
- `verify-browser-dist.mjs`: 21 shell assets, 75 public packages loaded and queried
  with actual WASM, four hand-audited count sorts. Largest application plus
  one-package gzip sum: 399,317 bytes against the provisional 10 MiB budget.
- TypeScript check passed during implementation. No Rust changes this pulse.

## Actual browser observation

`prepare-browser-catalog-preview.mjs` creates a disposable localhost copy with a
catalog-only fetch fault controlled through visible UI. Package requests, worker,
WASM and IndexedDB remain real. Its modified index intentionally fails shell
integrity and cannot establish offline-opening readiness.

In-app browser tab 21 at `http://localhost:8059/`:

1. Initial synthetic catalog 404 displayed unavailable status and Retry public
   catalog; public Load season was disabled, imports remained enabled.
2. Disabled the fault and retried: catalog reported all 75 packages.
3. Loaded repository 2024-25 regular and saved locally.
4. Enabled the fault and followed Restart at application root. Startup reported
   Restored your saved dataset while catalog status remained unavailable.
5. Queried `p>=100`; actual WASM returned six players, led by Nikita Kucherov
   with 121 points. Active state remained Saved locally, repository:20242025.
6. Removed the fault and retried: 75 packages returned, saved state and six-row
   result remained. No dataset reload or activation was requested by retry.

[Catalog unavailable with saved restore](browser-pulse-20-catalog-unavailable.png).
[Query from restored saved data](browser-pulse-20-saved-query.png).

## Remaining acceptance

The fixture proves a catalog-only failure path with engine assets available. It
does not prove server-stopped offline reopening, real browser file interchange,
live upstream access, physical mobile/assistive-technology coverage, full scoped
parity/resource measurements, or Pages deployment/rollback. Those gates remain
open. No publication or ship claim.
