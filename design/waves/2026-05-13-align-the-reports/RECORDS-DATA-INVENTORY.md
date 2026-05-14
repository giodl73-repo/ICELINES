# Records Data Inventory

## Decision

Symmetric player/team records are a first-class future `records` family, not a
`query --filter` extension and not a TUI-only feature. The implementation should
land in ViewModel/library code first, then expose CLI, TUI, and web from the same
record rows.

## Existing local sources

| Source | Location | What it has | Records fit |
|---|---|---|---|
| Season stats bundle | `data/seasons/*`, loaded through `stats_loader::load_into_repo` | Player/team season aggregates by NHL id. | Not enough for opponent-specific records. |
| Boxscore manifest | `~/.icelines/data/manifest/boxscores.json`, `DataKind::Boxscore` | Per-game raw NHL boxscore JSON after `icelines fetch boxscore`. | Primary local source for game/opponent records once populated. |
| Parsed boxscore | `icelines-fetch/src/nhl_api.rs::parse_boxscore` | Game id, home/away teams, score, goal rows by scorer name/team, goalie lines by id/name/team, skater lines by id/team/stats. | Enough for some game/opponent facts, incomplete for scorer-id and goalie-on-ice facts. |
| Query provider | `icelines-fetch/src/query_provider.rs` | Walks persisted boxscores and builds per-player game stat lines for sliding-window filters. | Proves the manifest-walk pattern for records. |
| EventStream score events | `icelines-core/src/event_stream.rs::ScorePayloadV1` | Final/period score snapshots plus favorited player entity refs. | Not enough for records; it intentionally stores summary events. |
| Shift/linemate boxscores | `icelines-fetch/src/shift_profile.rs` | Per-game player ids, teams, positions, EV TOI, shifts. | Useful pattern for player-game coappearance, not goal/fight records. |

## Metric feasibility

| Metric | Status | Reason |
|---|---|---|
| `teams-scored-against` for a player | Implementable after parser extension | Persisted raw boxscores have game teams and goal summary blocks. Current `Goal` exposes scorer name/team but not scorer id; add scorer id parsing before matching historical names. |
| `players-scored-against-team` for a team | Implementable after parser extension | Same goal rows can group by opposing team and scorer once scorer id is captured. |
| `goalies-scored-against` for a player | Needs play-by-play or richer goal detail | Current parsed boxscore lists game goalies but does not identify which goalie was in net for each goal. Inferring from final goalie lines is wrong for pulled goalies, goalie changes, empty-net goals, and shootouts. |
| `goalies-beaten-by-team` for a team | Needs play-by-play or richer goal detail | Same goalie-on-ice issue as player goalies scored against. |
| `fight-opponents` for a player | Needs penalty/play-by-play event participants | Current parser does not expose penalties, fighting majors, coincidental minors, or opponent participant ids. PIM totals on `SkaterLine` are aggregates and cannot identify opponents. |
| `team-fight-opponents` | Needs penalty/play-by-play event participants | Same limitation; must use event participant rows, not aggregate PIM. |
| generic head-to-head counts | Depends on metric | Game appearance head-to-head can use boxscore player ids. Event-specific head-to-head needs play-by-play event participants. |

## Required ViewModel inputs

`PlayerRecordsView` should not read files itself. It should consume normalized
records inputs:

```text
PlayerRecordGameInput {
  game_id,
  date,
  season,
  away_team,
  home_team,
}

PlayerGoalRecordInput {
  game_id,
  scorer_id,
  scorer_name,
  scorer_team,
  opponent_team,
  period,
  time_in_period,
  goalie_id: Option<u32>,
  goalie_name: Option<String>,
  empty_net: bool,
}

PlayerPenaltyRecordInput {
  game_id,
  player_id,
  player_name,
  player_team,
  opponent_id: Option<u32>,
  opponent_name: Option<String>,
  opponent_team: Option<String>,
  penalty_type,
  minutes,
  period,
  time_in_period,
}
```

`TeamRecordsView` can reuse the goal/penalty inputs and group by team/opponent.
Both ViewModels should emit stable rows with ids, display labels, counts,
first/last game ids, and source completeness warnings.

## Parser/store work before CLI records

1. Extend the persisted-boxscore parser to capture goal scorer ids when present
   in raw goal blocks. Preserve name fallback for older shapes, but mark rows
   without scorer ids as incomplete.
2. Add a play-by-play/event parser for goal goalie-on-ice, empty-net, and
   penalty/fighting participants. If the public NHL endpoint cannot provide
   historical coverage, surface that as a missing source warning instead of
   guessing.
3. Store parsed record inputs or raw event JSON under a manifest kind that can be
   walked like `DataKind::Boxscore`.
4. Build `PlayerRecordsView` / `TeamRecordsView` in library code.
5. Promote `records` in `icelines report list` from `planned` to `available`
   only after at least `teams-scored-against` has L0 ViewModel tests and L2 CLI
   tests.

## Implementation order

1. `teams-scored-against` and `players-scored-against-team`: easiest useful
   first slice once scorer ids are captured.
2. game-appearance head-to-head counts: feasible from skater/goalie player ids
   in boxscores, but should be labeled as appearances, not event outcomes.
3. `goalies-scored-against`: only after goalie-on-ice source validation.
4. `fight-opponents`: only after penalty/play-by-play participant source
   validation.

## Andre Burakovsky acceptance target

Use "Andre Burakovsky has scored against 33 NHL teams" as a user-facing
acceptance scenario for the future records CLI/screen, but do not hardcode the
number. The records implementation should compute the count from goal events and
display the team list plus first/last goal evidence for each opponent.
