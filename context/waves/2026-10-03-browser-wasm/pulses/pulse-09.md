# Browser WASM pulse 09 — storage failure recovery

Date: 2026-10-03. Parent: WP-BW-03 / REQ-BROWSER-001 / CHG-119.

## Implementation

Library reads now resolve only after their IndexedDB transaction completes.
Saves snapshot the input bytes and policy before asynchronous access; exceptions
while writing either the dataset or active pointer abort the whole transaction.
A failed cross-tab notification cannot leave a committed save pending or report
it as failed. Blocked opens reject with recovery instructions and close any
connection delivered after rejection. Future database versions and missing
required stores are refused without resetting or deleting the database.

The Saved library has a separate live status and Retry local storage action.
Failed reads clear stale library controls and saved-state claims while retaining
the current engine dataset, query results and exports. Storage failure status
remains visible independently of query/startup status. Restored access reloads
the saved records and updates the persistence state.

## Verification

`npm test` in `icelines-browser` passed all 60 Node tests, including five new
cases: failure of the active-pointer write rolls back a staged package; input
ownership precedes asynchronous access; denied storage preserves old records;
notification failure does not change commit success; and a future database
version preserves its original records. TypeScript compilation passed.
`node scripts/verify-browser-dist.mjs` verified 16 shell assets and loaded/queried
all 75 public packages through actual WASM. Whitespace checks passed.

The local acceptance harness at `http://127.0.0.1:8053/` uses the actual browser
IndexedDB and WASM engine with synthetic denial/quota fault signals. It is
prepared by `scripts/prepare-browser-storage-preview.mjs`; its controls and
fault script are outside the publishable distribution.

- Saved 20242025 regular-season data, then denied storage access and retried.
  Library became unavailable, but all 905 rows and package export remained usable.
- Restoring access and retrying recovered the saved state without reimport.
- A quota exception during the active-pointer write rejected a persistence-policy
  change and reset the checkbox. Reload restored 905 rows and the original false
  policy from the previous saved record.

Screenshot: [denied-access state](browser-pulse-09.png).
The harness modifies index.html, so offline shell integrity rejection is expected
there; this evidence does not test offline opening or real browser quota limits.

## Remaining gates

Saved-record content/schema validation, supported migrations, interrupted
transactions/migrations, real blocked/version-change coordination, storage-loss
reimport, and deployed browser behavior still need evidence. Live refresh,
scores/schedule, offline-open/update, mobile/accessibility, measured performance,
native ordering parity and Pages deployment/rollback gates remain open.
