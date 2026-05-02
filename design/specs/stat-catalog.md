# IceLines Stat Catalog — Specification

**Version**: 0.2 — Phase Lindsay (post-review)
**Date**: 2026-05-02
**Status**: Design — paired with `design/plans/2026-05-02-phaseLindsay-stat-catalog.md` v0.2.
8-role review applied (HART, KEEL, TAPE, FORGE, EDGE, BENCH, WIRE, GLASS).
Review summary at `design/plans/2026-05-02-phaseLindsay-review-summary.md`.
**Predecessor**: `design/specs/query-engine.md` (Tier 1/2/3 metric list — superseded by this catalog)

---

## Mission

A single registry for "what stats exist in this app." Every surface that
selects, sorts, filters, or displays a stat dispatches through this
catalog. Adding a new stat is one enum case + one match arm — never
N×7 copy-paste across the query screen, leaders CLI, depth chart, comps,
fantasy scheme, export, and player card.

The catalog is **not** the data model. The data model is `SeasonStats`
(typed fields). The catalog is the **read-side dispatch table** that
maps stat-as-name to value-on-view.

---

## Design constraints

1. **Single source of truth** — no surface looks at `view.goals` directly. Every read is `StatId::Goals.read(view)` or via category iteration.
2. **Position-aware** — `FaceoffWinPct` only applies to centers; `GoalieGames` only to goalies. The catalog answers `applies_to(position)`.
3. **Era-aware** — pre-2005 NHL data nulls hits/blocks. Every accessor returns `Option<f64>`. UI renders `None` as `—`, never as `0`.
4. **Stable iteration order** — `StatId::all()` and `StatCategory::members(c)` produce a deterministic order across runs. UI dropdown lists never reshuffle.
5. **No panics** — `read()` is total. A missing field returns `None`. Out-of-bounds `applies_to` is `false`. The catalog never trips an invariant from a malformed view.
6. **Composable** — every typed `SortMetric` value, every typed `PlayerFilter` field, becomes a `StatId` reference. Existing CLI flags map via alias.

---

## Surface area

The catalog lives in a new module `icelines-core::stats_catalog`.

### Public types

```rust
// 98 cases v0.2 — exhaustive, NOT #[non_exhaustive]. Compiler enforces
// "added a stat → updated everywhere" across all consumer surfaces.
pub enum StatId { /* 98 cases — see "Stat enumeration" below */ }
pub enum StatCategory { Identity, Scoring, SpecialTeams, TwoWay, TimeOnIce, OnIceGoals, Possession, Goalie, Derived }
pub enum StatUnit { Count, Pct, Per60, Seconds, Rate, Inverted /* lower-is-better */ }

pub struct StatFilter {
    pub stat: StatId,
    pub op: FilterOp,
    pub value: f64,
}
pub enum FilterOp { Min, Max, Equals }
```

### Public methods on `StatId`

| Method | Returns | Used by |
|---|---|---|
| `category(self) -> StatCategory` | the enum the stat belongs to | TUI section grouping; `members(category)` |
| `label(self) -> &'static str` | full human-readable: `"Hits per 60"` | dropdowns, table headers |
| `short_label(self) -> &'static str` | terse: `"Hits/60"`, `"H/60"` | column headers, CLI flag values |
| `cli_key(self) -> &'static str` | hyphen-case: `"hits-per-60"` | `--filter` / `--sort` parsing |
| `read(self, view: &PlayerView<'_>) -> Option<f64>` | the value, `None` if missing or N/A | every consumer |
| `unit(self) -> StatUnit` | how to format — count/pct/per60/seconds | display formatting |
| `higher_is_better(self) -> bool` | `true` for goals, `false` for GAA | percentile rendering, sort direction defaults |
| `applies_to(self, pos: Position) -> bool` | position-applicability | filter pre-validation, dropdown filtering |
| `default_in_career_table(self) -> bool` | `true` for the curated default columns | career table column selector default |
| `aliases(self) -> &'static [&'static str]` | legacy CLI flag strings (`"ppg"`, `"pts-pace"`) for back-compat | back-compat alias map |

### Static methods on `StatId`

```rust
pub fn all() -> &'static [StatId];                           // every variant, in stable order
pub fn members(c: StatCategory) -> impl Iterator<Item=Self>; // category subset
pub fn from_cli_key(s: &str) -> Option<StatId>;              // parse "hits-per-60" or alias
pub fn applicable_to(pos: Position) -> impl Iterator<Item=Self>; // for "Stats for centers" dropdown
```

---

## Stat enumeration (v0.1 — 120 cases planned, listed by category)

### `Identity` — 0 stats (kept as a category for the bios block; never selectable for sort/filter)

The `bios` data drives identity fields (full name, team, position, age,
draft, height, weight, nationality). These are not stats — they're
filter axes already typed on `PlayerFilter`. The catalog includes the
`Identity` category for completeness so TUI can render identity
metadata under the same panel structure.

### `Scoring` — 13 stats (from `summary`)

`Goals`, `Assists`, `Points`, `EvGoals`, `EvPoints`,
`PpGoals`, `PpPoints`, `ShGoals`, `ShPoints`,
`Gwg`, `OtGoals`, `Shots`, `ShootingPct`

### `SpecialTeams` — 14 stats (from `summary` + `powerplay` + `penaltykill` + `faceoffwins`)

| StatId | Source | Notes |
|---|---|---|
| `PpToiPerGame` | summary/powerplay | seconds |
| `ShToiPerGame` | summary/penaltykill | seconds |
| `PpGoalsPer60` | powerplay | rate |
| `PpPointsPer60` | powerplay | rate |
| `PpAssistsPer60` | powerplay | rate |
| `PpShootingPct` | powerplay | pct |
| `ShGoalsPer60` | penaltykill | rate |
| `ShPointsPer60` | penaltykill | rate |
| `PpGoalsAgainstPer60` | penaltykill | rate (PK metric) |
| `FaceoffWinPct` | summary | pct, centers only |
| `FaceoffWins` | faceoffwins | count |
| `FaceoffLosses` | faceoffwins | count |
| `OffensiveZoneFaceoffPct` | faceoffpercentages | pct |
| `DefensiveZoneFaceoffPct` | faceoffpercentages | pct |

### `TwoWay` — 16 stats (from `summary` + `realtime` + `penalties`)

`PlusMinus`, `Pim`,
`Hits`, `BlockedShots`, `Takeaways`, `Giveaways`, `MissedShots`,
`HitsPer60`, `BlockedShotsPer60`, `TakeawaysPer60`, `GiveawaysPer60`,
`PenaltiesDrawn`, `PenaltiesDrawnPer60`, `PenaltiesTakenPer60`,
`NetPenalties`, `NetPenaltiesPer60`

### `TimeOnIce` — 11 stats (from `timeonice`)

`TotalToi`, `TotalToiPerGame`,
`EvToi`, `EvToiPerGame`,
`PpToi`, `PpToiPerGame`,
`ShToi`, `ShToiPerGame`,
`Shifts`, `ShiftsPerGame`, `ToiPerShift`

### `OnIceGoals` — 9 stats (from `goalsForAgainst`)

`EvGoalsFor`, `EvGoalsAgainst`, `EvGoalsForPct`,
`PpGoalsFor`, `PpGoalsAgainst`,
`ShGoalsFor`, `ShGoalsAgainst`,
`EvenStrengthGoalDifference`, `EvenStrengthTimeOnIcePerGame`

### `Possession` — 10 stats (Tier 2 — `puckPossessions` + `scoringRates` + `summaryshooting`)

`SatPct` (Corsi%), `UsatPct` (Fenwick%),
`OffensiveZoneStartPct`, `DefensiveZoneStartPct`, `NeutralZoneStartPct`,
`OnIceShootingPct`,
`Goals5v5`, `Assists5v5`, `Points5v5`, `PointsPer60_5v5`

### `Goalie` — 18 stats (from goalie reports)

`GoalieGames`, `GoalieStarts`,
`Wins`, `Losses`, `OtLosses`, `Ties`,
`Saves`, `ShotsAgainst`, `GoalsAgainst`,
`SavePct`, `Gaa`, `Shutouts`,
`EvSavePct`, `PpSavePct`, `ShSavePct`,
`QualityStarts`, `QualityStartPct`, `RegulationWins`, `RegulationLosses`

### `Derived` — 7 stats (computed from Scoring + GP)

`Pace82`, `GoalsPer82`, `AssistsPer82`,
`PointsPerGame`, `GoalsPerGame`, `AssistsPerGame`,
`PaceSortKey` (the Hart pace + goals tiebreak)

**Total v0.1: 13 + 14 + 16 + 11 + 9 + 10 + 18 + 7 = 98 selectable stats.**

The number is approximate; the actual catalog will be settled when L.2
implementation lands. New stats added later add a `StatId` variant and a
`read` arm — the rest of the app inherits them automatically.

---

## Read dispatch — the contract

```rust
impl StatId {
    pub fn read(self, view: &PlayerView<'_>) -> Option<f64> {
        match self {
            // Identity — always None (not a stat); category-iterable but
            // not selectable.
            // Scoring (always present on a skater view; None for goalie).
            StatId::Goals       => Some(view.stats.totals.goals as f64),
            StatId::Assists     => Some(view.stats.totals.assists as f64),
            StatId::Points      => Some(view.stats.totals.points as f64),
            StatId::PpGoals     => Some(view.stats.totals.pp_goals as f64),
            // Realtime — None when the season's realtime data is missing
            // (pre-2005 league era OR snapshot/bundle gap).
            StatId::Hits        => view.stats.realtime.as_ref()
                                      .and_then(|r| r.hits.map(f64::from)),
            // Goalie-only — None for skater views.
            StatId::SavePct     => view.stats.goalie.as_ref()
                                      .and_then(|g| g.save_pct.map(f64::from)),
            // Derived — None when the underlying data isn't sufficient.
            StatId::Pace82      => view.pace_82(),
            StatId::PointsPerGame => {
                let gp = view.gp();
                if gp == 0 { None } else { Some(view.points() as f64 / gp as f64) }
            }
            // … one arm per StatId, ~98 total
        }
    }
}
```

`read()` is the **only** function that knows where the value lives. Everything else (sort, filter, display, export) calls into it. A change to where `Hits` is stored only touches one match arm.

---

## Position applicability

```rust
impl StatId {
    pub fn applies_to(self, pos: Position) -> bool {
        match self.category() {
            StatCategory::Goalie     => pos == Position::Goalie,
            StatCategory::Identity   => true,
            // Scoring/TwoWay/TimeOnIce/OnIceGoals/Possession/Derived: skaters
            _ if pos == Position::Goalie => false,
            // Faceoffs only meaningful for centers — but this is a per-stat
            // fact, not category-wide. Override at the leaf.
            _ => match self {
                StatId::FaceoffWinPct
                | StatId::FaceoffWins
                | StatId::FaceoffLosses
                | StatId::OffensiveZoneFaceoffPct
                | StatId::DefensiveZoneFaceoffPct => pos == Position::Center,
                _ => true,
            }
        }
    }
}
```

The TUI Queries screen uses this to grey out non-applicable stats when
filtering by a position-restricted pool. The CLI rejects a `--sort` on a
non-applicable stat (e.g., `--sort save-pct --pos C`).

---

## Filter semantics

```rust
pub struct StatFilter {
    pub stat: StatId,
    pub op: FilterOp,    // Min | Max | Equals
    pub value: f64,
}

impl PlayerFilter {
    pub fn matches_stat_filters(&self, view: &PlayerView<'_>) -> bool {
        for f in &self.stat_filters {
            // Skip non-applicable filters silently — DI-08.
            if !f.stat.applies_to(view.position()) {
                continue;
            }
            let actual = match f.stat.read(view) {
                Some(v) => v,
                None    => return false,  // missing data ≠ matches
            };
            let ok = match f.op {
                FilterOp::Min    => actual >= f.value,
                FilterOp::Max    => actual <= f.value,
                FilterOp::Equals => (actual - f.value).abs() < f64::EPSILON,
            };
            if !ok {
                return false;
            }
        }
        true
    }
}
```

The "missing-data treats as `false`" semantic is intentional: filtering
by `hits-min 100` should not include a player whose hit count is unknown
(pre-2005 era). Use a separate query without the filter to see them.

---

## CLI grammar

```bnf
filter      := <stat-key> <op> <number>
stat-key    := letter (letter | digit | "-")*    -- e.g. "hits-per-60"
op          := ">=" | "<=" | "==" | "="
number      := <decimal>
```

Whitespace allowed around `<op>`. Multiple `--filter` flags accumulate
(implicit AND). `--sort <stat-key>` accepts the same key set.

Existing typed flags (`--ppg-min`, `--gp-min`, `--toi-min`, `--plus-minus-min`,
`--shots-pg-min`) keep working. Internally they become typed slots on
`PlayerFilter`; the new `--filter` just appends to `stat_filters`.

Backward-compatibility aliases (existing strings → canonical StatId
short-key):

| Legacy flag | StatId |
|---|---|
| `pts-pace` | `Pace82` |
| `g-pace` | `GoalsPer82` |
| `pp-g-pace` | `PpGoalsPer60` |
| `sh-g-pace` | `ShGoalsPer60` |
| `gwg-pace` | not a Lindsay stat (no `gwg-per-60` published) — keep typed handler |
| `xg`, `cf-pct`, `xgf-pct` | `Possession` category — keep MoneyPuck path |
| `improvement` | not a stat — keep as a special sort mode |

---

## TUI integration

### Queries screen — categorized layout

```
┌─ Stats Query ──────────────────────────────────────────┐
│ Pos: [C/LW/RW/D/G ▼]   Team: [any ▼]   Age: [- to -]   │
│                                                         │
│ ▼ Scoring                G≥30     A≥40    P≥80          │
│ ▼ Two-way                Hits≥150  Blk≥75               │
│ ▶ Special Teams                                         │
│ ▶ Time on Ice                                           │
│ ▶ On-ice Goals                                          │
│ ▶ Possession                                            │
│                                                         │
│ Sort: [Pts/82 ▼]   Top: [20]                            │
└─────────────────────────────────────────────────────────┘
```

- `▶` collapsed, `▼` expanded; `<space>` toggles.
- Inside an expanded section: each stat is one row with `+` to add a filter, `Min`/`Max` toggle, value editor.
- Sort dropdown lists every `StatId` grouped by category.
- Position-restricted stats greyed out when the position filter excludes their applicability (e.g., `FaceoffWinPct` greys out for `Pos: D`).

### Player card career table

```
═════════════════════════════════════════════════════════════════
Connor McDavid · EDM · C · Age 28 · #97 · L
═════════════════════════════════════════════════════════════════
                                                  ← / → cycle column set
Season   GP  G  A  P  +/- PIM  PPG PPP  Shots  S%   TOI  Hits  Blk
2025-26  82  44 ..
2024-25  76  35 65 100 +22 8    10  35   280   12.5 20:32 22   55
2023-24  82  32 62  94 +24 12   13  35   294   10.9 22:12 18   45
2022-23  82  64 89 153 +22 36   21  56   352   18.2 22:39 15   38
2021-22  80  44 79 123 +28 28   13  44   314   14.0 22:27 17   42
```

- Default columns: a curated subset (`StatId::default_in_career_table` returns true).
- Column selector via `<` / `>`: rotates through preset templates (Scoring | Two-way | Special Teams | Time | All).
- User can save a custom column set via `Shift+S`; persisted in `~/.icelines/config.toml`.

---

## Site integration

The mkdocs site renders team pages using a curated subset of StatIds
(driven by `site_columns` config). Catalog entry-point per league
ensures cross-page consistency: every page that names a stat reads from
`StatId::label()`.

---

## Fantasy scheme integration

Existing fantasy schemes (TOML) use string keys for stat coefficients:

```toml
goals = 6
assists = 4
hits = 0.5
blocked_shots = 0.5
```

Migrated to StatId keys (canonical short-keys):

```toml
[scoring]
"goals" = 6
"assists" = 4
"hits" = 0.5
"blocked-shots" = 0.5
"pp-points-per-60" = 2.0      # NEW — was previously not expressible
"penalty-minutes-per-60" = -0.5
```

The scheme parser uses `StatId::from_cli_key` to convert string keys.
Unknown keys log a warning, default coefficient = 0. Schemes from
pre-Lindsay continue to work — the alias map handles legacy keys.

---

## Test contract

| Layer | Tests | Count est |
|---|---|---|
| L0 catalog | `read()` returns expected value for every StatId given a fixture view; `applies_to` truth table; `from_cli_key` round-trip; aliases parse | ~200 |
| L0 filter | `matches_stat_filters` against fixture views: Min/Max/Equals × every category | ~30 |
| L0 grammar | `parse_filter("hits-per-60>=2.0")` round-trips; whitespace tolerance; bad input rejected | ~15 |
| L1 fetch | Each new endpoint URL emits the right cayenneExp; mock fixture parses to typed Tier-1 struct | ~12 |
| L2 system | `query leaders --filter ... --sort ... --top 5` returns expected rows for a known fixture (bundled 2024-25) | ~8 |

---

## Open questions

1. **`fetched_reports: HashMap<ReportKind, serde_json::Value>`** — should this live on `SeasonStats` (per-window) or on a separate `ExtraReports` map keyed by `(player_id, season, season_type, kind)`? The latter is cleaner but adds a new lookup path; the former is closer to the existing shape but inflates `SeasonStats`. **Decision deferred to L.6**; current sketch leans toward separate map.

2. **Site backward-compat** — existing site templates may hard-code stat names. Migration path: grep for stat strings, replace with `StatId` lookups in one commit. **Decision deferred to L.5**.

3. **Goalie `bios` is its own endpoint** — today we use the skater bios as the identity source for goalies (because the existing pipeline didn't differentiate). Is this a real bug? `goalie/bios` returns goalie-shaped fields. **Decision: bundle goalie/bios in L.1**, switch the goalie identity path to read from it. Track the bug as a Lindsay deliverable.

4. **Per-game derived stats** — the catalog sketch has `PointsPerGame`, `GoalsPerGame`, etc. These compute from `Points / GP`. Should they live on the catalog (centralized read) or on `PlayerView` (already there)? **Recommendation**: catalog is the source of truth; `PlayerView` keeps the convenience methods that wrap `StatId::X.read(self)` for ergonomics.

---

## What's NOT in this spec

- Strength-state drilling beyond what the API endpoints already expose (5v5 from `scoringRates`, PP-only from `powerplay`). True strength-state filtering — "show me Player X's stats while playing against Top-10 PP teams" — needs play-by-play; out of scope.
- Score-state filtering. Same.
- Per-game game logs.
- Goalie endpoints that the server returns 500 for. Document; skip.
- Tier 2 typed schemas. They land as `serde_json::Value`; promote when usage demands.
