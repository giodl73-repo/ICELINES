# Phase Canadiens Strength - Event owner side

## Status

Closed - 2026-06-22

## Goal

Carry away/home event-owner context from official NHL play-by-play into the
shared scoring event model. This is the missing prerequisite for later
team-perspective strength labels such as power play, penalty kill, and even
strength.

## Scope

- Add `event_owner_side` to `ScoringEventInput` as an additive optional field.
- Populate it in the NHL play-by-play parser from `eventOwnerTeamId` plus the
  raw away/home team ids.
- Preserve existing owner id and owner abbreviation behavior.
- Assert the field in parser and Web JSON route coverage.
- Do not yet promote new `StatId` metrics or PP/PK leaderboards.

## Validation

```powershell
cargo fmt --check
cargo test -p icelines-fetch l0_parse_play_by_play_reads_shot_attempt_families
cargo test -p icelines-web --test l1_router rocket_game_scoring_json
git diff --check
```
