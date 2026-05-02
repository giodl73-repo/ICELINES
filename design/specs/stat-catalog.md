# IceLines Stat Catalog — Specification

**Version**: 0.3 — Phase Lindsay (R2-applied, spec-body sweep)
**Date**: 2026-05-02
**Status**: Design — paired with `design/plans/2026-05-02-phaseLindsay-stat-catalog.md` v0.3.
10-role review applied (HART, KEEL, TAPE, FORGE, EDGE, BENCH, WIRE, GLASS, SCOUT, PACE).
R1 summary at `design/plans/2026-05-02-phaseLindsay-review-summary.md`,
R2 summary at `design/plans/2026-05-02-phaseLindsay-r2-summary.md`.
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
// 107 cases v0.3 — exhaustive, NOT #[non_exhaustive]. Compiler enforces
// "added a stat → updated everywhere" across all consumer surfaces.
// (v0.2 was 98; v0.3 added the xG family per SCOUT R2 review.)
pub enum StatId { /* 107 cases — see "Stat enumeration" below */ }
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
| `label(self) -> &'static str` | full human-readable: `"Hits per 60"` | dropdowns, table headers, **site templates** |
| `short_label(self) -> &'static str` | terse: `"Hits/60"`, `"H/60"` | column headers (wide) |
| `narrow_label(self) -> &'static str` | abbreviated: `"H/60"`, `"P"` (PIM), `"Sh"` (Shots) | column headers when terminal width < 90 cols |
| `cli_key(self) -> &'static str` | hyphen-case: `"hits-per-60"` | `--filter` / `--sort` parsing, **CSS class suffix**, **URL anchor**, **HTTP JSON key** |
| `read(self, view: &PlayerView<'_>) -> Option<f64>` | the value, `None` if missing or N/A | every consumer |
| `unit(self) -> StatUnit` | how to format — count/pct/per60/seconds; selects equality tolerance | display formatting + `FilterOp::Equals` |
| `higher_is_better(self) -> bool` | `true` for goals, `false` for GAA | percentile rendering, sort direction defaults |
| `applies_to(self, pos: Position, is_goalie: bool) -> bool` | position-applicability (per-row, includes emergency-backup-goalie case) | filter pre-validation, dropdown filtering |
| `applies_to_era(self, season: Season) -> bool` | era-applicability — pre-2005 returns false for `Hits`/`Blocks`/`Takeaways`/`Giveaways`; pre-2007 returns false for `Possession` category | filter UX warning, greying |
| `available_since(self) -> Season` | first season with reliable data; `Season(0)` for always-available | era axis source-of-truth |
| `default_in_career_table(self, pos: Position) -> bool` | `true` for the curated default columns (per-position) | career table column selector default |
| `toml_aliases(self) -> &'static [&'static str]` | legacy snake_case keys (`"pp_goals"`, `"blocked_shots"`) for fantasy scheme TOML | scheme parser back-compat |
| `cli_aliases(self) -> &'static [&'static str]` | legacy hyphen-case CLI flag strings (`"pts-pace"`, `"ppg"`) | CLI back-compat |
| `sort_cmp(self, a: &PlayerView<'_>, b: &PlayerView<'_>) -> Ordering` | universal tiebreak: `(stat_value, nhl_id asc)`. `None` sorts last regardless of `higher_is_better`. | every catalog-driven sort |

### Static methods on `StatId`

```rust
pub fn all() -> &'static [StatId];                           // every variant, in stable declaration order
pub fn members(c: StatCategory) -> &'static [StatId];        // category subset, zero-alloc
pub fn from_cli_key(s: &str) -> Option<StatId>;              // parse "hits-per-60", cli_aliases, or toml_aliases
pub fn applicable_to(pos: Position, is_goalie: bool) -> &'static [StatId]; // for "Stats for centers" dropdown
```

### `ReportKind` (in `icelines-core::stats_catalog`)

```rust
pub enum ReportKind {
    // Tier 1 — bundled, typed
    SkaterSummary, SkaterBios, SkaterRealtime, SkaterTimeOnIce, SkaterGoalsForAgainst,
    GoalieSummary, GoalieBios, GoalieAdvanced, GoalieSavesByStrength,
    // Tier 2 — fetched, JSON blob
    SkaterPuckPossessions, SkaterScoringRates, SkaterSummaryShooting,
    SkaterPowerPlay, SkaterPenaltyKill, SkaterPenalties,
    SkaterFaceoffWins, SkaterFaceoffPercentages, SkaterShotType, SkaterScoringPerGame,
    GoalieStartedVsRelieved, GoalieDaysRest, GoaliePenaltyShots, GoalieShootout,
}

impl ReportKind {
    pub fn url_path(self) -> &'static str;                   // "/skater/realtime"
    pub fn supports(self, season_type: SeasonType) -> bool;  // false for endpoints that 500 on playoff
    pub fn tier(self) -> Tier;                               // Tier1 (typed) | Tier2 (JSON blob)
}
```

---

## Stat enumeration (v0.3 — 107 stats, listed by category)

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

### `Possession` — 15 stats (Tier 2 — `puckPossessions` + `scoringRates` + `summaryshooting` + xG)

`SatPct` (Corsi%), `UsatPct` (Fenwick%),
`OffensiveZoneStartPct`, `DefensiveZoneStartPct`, `NeutralZoneStartPct`,
`OnIceShootingPct`,
`Goals5v5`, `Assists5v5`, `Points5v5`, `PointsPer60_5v5`,

**xG family (SCOUT-B2 addition)** — sourced from MoneyPuck CSV (post-2007) or NHL Edge endpoints:
- `IxG` (individual expected goals, all situations)
- `IxgPer60`
- `OnIceXgFor` / `OnIceXgAgainst`
- `XgForPct`

### `Goalie` — 22 stats (from goalie reports + xG family)

`GoalieGames`, `GoalieStarts`,
`Wins`, `Losses`, `OtLosses`, `Ties`,
`Saves`, `ShotsAgainst`, `GoalsAgainst`,
`SavePct`, `Gaa`, `Shutouts`,
`EvSavePct`, `PpSavePct`, `ShSavePct`,
`QualityStarts`, `QualityStartPct`, `RegulationWins`, `RegulationLosses`,

**GSAx family (SCOUT-B1 addition)** — sourced from MoneyPuck or NHL Edge:
- `GoalieXgAgainst`
- `GoalieXgAgainstPer60`
- `GoalsSavedAboveExpected` (often abbreviated GSAx)
- `Gsax60` (per-60 rate)

### `Derived` — 7 stats (computed from Scoring + GP)

`Pace82`, `GoalsPer82`, `AssistsPer82`,
`PointsPerGame`, `GoalsPerGame`, `AssistsPerGame`,
`PaceSortKey` (the Hart pace + goals tiebreak)

**Total v0.3: 13 + 14 + 16 + 11 + 9 + 15 + 22 + 7 = 107 selectable stats.**
(v0.2 was 98; v0.3 adds the 9-stat xG family across Possession + Goalie per SCOUT-B1/B2.)

The number is approximate; the actual catalog will be settled when L.2
implementation lands. New stats added later add a `StatId` variant and a
`read` arm — the rest of the app inherits them automatically.

### `FaceoffWinPct` recategorization (SCOUT FIXIT L2-F2)

Despite being sourced from `summary` (which the Hart pipeline grouped
under "scoring"), `FaceoffWinPct` is read as a **two-way / 200-foot**
stat by hockey evaluators — most faceoffs happen at even strength, not
on special teams. Recategorized to `TwoWay` (was `SpecialTeams` in v0.2).
Per-strength faceoff splits (`PpFaceoffWinPct`, `ShFaceoffWinPct`,
`OffensiveZoneFaceoffPct`, `DefensiveZoneFaceoffPct`) stay in `SpecialTeams`.

### `EvenStrengthTimeOnIcePerGame` recategorization (SCOUT FIXIT L2-F3)

Currently in `OnIceGoals` because it's sourced from the
`goalsForAgainst` endpoint. **Hockey-domain category** is `TimeOnIce` —
it's a deployment/usage stat, not an on-ice goal stat. The endpoint
source is a TAPE concern, not a SCOUT one. Recategorized to `TimeOnIce`.

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
            // Every per-game/per-82 derived stat inherits the MIN_GP=10
            // guard (PACE-B2 R2 fix). `view.pace_82()` already enforces
            // it via `compute_pace_score`; the others enforce explicitly.
            StatId::Pace82      => view.pace_82(),
            StatId::PointsPerGame => {
                let gp = view.gp();
                if gp < icelines_core::MIN_GP { None }
                else { Some(view.points() as f64 / gp as f64) }
            }
            StatId::GoalsPerGame => {
                let gp = view.gp();
                if gp < icelines_core::MIN_GP { None }
                else { Some(view.goals() as f64 / gp as f64) }
            }
            // Per-60 rates inherit the MIN_GP guard PLUS a TOI floor —
            // a player with 1 game / 100 seconds of ice time produces
            // statistical noise. PACE-F1: soft floor at 300s.
            StatId::HitsPer60 => {
                let toi = view.total_toi_sec()?;
                if toi < 300 { return None; }
                let hits = view.hits()?;
                Some(hits as f64 / toi as f64 * 3600.0)
            }
            // … one arm per StatId, ~107 total
        }
    }
}
```

`read()` is the **only** function that knows where the value lives. Everything else (sort, filter, display, export) calls into it. A change to where `Hits` is stored only touches one match arm.

---

## Position + era applicability

```rust
impl StatId {
    /// Per-row applicability. Goalie-category stats apply when the
    /// view IS a goalie (per-row, via view.is_goalie() — covers the
    /// emergency-backup-goalie case where a skater becomes a goalie
    /// for one game). Faceoff-takers are gated to centers; on-ice
    /// stats (zone-starts, +/-) apply to every skater regardless of
    /// position.
    pub fn applies_to(self, pos: Position, is_goalie: bool) -> bool {
        match self.category() {
            StatCategory::Goalie    => is_goalie,
            StatCategory::Identity  => true,
            _ if is_goalie          => false,    // skater-only stats hidden on goalies
            _ => match self {
                // Faceoff-taker stats — centers only.
                StatId::FaceoffWinPct | StatId::FaceoffWins | StatId::FaceoffLosses
                    => pos == Position::Center,
                // Zone-start stats apply to ALL skaters on the ice for the
                // faceoff, not just the center taking it.
                _ => true,
            }
        }
    }

    /// Per-row era applicability. Pre-2005 nulls hits/blocks/takeaways/
    /// giveaways; pre-2007 nulls possession; on-ice TOI splits begin
    /// 1997-98 but are unreliable until 2005-06. Treat as None below
    /// the boundary.
    pub fn applies_to_era(self, season: Season) -> bool {
        season.0 >= self.available_since().0
    }

    pub fn available_since(self) -> Season {
        match self {
            // Realtime — 2005-06 (data exists 1997+ but unreliable)
            StatId::Hits | StatId::BlockedShots | StatId::Takeaways
            | StatId::Giveaways | StatId::MissedShots
            | StatId::HitsPer60 | StatId::BlockedShotsPer60
            | StatId::TakeawaysPer60 | StatId::GiveawaysPer60
                => Season(20052006),
            // Possession family — Corsi tracking starts 2007-08
            StatId::SatPct | StatId::UsatPct
            | StatId::OffensiveZoneStartPct | StatId::DefensiveZoneStartPct
            | StatId::NeutralZoneStartPct | StatId::OnIceShootingPct
            | StatId::Goals5v5 | StatId::Assists5v5 | StatId::Points5v5
            | StatId::PointsPer60_5v5
                => Season(20072008),
            // xG family — MoneyPuck starts 2007-08; NHL Edge from ~2024
            StatId::IxG | StatId::IxgPer60 | StatId::OnIceXgFor
            | StatId::OnIceXgAgainst | StatId::XgForPct
            | StatId::GoalieXgAgainst | StatId::GoalieXgAgainstPer60
            | StatId::GoalsSavedAboveExpected | StatId::Gsax60
                => Season(20072008),
            // Scoring/Identity/TimeOnIce/SpecialTeams basics — always available
            _ => Season(0),
        }
    }
}
```

The TUI Queries screen uses both gates to grey out non-applicable stats. The CLI
rejects a `--sort` on a non-applicable stat at parse time when position context
is known (e.g., `--sort save-pct --pos C`); for a mixed pool it silently skips
the row-level filter.

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
            // Type-aware tolerance for Equals (L2-B1):
            //   Count   → exact integer comparison
            //   Pct     → 1e-6 (storage precision allows this; f32→f64 round-trip preserves)
            //   Per60   → 1e-3 (rates rounded to 3 decimals at source)
            //   Seconds → exact (integer seconds in u32)
            //   Rate    → 1e-6
            //   Inverted → 1e-6
            let ok = match f.op {
                FilterOp::Min    => actual >= f.value,
                FilterOp::Max    => actual <= f.value,
                FilterOp::Equals => match f.stat.unit() {
                    StatUnit::Count | StatUnit::Seconds => (actual - f.value).abs() < 0.5,  // integer compare
                    StatUnit::Per60 => (actual - f.value).abs() < 1e-3,
                    StatUnit::Pct | StatUnit::Rate | StatUnit::Inverted
                        => (actual - f.value).abs() < 1e-6,
                },
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

- `▶` collapsed, `▼` expanded; **`Tab`** toggles section expansion. (`<space>` is reserved for the existing Queries-screen split-pane focus toggle in `app.rs`.)
- Inside an expanded section: each stat is one row with `+` to add a filter, `Min`/`Max` toggle, value editor.
- Sort dropdown lists every `StatId` grouped by category.
- Position-restricted stats greyed out when the position filter excludes their applicability (e.g., `FaceoffWinPct` greys out for `Pos: D`).

### Player card career table

```
═════════════════════════════════════════════════════════════════
Connor McDavid · EDM · C · Age 28 · #97 · L
═════════════════════════════════════════════════════════════════
                                                   [ / ]  cycle column set
Season   GP  G  A  P  +/- PIM  PPG PPP  Shots  S%   TOI  Hits  Blk
2025-26  82  44 ..
2024-25  76  35 65 100 +22 8    10  35   280   12.5 20:32 22   55
2023-24  82  32 62  94 +24 12   13  35   294   10.9 22:12 18   45
2022-23  82  64 89 153 +22 36   21  56   352   18.2 22:39 15   38
2021-22  80  44 79 123 +28 28   13  44   314   14.0 22:27 17   42
```

- Default columns: a curated subset (`StatId::default_in_career_table` returns true).
- Column selector via **`[`** / **`]`** (single keypress, vim-canonical bracket motion; unbound elsewhere in the TUI): rotates through preset templates (Scoring | Two-way | Special Teams | Time | All).
- User can save a custom column set via `Shift+S`; persisted in `~/.icelines/config.toml`.

---

## Site integration

The mkdocs site renders team pages using a curated subset of StatIds
(driven by `site_columns` config). Catalog entry-point per league
ensures cross-page consistency: every page that names a stat reads from
`StatId::label()`.

**L.5b sweep enumeration** — the four string surfaces:
1. **Rendered headers** in the markdown templates → `StatId::label()`.
2. **CSS class names** for stat cells → `format!("stat-{}", stat.cli_key())` → e.g. `.stat-pp-goals`.
3. **URL anchors** for jump-to-stat links → `stat.cli_key()` → e.g. `#hits-per-60`.
4. **Search-index terms** → free-form (not catalog-controlled), allowlist-gated by `icelines-site/.stat-name-allowlist`.

A grep-based CI test (run on `icelines-site/src/**/*.rs` and
`icelines-site/templates/**/*.{html,md}`) fails if any string matching
`\b(Goals|Assists|Points|Hits|Blocks|Saves|GAA)\b` appears outside
comments AND outside the allowlist. The pattern targets stat-name
literals; doc-comments (`// `) and HTML/markdown comments (`<!-- `)
are excluded.

## HTTP integration (axum server)

The fantasy axum server exposes:
- `GET /api/team/:name/roster` — returns each player as JSON keyed by `StatId::cli_key()`.
- `GET /api/standings` — scoring breakdown per fantasy team, again keyed by `StatId::cli_key()`.

Tier-2 reports are NOT visible through the HTTP server. Promotion to
Tier-1 (per AI-07 invariant) is a prerequisite for HTTP exposure.

L1 round-trip test (`tests/http_round_trip.rs`):
- Send `GET /api/team/test-team/roster`.
- Parse JSON.
- For each per-player object, call `StatId::from_cli_key(key)` for every key.
- Assert every key resolves.
- Assert the value matches `StatId::X.read(view)` for the corresponding view.

This locks the CLI ↔ HTTP key parity invariant (KEEL-B1, R1).

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
