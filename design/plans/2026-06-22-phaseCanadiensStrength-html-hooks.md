# Phase Canadiens Strength - HTML strength-state hooks

## Status

Closed - 2026-06-22

## Goal

Expose structured strength-state metadata in Web scoring HTML without changing
the visible table design. This gives future browser UI, tests, and scripts a
stable hook that does not require parsing display labels.

## Scope

- Add `data-situation-code`, `data-skater-state`, and
  `data-owner-strength-state` attributes to scoring situation summary rows.
- Keep the visible label unchanged.
- Assert the hooks through the cached game-scoring HTML route.
- Do not add new `StatId` keys, filters, or leaderboard semantics in this phase.

## Validation

```powershell
cargo fmt --check
cargo test -p icelines-web --test l1_router rocket_game_scoring
git diff --check
```
