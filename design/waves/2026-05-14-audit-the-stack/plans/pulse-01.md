# Pulse 01 - Web silent fallback hardening

## Findings

- R1 F-02: `/transactions` collapsed transaction fallback load errors into an empty table.
- R3 F-01: Favorites/Watchlist HTML parsed malformed active seasons as `Season(0)`.
- R5 F-02: These paths needed targeted web regressions before behavior changes.

## Scope

Replace silent success-shaped fallback behavior with typed error responses. Keep out-of-coverage transaction seasons as the existing explanatory empty state.

## Gates

- `cargo fmt --check`
- `cargo test -p icelines-web l1_favorites_and_watchlist_bad_active_season_return_typed_errors -- --nocapture`
- `cargo test -p icelines-web l1_transactions_json_missing_source_returns_typed_error -- --nocapture`
- `cargo test -p icelines-web`
- `cargo clippy --workspace -- -D warnings`

## Closeout

Closed in this pulse:

- Favorites and Watchlist HTML now return typed `BAD_REQUEST` error envelopes for malformed active seasons instead of building `Season(0)` contexts.
- Transactions web/API result building now distinguishes bad requests from missing transaction sources; missing in-coverage data returns a typed unavailable error instead of an empty success table.
- Dashboard transaction workspace summaries preserve the typed error title/detail.
- L1 regressions pin the invalid-season and missing-source paths.
