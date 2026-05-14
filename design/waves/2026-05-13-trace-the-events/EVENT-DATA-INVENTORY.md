# Event Data Inventory

## Purpose

The previous records wave intentionally left `goalies-scored-against` and
`fight-opponents` planned because the persisted boxscore projection did not
identify the goalie in net for each goal or the opponent in each fight. This
inventory validates the NHL play-by-play endpoint as the next source.

## Source inspected

| Source | Shape | Notes |
|---|---|---|
| `GET https://api-web.nhle.com/v1/gamecenter/{game_id}/play-by-play` | game envelope with `plays[]` | Public NHL web endpoint adjacent to the existing boxscore endpoint. |
| `plays[].typeDescKey == "goal"` | event row with `details` | Carries scorer/team ids and, when applicable, goalie-in-net id. |
| `plays[].typeDescKey == "penalty"` | event row with `details` | Carries penalty type, description, committed-by player, drawn-by player, duration, and owner team when available. |

## Goal records

Validated sample: `2023020001`.

Goal rows include:

```text
typeDescKey = "goal"
details.scoringPlayerId = 8476453
details.eventOwnerTeamId = 14
details.goalieInNetId = 8477424
details.awayScore / details.homeScore
```

Empty-net/schema-gap rows can omit `goalieInNetId`. Sample `2023020001` event
`179` is a goal row with `scoringPlayerId` and `eventOwnerTeamId` but no
`goalieInNetId`. Records must treat the goalie id as optional and skip no-goalie
rows for "goalies beaten" counts rather than assigning them to the last goalie
in the boxscore.

Recommended first projection:

| Field | Required? | Consumer |
|---|---:|---|
| `game_id` | yes | stable record key and manifest lookup |
| `event_id` | yes | event dedup within game |
| `game_date` | yes | output date/context |
| `period`, `time_in_period` | yes | row context and fight pairing |
| `event_owner_team_id` / abbrev | yes | scoring team/opponent team |
| `scoring_player_id` | yes for goal records | scorer grouping |
| `goalie_in_net_id` | optional | goalie-beaten grouping; absent means no counted goalie |
| `situation_code` | optional | future strength/empty-net validation, not a substitute for goalie id |

## Fight records

Validated sample: `2023020005`.

Fighting majors appear as reciprocal penalty rows at the same game time:

```text
typeDescKey = "penalty"
details.typeCode = "MAJ"
details.descKey = "fighting"
details.duration = 5
details.committedByPlayerId = 8471817
details.drawnByPlayerId = 8482964
details.eventOwnerTeamId = 10

typeDescKey = "penalty"
details.typeCode = "MAJ"
details.descKey = "fighting"
details.duration = 5
details.committedByPlayerId = 8482964
details.drawnByPlayerId = 8471817
details.eventOwnerTeamId = 8
```

Not every penalty row has `drawnByPlayerId` (for example delay-of-game), so the
fight parser must only use rows where the source explicitly names both players.
Because each fight can appear as two reciprocal rows, record counts need a
deduped pair key, not a raw event count.

Recommended fight key:

```text
(game_id, period, time_in_period, min(player_a, player_b), max(player_a, player_b), descKey)
```

This counts one fight per explicit pair and prevents reciprocal rows from
double-counting. If the endpoint emits only one explicit fighting row for a pair,
the same key still represents one fight.

## Data-layer implications

1. Add a play-by-play manifest kind instead of overloading boxscore data.
2. Persist raw play-by-play JSON under the data directory before deriving
   records, matching the current "raw source first" boxscore pattern.
3. Keep the typed projection narrow: goals and penalties are enough for this
   wave; hits, shots, and faceoffs can remain unmodeled unless a later wave
   needs them.
4. Treat all event participant ids as optional at the parser boundary and let
   ViewModels decide whether an incomplete row is countable.
5. Do not promote `goalies-scored-against` or `fight-opponents` in
   `report list` until cached play-by-play data and tests exist.

## Pulse map

| Pulse | Adds | Gates |
|---|---|---|
| 02 | `PlayByPlay` parser, raw fetch/store path, manifest kind, focused fetch tests | `cargo test -p icelines-fetch play_by_play`; `cargo clippy -p icelines-fetch -- -D warnings` |
| 03 | goalie-beaten core ViewModel rows, records provider bridge, CLI/web metric | focused core/fetch/CLI/web tests; `cargo fmt --check` |
| 04 | fight-opponent core ViewModel rows, reciprocal-pair dedup, CLI/web metric | focused core/fetch/CLI/web tests; `cargo fmt --check` |
| 05 | docs/catalog/surface parity updates and route inventory refresh | `proof check`; route inventory tests |
