# Pulse 04 - Fight-Opponent Records

## Goal

Promote fight-opponent records using explicit fighting-major participants from
cached play-by-play penalty rows.

## Implementation

1. Added `FightRecordInput`.
2. Added `PlayerRecordsView::fight_opponents`.
3. Added `TeamRecordsView::fight_opponents_by_team`.
4. Extended the records provider to parse `descKey = fighting` penalties with
   `committedByPlayerId` and `drawnByPlayerId`.
5. Deduped reciprocal fighting-major rows by
   `(game_id, period, time, min(player), max(player), fighting)`.
6. Added CLI metrics:
   - `records player <name> --metric fight-opponents`
   - `records team <ABBR> --metric fight-opponents-by-team`

## Gates

- `cargo test -p icelines-core fight_opponents`
- `cargo test -p icelines-fetch l1_fight_records_dedupe_reciprocal_penalties`
- `cargo test -p icelines-cli l2_records_player_fights_no_data_exits_zero_with_headers`
- `cargo check -p icelines-cli`

## Result

Fight records are available without using aggregate PIM totals. Every counted
fight comes from explicit event participants in cached play-by-play data.
