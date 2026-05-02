# IceLines Stat Catalog — Specification

**Version**: 0.4 — Phase Lindsay (R3-applied — spec-body sweep + R3 follow-through)
**Date**: 2026-05-02
**Status**: Design — paired with `design/plans/2026-05-02-phaseLindsay-stat-catalog.md` v0.4.
10-role review applied across three rounds (HART, KEEL, TAPE, FORGE, EDGE, BENCH, WIRE, GLASS, SCOUT, PACE).
R1 summary at `design/plans/2026-05-02-phaseLindsay-review-summary.md`,
R2 summary at `design/plans/2026-05-02-phaseLindsay-r2-summary.md`,
R3 summary at `design/plans/2026-05-02-phaseLindsay-r3-summary.md`.
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
// 108 cases v0.4 — exhaustive, NOT #[non_exhaustive]. Compiler enforces
// "added a stat → updated everywhere" across all consumer surfaces.
// (v0.2 was 98; v0.3 added the 9-stat xG family per SCOUT R2; v0.4 adds
// PpAssists raw + recategorizes FaceoffWinPct → TwoWay,
// EvenStrengthTimeOnIcePerGame → TimeOnIce per SCOUT R2 follow-through.)
pub enum StatId { /* 108 cases — see "Stat enumeration" below */ }
pub enum StatCategory { Identity, Scoring, SpecialTeams, TwoWay, TimeOnIce, OnIceGoals, Possession, Goalie, Derived }
pub enum StatUnit { Count, Pct, Per60, Seconds, Rate, Inverted /* lower-is-better */ }

pub struct StatFilter {
    pub stat: StatId,
    pub op: FilterOp,
    pub value: f64,
}
pub enum FilterOp { Min, Max, Equals }

/// Filter-grammar parse failure (FORGE-R2-B4 / EDGE-R2). Seven variants —
/// every malformed-input class in II-05 / II-06 maps to exactly one variant.
/// `Display` impl produces the user-facing error message; the CLI front-end
/// renders `eprintln!("error: {}", e)` and exits non-zero.
pub enum FilterParseError {
    EmptyInput,                       // ""
    EmptyStatKey,                     // ">=10"  (whitespace-only key also lands here)
    MissingOp { input: String },      // "hits10"
    MultipleOps { input: String },    // "hits>=>5"
    UnknownStat { key: String },      // "hots-per-60"
    BadNumber { token: String },      // "hits>=abc", "hits>=1,5" (locale comma)
    NotFinite { token: String },      // "hits>=NaN", "hits>=inf"
}

/// Runtime-only Tier-2 cache (HART-R2-B1, FORGE-R2-B1, WIRE-R2-B1).
/// `BTreeMap`, NOT `HashMap` — iteration order must be deterministic for
/// snapshot tests, debug dumps, and "list all fetched reports" UX. Keyed
/// by full window identity so eviction can cascade with the typed LRU.
/// Lives on `StatsRepository`; never persisted to disk (see "Repository
/// lifecycle — extra_reports" below).
pub type ExtraReports =
    std::collections::BTreeMap<(PlayerId, Season, SeasonType, ReportKind), serde_json::Value>;

/// Per-window file format for Tier-1 reports (WIRE-R2-B5).
/// Each typed substruct on `SeasonStats` is sourced from a SEPARATE
/// per-report file under `~/.icelines/snapshots/<season>/<season_type>/`.
/// Loaded lazily at `StatsRepository::load_window` time; merged onto the
/// `SeasonStats` row by the loader, never inlined into `stats.json`.
/// `bundle_schema_version=1` stays valid because no existing field shape
/// changes — these are NEW files, not new inline fields.
pub struct Tier1ReportFile {
    pub kind: ReportKind,            // SkaterTimeOnIce, SkaterGoalsForAgainst, …
    pub filename: &'static str,      // "timeonice.json", "goalsForAgainst.json"
    pub merge_target: MergeTarget,   // SeasonStats::time_on_ice, SeasonStats::goals_for_against, …
}
pub enum MergeTarget {
    SkaterRealtime, SkaterTimeOnIce, SkaterGoalsForAgainst,
    GoalieAdvanced, GoalieSavesByStrength,
    // … one variant per Tier-1 substruct on SeasonStats.
}

/// Per-window load-with-fallback (WIRE-R2-F5). Reads from primary snapshot
/// directory, falls back to bundled in-binary data for the recent 5
/// seasons + 33 historical bundled. Returns `None` only when neither
/// source has the file. Used by `StatsRepository::load_window` for every
/// Tier-1 substruct in the window.
pub fn load_report_with_fallback<T: for<'de> serde::Deserialize<'de>>(
    snapshot_dir: &Path,
    season: Season,
    season_type: SeasonType,
    file: &Tier1ReportFile,
) -> Result<Option<T>, LoadError>;
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

## Stat enumeration (v0.4 — 108 stats, listed by category)

### `Identity` — 0 stats (kept as a category for the bios block; never selectable for sort/filter)

The `bios` data drives identity fields (full name, team, position, age,
draft, height, weight, nationality). These are not stats — they're
filter axes already typed on `PlayerFilter`. The catalog includes the
`Identity` category for completeness so TUI can render identity
metadata under the same panel structure.

### `Scoring` — 14 stats (from `summary`)

`Goals`, `Assists`, `Points`, `EvGoals`, `EvPoints`,
`PpGoals`, `PpAssists`, `PpPoints`, `ShGoals`, `ShPoints`,
`Gwg`, `OtGoals`, `Shots`, `ShootingPct`

`PpAssists` (raw count, SCOUT-R2 L2-F5) is exposed as a first-class stat
alongside `PpPoints`/`PpGoals`. Existing per-60 derivative `PpAssistsPer60`
stays in `SpecialTeams`. CLI alias: `pp-assists`.

### `SpecialTeams` — 13 stats (from `summary` + `powerplay` + `penaltykill` + `faceoffwins`)

`FaceoffWinPct` is **not** in this category despite being sourced from the
`summary` endpoint — see `TwoWay` (SCOUT-R2 L2-F2). Per-strength faceoff
splits (zone-keyed) stay here.

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
| `FaceoffWins` | faceoffwins | count |
| `FaceoffLosses` | faceoffwins | count |
| `OffensiveZoneFaceoffPct` | faceoffpercentages | pct |
| `DefensiveZoneFaceoffPct` | faceoffpercentages | pct |

### `TwoWay` — 17 stats (from `summary` + `realtime` + `penalties`)

`PlusMinus`, `Pim`,
`Hits`, `BlockedShots`, `Takeaways`, `Giveaways`, `MissedShots`,
`HitsPer60`, `BlockedShotsPer60`, `TakeawaysPer60`, `GiveawaysPer60`,
`PenaltiesDrawn`, `PenaltiesDrawnPer60`, `PenaltiesTakenPer60`,
`NetPenalties`, `NetPenaltiesPer60`,
`FaceoffWinPct`

`FaceoffWinPct` lives here (not `SpecialTeams`) per SCOUT-R2 L2-F2 — most
faceoffs happen at even strength, not on special teams. `applies_to` still
gates this stat to `Position::Center` (per-row).

### `TimeOnIce` — 12 stats (from `timeonice` + `goalsForAgainst`)

`TotalToi`, `TotalToiPerGame`,
`EvToi`, `EvToiPerGame`, `EvenStrengthTimeOnIcePerGame`,
`PpToi`, `PpToiPerGame`,
`ShToi`, `ShToiPerGame`,
`Shifts`, `ShiftsPerGame`, `ToiPerShift`

`EvenStrengthTimeOnIcePerGame` is sourced from the `goalsForAgainst`
endpoint but is a **deployment** stat, not a goal stat (SCOUT-R2 L2-F3).
Endpoint sourcing is a TAPE concern; the catalog category reflects the
hockey-domain meaning. `read()` does NOT inherit the `OnIceGoals`
trade-window guard (DI-11) — this stat sums correctly across stints.

### `OnIceGoals` — 8 stats (from `goalsForAgainst`)

`EvGoalsFor`, `EvGoalsAgainst`, `EvGoalsForPct`,
`PpGoalsFor`, `PpGoalsAgainst`,
`ShGoalsFor`, `ShGoalsAgainst`,
`EvenStrengthGoalDifference`

(`EvenStrengthTimeOnIcePerGame` was here in v0.3 but recategorized to
`TimeOnIce` in v0.4 per SCOUT-R2 L2-F3. The eight remaining stats all
inherit DI-11: `read()` returns `None` when `view.was_traded_in_window()`.)

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

**Total v0.4: 14 + 13 + 17 + 12 + 8 + 15 + 22 + 7 = 108 selectable stats.**
(v0.2 was 98; v0.3 added the 9-stat xG family across Possession + Goalie
per SCOUT-B1/B2; v0.4 adds `PpAssists` raw count to Scoring (SCOUT-R2 L2-F5)
and applies recategorization moves SCOUT R2 flagged: `FaceoffWinPct` →
`TwoWay`, `EvenStrengthTimeOnIcePerGame` → `TimeOnIce`. Net delta: +1.)

The number is approximate; the actual catalog will be settled when L.2
implementation lands. New stats added later add a `StatId` variant and a
`read` arm — the rest of the app inherits them automatically.

### Recategorization rationale (SCOUT-R2 L2-F2 / L2-F3, applied in v0.4)

- **`FaceoffWinPct` → `TwoWay`** (was `SpecialTeams` in v0.3 prose but
  v0.3 spec body still listed it under SpecialTeams — fixed in v0.4).
  Most faceoffs happen at even strength, not on special teams; hockey
  evaluators read this as a 200-foot stat. Per-strength faceoff splits
  (`OffensiveZoneFaceoffPct`, `DefensiveZoneFaceoffPct`) stay in
  `SpecialTeams` — those are about deployment, not skill.
- **`EvenStrengthTimeOnIcePerGame` → `TimeOnIce`** (was `OnIceGoals` in
  v0.3 prose but v0.3 body still listed it under OnIceGoals — fixed in
  v0.4). Endpoint sourcing (`goalsForAgainst`) is a TAPE concern; the
  hockey-domain meaning is deployment / usage, which is `TimeOnIce`.
  Side effect: this stat is **exempt from DI-11** (the `OnIceGoals`
  trade-window guard) — TOI sums correctly across stints.

---

## Read dispatch — the contract

```rust
impl StatId {
    pub fn read(self, view: &PlayerView<'_>) -> Option<f64> {
        // DI-11 enforcement at category boundary (EDGE-R3 explicit guard).
        // OnIceGoals stats are last-stint-only; summing across stints is
        // wrong-data. The guard fires here, not at every match arm, so
        // adding a new OnIceGoals stat doesn't require remembering the
        // rule. `EvenStrengthTimeOnIcePerGame` is in TimeOnIce (v0.4
        // recategorization) so it does NOT short-circuit here — TOI
        // sums correctly across stints.
        if self.category() == StatCategory::OnIceGoals
            && view.was_traded_in_window()
        {
            return None;
        }

        match self {
            // Identity — always None (not a stat); category-iterable but
            // not selectable.
            // Scoring (always present on a skater view; None for goalie).
            StatId::Goals       => Some(view.stats.totals.goals as f64),
            StatId::Assists     => Some(view.stats.totals.assists as f64),
            StatId::Points      => Some(view.stats.totals.points as f64),
            StatId::PpGoals     => Some(view.stats.totals.pp_goals as f64),
            StatId::PpAssists   => Some(view.stats.totals.pp_assists as f64),  // v0.4 — SCOUT L2-F5
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
            // OnIceGoals (post-guard) — None when the goalsForAgainst
            // substruct is unloaded for this window.
            StatId::EvGoalsFor => view.stats.goals_for_against.as_ref()
                                       .map(|g| f64::from(g.ev_goals_for)),
            // … one arm per StatId, 108 total
        }
    }

    /// Multi-season aggregate read (PACE-R2 F3 — strict propagation).
    /// Used by `query player --seasons N` and TUI career table totals.
    /// Returns `Some(sum)` only when EVERY window in the slice has a
    /// `Some` from `read()`. ANY `None` (missing data, era gate, trade
    /// guard, MIN_GP floor) propagates as `None` — no silent zeros.
    /// For non-Count units (Pct, Per60, Rate), `aggregate_read` is NOT
    /// a sum — it routes through `category()` to compute the correct
    /// blend (e.g. weighted by GP for percentages, weighted by TOI for
    /// per-60 rates). Equivalent to the existing `view.career_totals()`
    /// helper but catalog-driven.
    pub fn aggregate_read(self, views: &[PlayerView<'_>]) -> Option<f64> {
        // (signature pinned — implementation in L.2; behavior locked here.)
        unimplemented!()
    }
}
```

`read()` is the **only** function that knows where the value lives. Everything else (sort, filter, display, export) calls into it. A change to where `Hits` is stored only touches one match arm.

The leading DI-11 guard at the top of `read()` makes the trade-window
short-circuit a property of the **category**, not of every match arm.
Adding a new `OnIceGoals` stat inherits the guard for free; moving a
stat OUT of `OnIceGoals` (as `EvenStrengthTimeOnIcePerGame` did in v0.4)
removes the guard automatically.

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
    pub value: f64,      // construction guarantees finite (parser rejects NaN/inf)
}

impl StatFilter {
    /// Construction is the only place a `StatFilter` can be made.
    /// `value.is_finite()` is enforced at construction; downstream code
    /// (sort, dedup, eval) can assume no NaN/inf ever appears (II-05,
    /// EDGE-R2). The CLI parser routes through this constructor; the TUI
    /// numeric-input field validates before constructing.
    pub fn new(stat: StatId, op: FilterOp, value: f64)
        -> Result<Self, FilterParseError>
    {
        if !value.is_finite() {
            return Err(FilterParseError::NotFinite { token: value.to_string() });
        }
        Ok(Self { stat, op, value })
    }
}

impl PlayerFilter {
    /// Same-StatId multi-filter normalization (EDGE-R2 / II-06).
    /// Two filters on the same StatId+op compose deterministically:
    ///   `--filter "hits-min 50" --filter "hits-min 100"`  →  effective `hits-min 100`
    ///   `--filter "hits-max 200" --filter "hits-max 150"` →  effective `hits-max 150`
    /// Mixed Min+Max on the same StatId compose to a closed range.
    /// `Equals` on the same StatId TWICE is rejected at parse time as
    /// `FilterParseError::MultipleOps` (no consistent normalization).
    pub fn normalize_stat_filters(&mut self) {
        // Group by (StatId, op kind), keep tightest bound per (stat, kind).
        // Implementation lands in L.2; behavior locked here.
    }

    pub fn matches_stat_filters(&self, view: &PlayerView<'_>) -> bool {
        for f in &self.stat_filters {
            // Skip non-applicable filters silently — DI-08.
            if !f.stat.applies_to(view.position(), view.is_goalie()) {
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
number      := <decimal>                          -- finite f64; NaN/inf REJECTED
```

Whitespace allowed around `<op>`. Multiple `--filter` flags accumulate
(implicit AND, then normalized per `PlayerFilter::normalize_stat_filters`).
`--sort <stat-key>` accepts the same key set.

Existing typed flags (`--ppg-min`, `--gp-min`, `--toi-min`, `--plus-minus-min`,
`--shots-pg-min`) keep working. Internally they become typed slots on
`PlayerFilter`; the new `--filter` just appends to `stat_filters`.

### Parse-error variants

Every malformed filter input maps to exactly one `FilterParseError` variant
(EDGE-R2, FORGE-R2-B4). The CLI front-end renders `eprintln!("error: {}",
err)` and exits non-zero. Variants and triggering inputs:

| Variant | Triggering input examples |
|---|---|
| `EmptyInput` | `""` (empty `--filter` value) |
| `EmptyStatKey` | `">=10"`, `"   >= 10"` (whitespace-only key) |
| `MissingOp` | `"hits10"`, `"hits 10"` (no op token) |
| `MultipleOps` | `"hits>=>5"`, `"hits===5"` (more than one op) |
| `UnknownStat` | `"hots-per-60"`, `"foo"` (parses but `from_cli_key` returns `None`) |
| `BadNumber` | `"hits>=abc"`, `"hits>=1,5"` (locale comma — explicit reject) |
| `NotFinite` | `"hits>=NaN"`, `"hits>=inf"`, `"hits>=-inf"` |

Backward-compatibility aliases (existing strings → canonical StatId
short-key):

| Legacy flag | StatId | Note |
|---|---|---|
| `pts-pace` | `Pace82` | |
| `g-pace` | `GoalsPer82` | |
| `pp-g-pace` | `PpGoalsPer60` | |
| `sh-g-pace` | `ShGoalsPer60` | |
| `gwg-pace` | not a Lindsay stat (no `gwg-per-60` published) — keep typed handler | intentional non-mapping |
| `xg`, `cf-pct`, `xgf-pct` | `Possession` category — keep MoneyPuck path | |
| `improvement` | not a stat — keep as a special sort mode | |
| `--ppg-min N` | **divergent** from catalog `points-per-game>=N` | legacy `--ppg-min` uses Hart `pace_82/82` semantic; new catalog `points-per-game` uses `points / gp`. Documented intentional split (L-B5) — both paths stay live. |

---

## Repository lifecycle — `extra_reports`

The Tier-2 `ExtraReports` cache (`BTreeMap<(PlayerId, Season, SeasonType,
ReportKind), serde_json::Value>`) lives on `StatsRepository` alongside the
typed-window LRU. Three rules govern it (HART-R3 / WIRE-R3 / PACE-R3):

### Cascade eviction (DI-12)

When the typed-window LRU evicts a `(season, season_type)` window, the
repository MUST cascade-evict every `extra_reports` entry whose key
prefix matches `(_, season, season_type, _)`. Without this rule, Tier-2
blobs leak across LRU sweeps and resident memory grows unbounded as the
user time-travels across seasons.

```rust
impl StatsRepository {
    fn evict_window(&mut self, key: WindowKey) {
        // Drop the typed substructs (existing).
        self.windows.remove(&key);
        // NEW v0.4: cascade-evict Tier-2 blobs for the same window.
        self.extra_reports.retain(|(_, season, season_type, _), _|
            (*season, *season_type) != (key.season, key.season_type)
        );
    }
}
```

L0 test (`l0_repo_extra_reports_cascade_evict_on_window_drop`) asserts
this property — fill primary LRU to capacity, force a window eviction,
assert `extra_reports` for that window is empty.

### Cap (DI-26)

`extra_reports` is capped at **4096 entries** (~40 MB ceiling at 10 KB/value
worst case). Insertion past the cap evicts the oldest entry by LRU order.
The cap is independent of the typed-window LRU (DEFAULT_LRU_CAP=8) — Tier-2
is a value cache, not a window cache.

L0 test (`l0_repo_extra_reports_cap_at_4096`) asserts insertion 4097 evicts
the oldest entry.

### Runtime-only (DI-27)

`extra_reports` is **never persisted to disk**. Fetching populates the
in-process map; subsequent runs re-fetch. Avoids file-format proliferation
and matches the "Tier-2 = on-demand" semantic. If a Tier-2 report graduates
to ≥2 surfaces, AI-07 mandates promotion to a typed Tier-1 substruct first
— at which point persistence is governed by the Tier-1 file format below,
not by the cache.

L1 test (`l1_repo_extra_reports_not_persisted`) asserts no file is written
under `~/.icelines/snapshots/` after `fetch_report_into_extra` runs.

### `repository_version` boundary check (HART-R3 / DI-28)

The version check fires at `StatsRepository::load_window`, NOT at
`repo_swap` (HART-R2-B2). An old binary opening a v=2 snapshot must error
at the file-open boundary with `LoadError::RepoVersionUnknown { found,
expected }`. Deferring the check until swap time leaves the repo in a
half-loaded state.

```rust
impl StatsRepository {
    pub fn load_window(&mut self, key: WindowKey) -> Result<(), LoadError> {
        let manifest_version = read_manifest_version(&self.snapshot_dir, key)?;
        if manifest_version > REPOSITORY_VERSION {
            return Err(LoadError::RepoVersionUnknown {
                found: manifest_version,
                expected: REPOSITORY_VERSION,
            });
        }
        // … typed substruct load …
    }
}
```

L1 test (`l1_repo_load_window_rejects_repository_version_2_on_v1_binary`)
synthesizes a v=2 manifest and asserts the v=1 binary errors cleanly.

---

## Tier-1 file format (WIRE-R3 / DI-09 elaboration)

Each typed Tier-1 substruct on `SeasonStats` is sourced from a SEPARATE
per-report file under `~/.icelines/snapshots/<season>/<season_type>/`.
The `bundle_schema_version=1` claim stays valid because no existing field
shape changes — these are NEW files, not new inline fields.

| Substruct on `SeasonStats` | Filename | Endpoint | Tier |
|---|---|---|---|
| `realtime` (existing) | `realtime.json` | `/skater/realtime` | 1 |
| `time_on_ice` (NEW) | `timeonice.json` | `/skater/timeonice` | 1 |
| `goals_for_against` (NEW) | `goalsForAgainst.json` | `/skater/goalsForAgainst` | 1 |
| `goalie` (existing) | `goalie-summary.json` | `/goalie/summary` | 1 |
| `goalie_advanced` (NEW) | `goalie-advanced.json` | `/goalie/advanced` | 1 |
| `goalie_saves_by_strength` (NEW) | `goalie-savesByStrength.json` | `/goalie/savesByStrength` | 1 |
| `goalie_bios` (NEW) | `goalie-bios.json` | `/goalie/bios` | 1 |

Tier-2 reports do NOT live on `SeasonStats` — they live in `extra_reports`
(the runtime-only `BTreeMap` above).

### Load path

`StatsRepository::load_window` reads each Tier-1 file via
`load_report_with_fallback<T>` (signature in §Public types):

1. Check the snapshot dir for `<season>/<season_type>/<filename>`.
2. If absent, fall back to bundled in-binary data via `bundled::report_for(season, season_type, kind)`.
3. If both absent, return `Ok(None)` — substruct stays `None` on `SeasonStats`.
4. Any deserialization or seasonId-fence failure errors out at this boundary; the substruct never gets a partial-load state.

### Per-endpoint seasonId fence (TAPE-R3 / DI-29)

Every Tier-1 deserializer asserts `row.seasonId == requested_season` for
every row in the file. Mismatch errors `LoadError::SeasonIdMismatch
{ expected, actual, endpoint }` BEFORE the substruct populates. Mirrors
the Hart.6.4 typed fence semantic for the new endpoints.

L1 test for each new endpoint:
`l1_<endpoint>_rejects_mismatched_season_id` synthesizes a row with
mismatched seasonId, asserts the fence fires.

The same fence applies on the Tier-2 `extra_reports` write path
(L-B1 / WIRE-B6) — reaffirmed here for symmetry.

### Rate-limit policy (TAPE-R3)

The fetch CLI is the only path that issues HTTP requests. It enforces:

1. **Sequential** — one in-flight request at a time. NHL stats API has no
   documented rate ceiling, but historical experience (Phase Hart.6, Phase
   Selke) shows degraded responses at >5 RPS sustained.
2. **Backoff on 429 / 5xx** — exponential, base 500ms, cap 30s, max 5 retries.
   Codified in `icelines_fetch::nhl_api::with_retry` (existing helper extended
   to cover the new 23 endpoints).
3. **Bundled-data fallback first** — `fetch report --kind X` checks the
   bundled cache before issuing a request. Forces network only when
   `--no-cache` is passed or the season isn't bundled.
4. **Concurrent-window guard** — concurrent `fetch report` invocations on
   the same `(kind, season, season_type)` triple are serialized via a
   filesystem lock at `~/.icelines/.fetch.lock`.

L2 test (`l2_fetch_report_serializes_concurrent_invocations`) launches two
sub-processes targeting the same window and asserts only one network
request is issued.

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

### DI-25 — frozen-golden semantics (FORGE-R3 / R2-B12 precision)

DI-25 reads "every pre-Lindsay scheme TOML loads byte-identical to its
frozen golden via the legacy-key alias map" — NOT round-trip
self-equality. The reference is a fixed pre-L.5 capture, not whatever
post-Lindsay output the current binary emits.

Specifically:

1. **Pre-L.5 capture step.** Before L.5 lands, run each of the five named
   legacy schemes through the scheme parser using the pre-Lindsay binary
   and serialize the resulting `Scheme` struct to a frozen TOML golden at
   `icelines-fetch/tests/fixtures/legacy_schemes/<name>.golden.toml`.
   Commit goldens.
2. **Post-L.5 assertion.** L1 test `l1_legacy_schemes_load_byte_identical`
   reloads the same TOML through the post-Lindsay binary, serializes,
   asserts byte-equality against the golden.
3. **Five named legacy schemes** (BENCH-R2 L2-B24): `yahoo-standard`,
   `espn-standard`, `custom-points-only`, `head-to-head-9cat`,
   `rotisserie-with-goalie`. Files at
   `icelines-fetch/tests/fixtures/legacy_schemes/<name>.toml`
   with companions `<name>.golden.toml`.

A round-trip self-equality test is weaker — it would pass even if the
post-Lindsay parser silently dropped a legacy-key alias.

---

## Test contract

| Layer | Tests | Count est | Notes |
|---|---|---|---|
| L0 catalog | `read()` returns expected value for every StatId × every fixture variant; `applies_to` truth table; `from_cli_key` round-trip; aliases parse | ~600 | Fixture variant catalog at `icelines-core/tests/fixtures/stat_catalog_variants.rs` enumerates: skater-modern, skater-pre-2005, center, goalie, traded-multistint, GP=0 (BENCH-R2 L2-B22). Cross-product = ~108 stats × 6 variants. |
| L0 filter | `matches_stat_filters` against fixture views: Min/Max/Equals × every category; multi-filter normalization (Min+Min → tightest; Min+Max → range; Equals+Equals → reject) | ~50 | EDGE-R2 multi-filter rules covered. |
| L0 grammar | `parse_filter("hits-per-60>=2.0")` round-trips; whitespace tolerance; every `FilterParseError` variant (~7) × ~3 trigger inputs each | ~25 | EDGE-R2 / II-05 grammar floor. |
| L0 repo | `extra_reports` cascade-evict on window drop; cap at 4096; runtime-only (no disk write) | ~6 | HART-R3 / DI-12, DI-26, DI-27. |
| L1 fetch | Each new endpoint URL emits the right cayenneExp; mock fixture parses to typed Tier-1 struct; per-endpoint seasonId fence; rate-limit retry/backoff | ~30 | TAPE-R3 fence + WIRE-R3 mock fixture coverage. |
| L1 repo | `repository_version=2` snapshot rejected by v=1 binary at `load_window`; `load_report_with_fallback` snapshot→bundled→None decision tree | ~5 | HART-R3 / DI-28 boundary check. |
| L1 schemes | Five named legacy schemes load byte-identical to frozen goldens (DI-25) | 5 | One per scheme. |
| L1 site | Generation-determinism: every header in rendered templates equals `StatId::label()` for the corresponding StatId; grep CI test passes for the 38-season parse-fence | ~3 | SI-03 + L-B20 cross-product. |
| L1 HTTP | Round-trip parity — JSON keys all parse via `from_cli_key`; values match `read(view)` | ~2 | KEEL-B1 / II-06 enforcement. |
| L2 system | `query leaders --filter ... --sort ... --top 5` returns expected rows for a known fixture (bundled 2024-25); legacy `--sort` parity stdout fence (capture pre-L.3, reassert post-L.3 + post-L.5) | ~10 | BENCH-R2 L2-B23 — TWO fences (sort ordering changes ride L.3, not L.5). |
| L2 fetch | `fetch report --kind X` serializes concurrent invocations on the same window | ~2 | TAPE-R3 lock guard. |

---

## Open questions

All v0.1 open questions resolved through R1/R2/R3:

1. ~~**`fetched_reports`** location~~ — **resolved v0.2 (L-B1) + v0.4 lifecycle pinning**. Tier-2 lives in a runtime-only `ExtraReports: BTreeMap` on `StatsRepository`, cascade-evicted with the typed LRU (DI-12), capped at 4096 entries (DI-26), never persisted to disk (DI-27).

2. ~~**Site backward-compat**~~ — **resolved v0.2 (L-B16) + v0.3 (L2-B20)**. Atomic L.5b sub-phase; four string surfaces enumerated (rendered headers, CSS class names, URL anchors, search-index terms); CI grep test on `\b(Goals|Assists|Points|Hits|Blocks|Saves|GAA)\b`.

3. ~~**Goalie `bios` endpoint**~~ — **resolved v0.2**. Bundled in L.1; goalie identity path switches to `goalie/bios`. `goalie-bios.json` per-window file format (see Tier-1 file format above).

4. ~~**Per-game derived stats placement**~~ — **resolved v0.2**. Catalog is source of truth; `PlayerView::points_per_game()` and friends are ergonomic wrappers that call `StatId::PointsPerGame.read(self)` internally. MIN_GP=10 guard inherited from catalog (PACE-B2 / v0.3 fix).

---

## What's NOT in this spec

- Strength-state drilling beyond what the API endpoints already expose (5v5 from `scoringRates`, PP-only from `powerplay`). True strength-state filtering — "show me Player X's stats while playing against Top-10 PP teams" — needs play-by-play; out of scope.
- Score-state filtering. Same.
- Per-game game logs.
- Goalie endpoints that the server returns 500 for. Document; skip.
- Tier 2 typed schemas. They land as `serde_json::Value`; promote when usage demands.
