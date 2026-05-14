# Pulse 02 - Identity resolution hardening

## Findings

- R3 F-02: Web Favorites/Watchlist links can use the first substring player match.
- R4 F-02: Multiple user-entered player paths rely on single-pid name resolution.

## Scope

Prefer canonical `player:<pid>` entity refs for persisted groups and require exact normalized-name matches before rendering confident `/player/:id` links for legacy string keys. Interactive paths that can be ambiguous should use `find_player_candidates`.

## Gates

- Add focused L0/L1 tests for ambiguous names and exact canonical ids.
- `cargo fmt --check`
- `cargo test -p icelines-web`
- `cargo test -p icelines-cli`
- `cargo clippy --workspace -- -D warnings`

## Closeout

Closed in this pulse:

- Added `find_player_candidate_by_id` for canonical `player:<pid>` group refs.
- Web Favorites/Watchlist now link canonical numeric player refs directly by id.
- Legacy name keys now render confident links only on exact normalized single-player matches; ambiguous substring inputs render unresolved text.
- Added L0 bundled identity regressions and an L1 Favorites route regression for canonical id plus ambiguous surname behavior.
