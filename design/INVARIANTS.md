# IceLines System Invariants

Properties that must always hold. Violation of any invariant is a bug, not a design trade-off.

Invariants are grouped by domain:

- **DI** — Data Invariants: properties of player and team data at any point in the pipeline
- **AI** — Algorithm Invariants: properties of the scoring and classification engine
- **II** — Interface Invariants: properties of the CLI commands and their inputs/outputs
- **SI** — Site Invariants: properties of the generated mkdocs site

---

## Status Table

| ID    | Domain    | Invariant | Status | Notes |
|-------|-----------|-----------|--------|-------|
| DI-01 | Data | Every `Player` in the pipeline has a non-empty `name` and a `team` that is in the canonical 32-team abbreviation list. Players that fail this check are rejected at CSV load time, not silently passed through. | OPEN | `TeamAbbr::parse()` will enforce this; not yet implemented |
| DI-02 | Data | A `Player` with `season_gp = Some(0)` must have `pace_score = None` and `fit_class = None`. A zero-GP player is never assigned a pace projection, even if they have nonzero points in the CSV. | OPEN | Scoring engine not yet implemented |
| DI-03 | Data | A `Player` with `season_gp = Some(n)` where `n < MIN_GP` must have `pace_score = None` and `fit_class = None`. The MIN_GP gate is enforced in the scoring engine, not at the caller. | OPEN | |
| DI-04 | Data | A `Player` with `nhl_id = None` (name resolution failed) must never appear on a lineup card or in a ranked output. Unresolved players are collected into a separate error report at the end of the pipeline. | OPEN | |
| DI-05 | Data | The `name_normalized` field of every `Player` is the result of applying `normalize_name()` to `name`. These two fields are always in sync — `name_normalized` is never set independently. | OPEN | |
| DI-06 | Data | Two `Player` records in the same pipeline run must not share the same (`nhl_id`, `season`) pair. A mid-season trade that produces two CSV rows for the same player must be detected and merged before the scoring engine runs. | OPEN | EDGE DI-06 / Sebastian Aho pattern |
| AI-01 | Algorithm | For any `Player` with `fit_class = Some(FitClass::Elite)`, `pace_score.pace_82` is ≥ the Elite threshold for that player's position group. The fit classification and pace score are always consistent — they are produced by the same function call and stored together. | OPEN | |
| AI-02 | Algorithm | The `sort_by_rank()` output is deterministic: given the same input `Vec<Player>`, the output is always the same ordering. The tiebreaker chain (pace_82 desc → goals_per_game desc → name asc) must be total — no two players can be considered equal by all three criteria simultaneously (names are unique in a pool). | OPEN | |
| AI-03 | Algorithm | A `DepthChart` for any team has at most 4 forward lines (12 forward slots) and at most 3 defense pairs (6 defense slots). A roster with more than 12 forwards in the CSV will have the excess players in `unplaced`, not in an overlong `forward_lines` array. | OPEN | |
| AI-04 | Algorithm | The fit thresholds used in `classify_fit()` are the same values documented in `docs/specs/rust-cli.md`. If the spec changes the threshold values, the implementation must change in the same commit. These cannot diverge. | OPEN | |
| II-01 | Interface | `icelines team <TEAM>` exits with code 0 if and only if the team was found, GP data was available (cached or freshly fetched) for at least one player on the team, and the lineup card was rendered without error. All other cases exit non-zero. | OPEN | |
| II-02 | Interface | `icelines rank` with no `--position` filter includes all positions except goalies. A goalie in the CSV is parsed but never included in ranking output and never placed on a lineup card. | OPEN | |
| II-03 | Interface | `--no-color` is always respected. Any terminal output path that produces ANSI color codes must check the `--no-color` flag first. If stdout is not a TTY, `--no-color` is implied automatically. | OPEN | |
| SI-01 | Site | Every generated team markdown file contains exactly one forward grid section and one defense grid section. A team file with no skaters in the CSV generates a page with empty grid sections and an explicit "No skaters in fantasy pool" message — not a missing section. | OPEN | |
| SI-02 | Site | The CSS fit class applied to a player cell in the site (`fit-elite`, `fit-solid`, `fit-buried`, `fit-stretch`) matches the `FitClass` computed by the scoring engine for that player. The mapping from `FitClass` variant to CSS class name is a constant, not a runtime string construction. | OPEN | |

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

---

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
| AI-10 | Algorithm | Every `PoachPlayerRow` has at least one `PoachExplanation`; a score without reasons is invalid. | OPEN (Selke fixture pending) |
| AI-11 | Algorithm | `PoachScore` is deterministic for a fixed fixture, clock, query, scoring scheme, source state, and watch/roster import generation. | OPEN (Selke fixture pending) |
| AI-12 | Algorithm | Unknown deployment or ownership state never subtracts from `PoachScore` by itself; only measured or estimated negative evidence may discount a player. | OPEN (Selke fixture pending) |
| AI-13 | Algorithm | Every `PoachScore` component exposes status, source/unit, clamp, and value before any renderer sees it. | OPEN (Selke fixture pending) |
| II-04 | Interface | `--sort <stat-key>` accepts every `StatId::cli_key()`; unknown keys exit non-zero with the list of valid keys. | VERIFIED (L.5.1) |
| II-05 | Interface | `--filter "<key><op><value>"` parses with op in `{>=, <=, ==, =}`; whitespace allowed; 7-variant `FilterParseError`; NaN/inf/locale-comma all parse-error. | VERIFIED (L.2.4 + L.3.1) |
| II-06 | Interface | `--filter` and `--sort` accept identical grammars and StatId key sets across `query leaders / player / compare / goalies` + `export md`. Same-StatId multi-filter normalization rule applies uniformly. | PARTIAL (L.3.1 wired `query leaders`; rolling to player/compare carries forward) |
| II-07 | Interface | CLI, TUI, web, markdown, and JSON poacher surfaces render from `PoachBoardView` or `PoachReportView`; renderers do not recompute ranking or recommendation logic. | OPEN (Selke implementation pending) |
| SI-03 | Site | Every site page that surfaces a stat name uses `StatId::label()`; site templates never hard-code a stat name string. | VERIFIED (L.5b — grep fence + allowlist) |
