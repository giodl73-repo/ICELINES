# IceLines Projection Engine — Specification

**Version**: 0.1  
**Date**: 2026-04-25  
**Status**: Draft — pre-implementation

---

## 1. Overview

The IceLines projection engine produces rest-of-season point projections for NHL skaters.
Projections are available at three levels of sophistication — pace, regressed, and composite —
selected by the user via `--mode`. The default mode is `regressed`, which applies a regression
toward career mean and is appropriate for most mid-season use cases.

All projections produce a **projected full-season total** expressed as `pts` (goals + assists)
over `remaining_games`, not as pts/82. This is the number that matters for fantasy decisions.

---

## 2. Projection Modes

### 2.1 Pace (Simple)

The simplest mode. Projects the player's current scoring rate forward over the remaining games
with no adjustment.

```
projected_pts = (season_points / season_gp) × remaining_games
```

Where:
- `season_points` = goals + assists from the current season (NHL API skater summary)
- `season_gp` = games played this season (NHL API player bio)
- `remaining_games` = total regular-season games − games already played for the player's team

**When to use**: Early in the season when career data is sparse, or when the user explicitly
wants an unregressed pace estimate to set a ceiling.

**Limitations**: Regression to the mean is not applied. A player on a hot 20-game streak will
project identically to a player 70 games into the same pace. The `regressed` mode addresses this.

---

### 2.2 Regressed (Default)

Blends the current season pace with the player's career pace. As the sample of current-season
games grows, the projection tilts toward this season's pace and away from the career mean.

```
proj_ppg = α × current_ppg + (1 − α) × career_ppg

α = min(season_gp / 50, 1.0)

projected_pts = proj_ppg × remaining_games
```

Where:
- `current_ppg` = `season_points / season_gp` — this season's points per game
- `career_ppg` = career regular-season points per game (see §4.1 for derivation)
- `α` = credibility weight — increases linearly from 0 at GP=0 to 1.0 at GP=50, then stays at 1.0
- At 50+ GP, the projection is equivalent to pure pace (current season fully trusted)
- At GP=0, the projection is pure career mean (no current-season data)

**Interpretation of α:**

| Season GP | α     | Weight on current pace | Weight on career mean |
|-----------|-------|------------------------|-----------------------|
| 0         | 0.00  | 0%                     | 100%                  |
| 10        | 0.20  | 20%                    | 80%                   |
| 25        | 0.50  | 50%                    | 50%                   |
| 40        | 0.80  | 80%                    | 20%                   |
| 50+       | 1.00  | 100%                   | 0%                    |

**When to use**: The standard projection for any regular-season analysis from November onward.
Automatically correct for regression without requiring user judgment.

**New player handling**: If the player has no prior NHL `seasonTotals` (e.g., a rookie), the
career mean cannot be computed. In this case, regressed mode falls back to pace mode and
annotates the output with `"career data unavailable — using pace"`.

---

### 2.3 Composite (Advanced)

Extends the regressed projection with two additional factors: age curve and schedule difficulty.

```
composite_pts = regressed_pts × age_factor × schedule_factor
```

#### Age Factor

The age curve models a player's expected performance relative to a peak-age player,
based on historical NHL production curves:

| Age     | Age factor |
|---------|------------|
| ≤ 22    | 0.92       |
| 23      | 0.95       |
| 24      | 0.97       |
| 25      | 0.99       |
| 26–27   | 1.00       |
| 28      | 0.99       |
| 29      | 0.98       |
| 30      | 0.97       |
| 31      | 0.95       |
| 32      | 0.93       |
| 33      | 0.91       |
| 34      | 0.89       |
| ≥ 35    | 0.87       |

These values represent the expected production multiplier relative to a 26–27-year-old player
at the same observed pace. After age 30, the factor declines approximately 2% per year.
Before age 24, the factor reflects the typical undershoot of young players relative to their
eventual peak.

The age factor is applied multiplicatively to the regressed projection. It does not re-estimate
the player's ceiling; it adjusts the expected outcome given where they are in their career arc.

**Age used**: The player's age on the date the projection is computed (from `birthDate` in the
bios API). Computed as floor((today − birthDate) / 365.25).

#### Schedule Factor

The schedule factor adjusts for the difficulty of a team's remaining schedule based on
opponent quality.

```
schedule_factor = 1.0 + k × (avg_remaining_opponent_rank_deviation)
```

Where:
- `avg_remaining_opponent_rank_deviation` = mean of (opponent_rank − league_median_rank)
  across the player's team's remaining games. Positive = facing below-average opponents
  (favorable). Negative = facing above-average opponents (tough).
- Opponent rank is derived from each opposing team's current season goals-against-per-game,
  inverted so that weaker defensive teams have higher (worse) ranks.
- `k = 0.015` — sensitivity constant. A team facing 10 games of average opponents scores 1.0.
  A team facing 10 games entirely against bottom-five defensive teams scores ~1.05.

**Schedule data source**: Remaining games fetched from `api-web.nhle.com/v1/schedule/{DATE}`
for each date from today through the end of the regular season (April 17). Cached per team
at `~/.icelines/cache/schedule/{SEASON}/{TEAM}.json`.

**When to use**: Late in the season when the remaining schedule is meaningful (under 20 games
remaining). Less useful early in the season when the schedule still averages out.

---

## 3. Confidence Band

All three modes produce a confidence band displayed as `±1σ` alongside the projected total.

The band is computed from the standard deviation of the player's game-by-game point totals
this season (0, 1, or 2+ points per game). This measures consistency, not projection quality.

```
σ_ppg = std_dev(per_game_points_this_season)
σ_proj = σ_ppg × sqrt(remaining_games)
```

The `±1σ` band represents the range within which the projection falls approximately 68% of
the time if the player continues performing at their current per-game distribution.

A high σ (streaky player) produces a wide band. A consistent player produces a narrow band.

**Data required**: Per-game point totals are derived from the player's game log (game-by-game
stats available via the player landing API game log endpoint). If fewer than 10 games are
available, the confidence band is not computed and the output notes `"insufficient sample for
confidence band"`.

---

## 4. Data Required

### 4.1 Career Pace (`seasonTotals`)

Career PPG is derived from the player's `seasonTotals` in the player landing API:

```
GET https://api-web.nhle.com/v1/player/{PLAYER_ID}/landing
```

Response field: `seasonTotals` — an array of season-by-season stats. Each entry contains:
- `season` — YYYYZZZZ season identifier
- `gameTypeId` — 2 for regular season
- `points` — total points for the season
- `gamesPlayed` — GP for the season

Career PPG is computed as the weighted mean across all prior regular seasons (excluding the
current season), weighted by `gamesPlayed`:

```
career_ppg = sum(points_s) / sum(gamesPlayed_s)   for all prior seasons s
```

Playoff seasons (`gameTypeId = 3`) are excluded from career PPG computation.

Cache location: `~/.icelines/cache/landing/{PLAYER_ID}.json`. TTL: 7 days (career data
changes slowly; a weekly refresh is sufficient).

### 4.2 Remaining Games

Remaining games for a player is computed as:

```
remaining_games = team_remaining_games
```

Where `team_remaining_games` is the number of regular-season games remaining on the player's
team's schedule. This is fetched from:

```
GET https://api-web.nhle.com/v1/schedule/now
```

And iterated forward by date until the regular season end date. The regular season end date
for a given season is known at build time (hardcoded per season) or derived as the last date
in the schedule feed where `gameType = 2`.

If the user passes `--games <N>`, that value overrides the computed `team_remaining_games`.

### 4.3 Current Season Stats

Same as used everywhere else in IceLines: cached bios + summary from `icelines fetch stats`.

---

## 5. `icelines project <PLAYER>` Command

```
icelines project <PLAYER> [OPTIONS]

Arguments:
  <PLAYER>     Player name (partial match OK) or NHL player ID

Options:
  --mode <MODE>          pace | regressed | composite  [default: regressed]
  --games <N>            Remaining games to project [default: computed from schedule]
  --season               Show by-season career comparison table
  --json                 Output JSON instead of formatted table
```

**Example output (default):**

```
Leon Draisaitl — EDM — C/LW — Age 28
Season: 2025-26  |  GP: 52  |  Pts: 67  |  PPG: 1.288

Mode: Regressed (α = 1.00 — full-season sample)
Career PPG: 1.241 (8 seasons, 620 GP)

Remaining games: 30 (team schedule, EDM)
Projected points: 39  [range: 31–46 at ±1σ]
Full-season total: 106 pts projected

  current pace:   38.6 pts remaining
  career regressed: 39.0 pts remaining
  (α=1.00 → identical at this sample size)

Career comparison (--season flag):
  2018-19  50 G,  55 A, 105 Pts  82 GP  1.280 PPG
  2019-20  23 G,  34 A,  57 Pts  71 GP  0.803 PPG  (COVID)
  2020-21  24 G,  48 A,  72 Pts  56 GP  1.286 PPG  (short)
  2021-22  55 G,  60 A, 115 Pts  80 GP  1.438 PPG
  2022-23  52 G,  64 A, 128 Pts  82 GP  1.561 PPG
  2023-24  36 G,  54 A,  92 Pts  76 GP  1.211 PPG
  2024-25  35 G,  49 A,  84 Pts  80 GP  1.050 PPG
  2025-26  28 G,  39 A,  67 Pts  52 GP  1.288 PPG  ← current
```

**JSON output (`--json`):**

```json
{
  "player_id": 8477934,
  "player_name": "Leon Draisaitl",
  "team": "EDM",
  "primary_position": "C",
  "eligible_positions": ["C", "LW"],
  "age": 28,
  "season": "20252026",
  "gp": 52,
  "season_points": 67,
  "current_ppg": 1.288,
  "career_ppg": 1.241,
  "mode": "regressed",
  "alpha": 1.0,
  "remaining_games": 30,
  "projected_remaining_pts": 39,
  "projected_total_pts": 106,
  "sigma_low": 31,
  "sigma_high": 46
}
```

---

## 6. `icelines project --team <TEAM>` Command

Projects all skaters on a team's current active roster.

```
icelines project --team <TEAM> [OPTIONS]

Options:
  --team <ABBREV>        Team abbreviation (e.g. SEA, EDM, COL)
  --pos <POSITIONS>      Comma-separated filter: C,LW,RW,D  [default: all]
  --mode <MODE>          pace | regressed | composite  [default: regressed]
  --json                 Output JSON instead of formatted table
```

**Example output:**

```
Seattle Kraken — Projections (Regressed, 30 games remaining)
Sorted by projected remaining points descending

Rank  Player              Pos  GP  Pts  PPG   Career   α     Proj  Range
  1   Matty Beniers       C    54  54   1.000  0.812  1.00   30   [24–36]
  2   Jaden Schwartz      LW   49  38   0.776  0.651  0.98   23   [17–29]
  3   Brandon Montour     D    53  44   0.830  0.650  1.00   25   [20–30]
  ...

Team total projected: 241 pts remaining (±34 at ±1σ)
```

Position filtering: `--pos C,LW` shows only centers and left wings. Forwards with
multi-position eligibility appear once, under their primary position.

---

## 7. Crate Placement

The projection engine lives in `icelines-core`, as it is pure computation with no I/O:

```
icelines-core/src/
  projection/
    mod.rs          — ProjectionMode enum, ProjectionResult struct
    pace.rs         — pace_project()
    regressed.rs    — regressed_project(), compute_alpha()
    composite.rs    — composite_project(), age_factor(), schedule_factor()
    confidence.rs   — compute_sigma()
```

The CLI command handlers in `icelines-cli` call into these functions after loading data from
`icelines-fetch` (career seasonTotals, current stats, schedule).

```rust
pub struct ProjectionResult {
    pub player_id: u32,
    pub mode: ProjectionMode,
    pub alpha: f64,               // 0.0–1.0 (regressed/composite modes)
    pub current_ppg: f64,
    pub career_ppg: Option<f64>,  // None for rookies
    pub remaining_games: u32,
    pub projected_remaining: f64,
    pub sigma: Option<f64>,       // None if insufficient per-game data
    pub age_factor: Option<f64>,  // Some(_) only in composite mode
    pub schedule_factor: Option<f64>, // Some(_) only in composite mode
}

pub enum ProjectionMode {
    Pace,
    Regressed,
    Composite,
}
```

---

## 8. Non-Goals

The following are explicitly out of scope for the projection engine:

- **Injury probability.** The engine projects as if the player will play all remaining games.
  Injury risk, return timelines, and IR status are not modeled.
- **Trade likelihood.** A traded player's remaining games are recomputed only if the user
  re-runs `icelines project` after the trade is reflected in the NHL API roster data.
- **Playoff modeling.** Rest-of-season projections cover the regular season only. Playoff
  appearances add games not reflected in the schedule API's regular-season feed.
- **Streak detection.** Hot or cold streaks are not detected or weighted separately. The
  regressed mode's α factor is the only time-in-season weighting applied.
- **Lineup-adjusted projections.** A player's line assignment (centering top line vs. fourth
  line) is not used to adjust the projection. This is a named limitation, not an oversight.
- **Category-league scoring.** Projections are points only (G + A). PIM, PPP, hits,
  blocks, and other category-league stats are out of scope.
