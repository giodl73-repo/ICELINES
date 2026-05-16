# Fantasy Daily Delta Inventory

Pulse 01 reviewed the current fantasy and game-data surfaces to choose a small,
offline-testable path for daily fantasy scoring.

## Existing Rails

| Rail | Current state | Reuse decision |
|---|---|---|
| Fantasy scoring scheme | `icelines-core/src/scheme.rs` owns `Scheme`, skater/goalie weights, season-score helpers, and category keys. | Reuse weights and add/derive daily-line adapters in core; do not duplicate math in surfaces. |
| Fantasy roster scoring | `FantasySimulationView` and `score_fantasy_roster` score season-to-date rosters from `PlayerView` pools. | Keep as season/projection scoring; daily delta gets a separate dated ViewModel. |
| Fantasy persistence | `FantasyDb` stores leagues, teams, active user team, and normalized rosters. | Reuse league/team/roster data; no new write contract until daily snapshots need persistence. |
| Game-night schemas | `SkaterNightLine` / `GoalieNightLine` gate physical stats until `GameState::{Final,Off}`. | Use the same finalization rule for daily fantasy rows. |
| Cached data | Foster/records/scoring surfaces already expect cached boxscore/play-by-play artifacts and expose missing-source warnings. | Daily delta reads cache and reports missing/unfinalized games; it must not fetch live data during scoring. |
| Existing fantasy surfaces | CLI `fantasy`, TUI fantasy/gaps/simulate, web `/fantasy`, and JSON gaps/simulate already expose read/product views. | Add daily read surfaces only after the shared ViewModel/data path is in place. |

## Gaps

| Gap | Impact | Pulse |
|---|---|---|
| No `FantasyDailyDeltaView` schema | Surfaces would otherwise invent output shapes independently. | 02 |
| No daily stat adapter from finalized game lines to fantasy scoring inputs | Existing `compute_fantasy_score` is season-oriented and includes a min-GP threshold. Daily scoring needs one-game stats without projection semantics. | 02 |
| No cache-backed builder tying FantasyDb rosters to dated game lines | Fantasy rosters can be listed, but not scored for a date. | 03 |
| No CLI/web/TUI route for daily results | Users cannot ask "what did my team score today?" | 04 |
| Docs do not describe data requirements | Users need to know boxscores must be cached/finalized. | 05 |

## Decisions

- "Daily delta" means fantasy points earned on a specific date by rostered
  players whose finalized game lines are cached.
- Missing cached games and unfinalized games are not zeros; they produce warnings
  and source-state/completeness markers.
- Team totals are sorted by daily points descending, then team name for stable
  ties. Player rows sort by daily points descending, then display name.
- The first implementation should be read-only. Persisting daily score history
  can be a follow-up if users need trend charts.

## Stop Conditions

- Stop if scoring would require live NHL calls without fixtures.
- Stop if the only available data path is season-to-date snapshot subtraction.
- Stop if CLI/web/TUI would need separate scoring implementations.
- Stop if unfinalized NHL boxscore defaults would be counted as real hits,
  blocks, takeaways, giveaways, saves, or goals against.
