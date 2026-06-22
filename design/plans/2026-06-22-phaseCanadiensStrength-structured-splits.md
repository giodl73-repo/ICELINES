# Phase Canadiens Strength - Structured scoring splits

## Status

Closed - 2026-06-22

## Goal

Make cached play-by-play scoring situation splits machine-readable, not only
display-readable. Major stats consumers need stable fields for raw situation
code, skater counts, and owner-perspective strength state before broader
leaderboards or filters are promoted.

## Scope

- Add optional `situation_code`, `skater_state`, and `owner_strength_state`
  fields to `ScoringSplitSummary`.
- Populate those fields for situation splits while leaving team and period split
  metadata unset.
- Keep the existing `label` field for HTML/templates and backward-compatible
  display use.
- Assert the structured fields through the Web scoring JSON route.
- Do not add new `StatId` keys, filters, or leaderboard semantics in this phase.

## Validation

```powershell
cargo fmt --check
cargo test -p icelines-core scoring::tests::l0_scoring
cargo test -p icelines-web --test l1_router rocket_game_scoring_json
git diff --check
```
