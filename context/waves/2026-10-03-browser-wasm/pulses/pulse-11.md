# Browser WASM pulse 11 — shared count ordering

Date: 2026-10-03. Parent: WP-BW-02 / REQ-PARITY-001 / REQ-BROWSER-001 / CHG-119.

## Change and compatibility

The four browser skater count sorts (points, goals, assists, GP) now delegate
to `StatId::sort_cmp` rather than maintaining a separate browser comparator.
The core catalog exposes `sort_values_cmp` for owned presentation projections;
repository-view sorting delegates to that same implementation. Its existing
rules are retained: metric direction, full precision, missing values last, and
ascending NHL player ID for metric ties.

Both native web leaderboard sorting paths (HTML and JSON) use that shared
comparison for these four count metrics. This intentionally changes web ties:
equal goals/assists/GP no longer prefer unrelated secondary points, and equal
points no longer prefer alphabetic names. The deterministic ID rule is already
the native CLI count-sort convention. Other native web metrics retain their
existing behavior. This is a specific count-order alignment, not an assertion
that every metric and native surface has identical ranking contracts.

The browser release verifier additionally exercises four hand-audited count
orders through the actual distributed WASM. Three players deliberately have
names and secondary points that conflict with the ID tie. This complements the
Rust fixture and full-season parity matrix rather than checking only counts or
reproducing the comparator in JavaScript.

## Verification

- `node scripts/build-browser.mjs --offline --check`: release WASM build/bindings,
  75 converted packages, shell verification, two portable-data tests, eight engine
  tests, 42 native-loader tests, two season parity tests (90 sort/floor/context
  combinations), seven catalog-conversion tests and 67 browser Node tests passed.
- Updated `node scripts/verify-browser-dist.mjs`: all four hand-audited count
  orders, 16 shell assets and all 75 packages passed using the rebuilt WASM.
- `cargo test -p icelines-core --lib stats_catalog::tests --offline --locked`:
  88 tests passed, including owned-value precision/ID/missing rules.
- `cargo test -p icelines-web --lib handlers::leaders::tests --offline --locked`:
  six tests passed, including tie ordering and template/JSON projection coverage.
- `cargo test -p icelines-web --lib l0_dashboard_game_summary_projects_detail_view
  --offline --locked`: passed. The first native web test build revealed two stale
  dashboard fixture initializers missing fields from existing game-view changes.
  Added those fixture fields; no game implementation behavior was changed.
- `cargo clippy -p icelines-wasm --lib --offline --locked -- -D warnings` passed.
  Formatting and whitespace checks passed for the touched Rust files.

The shared check script now includes core catalog and native web leaders tests.
Those newly added commands were run separately in this turn; the earlier running
build process had already loaded the prior script. No second complete rebuild
is claimed. Incremental hard-link fallback warnings did not fail compilation.

## Scope still open

The count-order discrepancy recorded in pulse 06 is reconciled. This evidence
does not establish all native leaderboard sort/rate contracts, native HTTP
end-to-end ordering, all query-provider requirements, deployed live acquisition,
scores/schedule, storage migrations, offline reopening/upgrades, accessibility,
resource budgets, or Pages publishing/rollback. Those plan gates remain open.
