# Phase Canadiens Strength - Event HTML strength hooks

## Status

Closed - 2026-06-22

## Goal

Align Web scoring event-detail rows with scoring situation summary rows by
exposing the same strength-state metadata on each individual cached play-by-play
event.

## Scope

- Add shared `ScoringEventInput` accessors for `skater_state` and
  `owner_strength_state`.
- Add `data-situation-code`, `data-skater-state`, and
  `data-owner-strength-state` attributes to Web scoring event-detail rows.
- Extend focused route coverage to prove both summary and event rows carry the
  metadata.
- Do not add new `StatId` keys, filters, or leaderboard semantics in this phase.

## Validation

```powershell
cargo fmt --check
cargo test -p icelines-core scoring::tests::l0_scoring
cargo test -p icelines-web --test l1_router rocket_game_scoring
git diff --check
```
