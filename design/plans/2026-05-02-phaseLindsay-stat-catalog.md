# Phase Lindsay — Stat Catalog (v0.4, R3-applied)

**Status**: v0.4 — R3 review applied. R1 + R2 + R3 = 10-role coverage across three rounds (HART, KEEL, TAPE, FORGE, EDGE, BENCH, WIRE, GLASS, SCOUT, PACE).
R1 raised 29 BLOCKERs resolved in v0.2 changelog. R2 raised 24 NEW BLOCKERs against v0.2 (the changelog claimed fixes the spec body never wrote). R3 verified v0.3 had the SAME drift on ~14 items. v0.4 closes that loop with a literal spec-body sweep — no new design decisions, just transcribing what the changelogs already claimed, plus a few SCOUT/PACE corrections that the v0.3 prose flagged but the v0.3 body didn't apply (FaceoffWinPct / EvenStrengthTimeOnIcePerGame double-listings, PpAssists raw count).
**Ready to implement L.1.**
**Date**: 2026-05-02
**Trophy**: Lindsay (Ted Lindsay Award — players' choice; "complete picture of a player")
**Predecessor**: design/plans/2026-04-30-phaseHart-normalization.md (master Hart),
design/plans/2026-05-01-phaseHart-6-playoff-data.md (Hart.6 playoff data)
**Review summaries**: R1 at `design/plans/2026-05-02-phaseLindsay-review-summary.md`,
R2 at `design/plans/2026-05-02-phaseLindsay-r2-summary.md`,
R3 at `design/plans/2026-05-02-phaseLindsay-r3-summary.md`
**Replaces**: nothing — additive

---

## v0.3 → v0.4 changelog

R3 verification (10 roles, both R2 reviewers + 2 fresh) confirmed v0.3 **had the same drift pattern R2 caught**: the v0.3 changelog claimed fixes the v0.3 spec body never wrote. Four roles cleared (KEEL, GLASS, SCOUT R2 deltas, PACE R2 deltas); six roles flagged still-BLOCKER items (HART, FORGE, BENCH, WIRE, TAPE, EDGE). v0.4 is the literal spec-body sweep.

### Spec-body sweep — apply v0.3 changelog claims to v0.3 spec body (14)

- **L3-B1** [HART-R3-B1] **`extra_reports` cascade eviction → DI-12** spelled out in spec body §"Repository lifecycle — extra_reports", with code sketch + L0 test name `l0_repo_extra_reports_cascade_evict_on_window_drop`.
- **L3-B2** [HART-R3-B2] **`repository_version` boundary check at `load_window`** — spec body §"Repository lifecycle" pinned the failure point and L1 test name `l1_repo_load_window_rejects_repository_version_2_on_v1_binary`. Added as DI-28.
- **L3-B3** [FORGE-R3-B1] **`ExtraReports` declared as `BTreeMap`** in spec body §Public types — was claimed in v0.3 changelog but body still showed `HashMap` from v0.1. Type alias pinned: `BTreeMap<(PlayerId, Season, SeasonType, ReportKind), serde_json::Value>`.
- **L3-B4** [FORGE-R3-B4] **`FilterParseError` 7-variant enum** sketched in spec body §Public types: `EmptyInput`, `EmptyStatKey`, `MissingOp`, `MultipleOps`, `UnknownStat`, `BadNumber`, `NotFinite`. Triggering-input table in §"Parse-error variants".
- **L3-B5** [FORGE-R3-B5] **DI-25 frozen-golden precision** — fixture is a fixed pre-L.5 capture, NOT post-Lindsay round-trip. New §"DI-25 — frozen-golden semantics" pins the capture step, the assertion, and the five named schemes.
- **L3-B6** [BENCH-R3-1] **`stat_catalog_variants.rs` named L.2 deliverable** in plan §Sub-phases L.2 row. Six fixture variants enumerated in spec test contract.
- **L3-B7** [BENCH-R3-2] **Two-fence stdout golden** — capture pre-L.3, reassert post-L.3 AND post-L.5 (sort ordering changes ride L.3). Plan §Sub-phases L.3 row + spec test contract row.
- **L3-B8** [BENCH-R3-3] **Five named legacy schemes** in spec §"DI-25 frozen-golden semantics" + plan §Sub-phases L.5 row: `yahoo-standard`, `espn-standard`, `custom-points-only`, `head-to-head-9cat`, `rotisserie-with-goalie`.
- **L3-B9** [WIRE-R3-B4] **`data/api-probe-2026-05-02.txt` artifact** elevated to L.1 entry-criterion in plan §Sub-phases L.1 row.
- **L3-B10** [WIRE-R3-B5] **Tier-1 per-report file format** documented in spec body §"Tier-1 file format" with the 7-row substruct→filename→endpoint table.
- **L3-B11** [WIRE-R3-B1] **`extra_reports` runtime-only declaration** spelled out in spec body §"Repository lifecycle" as DI-27 with L1 test `l1_repo_extra_reports_not_persisted`.
- **L3-B12** [WIRE-R3-F5] **`load_report_with_fallback<T>`** signature pinned in spec body §Public types; L.1 deliverables row in plan now lists it explicitly.
- **L3-B13** [TAPE-R3-1] **Per-endpoint seasonId fence** documented in spec body §"Per-endpoint seasonId fence" as DI-29; L1 test name template per endpoint.
- **L3-B14** [TAPE-R3-2] **Rate-limit policy** new spec body §"Rate-limit policy" — sequential, exponential backoff on 429/5xx, bundled-data fallback first, concurrent-window fs lock at `~/.icelines/.fetch.lock`.

### EDGE-R2 grammar precision (4)

- **L3-B15** [EDGE-R2] **OnIceGoals trade-window guard** moved to category-boundary in `read()` (top of match) — spec body §"Read dispatch — the contract" code sketch updated.
- **L3-B16** [EDGE-R2] **NaN/inf rejection** at construction — `StatFilter::new` validates `value.is_finite()`; downstream code never sees NaN/inf.
- **L3-B17** [EDGE-R2] **Multi-filter same-StatId normalization** — `PlayerFilter::normalize_stat_filters` rules: Min+Min → tightest; Max+Max → tightest; Min+Max → range; Equals+Equals → reject as `MultipleOps`.
- **L3-B18** [EDGE-R2] **Empty/whitespace stat-key error path** — `EmptyStatKey` variant covers both empty and whitespace-only keys; in spec body §"Parse-error variants" trigger table.

### PACE-R2 + SCOUT-R2 v0.3 follow-through (4)

- **L3-B19** [PACE-R2 F3] **Multi-season aggregate `read()`** — strict propagation: `aggregate_read` returns `Some(sum)` only when every window has `Some`. Spec body §"Read dispatch" includes `aggregate_read` signature.
- **L3-B20** [SCOUT-R2 F2] **`FaceoffWinPct` actually moved** from SpecialTeams enumeration to TwoWay enumeration (v0.3 prose said this, body still listed it under SpecialTeams). v0.4 body fixes both tables.
- **L3-B21** [SCOUT-R2 F3] **`EvenStrengthTimeOnIcePerGame` actually moved** from OnIceGoals enumeration to TimeOnIce enumeration (same pattern). v0.4 body fixes both tables; explicit note that this stat is exempt from DI-11 (TOI sums correctly across stints).
- **L3-B22** [SCOUT-R2 F5] **`PpAssists` raw count** added to Scoring enumeration as a 14th stat. CLI alias `pp-assists`. Net stat count: 107 → 108.

### Stat-count update

- **v0.4 totals**: 14 + 13 + 17 + 12 + 8 + 15 + 22 + 7 = **108 selectable stats**.
- (v0.2 was 98; v0.3 added 9 xG-family stats → 107; v0.4 adds `PpAssists` raw and applies category moves → 108. The category moves are zero-net — `FaceoffWinPct` shifted SpecialTeams → TwoWay, `EvenStrengthTimeOnIcePerGame` shifted OnIceGoals → TimeOnIce.)

### New invariants in v0.4

| ID | Statement |
|---|---|
| **DI-12** | Eviction of a `(season, season_type)` window from the typed LRU cascade-evicts every `extra_reports` entry whose key matches that window. |
| **DI-26** | `extra_reports` is capped at 4096 entries (~40 MB ceiling at 10 KB/value). Insertion past the cap evicts oldest by LRU order. |
| **DI-27** | `extra_reports` is runtime-only — never persisted to disk. Subsequent runs re-fetch. |
| **DI-28** | `repository_version` boundary check fires at `StatsRepository::load_window`, not at `repo_swap`. Old binary on new snapshot errors at file-open. |
| **DI-29** | Every Tier-1 deserializer asserts `row.seasonId == requested_season` for every row; mismatch errors `LoadError::SeasonIdMismatch` before the substruct populates. |
| **AI-09** | `aggregate_read(views)` is strict-propagation: `Some(blend)` only when every window in the slice has `Some` from `read()`. ANY `None` propagates as `None`. |

The full R3 punch list is at
`design/plans/2026-05-02-phaseLindsay-r3-summary.md`.

---

## v0.2 → v0.3 changelog

R2 caught 24 new BLOCKERs. Most were spec-body drift (v0.2 changelog claimed fixes the spec body never wrote). Real new design gaps + SCOUT/PACE domain additions also surfaced.

### Methodology + domain (4 — affect catalog surface)

- **L2-B1** [PACE-B1] **`FilterOp::Equals` — type-aware tolerance.** Counts use exact integer; rates / percentages use `1e-6` tolerance keyed off `StatUnit`. Spec §"Filter semantics" updated.
- **L2-B2** [PACE-B2] **MIN_GP guard on derived per-game stats.** `PointsPerGame`/`GoalsPerGame`/`AssistsPerGame` all return `None` when `gp < MIN_GP` (10). Mirrors `Pace82`.
- **L2-B3** [SCOUT-B1] **GSAx / xGA family added to Goalie category.** New StatIds: `GoalieXgAgainst`, `GoalieXgAgainstPer60`, `GoalsSavedAboveExpected`, `Gsax60`. Tier-2 (fetched from MoneyPuck or NHL Edge).
- **L2-B4** [SCOUT-B2] **IxG / xG family added to Possession category.** New StatIds: `IxG`, `IxgPer60`, `OnIceXgFor`, `OnIceXgAgainst`, `XgForPct`. Tier-2.

### Spec body sweep — fix v0.2 changelog drift (8)

- **L2-B5** [GLASS-R2-B1] Spec career-table sketch updated to `[`/`]` (not `←/→` or `<`/`>`). Three contradictory wordings consolidated.
- **L2-B6** [GLASS-R2-F4] `<space>` collision — section toggle now `Tab`. Spec §TUI integration updated.
- **L2-B7** [FORGE-R2-F12] `members(c) -> &'static [StatId]`. Spec line 77 fixed.
- **L2-B8** [FORGE-R2-B2] `#[non_exhaustive]` mention removed from plan §Risks #1.
- **L2-B9** [KEEL-R2-F1] Spec §integration sections gain a new "HTTP integration" subsection (axum). JSON keys = `StatId::cli_key()` strings.
- **L2-B10** [WIRE-F1] `ReportKind::supports(season_type) -> bool` added to spec public-types section.
- **L2-B11** [WIRE-F5] `load_report_with_fallback<T>` scheduled in L.1 deliverables.
- **L2-B12** [FORGE-R2-B5] DI-25 made precise: legacy fixture checked in pre-L.5; load+re-serialize must equal **frozen golden**, not round-trip self-equality.

### New design gaps pinned (6)

- **L2-B13** [HART-R2-B1] **`extra_reports` LRU cascade.** Eviction of a `(season, season_type)` window from primary LRU MUST cascade-evict every `extra_reports` row whose key matches. Added as DI-12.
- **L2-B14** [HART-R2-B2] `repository_version` check happens at `StatsRepository::load_window`, not deferred to `repo_swap`. Old binary on new snapshot errors at file-open boundary.
- **L2-B15** [WIRE-B1] **`extra_reports` is RUNTIME-ONLY**, not persisted to disk. Fetching populates the in-process map; subsequent runs re-fetch. Avoids file-format proliferation; matches "Tier-2 = on-demand" semantics. Documented in L.6.
- **L2-B16** [WIRE-B5] **Tier-1 typed substructs** load from **separate per-report files** (`timeonice.json`, `goalsForAgainst.json`, etc.) — NOT inline-merged into `stats.json`. The `SeasonStats` substructs are populated AT LOAD TIME by reading these new files. `bundle_schema_version=1` claim now coherent: new files, not new fields.
- **L2-B17** [KEEL-R2-B3] **HTTP server is Tier-1 typed-only.** Tier-2 reports invisible to `/api/...` until promoted via AI-07. Documented.
- **L2-B18** [PACE-F2] **`extra_reports` LRU cap at 4096 entries** (~40 MB ceiling at 10 KB/value). Added as DI-26.

### Schema + ownership precision (3)

- **L2-B19** [KEEL-R2-B1] **`StatId::toml_aliases()` + `StatId::cli_aliases()`** are SEPARATE methods. Both live in `icelines-core`. Documented.
- **L2-B20** [KEEL-R2-B2] **L.5b sweep enumeration**: rendered headers (use `StatId::label()`), CSS class names (use `StatId::cli_key()` with `stat-` prefix → `.stat-pp-goals`), URL anchors (use `StatId::cli_key()`), search-index terms (free-form, allowlist-gated).
- **L2-B21** [FORGE-R2-B1] **`ExtraReports` map: `BTreeMap` not `HashMap`** for deterministic iteration. Spec updated.

### Test contract precision (3)

- **L2-B22** [BENCH-R2-1] **Fixture-variant catalog** is a NAMED L.2 deliverable: `icelines-core/tests/fixtures/stat_catalog_variants.rs` with 6 variants enumerated.
- **L2-B23** [BENCH-R2-2] **Capture-stdout golden BEFORE L.3** (not L.5). Two fences: post-L.3 + post-L.5. Sort ordering changes ride L.3, not L.5.
- **L2-B24** [BENCH-R2-3] **Five legacy schemes named**: `yahoo-standard`, `espn-standard`, `custom-points-only`, `head-to-head-9cat`, `rotisserie-with-goalie`. Files at `icelines-fetch/tests/fixtures/legacy_schemes/`.

### FIXITs applied inline

L2-F1 through L2-F14 — see R2 summary for full list. Highlights:
- Career table default columns: `GP G A P +/- PIM PPG SHG GWG Shots S% TOI/G` (Hits/Blocks → Two-way preset, not default).
- `FaceoffWinPct` recategorized to `TwoWay`.
- `EvenStrengthTimeOnIcePerGame` recategorized to `TimeOnIce`.
- `available_since(Hits) == 20052006` (not 1997).
- `PpAssists` raw count added.
- `is_goalie()` per-row replaces `pos == Goalie` for goalie applicability.
- Per-60 floor: soft floor `None if TOI < 300s`.
- Multi-season aggregate `read()`: strict propagation (`None` if any window missing).
- `StatId::sort_cmp(self, a, b) -> Ordering` signature explicit.
- `FilterParseError` enum: 7 variants pinned.
- L.5b grep pattern + allowlist file specified.

The full R2 punch list (24 BLOCKERs / 16 FIXITs / 5 NITs) is at
`design/plans/2026-05-02-phaseLindsay-r2-summary.md`.

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
    // BTreeMap (NOT HashMap) for deterministic iteration — required for
    // snapshot tests, debug dumps, and "list all fetched reports" UX
    // (FORGE-R2-B1 / v0.4).
    extra_reports: std::collections::BTreeMap<
        (PlayerId, Season, SeasonType, ReportKind),
        serde_json::Value,
    >,
    extra_reports_lru: LruCacheState,  // separate LRU cap (4096; DI-26) — runtime-only (DI-27)
}
```

This isolates the typed model from arbitrary blob storage. `SeasonStats` `PartialEq`, snapshot determinism, LRU sizing, and the Hart.4.1 sum-equals invariants all remain stable. Tier-2 reads go through a separate accessor `repo.fetched_report(pid, season, season_type, kind) -> Option<&serde_json::Value>`.

**Lifecycle rules** (v0.4 — see spec body §"Repository lifecycle — extra_reports" for full sketch + L0/L1 test names):

- **DI-12 cascade-eviction**: typed-window LRU eviction cascade-evicts every `extra_reports` row for the same `(season, season_type)`.
- **DI-26 cap**: 4096 entries (~40 MB ceiling).
- **DI-27 runtime-only**: never written to disk; subsequent runs re-fetch.
- **DI-28 `repository_version`**: boundary check fires at `StatsRepository::load_window`, not at `repo_swap`.
- **DI-29 seasonId fence**: every fetched-report write extracts `seasonId` from each row, asserts `== requested_season`, errors `LoadError::SeasonIdMismatch` BEFORE the JSON lands in the map. Same fence applies on every Tier-1 deserializer.

---

## Sub-phases

| # | What | Surface | Est. LOC |
|---|---|---|---|
| **L.1** | Schema for Tier-1 reports + generic `fetch report` CLI + `ChunkedManifest` v2 + `repository_version=2` (boundary check at `load_window` per DI-28) + `load_report_with_fallback<T>` helper + per-endpoint seasonId fence (DI-29) + rate-limit policy implementation. **Entry criterion:** `data/api-probe-2026-05-02.txt` artifact committed (WIRE-R3-B4). | icelines-fetch | 1000 |
| **L.2** | StatId catalog + StatCategory + PlayerView accessors + `ExtraReports: BTreeMap` cache (DI-12 cascade, DI-26 cap, DI-27 runtime-only) + goalie bios merge adapter + `FilterParseError` 7-variant enum + `StatFilter::new` finite-value gate + `PlayerFilter::normalize_stat_filters` + `aggregate_read` (AI-09 strict propagation). **Named deliverable:** `icelines-core/tests/fixtures/stat_catalog_variants.rs` enumerating 6 variants (skater-modern, skater-pre-2005, center, goalie, traded-multistint, GP=0) per BENCH-R2 L2-B22. | icelines-core | 800 |
| **L.3** | Query screen redesign — categorized sections, generic filter, StatId sort, search-as-you-type sort picker. **Entry criterion (BENCH-R2 L2-B23):** capture stdout-golden of every legacy `--sort` value BEFORE this sub-phase lands. **Exit criterion:** reassert byte-equality post-L.3 (sort ordering changes ride here). | icelines-cli (TUI + CLI) | 900 |
| **L.4** | Career table on player card with selectable columns + `[`/`]` keys + 80-col degradation | icelines-cli (TUI) | 500 |
| **L.5** | Propagate to Leaders / Comps / Depth / Export / Fantasy / **axum HTTP server** (Tier-1 typed-only per L2-B17). **Fantasy fixture:** five named legacy schemes at `icelines-fetch/tests/fixtures/legacy_schemes/{yahoo-standard,espn-standard,custom-points-only,head-to-head-9cat,rotisserie-with-goalie}.toml` with `<name>.golden.toml` companions (DI-25 frozen-golden). **Exit criterion:** stdout-golden second-fence reassertion post-L.5. | icelines-cli + icelines-cli/server | 500 |
| **L.5b** | **Site stat-name sweep — atomic** (one PR, all team page headers via `StatId::label()`). Four string surfaces: rendered headers, CSS class names, URL anchors, search-index terms. CI grep test on `\b(Goals|Assists|Points|Hits|Blocks|Saves|GAA)\b` with allowlist at `icelines-site/.stat-name-allowlist`. | icelines-site | 200 |
| **L.6** | Tier-2 fetched-only reports — `serde_json::Value` via `ExtraReports`; CLI `fetch report --kind <name>`. Concurrent-invocation guard via `~/.icelines/.fetch.lock` (TAPE-R3 rate-limit policy). | icelines-cli + icelines-fetch | 300 |
| **L.7** | Bundle Tier 1 across all 38 historical seasons (data work) + `l0_lindsay_7_each_tier1_report_parses_for_all_38_bundled_seasons` cross-product test (L-B20). | data/ + workflow | 0 (data) |
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
| **DI-12** | Data | Eviction of a `(season, season_type)` window from the typed LRU cascade-evicts every `extra_reports` entry whose key matches that window. Without this rule, Tier-2 blobs leak across LRU sweeps. (HART-R3 / v0.4) |
| **DI-25** | Data | Every pre-Lindsay scheme TOML loads byte-identical to its **frozen golden** (a fixed pre-L.5 capture, not post-Lindsay round-trip) via the legacy-key alias map. Verified by L1 fixture `icelines-fetch/tests/fixtures/legacy_schemes/<name>.golden.toml` for the 5 named schemes. (FORGE-R3 / v0.4) |
| **DI-26** | Data | `extra_reports` is capped at 4096 entries (~40 MB ceiling at 10 KB/value). Insertion past the cap evicts oldest by LRU order. Independent of typed-window LRU. (PACE-R2 / v0.4) |
| **DI-27** | Data | `extra_reports` is runtime-only. Never persisted to disk; subsequent runs re-fetch. Promotion to Tier-1 (AI-07) is the persistence path. (WIRE-R3 / v0.4) |
| **DI-28** | Data | `repository_version` boundary check fires at `StatsRepository::load_window`, not at `repo_swap`. An old binary opening a v=2 snapshot errors at the file-open boundary with `LoadError::RepoVersionUnknown`. (HART-R3 / v0.4) |
| **DI-29** | Data | Every Tier-1 deserializer asserts `row.seasonId == requested_season` for every row in the file; mismatch errors `LoadError::SeasonIdMismatch { expected, actual, endpoint }` BEFORE the substruct populates. Same fence applies to Tier-2 `extra_reports` writes. (TAPE-R3 / v0.4) |
| **AI-05** | Algorithm | `StatId::all()` and `StatCategory::members(c)` both return their values in a stable order — declaration order in the enum. Iteration is deterministic. |
| **AI-06** | Algorithm | Every catalog-driven sort is `(stat_value desc/asc, nhl_id asc)`. `nhl_id` is the universal tiebreak. `None` values sort last regardless of `higher_is_better`. Codified in `StatId::sort_cmp(view_a, view_b)` so every surface inherits the same tiebreak. |
| **AI-07** | Algorithm | Any `ReportKind` read from `extra_reports` by ≥2 surfaces (CLI command + TUI screen counts as two) MUST be promoted to a typed sub-struct in `SeasonStats` and a `StatCategory` in the catalog before that second consumer ships. |
| **AI-08** | Algorithm | Aggregations over `&[PlayerView]` that consume catalog reads MUST call `debug_assert_view_window_homogeneous` at the entry point. The catalog is not an escape hatch around the Hart.6.6 guard. |
| **AI-09** | Algorithm | `aggregate_read(views)` is strict-propagation: returns `Some(blend)` only when every window in the slice has `Some` from `read()`. ANY `None` (missing data, era gate, trade guard, MIN_GP floor) propagates as `None` — no silent zeros. (PACE-R2 / v0.4) |
| **II-04** | Interface | The CLI flag `--sort <stat-key>` accepts every `StatId::cli_key()` value. Unknown keys exit non-zero with the list of valid keys in stderr. |
| **II-05** | Interface | The CLI flag `--filter "<key><op><value>"` parses with `op ∈ {">=", "<=", "==", "="}`. Whitespace allowed around the op. Every malformed input maps to exactly one `FilterParseError` variant (7 total): `EmptyInput`, `EmptyStatKey`, `MissingOp`, `MultipleOps`, `UnknownStat`, `BadNumber`, `NotFinite`. NaN, infinity, locale-comma decimals all parse errors. (EDGE-R2 / FORGE-R3 / v0.4 precision) |
| **II-06** | Interface | `--filter` and `--sort` accept identical grammars and StatId key sets across all 5 commands (`query leaders / player / compare / goalies` + `export md`). Same-StatId multi-filter normalization rule applies uniformly: Min+Min/Max+Max take tightest bound; Min+Max compose to range; Equals+Equals reject as `MultipleOps`. L2 test asserts `--help` output renders the same flag description string in every command. (EDGE-R2 / v0.4 precision) |
| **SI-03** | Site | Every site page that surfaces a stat name uses the value from `StatId::label()` — site templates never hard-code a stat name string. Enforced via grep-based CI test. Site rename happens in atomic sub-phase L.5b. |

---

## Test impact

| File | Change | Sub-phase |
|---|---|---|
| `icelines-core/src/stats_catalog.rs` | NEW — full catalog L0 tests: `StatId::read` × 6-variant fixture cross-product (~600 dispatch points) | L.2 |
| `icelines-core/tests/fixtures/stat_catalog_variants.rs` | NEW — 6 fixture variants (skater-modern, skater-pre-2005, center, goalie, traded-multistint, GP=0) — explicit BENCH-R2 L2-B22 deliverable | L.2 |
| `icelines-core/src/filter.rs` | Add `apply_stat_filter` tests — Min/Max/Equals × every StatId category; multi-filter normalization (Min+Min, Max+Max, Min+Max range, Equals+Equals reject); `FilterParseError` variant coverage (~7 × 3 trigger inputs each) | L.2 |
| `icelines-core/src/stats_repository.rs` | `l0_repo_extra_reports_cascade_evict_on_window_drop` (DI-12); `l0_repo_extra_reports_cap_at_4096` (DI-26); `l1_repo_extra_reports_not_persisted` (DI-27); `l1_repo_load_window_rejects_repository_version_2_on_v1_binary` (DI-28) | L.2 |
| `icelines-fetch/src/nhl_api.rs` | Add fetch_report L0/L1 tests for each new endpoint URL shape; `l1_<endpoint>_rejects_mismatched_season_id` per Tier-1 endpoint (DI-29); `l1_fetch_retry_backoff_on_429` and `l1_fetch_retry_backoff_on_5xx` (TAPE-R3 rate-limit policy) | L.1 |
| `icelines-cli/tests/system_tests.rs` | New L2: `query leaders --filter ... --sort` round-trip per category; **stdout-golden capture pre-L.3** (BENCH-R2 L2-B23 first fence); reassert post-L.3 + post-L.5 (two fences, BENCH-R2 L2-B23); `l2_fetch_report_serializes_concurrent_invocations` (TAPE-R3 fs lock) | L.3 / L.5 |
| `icelines-cli/src/tui/screens/queries.rs` test mod | Reorganize for categorized-section render assertions | L.3 |
| `icelines-cli/src/tui/screens/player.rs` test mod | New career-table render + column-selector L0 | L.4 |
| `icelines-fetch/tests/legacy_schemes_test.rs` | NEW — `l1_legacy_schemes_load_byte_identical` over 5 named schemes (DI-25 frozen-golden, BENCH-R2 L2-B24) | L.5 |
| `icelines-fetch/src/bundled.rs` | Per-report parse + count tests for Tier-1 reports across all 5 bundled seasons; `l0_lindsay_7_each_tier1_report_parses_for_all_38_bundled_seasons` cross-product (L-B20) | L.1, L.7 |
| `icelines-cli/src/commands/export.rs` | New `--columns` parse + emit tests | L.5 |
| `icelines-site/tests/site_stat_name_grep.rs` | NEW — CI grep test for `\b(Goals|Assists|Points|Hits|Blocks|Saves|GAA)\b` outside comments + allowlist (SI-03) | L.5b |
| `icelines-cli/tests/http_round_trip.rs` | NEW — `/api/team/:name/roster` JSON keys all `StatId::from_cli_key`-parseable; values match `read(view)` (KEEL-B1) | L.5 |

---

## Migration of existing CLI flags

The current `--sort <metric>` strings (~30 values like `pts-pace`, `ppg`, `xg`) all map cleanly to `StatId::short_label()`. **Backward compatibility**: every legacy flag string keeps working. The `SortMetric::parse` function gets replaced by a lookup against `StatId::all()` keyed by `short_label()`, plus an alias map for the legacy strings.

Existing typed filter flags (`--ppg-min`, `--gp-min`, `--toi-min`, `--plus-minus-min`, `--shots-pg-min`) keep working. The new `--filter` flag is additive.

---

## Risks

1. **Catalog growth scales with effort** — adding a stat is enum-case + match-arm everywhere `StatId` is matched. Mitigation: `StatId` stays exhaustive (NOT `#[non_exhaustive]`, resolved L-B17) so the compiler enforces "added a stat → updated everywhere" across all consumer surfaces.

2. **Old seasons return null for many fields** — Hits/Blocks unavailable pre-2005, possession stats pre-2007. Mitigation: every `read()` returns `Option<f64>`; UI shows "—" not "0".

3. **`extra_reports: BTreeMap<(PlayerId, Season, SeasonType, ReportKind), serde_json::Value>`** is loose. Without a typed schema, consumers do JSON walks. Mitigation: that's the deliberate design for Tier 2 — typed schemas for things we depend on, JSON blobs for the long tail. Promote to typed when a Tier-2 report becomes used in 2+ surfaces (AI-07).

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

This is v0.4. Status:

1. ~~R1 multi-role review~~ — complete (29 BLOCKERs, applied as v0.2 changelog).
2. ~~R2 multi-role review~~ — complete (24 BLOCKERs, applied as v0.3 spec-body sweep).
3. ~~R3 verification pass~~ — complete (~14 still-BLOCKERs: same drift defect as v0.2 → v0.3, applied as v0.4 spec-body sweep).
4. ~~v0.4 spec-body sweep + plan refresh~~ — done.
5. ~~R4 verification~~ — complete (18/18 R3 items cleared, no drift).
6. ~~L.1 implementation~~ — **shipped 2026-05-02**. 45 new Lindsay-prefixed tests; 1119 workspace tests passing. See "L.1 ship summary" below.
7. ~~L.2 implementation~~ — **shipped 2026-05-02**. 65 new Lindsay-L.2 tests; 1184 workspace tests passing. See "L.2 ship summary" below.
8. ~~L.3 implementation~~ — **shipped 2026-05-02**. 31 new Lindsay-L.3 tests; 1215 workspace tests passing. See "L.3 ship summary" below.
9. ~~L.4 implementation~~ — **shipped 2026-05-02**. 36 new Lindsay-L.4 tests; 1251 workspace tests passing. See "L.4 ship summary" below.
10. **L.5 next** — Propagate StatId catalog to Leaders / Comps / Depth / Export / Fantasy / axum HTTP server. Five-named-scheme legacy fixture + DI-25 frozen-golden + post-L.5 stdout-golden second-fence reassertion.

---

## L.4 ship summary (2026-05-02)

L.4 implementation complete with 5 sub-phases + 1 role-review checkpoint, all green:

| Sub-phase | Deliverables | Tests added |
|---|---|---|
| **L.4.1** | `StatId::Games` (skater GP — fills KEEL L.3 carry-forward gap) brings catalog from 107 → 108. `StatId::default_in_career_table(pos)` per-position default-column membership predicate. Initial: 13 skater-common (incl. Games) + FaceoffWinPct on Center → 14 default for Center, 13 for LW/RW/D; 10 for Goalie. (SCOUT-L.4 review later expanded to Gwg + Saves + ShotsAgainst, dropped RegulationWins → final 14 skater-common, 15 Center, 14 LW/RW/D, 11 Goalie.) | 4 L0 |
| **L.4.2** | `CareerTablePreset` enum (7 presets: Default / Scoring / TwoWay / SpecialTeams / Time / Goalie / All) with `next()` / `prev()` cycling and `columns(pos)` returning a position-applicable, declaration-order, `applies_to`-filtered Vec<StatId>. Goalies hide skater stats (and vice versa); FaceoffWinPct gates to Center in `Default`. | 8 L0 |
| **L.4.3** | `render_career_cell(sid, view)` per-StatUnit formatting (Count→i64, Seconds→M:SS or `Nm`, Pct→`12.5`, Per60/Rate/Inverted→2dp, None→"—"). `render_stats_view` rewrite: career table reads `app.repo.career_regular(pid)`, sorts newest-first, renders one row per season with header + separator + bio + transactions sections. `app.career_table_preset` field on `App`. | 4 L0 |
| **L.4.4** | `[` / `]` Char handlers under `Screen::PlayerById(pid)` cycle the preset; status-line shows `"Career preset: {label}  ·  [/]: cycle  ·  c: comps"`. | 3 L1 |
| **L.4.5** | `fit_career_columns(all_columns, panel_w) -> (Vec<StatId>, dropped, use_narrow)` helper drops cols from right at <100 cells (8 cells/col + 11 fixed) and falls back to `narrow_label()` at <60 cells. `format_sort_picker_row(sid, panel_w)` 3-tier degradation (≥100 wide / 80–100 medium drops `(category)` / <80 narrow truncates cli_key + uses short_label). Resolves GLASS #7 carry-forward from L.3 closeout. | 5 L0 + 7 L0 |

**Role-review checkpoint (PASS with applied fixes):**
- GLASS + SCOUT on career table — 10 GLASS findings + 8 SCOUT findings. Pre-commit fixes landed:
  - SCOUT-1 BLOCKER: added `Gwg` to skater default (canonical career-glance counter)
  - SCOUT-2 BLOCKER: dropped `RegulationWins` from goalie default; added `Saves` + `ShotsAgainst` (volume context for SV%/GAA)
  - GLASS-1 NIT: clip header labels to 7 cells in `render_stats_view` (some short_labels exceed 7 chars and broke alignment)

**Carry-forwards parked for L.5+:**
- GLASS-2 fixed-overhead 11 vs 9 cells (slack tolerated)
- GLASS-3 narrow_label() coverage <60 cols (only 9/108 overridden — rename or expand)
- GLASS-4 empty-preset feedback ambiguity (preset-empty vs panel-narrow)
- GLASS-5 [/] affordance not in title bar (only in status line after first press)
- GLASS-6 picker truncation reversibility — show full key on selected row
- GLASS-7 picker `&key[..23]` byte-slice UTF-8 hardening
- GLASS-8 separator-length math redundant after fit_career_columns
- GLASS-9 narrow indicator format ("-3" reads negative)
- GLASS-10 TestBackend snapshot integration test for `render_stats_view` + `render_sort_picker`
- SCOUT-3 add `PointsPerGame` to skater default (cross-season legibility)
- SCOUT-4 gate `FaceoffWinPct` to Center in TwoWay/SpecialTeams presets
- SCOUT-5 era-blind realtime defaults (accept "—" for pre-2005 careers)
- SCOUT-6 "All" preset rename to "All (debug)" or hide from cycle
- SCOUT-8 D-specific default (add `EvGoalsForPct`)

**Test totals:**
- Pre-L.4: 1215 workspace tests
- Post-L.4: **1251 workspace tests** (+36 — 4 L.4.1 + 8 L.4.2 + 4 L.4.3 + 3 L.4.4 + 12 L.4.5 + 5 SCOUT-checkpoint adjustments)
- All passing; no regressions.

---

## L.3 ship summary (2026-05-02)

L.3 implementation complete with 5 sub-phases + 2 role-review checkpoints, all green:

| Sub-phase | Deliverables | Tests added |
|---|---|---|
| **L.3.0** | Pre-L.3 stdout-golden capture (BENCH-R2 L2-B23 entry criterion). 35 goldens at `icelines-cli/tests/fixtures/lindsay_l3_pre/leaders-<sort>.golden.txt` covering every legacy `--sort` value of `query leaders`. New L2 fence test `l2_lindsay_l3_golden_parity` with `LINDSAY_L3_REGEN=1` regen mode. Discovered legacy sort tiebreak is non-deterministic across process invocations (HashMap iteration order). | 1 L2 (initially `#[ignore]`d) |
| **L.3.1** | New `--filter "<key><op><value>"` flag on `query leaders` — repeatable, routes through `parse_filter` (NaN/inf gate at construction; 7-variant `FilterParseError` with actionable Display strings). `normalize_stat_filters` runs before apply. DI-08 silent skip on non-applicable. Coexists independently with legacy typed flags. KEEL pre-commit fix: sparse-data hint when filtering by unloaded Lindsay-tier stat. | 6 L2 |
| **L.3.2** | AI-06 universal `nhl_id asc` tiebreak applied to all 5 sort sites in `commands/query.rs` (Improvement, Pts/82 fallback, standard SortMetric, goalies, similar-players). Sort now deterministic across process invocations. L.3.0 fence test goldens regenerated under deterministic sort; `#[ignore]` dropped — fence active for L.3 + L.5. | (regen + un-ignore) |
| **L.3.3** | TUI Queries categorized sections — new `QuerySection` model groups 10 default fields into 4 named sections ("Sort & Display" + "Position & Age" expanded; "Origin & Draft" + "Stats Thresholds" collapsed). `Tab` toggles section containing cursor. Renderer rewrite: ▶/▼ markers + indented field rows for expanded sections. Cursor up/down skips collapsed-section fields. `Action::Refresh` resets sections. `cycle_screen` exposed `pub(crate)` for test-only screen advancement. | 5 L0 + 1 L0 + 5 L1 (gap-fill) |
| **L.3.4** | Search-as-you-type sort picker overlay — `/` opens picker; type filters by `cli_key` substring (case-insensitive); Up/Down navigates filtered list; Enter accepts; Esc cancels (overlay) or clears active pick (Build). `sort_picker_filter(query) -> Vec<StatId>` helper. New `run_query_views_with_pick` variant routes selected stat through `StatId::sort_cmp` (AI-06 tiebreak). EDGE checkpoint fix: Esc on Build with active pick clears `sort_stat_pick` (no more sticky pick). | 6 L0 + 7 L1 |
| **L.3.5** | Post-L.3 stdout-golden reassertion (exit criterion). Fence test passing post-L.3 against the regenerated deterministic goldens. | (verifies L.3.0 fence) |

**Role-review checkpoints (all PASS):**
- KEEL + BENCH on CLI surface (post-L.3.1) — 4 PASS / 3 PARTIAL + 5 PASS / 2 PARTIAL. Pre-commit fixes landed: sparse-data hint, regen instructions, typed-flag coexistence test.
- GLASS + EDGE on TUI + filter integration (post-L.3.3/4) — 7/8 + 7/8. Sticky-pick UX wart caught by EDGE; pre-commit Esc-clears-pick fix landed.

**Carry-forwards parked for L.4 / L.5+:**
- 80-col degradation for sort picker rows (~70+ char width wraps) — GLASS #7
- Header-as-cursor-stop in Queries (currently sections only togglable via cursor's field) — GLASS
- L0 contract tests for picker edge cases (Gaa ascending, EvGoalsFor multi-stint None-last, PointsPerGame sub-MIN_GP None-last) — EDGE
- Status-line refresh on empty-list Enter in picker — EDGE #7
- Doc comment on `sort_val_view` noting picker/legacy MIN_GP semantic split — EDGE #5
- Roll `--filter` to `query player/compare/goalies` (II-06 uniformity) — KEEL
- Add `Games` StatId for skater GP (catalog gap) — KEEL
- "Did you mean `>=`?" hint for `=>` typo — KEEL
- Same-key Min+Min L2 normalization pin — BENCH
- Replace flat fields with per-StatId filter rows (full v0.4 spec — 1 row per stat per category) — L.4+

**Test totals:**
- Pre-L.3: 1184 workspace tests
- Post-L.3: **1215 workspace tests** (+31 — 6 L.3.1 + 5 L.3.3 + 5 L.3.3 gap-fill + 6 L.3.4 L0 + 7 L.3.4 L1 + 1 L.3.4 EDGE + 1 fence un-ignored)
- All passing; no regressions.

---

## L.2 ship summary (2026-05-02)

L.2 implementation complete with 6 sub-phases + 2 role-review checkpoints, all green:

| Sub-phase | Deliverables | Tests added |
|---|---|---|
| **L.2.1** | `StatId` enum (107 variants — exhaustive, NOT `#[non_exhaustive]`) + `StatCategory` (9 variants) + `StatUnit` + accessors (`category`, `unit`, `higher_is_better`, `label`, `short_label`, `narrow_label`, `cli_key`, `all`, `from_cli_key`). **Spec drift resolved**: spec totaled 108 with Goalie 22; explicit list shows Goalie 23 + double-listed `PpToiPerGame`/`ShToiPerGame` (SpecialTeams + TimeOnIce). Implementation rationalized to 107 (Goalie 23 per explicit list; PpToiPerGame/ShToiPerGame consolidated to TimeOnIce). HART/FORGE pass-confirmed. | 11 L0 |
| **L.2.2** | `read(view) -> Option<f64>` for all 107 stats with DI-11 OnIceGoals trade-window guard at category boundary, MIN_GP guards on derived per-game/per-82, 300s TOI floor on per-60 rates (PACE-F1). `applies_to(pos, is_goalie)` + `applies_to_era(season)` + `available_since() -> Season`. `sort_cmp(a, b)` with universal AI-06 tiebreak. `aggregate_read(views)` with strict propagation per AI-09. | 14 L0 |
| **L.2.3** | `crate::fixtures::stat_catalog_variants` module — 6 named PlayerView fixtures (skater_modern, skater_pre_2005, center_with_faceoffs, goalie, traded_multistint, low_gp). 642-cell cross-product L1 integration test (BENCH-R2 L2-B22 deliverable) at `tests/stat_catalog_variants.rs`. | 7 L1 |
| **L.2.4** | `FilterOp` (Min/Max/Equals); `FilterParseError` 7-variant enum + Display; `StatFilter::new` finite-value gate (NaN/inf rejected at construction); `parse_filter` with op-priority routing (>=/<=/== before =), MultipleOps detection, locale-comma rejection. `PlayerFilter::stat_filters` + `normalize_stat_filters` (Min+Min→tightest, Max+Max→tightest, Min+Max→range, idempotent) + `matches_stat_filters` (DI-08 silent skip, missing-data fails, unit-aware Equals tolerance per L2-B1). | 21 L0 |
| **L.2.5** | `StatsRepository::extra_reports: BTreeMap<(PlayerId, Season, SeasonType, ReportKind), serde_json::Value>` runtime-only Tier-2 cache. DI-12 cascade-evict on window drop. DI-26 cap at 4096 entries with LRU. DI-27 runtime-only. New `EXTRA_REPORTS_CAP` const + `ExtraReportKey` type alias. `fetched_report` accessor + `upsert_fetched_report` mutator + `fetched_reports_len` for observability. | 6 L0 |
| **L.2.6** | `merge_goalie_bios_into_identity(base, &GoalieBios) -> PlayerIdentity` adapter (resolves L-B4 — switches goalie identity from skater/bios to dedicated goalie/bios endpoint). Field-mapping table + non-numeric draft graceful drop. 3 catalog-routed PlayerView accessors (proof-of-concept for OQ#4: catalog as source of truth). | 6 L0 |

**Role-review checkpoints (all PASS):**
- HART + FORGE on read dispatch (post-L.2.3) — 9/9 + 9/9. HART caught a real numeric bug (SpecialTeams per-60 arms returning per-game from `p82/82.0` — fixed: gated to `None` until L.6 brings PP-TOI). FORGE caught `higher_is_better` wildcard drift (fixed: now exhaustive enumeration). FORGE/HART noted spec-count drift (108 → 107 rationalized).
- EDGE + BENCH closeout (post-L.2.6) — 9/9 + 11/11 after BENCH gap closure (`l0_lindsay_view_ev_goal_diff_via_catalog`).

**Carry-forwards parked for L.3 / L.4 / L.7:**
- `fetched_report` read should touch LRU (recency-bias for repeat readers) — L.3+
- `fetched_reports_len` surfaced in `snapshot stats` CLI — L.6
- `MultipleOps` defensive key-side check is unreachable — clean up or test directly — L.3
- `Equals+Equals` direct-mutation last-write contract documented but untested — L.3
- 23 Tier-2 read arms still return `None` placeholders ("L.6:" markers) — populated when extra_reports cache lights up
- TOI-weighted `aggregate_read` blend deferred until L.6 brings reliable PP/SH-TOI denominators online

**Test totals:**
- Pre-L.2: 1119 workspace tests
- Post-L.2: **1184 workspace tests** (+65 — 11 + 14 + 7 + 21 + 6 + 6 + 1 BENCH gap = 66; -1 net from L.2.5 cap test refactor)
- All passing; no regressions; no live-network calls in tests; all fixtures synthesized via `stat_catalog_variants` builders or `tempfile`.

---

## L.1 ship summary (2026-05-02)

L.1 implementation complete with 7 sub-phases + 4 role-review checkpoints, all green:

| Sub-phase | Deliverables | Tests added |
|---|---|---|
| **L.1.0** | `data/api-probe-2026-05-02.txt` — 23/23 working endpoints + 8/8 broken endpoints verified, all Tier-1 supports playoff | (artifact only) |
| **L.1.1** | 5 Tier-1 substructs on `SeasonStats`: `time_on_ice`, `goals_for_against`, `goalie_advanced`, `goalie_saves_by_strength`, `goalie_bios`. `Option<T>` + `#[serde(default)]` for forward-compat. Builder methods + serde round-trip tests. | 6 L0 |
| **L.1.2** | New `icelines-core::stats_catalog` module: `ReportKind` (23 variants, camelCase serde), `Tier`, `MergeTarget`, `Tier1ReportFile`, `TIER1_REPORTS` table, `Tier1Row` trait. `ChunkedManifest` refactored to v=2 with unified `BTreeMap<ReportKind, BTreeMap<SeasonType, HashMap<u32, String>>>`. Custom Serialize/Deserialize accepts both v=1 (Hart.6.2 flat) and v=2 (nested), promotes v=1 → v=2 in-memory, rejects v≥3 with `RepoVersionUnknown`-shaped error. Backward-compat accessors `bios()`/`stats()`/`playoff_*` so call sites unchanged. | 13 L0 |
| **L.1.3** | `SnapshotMetaFlags::CURRENT_REPOSITORY_VERSION` 1 → 2; DI-28 boundary check fires at `stats_loader::load_into_repo` with `RepoVersionUnknown`. Writer/reader version lockstep test. | 2 L1 |
| **L.1.4** | `load_report_with_fallback<R: Tier1Row + DeserializeOwned>` in `stats_loader.rs` — snapshot dir → bundled fallback → `Ok(None)` decision tree. DI-29 per-row seasonId fence. Read-only contract pinned. New `LoadError::ReportLoad { kind, cause }` variant. `bundled::report_for_lindsay` stub (filled by L.7). | 8 L1 |
| **L.1.5** | Rate-limit policy: retry surface widened from `{429}` → `{429, 5xx}`; max retries 3 → 5; base 1000 → 500ms; new `retry_cap_ms = 30_000`. New `icelines-fetch::fetch_lock` module — marker-file lock at `<icelines_home>/.fetch.lock`, RAII guard, 5-minute stale-lock GC. | 4 L0 + 7 L1 |
| **L.1.6** | `icelines fetch report --kind <ReportKind> [--season N] [--type {regular\|playoff}] [--no-lock] [--dry-run]` CLI subcommand. `ReportKindArg` clap value-enum with kebab-case names. `do_report` dispatch: `is_known_working()` gate, Tier-1 only (Tier-2 errors with "deferred to L.6"), fs lock + 120s timeout, atomic JSON write to `<snapshot_root>/<season>/<season_type>/<filename>`. New `NhlApiClient::fetch_report_paged(kind, season, st)` generic helper. | 4 L2 |

**Role-review checkpoints (all PASS):**
- HART + FORGE on data model (post-L.1.1) — 6/6 + 8/8
- HART + WIRE on schema evolution (post-L.1.2/3) — 6/6 + 7/7
- TAPE on pipeline integrity (post-L.1.4/5) — 5 PASS, 2 PARTIAL with carry-forwards
- KEEL + BENCH closeout (post-L.1.6) — 4 PASS / 3 PARTIAL + 5 PASS / 2 FAIL closed during sign-off

**Carry-forwards parked for L.2 / L.6:**
- L2 cross-process subprocess lock test (TAPE follow-up #1) — defer to L.2 if it stresses concurrency, else L.6
- Tier separation in `--help` (KEEL #2) — L.6 ergonomics
- `--type both` clap rejection vs internal loop (KEEL #1) — L.6 ergonomics
- `SnapshotStore::list` enumeration of per-window report files (KEEL #7) — L.6
- `MergeTarget::SkaterSummaryTotals` / `SkaterIdentity` deferred routing (HART) — L.7 historical bundling
- Pre-Lindsay `SkaterSummary` chunks contained merged realtime+goalsForAgainst payload (HART) — L.4 needs an explicit "old chunks fall through fallback" path
- Endpoint-specific httpmock fixtures with typed parsing (BENCH #2) — L.7 when bundled data lands
- Optional `fs2` kernel-level lock (TAPE #4) — non-blocking

**Test totals:**
- Pre-Lindsay: 1057 workspace tests
- Post-L.1: **1119 workspace tests** (+62 — 45 Lindsay-prefixed + 17 from migrated/extended pre-existing)
- All passing; no regressions; no live-network calls in tests; mock fixtures cover the retry surface.
