# Phase Lindsay — Stat Catalog (v0.2, post-review)

**Status**: v0.2 — 8-role review punch list applied (HART, KEEL, TAPE, FORGE, EDGE, BENCH, WIRE, GLASS).
24 BLOCKERs resolved inline. Ready to implement. SCOUT + PACE deferred to v0.3 if implementation surfaces concerns.
**Date**: 2026-05-02
**Trophy**: Lindsay (Ted Lindsay Award — players' choice; "complete picture of a player")
**Predecessor**: design/plans/2026-04-30-phaseHart-normalization.md (master Hart),
design/plans/2026-05-01-phaseHart-6-playoff-data.md (Hart.6 playoff data)
**Review summary**: design/plans/2026-05-02-phaseLindsay-review-summary.md
**Replaces**: nothing — additive

---

## v0.1 → v0.2 changelog

Twenty-four BLOCKERs across 8 reviews. Punch list condensed:

### Data-model decisions pinned (was deferred in v0.1)

- **L-B1** — `fetched_reports` blob removed from `SeasonStats`. Tier-2 reports live in a separate `ExtraReports` map owned by `StatsRepository`, keyed `(player_id, season, season_type, kind)`, with its own LRU. Decided BEFORE L.1 (was Open Question 1).
- **L-B10** — `repository_version` bumps to 2 (model gains typed substructs). `bundle_schema_version` stays at 1 (new files, not new fields). Old binaries error cleanly with `RepoVersionUnknown`.
- **L-B11** — `ChunkedManifest` refactors to `HashMap<(ReportKind, SeasonType), HashMap<u32, String>>` with custom Deserialize promoting old flat fields. `ChunkedManifest::version` bumps to 2.
- **L-B17** — `StatId` stays exhaustive (NOT `#[non_exhaustive]`). Compiler enforces "added a stat → updated everywhere."
- **L-F2** — `ReportKind` lives in `icelines-core::stats_catalog`. `icelines-fetch` imports it. Per the dependency chain.

### Position + era applicability (was incomplete in v0.1)

- **L-B2** — `StatId::read(view)` is row-local. Aggregations over `&[PlayerView]` MUST call `debug_assert_view_window_homogeneous` at the entry point (Hart.6.6). Added as an explicit invariant.
- **L-B3** — `applies_to(self, position)` clarified: per-row, evaluated per-view at filter time. Added `available_since: Season` and `applies_to_era(season)` for era axis (pre-2005 nulls hits/blocks; pre-2007 nulls possession).
- **L-B5** — Legacy `--ppg-min` keeps the `pace_82/82` semantic; new `--filter "points-per-game>=X"` uses catalog `points/gp`. Documented as an intentional split — not all legacy CLI flags map 1:1 to a catalog stat.
- **L-B6** — Per-60 division on zero TOI returns `None`. Added `view.total_toi_sec() -> Option<u32>` accessor; every per-60 arm routes through it.
- **L-B7** — `OnIceGoals` category returns `None` from `read()` when `view.was_traded_in_window() == true`. Per-team semantics — summing across stints is wrong-data.
- **L-B8** — `parse_filter` rejects NaN / infinity. Added to II-05.
- **L-B9** — Universal sort tiebreak: `(stat_value, nhl_id asc)`. `None` sorts last regardless of `higher_is_better`. Added as AI-06.

### 4-surface convergence (was incomplete in v0.1)

- **L-B14** — axum HTTP server added to L.5 deliverables. `/api/team/:name/roster` and `/api/standings` JSON keys use `StatId::cli_key()` strings. L1 round-trip test asserts `from_cli_key()` parses every key the server emits.
- **L-B15** — Fantasy scheme TOML migration: TOML uses snake_case (`pp_goals`), CLI uses hyphen (`pp-goals`). Two parallel alias maps. New invariant DI-25: every pre-Lindsay scheme TOML loads byte-identical via the legacy-key alias map. Required L1 fixture: ≥5 known-shaped legacy schemes in `icelines-fetch/tests/fixtures/legacy_schemes/`.
- **L-B16** — Site rename atomic — new sub-phase **L.5b** ("site stat-name sweep — one commit, one PR, all team page headers read from `StatId::label()`"). L1 generation-determinism test asserts headers match `StatId::label()` verbatim.

### Tier-1 schema details (was unspecified in v0.1)

- **L-B12** — Mock fixture coverage mandatory. Each of 18 new endpoint URLs gets an `httpmock` fixture serving a captured real response. Budget ~54 fixture files in `icelines-fetch/tests/fixtures/api_<endpoint>/`.
- **L-B13** — Endpoint probe artifact required. Commit `data/api-probe-2026-05-02.txt` with exact URLs tested, response status, sample row per endpoint. Hart.6 precedent.
- **L-B4** — Goalie bios merge_with policy needs explicit field-mapping table + L0 test. `shoots_catches` → `catches`, `position_code` always "G", `first_season_for_game_type` semantics differ. Add adapter + proptest in L.2.

### Test contract corrections (was under-specified in v0.1)

- **L-B18** — L0 fixture coverage strategy: table-driven proptest over `StatId::all() × {skater-modern, skater-pre-2005, center, goalie, traded-multistint, GP=0}`. Asserts read/applies_to/from_cli_key for every cell. ~600 dispatch points covered, not 200.
- **L-B19** — Legacy `--sort` parity fence: capture stdout for every legacy `--sort` value (~30 strings) BEFORE L.5; reassert byte-equality AFTER. L2 test in `system_tests.rs`.
- **L-B20** — L.7 ships 9 reports × 38 seasons = 342 files. Need `l0_lindsay_7_each_tier1_report_parses_for_all_38_bundled_seasons` cross-product test.
- **L-B21** — Filter grammar: ~20 malformed-input classes minimum (empty, whitespace, missing op, NaN, infinity, scientific notation, comma decimal, multiple ops, Unicode confusables, locale-specific separators).

### TUI corrections (was incomplete in v0.1)

- **L-B22** — `<` / `>` keybind dropped. Use `[` / `]` for column cycling on the career table (single keypress, vim-canonical for bracket motion). Lock in spec.
- **L-B23** — Nav bar overflow at 100 cols. Column-selector indicator goes in player card title block (free real estate at `player.rs:90`), NOT nav bar. Add 100-col snapshot test asserting nav fits.
- **L-B24** — Career-table column overflow at 80 cols. Spec specifies degradation: drop columns from right when width < 100 cols. Abbreviated column headers ("PIM" → "P", "Shots" → "Sh") at <90 cols. Documented in L.4.

### Open questions resolved

- **OQ#1** (was: `fetched_reports` location) — resolved via L-B1: separate `ExtraReports` map.
- **OQ#3** (was: goalie/bios switch) — resolved: bundle goalie/bios in L.1; switch goalie identity path; track skater-bios-fallback deprecation.
- **OQ#4** (was: derived stats placement) — resolved: catalog is source of truth; PlayerView accessors stay as ergonomic wrappers calling `StatId::X.read(self)`.

The full punch list (29 BLOCKERs / 41 FIXITs / 33 NITs across 8 reviews) is at
`design/plans/2026-05-02-phaseLindsay-review-summary.md`.

---

## Goal

Make every NHL stat the API exposes — across **23 working endpoints** for skaters
and goalies — usable everywhere in the app without per-stat boilerplate.

After Phase Lindsay:

```bash
icelines fetch report --kind realtime --season 20242025 --type playoff
icelines query leaders --filter "hits-per-60>=2.0,blocks>=50" --sort pp-points-per-60 --top 20
```

Every stat the pipeline knows about is selectable in:
- `query leaders / player / compare / goalies` (filter + sort)
- TUI Queries screen (categorized sections, `<` / `>` cycle)
- TUI Player card career table (column selector)
- Depth chart scoring mode (any per-stat or composite)
- `export md` (column selector)
- `fantasy` scheme (per-stat coefficients in TOML)
- L0/L1 fixtures (no manual struct mutation per new field)

The platform invariant: **adding a new stat is one enum case + one match arm**, never N×7 copy-paste across surfaces.

---

## Why now

Hart phase normalized the data model around `(player_id, season, season_type)`.
The data path is clean. The **read-side** is still hand-coded per stat: every
new stat needs a new `SortMetric` variant, a new `--flag` on `LeadersArgs`,
new wiring on Queries TUI, a new export column, a new test.

Hart.6.9 just exposed the pain — adding Hits/Blocks to the career table requires
threading the stat through 7 surfaces. Multiplying that by the 100+ stats the
NHL API exposes is unsustainable.

Lindsay introduces a **stat catalog** (`StatId` enum + `StatCategory` taxonomy)
as the single source of truth. Every surface dispatches through it.

---

## Endpoint inventory (verified 2026-05-02)

### Skater — 15 working, 1 broken

| Endpoint | Tier | Bundled today? | Notes |
|---|---|---|---|
| `summary` | **1 (bundle)** | yes | G/A/P/+/-/PIM/PPG/PPP/SHG/GWG/Shots/S%/TOI/FO% |
| `bios` | **1 (bundle)** | yes | identity (full name, birth, draft, height, weight) |
| `realtime` | **1 (bundle)** | no | hits/blocks/takeaways/giveaways/missed-shots + per-60 rates |
| `timeonice` | **1 (bundle)** | no | EV/PP/SH/OT TOI splits, shifts, TOI/shift |
| `goalsForAgainst` | **1 (bundle)** | no | on-ice goal differential at EV/PP/SH |
| `puckPossessions` | 2 (fetch) | no | Corsi/Fenwick (SAT/USAT), zone-start splits — post-2007 |
| `scoringRates` | 2 (fetch) | no | 5v5-only G/60, A/60, P/60, primary/secondary |
| `summaryshooting` | 2 (fetch) | no | SAT/USAT by score state (ahead/behind/tied/close) |
| `powerplay` | 2 (fetch) | no | PP-only breakdowns (some overlap with summary) |
| `penaltykill` | 2 (fetch) | no | SH-only breakdowns |
| `penalties` | 2 (fetch) | no | Minor/major/match/misconduct counts, draws, per-60 |
| `faceoffwins` | 2 (fetch) | no | Centers — D/N/O zone × EV/PP/SH wins/losses |
| `faceoffpercentages` | 2 (fetch) | no | Same shape, percentages |
| `shottype` | 2 (fetch) | no | Backhand/wrist/snap/slap/tip/wrap goals + S% |
| `scoringpergame` | 2 (fetch) | no | Per-game versions (mostly redundant with summary) |
| ~~`advanced`~~ | — | — | Server returns 500 — endpoint disabled |

### Goalie — 8 working, 7 broken

| Endpoint | Tier | Bundled today? | Notes |
|---|---|---|---|
| `summary` | **1 (bundle)** | yes | W/L/SV%/GAA/SO/Saves/SA/TOI |
| `bios` | **1 (bundle)** | no | identity (we use skater bios as fallback today) |
| `advanced` | **1 (bundle)** | no | quality starts, complete-game %, regulation W/L, shots/60 |
| `savesByStrength` | **1 (bundle)** | no | EV/PP/SH save% splits |
| `startedVsRelieved` | 2 (fetch) | no | Relief vs start splits |
| `daysrest` | 2 (fetch) | no | Performance by days off |
| `penaltyShots` | 2 (fetch) | no | Penalty shot SV% |
| `shootout` | 2 (fetch) | no | Shootout stats |
| ~~`realtime`, `savePercentage`, `goalsfor/againstbystrength`, `penaltykill`, `percentages`, `shottype`, `timeonice`~~ | — | — | All return 500 — server doesn't expose for goalies |

**Tier 1 totals**: 5 skater reports + 4 goalie reports = **9 reports × 38 seasons = 342 JSON files added to `data/seasons/`** (including the 5 already there: `summary`, `bios`, `realtime`, `timeonice`, `goalsForAgainst` for skaters; `summary`, `bios`, `advanced`, `savesByStrength` for goalies).

**Tier 2 totals**: 10 skater reports + 4 goalie reports = **14 fetchable-only**.

---

## The new data model

### Catalog: `StatId` + `StatCategory`

Single enum lives in `icelines-core::stats_catalog` (new module):

```rust
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum StatId {
    // ── Scoring (summary) ─────────────────────────────────────
    Goals, Assists, Points, PpGoals, PpPoints, ShGoals, ShPoints,
    Gwg, OtGoals, Shots, ShootingPct, EvGoals, EvPoints,
    // ── Two-way (summary + realtime) ─────────────────────────
    PlusMinus, Pim, Hits, BlockedShots, Takeaways, Giveaways, MissedShots,
    HitsPer60, BlockedShotsPer60, TakeawaysPer60, GiveawaysPer60,
    // ── Time on ice (timeonice) ──────────────────────────────
    TotalToi, EvToi, PpToi, ShToi, OtToi, Shifts, ToiPerShift,
    // ── On-ice goals (goalsForAgainst) ───────────────────────
    EvGoalsFor, EvGoalsAgainst, EvGoalDiff,
    PpGoalsFor, PpGoalsAgainst, ShGoalsFor, ShGoalsAgainst,
    // ── Faceoffs (summary; centers only) ─────────────────────
    FaceoffWinPct, FaceoffWins, FaceoffLosses,
    // ── Goalie (goalie/summary + advanced + savesByStrength) ─
    Wins, Losses, OtLosses, Saves, ShotsAgainst, SavePct, Gaa, Shutouts, GoalieGames,
    EvSavePct, PpSavePct, ShSavePct,
    QualityStarts, QualityStartPct, RegulationWins, RegulationLosses,
    // ── Per-game derived ─────────────────────────────────────
    Pace82,           // (G+A) / GP * 82
    GoalsPer82,
    PointsPerGame,
    // (more added here over time — single dispatch table)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum StatCategory {
    Identity,           // bios — never selectable for sort/filter
    Scoring,            // G, A, P, S%
    SpecialTeams,       // PP/SH/Faceoffs
    TwoWay,             // +/-, Hits, Blocks
    TimeOnIce,          // TOI splits
    OnIceGoals,         // GF/GA at EV/PP/SH
    Possession,         // Corsi/SAT (Tier 2)
    Goalie,             // W/L/SV%/GAA
    Derived,            // Pts/82, PPG, GPG (computed from above)
}

impl StatId {
    pub fn category(self) -> StatCategory;
    pub fn label(self) -> &'static str;          // "Hits", "PP G/60"
    pub fn short_label(self) -> &'static str;    // "Hits", "PPG/60"
    pub fn read(self, view: &PlayerView<'_>) -> Option<f64>;
    pub fn unit(self) -> StatUnit;               // Count | Pct | Per60 | Seconds | Rate
    pub fn higher_is_better(self) -> bool;       // true for goals, false for GAA
    pub fn applies_to(self, position: Position) -> bool;  // FaceoffWinPct only for centers, Goalie* only for G
    pub fn all() -> &'static [StatId];
    pub fn category_members(c: StatCategory) -> impl Iterator<Item = StatId>;
}
```

### Generic filter

```rust
pub struct StatFilter {
    pub stat: StatId,
    pub op: FilterOp,
    pub value: f64,
}
pub enum FilterOp { Min, Max, Equals }

pub struct PlayerFilter {
    // Existing identity filters (teams, positions, age, draft, nationality, etc.)
    // stay typed — they hit indexed paths, not stat values.

    // NEW: generic stat filters — any number of (StatId, op, value) tuples
    pub stat_filters: Vec<StatFilter>,
}
```

The existing typed filters (`teams`, `positions`, `age_min`, etc.) stay typed — they're indexed and don't fit the stat catalog. The new field is purely additive.

### Schema additions

`SeasonStats` gains optional report-keyed substructs (Tier 1, typed):

```rust
pub struct SeasonStats {
    // existing fields unchanged …
    pub realtime: Option<RealtimeStats>,        // existing
    pub advanced: Option<AdvancedStats>,        // existing (MoneyPuck)
    pub goalie: Option<GoalieSeasonStats>,      // existing

    // NEW (Tier 1 bundled reports — typed):
    pub time_on_ice: Option<TimeOnIceStats>,
    pub goals_for_against: Option<GoalsForAgainstStats>,
    pub goalie_advanced: Option<GoalieAdvancedStats>,
    pub goalie_saves_by_strength: Option<GoalieSavesByStrengthStats>,
}
```

Each typed Tier-1 substruct is a flat field set matching the API row shape, with `Option<u32>` / `Option<f64>` for fields the league didn't track in old seasons. Every substruct's doc-comment states explicitly: "this is per-(player_id, season, season_type); `None` when the loader hasn't fetched the corresponding report for *this* window."

**Tier 2 reports do NOT live on `SeasonStats`** (resolved L-B1 from v0.1 review). They go into a separate cache owned by `StatsRepository`:

```rust
pub struct StatsRepository {
    // existing fields …
    extra_reports: HashMap<(PlayerId, Season, SeasonType, ReportKind), serde_json::Value>,
    extra_reports_lru: LruCacheState,  // separate LRU policy from the typed model
}
```

This isolates the typed model from arbitrary blob storage. `SeasonStats` `PartialEq`, snapshot determinism, LRU sizing, and the Hart.4.1 sum-equals invariants all remain stable. Tier-2 reads go through a separate accessor `repo.fetched_report(pid, season, season_type, kind) -> Option<&serde_json::Value>`.

**SeasonId fence on `extra_reports` writes**: every fetched-report write extracts `seasonId` from each row, asserts `== requested_season`, and errors `LoadError::SeasonIdMismatch` BEFORE the JSON lands in the map. Mirrors Hart.6.4 typed fence semantics for the untyped path.

---

## Sub-phases

| # | What | Surface | Est. LOC |
|---|---|---|---|
| **L.1** | Schema for Tier-1 reports + generic `fetch report` CLI + `ChunkedManifest` v2 + `repository_version=2` | icelines-fetch | 1000 |
| **L.2** | StatId catalog + StatCategory + PlayerView accessors + `ExtraReports` cache + goalie bios merge adapter | icelines-core | 800 |
| **L.3** | Query screen redesign — categorized sections, generic filter, StatId sort, search-as-you-type sort picker | icelines-cli (TUI + CLI) | 900 |
| **L.4** | Career table on player card with selectable columns + `[`/`]` keys + 80-col degradation | icelines-cli (TUI) | 500 |
| **L.5** | Propagate to Leaders / Comps / Depth / Export / Fantasy / **axum HTTP server** | icelines-cli + icelines-cli/server | 500 |
| **L.5b** | **Site stat-name sweep — atomic** (one PR, all team page headers via `StatId::label()`) | icelines-site | 200 |
| **L.6** | Tier-2 fetched-only reports — `serde_json::Value` via `ExtraReports`; CLI `fetch report --kind <name>` | icelines-cli + icelines-fetch | 300 |
| **L.7** | Bundle Tier 1 across all 38 historical seasons (data work) + 38-season parse-fence test | data/ + workflow | 0 (data) |
| **L.8** | Docs + integration cross-refs (`data-sources.md` becomes endpoint inventory) | design/specs + design/plans | 0 (docs) |

**Total: ~4,200 LOC + data work, ~8 days.** (was ~3,300 LOC / 7 days in v0.1; review surfaced ~900 LOC of additional infrastructure we'd otherwise hand-roll later).

### L.1 — Fetch CLI generic

```bash
icelines fetch report --kind realtime    --pos skater --season 20242025 --type playoff
icelines fetch report --kind timeonice   --pos skater --season 20242025
icelines fetch report --kind savesByStrength --pos goalie --season 20242025
icelines fetch report --kind shottype    --pos skater --season 20242025  # Tier 2 — JSON only
```

All 23 working endpoints reachable through this command. Tier 1 reports get typed deserialization; Tier 2 stores raw `serde_json::Value` for flexibility.

### L.2 — Catalog dispatch

`StatId::read(self, view: &PlayerView<'_>)` is the single source of truth for "what value goes with which stat." Every surface calls this; no surface looks at `view.goals` directly.

### L.3 — Query screen redesign

CLI:
```bash
icelines query leaders \
  --filter "hits-per-60>=2.0" \
  --filter "blocks>=50" \
  --filter "pp-toi>=180" \
  --sort pp-points-per-60
```

`--filter` repeats; each value is parsed as `<stat-key><op><value>` where `op` is `>=` / `<=` / `==`. Stat keys come from `StatId::short_label()`.

TUI Queries screen:

```
┌─ Stats Query ─────────────────────────────────┐
│ ▼ Scoring             G≥30  A≥40  P≥80         │
│ ▶ Special Teams                                │
│ ▼ Two-way             Hits≥150  Blocks≥75      │
│ ▶ Time on Ice                                  │
│ ▶ Possession                                   │
│                                                │
│ Sort: Pts/82 ▼      Top: 20                    │
│ ──────────────────────────────────────────────│
│ #1  C McDavid       EDM   80GP  2.10  142 ★    │
│ ...                                            │
└────────────────────────────────────────────────┘
```

Each section is a `StatCategory`; expanding shows its stats with `min`/`max` editors. Sort dropdown lists every `StatId` grouped by category.

### L.4 — Career table

Player card replaces 3 sparklines with a hockey-reference-style table:

```
═══════════════════════════════════════════════════════════════════
Connor McDavid · EDM · C · Age 28 · #97 · L
═══════════════════════════════════════════════════════════════════
Season   GP  G  A  P  +/- PIM  PPG PPP  Shots  S%   TOI  Hits Blk
2025-26  82  44 ..
2024-25  76  35 65 100 +22 8    10  35   280   12.5 20:32 22  55
2023-24  82  32 ...
```

Columns selected via `<` `>` keys; defaults to a curated set. Persisted in user config so the columns stay across launches.

### L.5 — Propagation

- **Leaders/CLI**: `SortMetric` becomes `StatId`. Pre-Lindsay flag-string parsing maps to StatId for back-compat.
- **Comps**: similarity computed in StatId-space across a configured subset.
- **Depth**: `ScoringMode::Custom(StatId)` adds the catalog as the depth-rank metric source.
- **Export**: `--columns "g,a,p,hits,blocks"` parses StatId list.
- **Fantasy scheme**: TOML coefficient keys become StatId names.

### L.6 — Tier 2 generic path

Tier 2 reports don't get typed structs. They land as `serde_json::Value` in `fetched_reports` keyed by `ReportKind`. Query screen extends this with a "raw report" view: pick a report kind, see the rows in a generic table with all fields.

### L.7 — Historical bundling

Mechanical: for each of 38 seasons, fetch the 5 new Tier-1 skater reports + 3 new Tier-1 goalie reports (already-bundled `summary`/`bios`/`goalie-summary`/`goalie-bios` skip). ~9 fetches × 38 seasons ≈ 340 API calls, ~10 minutes against the live API. Workflow `data-bundle.yml` extends to pack the new files.

### L.8 — Cross-reference docs

- Update `design/specs/query-engine.md` v0.2: replace the Tier 1/2/3 inline metric list with "see `StatId` catalog at `icelines-core::stats_catalog`".
- Update `design/specs/data-sources.md` to reference the 23 working endpoints + bundled tier list.
- New section in `design/specs/player-analysis.md`: "Career table column selection."
- Update `design/INVARIANTS.md` with new invariants (see below).

---

## New invariants

| ID | Domain | Statement |
|---|---|---|
| **DI-07** | Data | Every `StatId::read(view)` is total: returns `Some(value)` if the underlying data is present and `applies_to(view.position())` and `applies_to_era(view.season())`, else `None`. No accessor panics. |
| **DI-08** | Data | A `StatFilter` whose `StatId::applies_to(position)` returns false for the player's position is silently dropped at row-level iteration; CLI front-end **rejects** the same filter at parse time when position context is known. (Split per L-F1.) |
| **DI-09** | Data | Every Tier-1 substruct on `SeasonStats` is window-keyed `(season, season_type)` and is `None` when the loader hasn't fetched the corresponding report for *this* window. |
| **DI-10** | Data | `StatId::read(view)` is row-local: a pure function of `&PlayerView<'_>`. No repository, no global state, no league context. Future stats needing context get a separate `read_with_context(view, ctx)` API — never pollute `read()`. |
| **DI-11** | Data | `OnIceGoals` category stats are last-stint-only. `read()` returns `None` when `view.was_traded_in_window() == true`. Per-team semantics — summing across stints is wrong-data. |
| **DI-25** | Data | Every pre-Lindsay scheme TOML loads byte-identical to its post-Lindsay output via the legacy-key alias map. Verified by L1 fixture `tests/fixtures/legacy_schemes/`. |
| **AI-05** | Algorithm | `StatId::all()` and `StatCategory::members(c)` both return their values in a stable order — declaration order in the enum. Iteration is deterministic. |
| **AI-06** | Algorithm | Every catalog-driven sort is `(stat_value desc/asc, nhl_id asc)`. `nhl_id` is the universal tiebreak. `None` values sort last regardless of `higher_is_better`. Codified in `StatId::sort_cmp(view_a, view_b)` so every surface inherits the same tiebreak. |
| **AI-07** | Algorithm | Any `ReportKind` read from `extra_reports` by ≥2 surfaces (CLI command + TUI screen counts as two) MUST be promoted to a typed sub-struct in `SeasonStats` and a `StatCategory` in the catalog before that second consumer ships. |
| **AI-08** | Algorithm | Aggregations over `&[PlayerView]` that consume catalog reads MUST call `debug_assert_view_window_homogeneous` at the entry point. The catalog is not an escape hatch around the Hart.6.6 guard. |
| **II-04** | Interface | The CLI flag `--sort <stat-key>` accepts every `StatId::cli_key()` value. Unknown keys exit non-zero with the list of valid keys in stderr. |
| **II-05** | Interface | The CLI flag `--filter "<key><op><value>"` parses with `op ∈ {">=", "<=", "==", "="}`. Whitespace allowed around the op. NaN, infinity, locale-comma decimals are all parse errors. Unknown keys exit non-zero. |
| **II-06** | Interface | `--filter` and `--sort` accept identical grammars and StatId key sets across all 5 commands (`query leaders / player / compare / goalies` + `export md`). L2 test asserts `--help` output renders the same flag description string in every command. |
| **SI-03** | Site | Every site page that surfaces a stat name uses the value from `StatId::label()` — site templates never hard-code a stat name string. Enforced via grep-based CI test. Site rename happens in atomic sub-phase L.5b. |

---

## Test impact

| File | Change | Sub-phase |
|---|---|---|
| `icelines-core/src/stats_catalog.rs` | NEW — full catalog L0 tests (every StatId reads correctly from a fixture view) | L.2 |
| `icelines-core/src/filter.rs` | Add `apply_stat_filter` tests — Min/Max/Equals × every StatId category | L.2 |
| `icelines-fetch/src/nhl_api.rs` | Add fetch_report L0/L1 tests for each new endpoint URL shape | L.1 |
| `icelines-cli/tests/system_tests.rs` | New L2: `query leaders --filter ... --sort` round-trip per category | L.3 |
| `icelines-cli/src/tui/screens/queries.rs` test mod | Reorganize for categorized-section render assertions | L.3 |
| `icelines-cli/src/tui/screens/player.rs` test mod | New career-table render + column-selector L0 | L.4 |
| `icelines-fetch/src/bundled.rs` | Per-report parse + count tests for Tier-1 reports across all 5 bundled seasons | L.1, L.7 |
| `icelines-cli/src/commands/export.rs` | New `--columns` parse + emit tests | L.5 |

---

## Migration of existing CLI flags

The current `--sort <metric>` strings (~30 values like `pts-pace`, `ppg`, `xg`) all map cleanly to `StatId::short_label()`. **Backward compatibility**: every legacy flag string keeps working. The `SortMetric::parse` function gets replaced by a lookup against `StatId::all()` keyed by `short_label()`, plus an alias map for the legacy strings.

Existing typed filter flags (`--ppg-min`, `--gp-min`, `--toi-min`, `--plus-minus-min`, `--shots-pg-min`) keep working. The new `--filter` flag is additive.

---

## Risks

1. **Catalog growth scales with effort** — adding a stat is enum-case + match-arm everywhere `StatId` is matched. Mitigation: use exhaustive `#[non_exhaustive]` only on the public re-export boundary; internal matches stay non-exhaustive so the compiler enforces "added a stat → updated everywhere."

2. **Old seasons return null for many fields** — Hits/Blocks unavailable pre-2005, possession stats pre-2007. Mitigation: every `read()` returns `Option<f64>`; UI shows "—" not "0".

3. **`fetched_reports: HashMap<ReportKind, serde_json::Value>`** is loose. Without a typed schema, consumers do JSON walks. Mitigation: that's the deliberate design for Tier 2 — typed schemas for things we depend on, JSON blobs for the long tail. Promote to typed when a Tier-2 report becomes used in 2+ surfaces.

4. **Query screen redesign churn** — categorized-sections layout is a real UX change. Mitigation: preserve the existing flat-list path as a fallback if any user reports the new sections are worse.

5. **Performance: 38 seasons × 9 reports × 100s of players = ~10MB+ extra binary size.** Tier 1 IS bundled in-binary for the recent 5 seasons but not the historical 33 — those go through the install tarball pipeline. Mitigation: only the 5 in-binary seasons grow the binary; ~5 × 9 reports ≈ 5MB additional. Acceptable.

6. **Backward compat for query filter syntax** — the new `--filter "key>=value"` parser must coexist with legacy `--ppg-min 0.8` style. Mitigation: legacy flags get parsed first into typed filter slots; `--filter` only appends to the generic stat-filter list.

7. **Migration noise** — Phase Lindsay touches every CLI surface. Mitigation: small sub-phases (L.1–L.8) with green tests at every commit; revert any sub-phase independently.

---

## What's NOT in this spec

- **Strength-state drilling** (5v5/PP/PK separation in `query`). The `scoringRates` and `puckPossessions` reports give 5v5; PP/PK are already broken out in `powerplay` / `penaltykill`. Full strength-state would need shift-data joins — Phase 5C territory, separate effort.
- **Score-state filtering** (tied/leading/trailing). `summaryshooting` exposes some of this for SAT; full coverage needs play-by-play shift parsing — out of scope.
- **Per-game stats**. Same "needs play-by-play" issue. Lindsay covers per-season totals only.
- **Goalie seam fixes**. The half-broken goalie endpoints (realtime, savePercentage, etc.) are server-side issues. Skip them; document.

---

## Next step

This is v0.1. Pending:

1. Multi-role review (HART, KEEL, TAPE, FORGE, EDGE, BENCH, WIRE, GLASS, PACE, SCOUT — at least HART/KEEL/FORGE/EDGE/BENCH/WIRE because the surface area is large)
2. Punch-list applied → v0.2
3. INDEX.md entry + cross-reference into Hart spec section "Sub-phases (review-revised)" pointing forward to Lindsay
4. Implementation kickoff at L.1
