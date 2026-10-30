# Browser WASM pulse 06 — shared ordering and qualification

Date: 2026-10-03. Parent: WP-BW-01 / REQ-BROWSER-001 / CHG-119.

## Changes

Browser pace ordering now calls `sort_views_by_pace`, preserving the shared
goals-per-82 tie-break and final player ID ordering. Previously it sorted only
the displayed pace value and ID. Browser goalie ordering now calls the same
`GoalieLeaderboardSort` comparator used by native goalie views.

Extracted native goalie qualification into the pure core helper
`qualified_goalie_games`: regular 5 GP, playoff 1 GP. The native web route delegates
to it with unchanged threshold behavior. The browser request accepts an omitted
or null minimum for that default, and still accepts explicit numeric values;
zero includes all players. Skaters default to zero. Query evidence exports both
the applied minimum and the shared 10-GP pace guard. UI help explains the blank
default and zero override. Saved dataset policy is unchanged.

## Verification

`node scripts/build-browser.mjs --offline --check` passed: 2 portable-data,
7 WASM engine, 42 native loader, 2 expanded native/browser integration,
7 Python catalog and 37 Node tests (97 total). The two parity integration tests
now compare full ordering across regular/playoff contexts, nine supported
skater/goalie sorts, and default/0/1/5/10 GP floors (90 combinations total).
The pace fixture has hand-audited expected IDs for equal pace, goal tie-breaking,
deterministic equal-value ordering and the 9/10-GP boundary. Wire tests preserve
explicit zero versus omitted/null defaults.

All 75 catalog packages were loaded/queried through rebuilt actual WASM; all
15 shell assets passed distribution integrity verification.
`cargo check -p icelines-web --lib --offline --locked` passed. Scoped whitespace
and Rust formatting checks passed. `cargo clippy -p icelines-wasm --lib --offline
--locked -- -D warnings` passed. Existing incremental hard-link fallback warnings
in test builds are non-failing.

Actual browser preview at `http://127.0.0.1:8050/` confirmed 20242025 regular
goalies: blank default discloses minimum 5 and returns 81 rows; explicit zero
discloses minimum 0 and returns 103 rows. Loading playoffs with blank default
discloses minimum 1 and returns 27 rows. No fixture was automatically saved.
[Browser evidence](browser-pulse-06.png). Server session 97595, browser tab 7
marked for continuation.

## Open gates

Existing native web skater count sorting uses points/name secondary keys while
CLI queries use player IDs. Browser count sorting currently follows CLI; this
pulse does not claim complete ordering parity with every native surface. That
cross-surface inconsistency needs reconciliation and acceptance evidence.
Deployed live data, Pages execution/publication and rollback, offline reopening
and upgrades, worker recovery, storage lifecycle, URL restoration and resource
budgets remain open. The full goal remains active; no shipped status is promoted.
