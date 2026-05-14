# Pulse 03 - Goalies-Scored-Against Records

## Goal

Promote goalie-backed records from planned to available by using cached
play-by-play goal rows with explicit `goalieInNetId`.

## Implementation

1. Added `PlayerRecordsView::goalies_scored_against`.
2. Added `TeamRecordsView::goalies_beaten_by_team`.
3. Extended the records provider to project play-by-play goals into
   `PlayerGoalRecordInput` with goalie ids/names when available.
4. Added CLI metrics:
   - `records player <name> --metric goalies-scored-against`
   - `records team <ABBR> --metric goalies-beaten-by-team`
5. Preserved the no-inference rule: empty-net/no-goalie rows are excluded.

## Gates

- `cargo test -p icelines-core goalies_scored_against`
- `cargo test -p icelines-cli l2_records_player_goalies_no_data_exits_zero_with_headers`
- `cargo check -p icelines-cli`

## Result

Goalie records are available from the shared records ViewModels and CLI. They
depend on `icelines fetch play-by-play --date YYYY-MM-DD` for local source data.
