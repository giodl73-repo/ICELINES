# Fantasy Matchup Inventory

Pulse 01 reviewed the fantasy league, daily-delta, date/timeframe, and surface
rails to turn the Tier 3 weekly head-to-head backlog item into executable
pulses.

## Existing Rails

| Rail | Current state | Reuse decision |
|---|---|---|
| Fantasy leagues and rosters | `FantasyDb` stores leagues, teams, active user team, and normalized rosters in `fl_leagues`, `fl_teams`, and `fl_roster`. | Add matchup schedule rows beside the existing fantasy tables; preserve league/team ownership. |
| Fantasy scoring | `Scheme` plus Score the Day daily skater/goalie adapters compute one-game fantasy points from finalized game lines. | Weekly matchups aggregate daily points; no new category formula. |
| Daily scoring contract | `FantasyDailyDeltaView` exposes dated team/player rows, source state, and warnings from cached finalized boxscores. | Reuse daily rows as the scoring input for each date in a week. |
| Date windows | `Timeframe::Week` already resolves a date to ISO Monday-Sunday. | Use the same helper for weekly matchup boundaries. |
| Cached game data | `DataStore` and the boxscore manifest carry cached NHL boxscores; missing cache is already reported by daily delta. | Weekly builder walks the week and merges daily source state/warnings; it must not fetch live data. |
| Existing surfaces | CLI/web/TUI/dashboard already have fantasy read/product surfaces and command handoffs. | Add thin matchup read/setup surfaces only after the shared ViewModel and data path exist. |

## Gaps

| Gap | Impact | Pulse |
|---|---|---|
| No weekly matchup ViewModel | Surfaces would invent incompatible output shapes for matchup rows, byes, standings hints, and source state. | 02 |
| No matchup schedule persistence | FantasyDb can list teams but cannot say who plays whom in a week. | 03 |
| No weekly builder | Daily results exist, but no shared path sums dates into matchups and handles missing daily source state consistently. | 03 |
| No matchup CLI/web/TUI affordance | Users cannot configure or inspect weekly head-to-head matchups. | 04 |
| Docs/backlog still list the feature as future work | Users and future agents need setup commands, data requirements, and surface parity truth. | 05 |

## Decisions

- "Weekly matchup" means a local head-to-head pairing for a week, scored from
  cached finalized daily fantasy points for that week.
- Week selection accepts any date in the week; the ViewModel reports the resolved
  Monday `week_start` and Sunday `week_end`.
- Matchup rows compare two teams by weekly points. Ties are explicit; a bye is a
  supported schedule shape for odd-sized leagues and is not a win by default.
- Missing schedule rows produce an empty/setup state, not synthetic pairings.
- Missing cached boxscores and unfinalized daily game lines propagate warnings
  and partial completeness; they are not counted as hidden zeros.
- The first implementation persists schedule rows locally and does not import
  Yahoo/private schedules.

## Proposed Data Shape

The schedule pulse should add a FantasyDb-owned table similar to:

| Column | Purpose |
|---|---|
| `id` | Stable local UUID for the matchup row. |
| `league_id` | Owning fantasy league. |
| `week_start` | ISO `YYYY-MM-DD` Monday for the matchup week. |
| `home_team_id` | Required team in the pairing. |
| `away_team_id` | Optional opponent; `NULL` means bye. |
| `created_at` | Local creation timestamp. |

The table should enforce one matchup slot per team per week and reject duplicate
home/away pairings for the same league/week.

## Stop Conditions

- Stop if weekly scoring requires a ViewModel field that daily delta cannot
  provide without re-scoring in a renderer.
- Stop if a route would mutate the matchup schedule through GET.
- Stop if tests require live NHL or Yahoo data without fixtures.
- Stop if missing cache, missing schedule, or unfinalized games would be
  represented as successful zero-point results.
