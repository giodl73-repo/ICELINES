# Spark.1 - Projection Inventory and Assumptions Contract

## Purpose

Sim the Spark extends Rocket Richard from "what happened" and "what is trending"
to descriptive scoring outlooks. This inventory keeps the wave inside IceLines'
owned lane: rate normalization, source-state disclosure, and hockey context. It
does not authorize betting odds, win probability, proprietary expected-goals
equivalence, or live-fetching GET surfaces.

## Existing player pace inputs

| Input | Current owner | Contract | Spark use |
|---|---|---|---|
| `MIN_GP` | `icelines-core/src/model.rs` | Constant is `10`; derived per-game/per-82 rates are unavailable below it. | Reuse as the first sample-size floor for season-to-date player pace. |
| `compute_pace_score(goals, assists, gp)` | `icelines-core/src/scoring.rs` | Returns `None` when `gp < MIN_GP`; computes `pace_82 = (goals + assists) / gp * 82` and `goals_per_82 = goals / gp * 82`. | Reuse formula semantics for point and goal pace, but expose goal pace explicitly for Rocket. |
| `PaceScore` | `icelines-core/src/model.rs` / `season_stats.rs` | Carries `pace_82`, `goals_per_82`, `raw_points`, and `gp`; sort key uses goals/82 as a small tiebreaker. | Existing skater leaderboard source for season-to-date scoring pace. |
| `PlayerView` | `icelines-core/src/stats_repository.rs` | Exposes `gp`, goals, assists, points, shots, `pace_score`, `pace_82`, `goals_per_82`, `shots_per_82`, and catalog-routed per-game accessors. | First player outlook ViewModel should build from `PlayerView`, not from CLI formatting. |
| `StatId` catalog | `icelines-core/src/stats_catalog.rs` | Provides `pace-82`, `goals-per-82`, `assists-per-82`, `points-per-game`, `goals-per-game`, and related aliases with `MIN_GP` guards. | Reuse stat keys and aliases for output labels where possible. |

Existing surfaces already expose pieces of this contract:

- `icelines rank` sorts skaters with `sort_views_by_pace` and emits `pts_per_82`
  plus `goals_per_82`.
- TUI `Projections` renders `PlayerView::pace_82()` as "Pts/82" with no
  rest-of-season claim.
- `LeadersView::skater_pace` is a reusable ViewModel-backed league pace board.
- `icelines project` is CLI-only and returns rest-of-season projected points from
  `compute_projection`, but it is not a shared ViewModel and it estimates age
  from a hardcoded 2026 reference. Spark should not copy that command's output
  shape into web/API contracts without first moving the assumptions into core.

## Existing rest-of-season projection inputs

`icelines-core/src/projection.rs` contains a point-projection engine:

- `ProjectionMode::Pace`: current PPG times remaining games.
- `ProjectionMode::Regressed`: `alpha * current_ppg + (1 - alpha) * career_ppg`,
  where `alpha = min(gp / 50, 1.0)`.
- `ProjectionMode::Composite`: regressed PPG times an age factor; schedule factor
  is currently a placeholder.
- `per_game_sigma`: `0.65 / sqrt(gp)` for the confidence band, with `gp == 0`
  returning the input PPG as the sigma.

This code is useful prior art, but it is points-first and CLI-shaped. For Rocket
Richard, Pulse 02 should either extract a generic core pace row that can handle
goals, points, and shots, or define a separate goal-scoring pace contract. Do not
name a goal output "projected points", and do not imply the composite mode is a
calibrated scoring forecast.

## Rocket scoring trend inputs

`PlayerScoringProfileView` already carries `trends: Vec<PlayerScoringTrendRow>`.
Each row has:

- fixed windows: last 3 games, last 5 games, last 10 games, and season loaded;
- `games_loaded`, `events_loaded`, `source_loaded`, and `source_partial`;
- `summary` counts for goals, shots on goal, misses, blocks, attempts, and
  unblocked attempts;
- nullable `shot_pct`, where zero shots on goal means `None`;
- `bucket_counts` for the IceLines inside-shot proxy: crease, inside, slot,
  outside, and unknown.

Important limitation: trend windows are built from scoring-event games for the
player. A loaded game where the player had zero scoring events is not guaranteed
to appear in `games_loaded`. Therefore these rows are safe for "recent pressure"
and "conversion/inside-look context", but they are not by themselves a true
recent all-games pace denominator. If Pulse 02 needs recent goals-per-game, it
must join against cached boxscore game lines or another loaded zero-game source.

## Team outlook inputs

`TeamSeasonView` already computes useful descriptive season context:

- headline: record, points, points percentage, goals for, goals against, and goal
  differential;
- splits: home, away, and one-goal records with goals for/against;
- form: last 5 record, last 10 record, and last 10 goal differential;
- remaining schedule: remaining games, home/away counts, and next opponents;
- rows: per-game team score, opponent score, result, state, and playoff flag;
- source state: schedule is complete when the view is constructed; standings may
  be missing, in which case completeness becomes partial.

Current caution: `icelines team-season` and web `/team/:abbrev/season` currently
fetch schedule/standings from the NHL API in the command/handler path. Spark
GET surfaces must not reuse those live-fetching paths as-is. Team outlook work
should either read cached `DataKind::Schedule` entries through `DataStore` or
accept already-loaded `ScheduledGameInput` rows from an explicit fetch/Admin
mutation path.

`icelines-fetch/src/schedule_remaining.rs` can count remaining regular-season
games from cached schedule JSON. That is the safest first source for
rest-of-season denominators when schedule cache exists.

## Source-state contract

Spark ViewModels need to preserve these distinct states:

| State | Meaning | Required behavior |
|---|---|---|
| Season stats missing | No `StatsRepository` window for the selected season/type. | Return empty/missing-source state; do not fabricate zero pace. |
| Below sample floor | Stats exist but `gp < MIN_GP`. | Expose raw totals and a `below_threshold` status; keep per-82/rest-of-season rates nullable. |
| Season stats loaded, play-by-play missing | Season pace can be shown, but Rocket trend context cannot. | Mark trend/source context unavailable; do not show "zero inside looks". |
| Play-by-play loaded with zero events | Source is loaded, but no matching events occurred. | Show zero event counts with `source_loaded = true`; shot percentage remains `None` when SOG is zero. |
| Partial cache | Some windows are missing or source state is partial. | Surface partial completeness and avoid confident "hot/cooling" labels. |

## Allowed formulas and labels

First implementation should use side-by-side descriptive rows:

1. **Season-to-date 82-game pace**: `(stat / gp) * 82`, regular season only,
   nullable when `gp < MIN_GP`.
2. **Projected finish from loaded remaining games**: `current_total + (stat / gp)
   * remaining_games`, nullable when remaining-game cache is unavailable or
   `gp < MIN_GP`.
3. **Recent pressure context**: last 3/5/10 scoring-event summaries from
   `PlayerScoringTrendRow`, labeled as pressure/conversion context rather than
   all-games pace unless zero-event games are included.
4. **Range or band**: allowed only if its sigma and assumptions are stated in the
   ViewModel docs and tests. The existing point-projection sigma is prior art,
   not automatically calibrated for goals.

Allowed copy: "on pace", "tracking toward", "recent pressure", "loaded trend",
"below sample floor", "partial source", and "descriptive outlook".

Banned copy: "odds", "win probability", "betting edge", "expected-goals model",
"high-danger parity", "guaranteed finish", and any proprietary xG equivalence.

## Proposed implementation split

| Pulse | Scope | Notes |
|---|---|---|
| 02 - Player scoring pace ViewModel | Add core-owned player outlook rows for goals, points, and shots using `PlayerView`; include nullable projected-finish values when remaining games are provided. | L0 only first; no route math. |
| 03 - Team scoring outlook ViewModel | Add core-owned team GF/GA pace and recent-form outlook from already-loaded `TeamSeasonView`/schedule inputs. | Must not call NHL API from GET. |
| 04 - Surface parity and docs | Wire stable ViewModels to web/API and docs, then CLI/TUI only if the ViewModel shape is final. | Add L1 JSON/source-state tests. |
| 05 - Wave closeout | Close documentation, verify gates, and hand off any non-Rocket follow-ups. | Do not close with unchecked pulse gates. |

## Required tests before implementation ships

L0 core tests:

- `gp = 0` and `gp = 9` produce a below-threshold/null pace state.
- `gp = 10` is exactly the first valid threshold.
- Goal pace known value: 20 goals in 40 GP gives 41.0 goals/82.
- Projected finish known value: 20 goals in 40 GP with 42 remaining gives 41.0
  projected final goals.
- Zero shots on goal keeps conversion as `None`, not `0.0`.
- Tied pace rows use the documented tiebreaker, then deterministic player ID or
  name ordering.
- Trend source states distinguish missing play-by-play from loaded zero-event
  play-by-play.

L1 fetch/web tests:

- Cached schedule/play-by-play fixtures feed the ViewModel without network calls.
- Missing schedule cache makes projected finish nullable while keeping season
  pace available.
- Web JSON includes source-state/completeness and does not warm caches on GET.
- Team outlook JSON distinguishes no games loaded from loaded games with zero
  goals.

L2 CLI tests:

- Any new command or flag prints the same key fields as the ViewModel JSON.
- Below-threshold players produce an explicit sample-floor message, not a
  success-shaped zero projection.

## Role review findings

- **pace**: Use season-to-date 82-game pace as the first stable formula, with
  projected finish only when remaining games are loaded. Do not carry over the
  existing point-only `project` command as the Rocket goal contract without a
  core ViewModel and tests.
- **scout**: Keep labels descriptive and conditional. "Recent pressure" is safer
  than "hot" until zero-event game denominators are present.
- **wire**: Team outlook must avoid the current live-fetching team-season web/CLI
  paths for GET surfaces. Use cache/Admin/fetch boundaries.
- **bench**: The first code pulse needs known-value tests for threshold, formula,
  tie, zero-shot, and source-state cases before wiring any surface.
