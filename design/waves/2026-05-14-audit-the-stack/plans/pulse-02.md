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
