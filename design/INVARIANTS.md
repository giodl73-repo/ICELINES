# IceLines System Invariants

Properties that must always hold. Violation of any invariant is a bug, not a design trade-off.

Invariants are grouped by domain:

- **DI** — Data Invariants: properties of player and team data at any point in the pipeline
- **AI** — Algorithm Invariants: properties of the scoring and classification engine
- **II** — Interface Invariants: properties of the CLI commands and their inputs/outputs
- **SI** — Site Invariants: properties of generated docs/site/export artifacts

---

## Status Table

| ID    | Domain    | Invariant | Status | Notes |
|-------|-----------|-----------|--------|-------|
| DI-01 | Data | Every skater/goalie row admitted to `StatsRepository` has a canonical `PlayerId`, non-empty display name, canonical team abbreviation when team context is present, and a window key `(season, season_type)`. Rows that cannot satisfy those fields are rejected at load/fetch boundaries, not silently ranked. | VERIFIED | Hart/Lindsay repository fixture and loader tests; current domain type is `PlayerView`, not the retired CSV-era `Player` |
| DI-02 | Data | A stat row with zero games played or insufficient sample remains readable as raw data, but derived pace/projection metrics return `None` through `StatId::read` or the relevant ViewModel field. Callers do not manufacture zero-valued projections. | VERIFIED | `StatId::read` totality and fantasy/ViewModel fixture tests |
| DI-03 | Data | Minimum-game and applicability gates live in the stat catalog/ViewModel builder, not in renderers. CLI, TUI, web, and markdown surfaces may filter or format rows but must not reimplement sample-size eligibility. | VERIFIED | Lindsay stat catalog tests and Campbell ViewModel adapter tests |
| DI-04 | Data | Name-based user input resolves to canonical IDs or normalized roster keys before it enters ranked/product output. Ambiguous or invalid names surface as typed errors/warnings instead of producing anonymous rows. | VERIFIED | CLI/web player resolution, fantasy scenario canonicalization, and invalid-drop tests |
| DI-05 | Data | Normalized player keys are produced with `normalize_name()` at mutation/import boundaries; persisted fantasy/group/watch records store canonical normalized keys or typed `entity_ref` values. | VERIFIED | Group/fantasy DB round-trip tests and fantasy add/drop scenario tests |
| DI-06 | Data | Duplicate player identity within one repository window must collapse to one canonical row or be represented as explicit per-window/team context; rankings and ViewModels use `PlayerId` tie-breaks and never rank the same `(player_id, window)` twice as an accidental duplicate. | PARTIAL | Current `StatsRepository`/sort paths are ID-keyed; transaction/player-link edge cases remain tracked in PITFALLS |
| AI-01 | Algorithm | Ranking/classification outputs are produced by shared query/catalog/ViewModel builders. Renderers must not compute independent fit, pace, poach, fantasy, or depth scores. | VERIFIED | Campbell/Selke ViewModel gates across CLI/TUI/web |
| AI-02 | Algorithm | Any ordered leaderboard or board is deterministic for fixed repository, query, scoring scheme, and source state. Stable tie-breaks include canonical player/team IDs where available. | VERIFIED | `GoalieLeaderboardSort`, `StatId::sort_cmp`, poach/fantasy ViewModel fixture tests |
| AI-03 | Algorithm | Team-depth and depth-chart ViewModels expose bounded line/pair/slot rows plus explicit empty/unplaced state; renderers do not infer roster structure from raw arrays. | VERIFIED | `TeamDepthView`, `TeamDepthChartView`, `TeamTradeImpactView`, and TUI/web/markdown/CLI adapter tests |
| AI-04 | Algorithm | Public thresholds, weights, and scoring schemes are catalog/scheme data. If a renderer needs a label or explanation, it reads it from the ViewModel or scheme metadata rather than hard-coding algorithm constants. | VERIFIED | Lindsay stat catalog, fantasy scheme, and poach explanation tests |
| II-01 | Interface | Core product commands and routes render from shared ViewModels or documented DTO projections from ViewModels; route/command claims must appear in `surface-parity.md`. | VERIFIED | Ted Lindsay route inventory and Campbell surface parity matrix |
| II-02 | Interface | Skater and goalie surfaces remain separated by typed ViewModels where their stat semantics differ; all-position queries may combine them only through explicit mixed-surface contracts. | VERIFIED | `LeadersView`, `GoaliesView`, team-depth goalie sections, and query goalies tests |
| II-03 | Interface | Terminal color/style is presentation-only. Data contracts, JSON, markdown, and ViewModels never encode ANSI or renderer-specific class/style strings. | ENFORCED | ViewModel spec bans renderer-specific styles; terminal output remains guarded by surface adapters |
| SI-01 | Site | Static/generated docs and exports are not shipped-route truth. Route truth is `surface-parity.md` plus `ted_lindsay_route_inventory.rs`; generated docs may summarize only rows marked done or clearly partial. | VERIFIED | Route inventory gate |
| SI-02 | Site | Static/export pages that surface stats, players, teams, or reports render from the same ViewModel/report contracts as CLI/TUI/web, or carry an explicit matrix exception. | VERIFIED | Markdown exports and generated team pages are ViewModel-backed; generated team pages are fenced by `l1_render_team_page_uses_team_depth_view_slots` |

---

## Adding Invariants

When a new invariant is identified (by any role, in any session), add it to the table above:

1. Assign the next ID in the appropriate domain sequence
2. State the invariant as a property that is either true or false — not a goal or a guideline
3. Set status to **OPEN**
4. Add a test reference once a test enforcing the invariant exists
5. Set status to **VERIFIED** only when the test passes in CI

An invariant with no test is a promise. An invariant with a passing test is a guarantee.

## Status Codes

- **OPEN** — invariant is stated, no enforcement mechanism or test yet
- **ENFORCED** — structural enforcement exists (type system, validation at boundary) but no test
- **VERIFIED** — a test would fail if the invariant were violated, and the test passes
- **PARTIAL** — enforced for current high-value surfaces, with a named carry-forward

---

## Legacy invariant cleanup

The original DI-01..DI-06, AI-01..AI-04, II-01..II-03, and SI-01..SI-02
statements described the pre-Hart CSV `Player` / `FitClass` / generated-team-site
architecture. They are superseded by the `StatsRepository + PlayerView + typed
ViewModel` architecture above. The failure modes were not deleted; they remain
as historical pitfalls in `design/PITFALLS.md` and should be reintroduced only
as current invariants with current type names and test references.

## Phase Lindsay invariants (DI-07 through DI-29 / AI-05 through AI-09 / II-04 through II-06 / SI-03)

The full Lindsay invariant table is the source of truth in
`design/plans/2026-05-02-phaseLindsay-stat-catalog.md` (§"New
invariants"). They are summarized here for cross-reference; the
phase plan owns the canonical statements and the test references.

| ID | Domain | Headline | Status |
|---|---|---|---|
| DI-07 | Data | `StatId::read(view)` is total — no panics; `Some` iff data + applies_to + applies_to_era; else `None`. | VERIFIED (L.2.3 cross-product 642-cell test) |
| DI-08 | Data | `StatFilter` non-applicable to position is silently dropped at row iteration; CLI rejects at parse time when position context is known. | VERIFIED (L.2.4) |
| DI-09 | Data | Every Tier-1 substruct on `SeasonStats` is window-keyed `(season, season_type)` and is `None` when not fetched. | VERIFIED (L.1) |
| DI-10 | Data | `StatId::read(view)` is row-local — no repository, no global state. Future context-needing stats get `read_with_context`. | ENFORCED (signature) |
| DI-11 | Data | `OnIceGoals` category stats are last-stint-only — `read()` returns `None` when `was_traded_in_window()`. | VERIFIED (L.2.3) |
| DI-12 | Data | Eviction of a window from typed LRU cascade-evicts every `extra_reports` entry whose key matches that window. | VERIFIED (L.2.5) |
| DI-25 | Data | Every pre-Lindsay scheme TOML loads byte-identical to its frozen golden via the legacy-key alias map. | VERIFIED (L.5.5 — 3 built-in schemes; full 5-named corpus carries forward) |
| DI-26 | Data | `extra_reports` capped at 4096 entries with LRU eviction past the cap. | VERIFIED (L.2.5) |
| DI-27 | Data | `extra_reports` is runtime-only — never persisted; subsequent runs re-fetch. | VERIFIED (L.2.5) |
| DI-28 | Data | `repository_version` boundary check fires at `StatsRepository::load_window`; old binary on v=2 snapshot errors at file-open. | VERIFIED (L.1) |
| DI-29 | Data | Every Tier-1 deserializer asserts `row.seasonId == requested_season` for every row; mismatch errors before the substruct populates. | VERIFIED (L.1) |
| AI-05 | Algorithm | `StatId::all()` and `StatCategory::members(c)` return values in a stable declaration order. Iteration is deterministic. | VERIFIED (L.2.1) |
| AI-06 | Algorithm | Every catalog-driven sort is `(stat_value desc/asc, nhl_id asc)`. Codified in `StatId::sort_cmp`. None values sort last regardless of `higher_is_better`. | VERIFIED (L.2.2 + L.3.2 + L.5.1) |
| AI-07 | Algorithm | Any `ReportKind` read by ≥2 surfaces MUST be promoted to a typed sub-struct + a `StatCategory` before the second consumer ships. | OPEN (L.6 sets up the boundary; promotion happens case-by-case) |
| AI-08 | Algorithm | Aggregations over `&[PlayerView]` that consume catalog reads MUST call `debug_assert_view_window_homogeneous`. | VERIFIED (Hart.6.6 + L.2.2) |
| AI-09 | Algorithm | `aggregate_read(views)` is strict-propagation — Some only when every window has Some; any None propagates. | VERIFIED (L.2.2) |
| AI-10 | Algorithm | Every `PoachPlayerRow` has at least one `PoachExplanation`; a score without reasons is invalid. | VERIFIED (Selke `poach_fixture_satisfies_contract_invariants`) |
| AI-11 | Algorithm | `PoachScore` is deterministic for a fixed fixture, clock, query, scoring scheme, source state, and watch/roster import generation. | VERIFIED (Selke `poach_contract_fixture_serializes_context_score_and_explanations`) |
| AI-12 | Algorithm | Unknown deployment or ownership state never subtracts from `PoachScore` by itself; only measured or estimated negative evidence may discount a player. | VERIFIED (Selke `unknown_deployment_and_availability_are_not_negative_evidence`) |
| AI-13 | Algorithm | Every `PoachScore` component exposes status, source/unit, clamp, and value before any renderer sees it. | VERIFIED (Selke `poach_contract_fixture_serializes_context_score_and_explanations`) |
| AI-14 | Algorithm | Fantasy roster-gap and league-simulation scoring/projection logic lives in core ViewModels/helpers; CLI, TUI, web HTML, and web JSON may assemble inputs and render outputs, but must not fork category weights, add/drop scenario resolution, or projected-score math. | VERIFIED (Selke fantasy views: `FantasyRosterGapView`, `FantasySimulationView`, focused CLI/TUI/web gates) |
| II-04 | Interface | `--sort <stat-key>` accepts every `StatId::cli_key()`; unknown keys exit non-zero with the list of valid keys. | VERIFIED (L.5.1) |
| II-05 | Interface | `--filter "<key><op><value>"` parses with op in `{>=, <=, ==, =}`; whitespace allowed; 7-variant `FilterParseError`; NaN/inf/locale-comma all parse-error. | VERIFIED (L.2.4 + L.3.1) |
| II-06 | Interface | `--filter` and `--sort` accept identical grammars and StatId key sets across `query leaders / player / compare / goalies` + `export md`. Same-StatId multi-filter normalization rule applies uniformly. | PARTIAL (L.3.1 wired `query leaders`; rolling to player/compare carries forward) |
| II-07 | Interface | CLI, TUI, web, markdown, and JSON poacher surfaces render from `PoachBoardView` or `PoachReportView`; renderers do not recompute ranking or recommendation logic. | VERIFIED (Selke CLI/TUI/web/report slices through shared ViewModels) |
| II-08 | Interface | Fantasy read/product surfaces render from `FantasyRosterGapView`, `FantasySimulationView`, or `PoachBoardView`/`PoachReportView`; scenario errors must preserve canonical player-name resolution and invalid-drop messages across CLI, TUI, web HTML, and web JSON. | VERIFIED (Selke fantasy parity gates) |
| II-09 | Interface | Cross-surface mutations resolve through typed intent/result contracts before rendering or redirecting; ad hoc mutation responses must not become public JSON/admin contracts. | PARTIAL (Campbell follow-up: favorites, watch rules, season type, config, data, and snapshot intents now project `MutationResultView`; remaining work is full admin/web wiring) |
| SI-03 | Site | Every site page that surfaces a stat name uses `StatId::label()`; site templates never hard-code a stat name string. | VERIFIED (L.5b — grep fence + allowlist) |
