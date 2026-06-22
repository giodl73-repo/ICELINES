# Phase Canadiens Strength - Owner-perspective labels

## Status

Closed - 2026-06-22

## Goal

Promote cached play-by-play scoring situation splits from count-only labels to
event-owner strength labels. Users should see even strength, power play, and
penalty kill context without losing the raw NHL `situationCode`.

## Scope

- Use `ScoringEventInput.event_owner_side` plus the four-character NHL
  `situationCode` to derive owner-perspective labels.
- Render equal skater counts as `even strength`, owner skater advantage as
  `power play`, and owner skater disadvantage as `penalty kill`.
- Preserve the away/home skater count and raw code in the label, for example
  `power play 5v4 (1541)`.
- Fall back to count-only labels when owner side is unavailable.
- Do not add new `StatId` keys, filters, or leaderboard semantics in this phase.

## Validation

```powershell
cargo fmt --check
cargo test -p icelines-core scoring::tests::l0_scoring
cargo test -p icelines-web --test l1_router rocket_game_scoring_json
git diff --check
```
