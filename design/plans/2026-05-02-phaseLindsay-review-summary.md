# Phase Lindsay v0.1 — 8-Role Review Summary

**Date**: 2026-05-02
**Reviewers**: HART, KEEL, TAPE, FORGE, EDGE, BENCH, WIRE, GLASS (8 of 10 — SCOUT and PACE deferred to v0.2)
**Verdict**: ship v0.2 with consolidated punch list applied. Three structural decisions must be pinned before L.1 starts.

---

## Headline

The dispatch-table design (`StatId::read(view)` as the single source of truth) is structurally sound. Multiple roles independently endorsed it. **The defects cluster in three areas**:

1. **`fetched_reports` blob on `SeasonStats`** — HART, FORGE, WIRE, TAPE all flag this. Must move to a separate `ExtraReports` map keyed by `(player_id, season, season_type, kind)`. Decide BEFORE L.1, not deferred to L.6.
2. **Schema-version axis** — WIRE, FORGE flag. Bump `repository_version` to 2 (`SeasonStats` typed-substruct addition is a model change); leave `bundle_schema_version` at 1 (new files, not new fields).
3. **Position + era applicability** — HART, EDGE, GLASS flag. `applies_to` needs an era axis (`available_since: Season`). Position is per-row, evaluated per-view, not statically per-pool.

Total: **29 BLOCKERs, 41 FIXITs, 33 NITs** across 8 reviewers (with significant overlap; consolidated below).

---

## Consolidated BLOCKERs (must fix in v0.2)

### Data model (4)

- **L-B1** [HART-B1, FORGE-B3, WIRE-B1, TAPE-B1] **`fetched_reports: HashMap<ReportKind, serde_json::Value>` violates the model.** Move Tier-2 reports to a separate `ExtraReports` map owned by `StatsRepository` with its own LRU policy. `SeasonStats` stays typed-only. **Decide before L.1.**
- **L-B2** [HART-B3] **Catalog has no homogeneity guard for cross-row aggregations.** `StatId::read(view)` is row-local; the spec must say so explicitly and require `debug_assert_view_window_homogeneous` at every aggregation entry point.
- **L-B3** [HART-B2, EDGE-B06, GLASS] **`applies_to(self, position)` is incomplete.** Missing era-axis (`available_since: Season`); doesn't match Hart's per-row position semantic. Add `available_since` and clarify position-applicability is per-row.
- **L-B4** [TAPE-B2] **Goalie bios `merge_with` untested.** Lindsay proposes bundling `goalie/bios` and switching the goalie identity path. Need explicit field-mapping table + L0 test for goalie merge.

### Semantics + edge cases (5)

- **L-B5** [FORGE-B1] **Legacy `--ppg-min` semantic ≠ catalog `PointsPerGame`.** `filter.rs:200-212` computes `pace_82/82`; catalog spec says `points/gp`. Pick one and document. **Recommendation: legacy flag keeps `pace_82/82`; new `--filter "points-per-game>=X"` uses catalog `points/gp`.**
- **L-B6** [EDGE-B01] **Per-60 division on zero TOI is undefined.** Add `view.total_toi_sec() -> Option<u32>`; every per-60 arm returns `None` when total TOI = 0.
- **L-B7** [EDGE-B02] **`OnIceGoals` for traded players is meaningless.** `goalsForAgainst` is per-team. `read()` must return `None` for `OnIceGoals` category when `was_traded_in_window() == true` (or expose `*LastStint` variants explicitly).
- **L-B8** [EDGE-B03] **NaN / infinity filter values must be rejected at parse time.** Add to `parse_filter`; codify in II-05.
- **L-B9** [EDGE-F10, BENCH] **Sort tiebreak codification.** Every catalog-driven sort is `(stat_value, nhl_id asc)` — `None` sorts last regardless of `higher_is_better`. Add as AI-06 invariant + `StatId::sort_cmp(view_a, view_b)` helper.

### Schema + storage (4)

- **L-B10** [WIRE-B2, WIRE-F2] **Schema-version axis must be pinned.** Bump `repository_version` to 2 (`SeasonStats` gains typed substructs); leave `bundle_schema_version` at 1 (new files, not fields). Old binaries error cleanly with `RepoVersionUnknown`.
- **L-B11** [WIRE-F3] **`ChunkedManifest` won't scale.** Refactor to `HashMap<(ReportKind, SeasonType), HashMap<u32, String>>` with custom Deserialize promoting old flat fields. Bump `ChunkedManifest::version` to 2.
- **L-B12** [WIRE-B3] **Mock fixture coverage is mandatory, not "brief mention".** Each of 18 new endpoint URLs needs an `httpmock` fixture serving a captured real response. Budget ~54 fixture files.
- **L-B13** [WIRE-B4] **Endpoint name casing not validated against live server.** Commit `data/api-probe-2026-05-02.txt` listing exact URLs tested + sample responses. Hart.6 had the same requirement.

### 4-surface convergence (3)

- **L-B14** [KEEL-B1] **axum HTTP server missing from the convergence claim.** `/api/team/:name/roster` and `/api/standings` JSON keys must be `StatId::cli_key()` strings. Add to L.5.
- **L-B15** [KEEL-B2, BENCH-B3] **Fantasy scheme TOML migration has no back-compat layer.** TOML uses `pp_goals` (snake_case), CLI uses `pp-goals` (hyphen). Add explicit alias for both spaces + L1 test that loads ≥5 known-shaped legacy schemes byte-identical.
- **L-B16** [KEEL-B3] **Site invariant SI-03 will create silent CLI/site divergence during transition.** Either dedicated sub-phase L.5b (atomic site rename commit) OR a CI lint that fails on hand-written stat names in `icelines-site/src/`.

### Type system (2)

- **L-B17** [FORGE-B2] **`#[non_exhaustive]` is backwards.** Drop the `#[non_exhaustive]` mention; keep `StatId` exhaustive so the compiler enforces "added a stat → updated everywhere."
- **L-B18** [BENCH-B1] **L0 fixture coverage strategy is wrong.** "200 hand-written cases" won't cover ~98 stats × 6 fixture variants (skater-modern, skater-pre-2005, center, goalie, traded-multistint, GP=0). Mandate table-driven proptest over `StatId::all() × fixture variants` for read/applies_to/from_cli_key.

### Tests (5)

- **L-B19** [BENCH-B2] **No legacy `--sort` parity fence.** Capture stdout for every legacy `--sort` value (~30 strings) BEFORE L.5; reassert byte-equality AFTER.
- **L-B20** [BENCH-B4] **Tier-1 historical bundling parse fence missing.** L.7 ships 9 reports × 38 seasons = 342 files; need `l0_lindsay_7_each_tier1_report_parses_for_all_38_bundled_seasons` cross-product test.
- **L-B21** [BENCH-B5] **Generic filter grammar — 15 cases insufficient.** Minimum ~20 malformed-input classes (empty, whitespace, missing op, NaN, infinity, scientific notation, comma decimal, multiple ops, Unicode confusables, etc.).

### TUI (3)

- **L-B22** [GLASS-B1] **`<` / `>` keys are not bound and conflict on Shift-comma/period.** Use `[` / `]` (single keypress, vim-canonical) or `Tab`/`Shift+Tab` for column cycler. Lock in spec.
- **L-B23** [GLASS-B2] **Nav bar overflows at 100 cols once Lindsay lands.** Place column-selector indicator in player card title block, not nav bar. Add 100-col snapshot test.
- **L-B24** [GLASS-B3] **Career-table column overflow at 80 cols.** Spec must specify degradation: drop columns from right when narrow, OR horizontal scroll, OR abbreviated mode.

---

## Key FIXITs (apply in v0.2)

- **L-F1** [HART-F1, EDGE-F08] DI-08 silent-skip is too lenient — CLI rejects non-applicable filter at parse time when position context is known. Split into row-level (skip) vs CLI-level (reject).
- **L-F2** [HART-F4] `ReportKind` lives in `icelines-core`, not `icelines-fetch`. Per the dependency chain.
- **L-F3** [HART-F5] `Pace82`, `PaceSortKey` → catalog calls existing `PlayerView::pace_82()` accessor; never reaches into `view.stats.totals` directly.
- **L-F4** [HART-F6, BENCH-F2] Add CI-guard for "no surface looks at `view.goals` directly" — grep-based test in `tests/lints/`.
- **L-F5** [KEEL-F1] `<space>` keybind conflict — pick one (recommend `Tab` for section toggle since `<space>` is taken on Queries for results-focus).
- **L-F6** [KEEL-F2] **Promotion rule for Tier-2 reports**: any `ReportKind` consumed by ≥2 surfaces MUST be promoted to typed before second consumer ships. Add as AI-06.
- **L-F7** [KEEL-F3] Single `[stats_catalog]` section in `config.toml` with `career_table_columns` + `queries_default_sort` keys.
- **L-F8** [TAPE-F1] CI probe-endpoints test — nightly, not PR; opens GitHub Issue on regression.
- **L-F9** [TAPE-F2] User-facing warning when filter excludes >50% of pool due to era unavailability.
- **L-F10** [TAPE-F3] Verify Tier-1 endpoints' seasonId presence before authoring typed structs.
- **L-F11** [TAPE-F4] Rate-limit policy in spec — 1 req/sec sustained, 5/sec burst, exponential backoff on 429.
- **L-F12** [FORGE-F1] `members(c) -> &'static [StatId]` instead of iterator. Const arrays per category.
- **L-F13** [FORGE-F2] `from_cli_key` via `match` literals on `&str` (compiler optimizes); not `phf::Map`.
- **L-F14** [FORGE-F3] `parse_filter` returns `Result<StatFilter, FilterParseError>` with typed error enum.
- **L-F15** [WIRE-F1] `ReportKind::supports(season_type) -> bool`; CLI rejects unsupported pairs pre-fetch.
- **L-F16** [WIRE-F4] String-or-number deserializer for Tier-1 percentage fields (NHL API has been observed to send strings).
- **L-F17** [WIRE-F5] Extract `load_report_with_fallback<T>(kind, season, season_type, store)` generic — replaces 9 copy-paste chains.
- **L-F18** [WIRE-F6] Single source of truth for endpoint inventory: rewrite `data-sources.md` §Tier 2a; remove inline tables from plan/spec.
- **L-F19** [EDGE-F11] `fetched_reports` corrupt JSON: skip + WARN, never abort the whole load.
- **L-F20** [EDGE-F12] Career table user prefs vs new bundled defaults: keep user's set silently; add `> reset` in TUI help.
- **L-F21** [BENCH-F1] DI-07 fence in catalog read path: mismatched-seasonId rows return `None` from `read()`.
- **L-F22** [BENCH-F3] Career table cycling — bidirectional symmetry test.
- **L-F23** [BENCH-F4] TOML round-trip persistence test (HOME=tempdir).
- **L-F24** [BENCH-F5] TUI Queries snapshot location — extend `userflow/queries_categorized.rs`. Min 4 goldens.
- **L-F25** [BENCH-F6] `applies_to` truth table — exhaustive Position × StatId fixture.
- **L-F26** [GLASS-F2] Sort dropdown with 98 entries needs search (`:` opens picker, type substring).
- **L-F27** [GLASS-F3] Greyed-out non-applicable stats: status-line tooltip on cursor.
- **L-F28** [GLASS-F5] Persistence split: TOML for config, App state for session.
- **L-F29** [GLASS-F6] First-run banner when >50% career-table cells are `None`: `"Run icelines data install for full historical stats"`.

---

## NITs (defer to post-merge cleanup)

Twenty-eight NITs across the 8 reviews. Notable themes:

- **120 vs 98 stat count drift** in spec/plan (HART-N2, KEEL-N2, FORGE-N3, BENCH-N1) — reconcile.
- **`StatUnit::GoalsAgainstAverage`** (FORGE-N1) — fold into `Inverted` or formatter override.
- **Goalie skating-out edge case** (HART-N4) — `is_goalie()` per-row, not `pos == Goalie`.
- **`aliases()` collision check** (FORGE-N5, BENCH-N3) — compile-time / startup `debug_assert`.
- **Test count baseline tracking** (BENCH-N5) — publish per-phase L0/L1/L2 deltas.
- **PITFALLS.md update** (EDGE) — add per-60 zero-TOI, on-ice trade semantics, era axis.
- **ARCHITECTURE.md** (KEEL-N3) — add stats_catalog arrow to data-spine diagram.

---

## Reviewers' files for full text

The full agent transcripts (all 8 reviews) are saved as part of the conversation history; cite them as needed for context. Key file/line citations from each:

| Role | Key citations |
|---|---|
| HART | `stats_repository.rs:36, 75-104, 482-517, 710`; `season_stats.rs:188-221, 223-229`; `cross_team.rs:14-39` |
| KEEL | `ARCHITECTURE.md:14-23`; `fantasy-scheme.md:398-403`; `tui/event.rs:26`; `app.rs:497-505, 1175, 1227, 1271` |
| TAPE | `identity.rs:99`; `schema.rs:57-89, 154-190`; `stats_loader.rs:120` |
| FORGE | `filter.rs:200-212`; `scoring.rs:8`; `commands/query.rs:23, 72, 1365+`; `stats_repository.rs:710` |
| EDGE | `stats_repository.rs:530, 537, 550, 637-700, 703-706` |
| BENCH | `bundled.rs:669-685`; `fixtures.rs:9-11`; `system_tests.rs` |
| WIRE | `snapshot.rs:115-135, 1064-1106`; `stats_loader.rs:33-43, 257-273`; `nhl_api.rs:124-201`; `season_stats.rs:188-221` |
| GLASS | `tui/event.rs:47-85`; `screens/mod.rs:123-177`; `screens/queries.rs:31-44`; `screens/depth.rs:1-80`; `screens/player.rs:87-100` |

---

## What v0.2 does

1. Apply L-B1 through L-B24 to the plan + spec.
2. Defer all FIXITs marked "v0.2" — they get applied to the spec/plan (doc-only).
3. NITs: park in `design/PITFALLS.md` and `INDEX.md` follow-up list.
4. Add SCOUT and PACE reviews in v0.3 if needed (the implementation will surface their concerns).
5. Tag the v0.2 plan + spec; INDEX.md status flips Active (Draft) → Active (v0.2 — ready to implement).
