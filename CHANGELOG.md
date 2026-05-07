# IceLines Changelog

## v0.20.0 — 2026-05-07 — Phase Art Ross

Headline: **The query system is the centerpiece of IceLines now.
Unified parser → planner → executor lands. Sliding-window streak
atoms, historical EVER queries across all 38 bundled seasons,
cross-league career atoms, on-demand data fetch, `--explain`
plan tree.** ~3 100 → 3 500 tests; +424 in the phase.

### Vision queries that now parse + evaluate

```bash
# "5 goals over 10 games, age <= 25" — current-season streak
icelines query leaders --filter "g.last10g>=5 AND age<=25"

# Same question over the player's entire career, across all 38
# bundled seasons (intra-season only, axis-typed, lockout skip)
icelines query leaders --filter "g.any10g>=5 EVER AT age<=25"

# "Junior elite cohorts" — cross-league career filter
icelines query leaders --filter "league.tier=Junior AND p.career.junior>=200"

# Inspect any plan tree without running the query
icelines query leaders --filter "g.last10g>=5 AND age<=25" --explain
```

### Added — sub-phases A.0 through A.5

- **A.0 IR + planner skeleton.** New `icelines-query::plan` module
  with n-ary `Constraint::All(Vec)/Any(Vec)/Not(Box)` IR + typed
  `Predicate { Scalar, Member, Pattern, Range }` (shape-by-
  construction makes invalid combinations like `LIKE 5` fail at
  parse, not at evaluate). `parse_query(FilterInput) -> Result<
  QueryPlan, Vec<ParseError>>` is the single front door for CLI /
  web / TUI. `DataProvider` trait owned by `icelines-query` (the
  dependency-inversion seam — preserves the crate-layering rule).
  `EvalCtx` is `!Send`-pinned via `compile_fail` doctest.
- **A.1 grammar expansion.** Strict `<` / `>` / `!=` operators
  (with `<>` typo hint suggesting `!=`); `IN (a,b,c)` / `NOT IN`
  set membership (empty `IN ()` rejected at parse); `BETWEEN x
  AND y` numeric range; `LIKE "Mc*"` with NFD-normalization so
  ASCII patterns reach Slafkovský / Stützle / Kämpf / Björk;
  `~` / `!~` substring sugar. Plus 7 new bio atoms: `pos=C`,
  `team=EDM`, `team.any=EDM`, `draft-round<=2`,
  `draft-overall<=10`, `birth-state=ON`, `nationality=USA`,
  `rookie-season>=20212022`.
- **A.2 sliding-window atoms.** New atom shape `<stat>.last<N><u>`
  where `u` is `g` (games), `d` (days), `w` (weeks), or `m`
  (months). Optional scope modifiers: `.allteams` (any stint
  this season) / `.career` (cross-season tail). `WindowPolicy`
  enum: `RequireFull` (default) / `AllowPartial` /
  `AllowPartialAbove(N)`. Mid-season-trade aware — current-team
  filter applies BEFORE the trailing-N cut.
- **A.2.4 IcelinesProvider + CLI wiring.** `IcelinesProvider`
  walks the boxscore manifest + builds `GameStatLine` records
  for sliding-window evaluation. CLI's filter dispatch routes
  needs-provider filters through the new pipeline; legacy
  pipeline preserved for everything else.
- **A.2.5 polish (12 review action items).** Killed silent
  placeholders: legacy `Constraint::matches` deleted; `team.career=`
  rejected at parse with `FeatureNotYet { ships_in: "A.4" }`;
  `name LIKE` error message no longer claims a field that doesn't
  exist; `current_team=None` returns Empty instead of falling
  back to all-stints. `EvalCtx::new` no longer calls `Utc::now()` —
  takes explicit `today` + `season`; `from_clock(&dyn Clock)`
  integrates Foster F.0's `MockClock` for tests. Diagnostics:
  `g.last10z>=5` → `UnknownWindowUnit { unit: 'z' }` with g/d/w/m
  suggestions; `g.last0g>=5` → `ZeroWindowSize`; `g.last1000g>=5`
  → `WindowSizeOutOfRange { size: 1000, max: 255 }` (no silent
  truncation). `parse_or` / `parse_and` use clean accumulator
  pattern (no sentinel-replacement smell).
- **A.2.6 coverage close.** 11 end-to-end executor tests with
  synthetic StatsRepository + canned MockProvider exercising
  `sliding_window_matches` against a real `PlayerView`. 6
  `--explain` golden snapshot tests pinning exact tree
  rendering. 5 missing `IncompatiblePredicate` parser tests.
- **Wave 12 — 200 adversarial filter scenarios** on the new
  grammar. Surfaced 1 real bug (`rookie-season>=N` was only in
  the text-field map; numeric path returned `UnknownStat`).
- **A.3 historical EVER + AT-age slicing.** New atoms:
  `p.career>=500` (LifetimeSum), `p.streak>=15` (LongestStreak),
  `g.any10g>=5 EVER` (AnyWindow — short-circuits on first
  satisfying season), `g.seasons-with>=5` (SeasonsWith). `AT
  age<=22` modifier on any career atom — supports scalar
  (`age<=22`, `age<25`) and range (`age BETWEEN 20 AND 25`)
  predicate shapes. Lockout 2004-05 skipped (no data, no
  partial-mark) per the spec. HR Feb-1 age convention via
  existing `compute_age`.
- **A.4 cross-league career atoms.** `league=OHL` /
  `league NOT IN (NHL)` / `league.tier=Junior` (uses Phase
  Calder's canonical `LeagueTier` classification — Pro /
  Junior / College / International / Other). Stat-aggregate
  3-dot keys: `p.career.junior>=200`, `p.career.nhl>=500`,
  `p.career.ohl>=300`. `IcelinesProvider::fetch_career_history`
  reads `~/.icelines/career_history.json` (Phase Calder cache).
- **A.5 `--explain` flag.** `icelines query leaders --filter X
  --explain` prints the parsed `QueryPlan` tree + data
  requirements without running the query. Pair with `--json`
  for the `explain.v1` envelope (frozen v1 — additive changes
  only; breaking changes ship as `explain.v2`). Useful for
  debugging complex filters and confirming how the planner
  routes atoms across legacy / sliding-window / career-aggregate
  / cross-league sub-evaluators.

### Architecture

- **One front door, four sub-evaluators.** `parse_query` consumes
  any `FilterInput` ({Cli(String), Form(String), Tui(Vec<C>)})
  and produces a typed `Constraint` tree. `Constraint::matches(
  view, &EvalCtx)` walks the tree once; n-ary `All`/`Any` short-
  circuit naturally. Routing: `Bio` and `SeasonStat` need only
  the active-season repo; `SlidingWindow` calls
  `provider.fetch_game_lines`; `CareerAggregate` walks per-
  season game streams; `CareerLeague` reads career history.
- **Layering preserved.** `icelines-query` does NOT import
  `icelines-fetch`. The `DataProvider` trait owned by query is
  implemented by `icelines-fetch::query_provider::IcelinesProvider`
  and injected by the surface (CLI / web / TUI).
- **Fail-closed defaults.** Missing data → atom returns false.
  Wrapping in `NOT` flips the legacy missing-data semantic
  through correctly. `--strict` mode (when wired in v0.21) gates
  partial-data results.
- **Backward compat.** Every filter expression that parsed in
  v0.19.1 continues to parse and produce identical results,
  including the FIXED behavior of the 3 Wave 11 production bugs
  (goalie compound rewrite, paren-wrapped bio atoms,
  `--filter`+`--week` loud rejection).

### Test budget

- v0.19.1 baseline: 3 081 workspace tests
- v0.20.0: 3 505 (+424 across the phase)
  - 45 A.0 + 64 A.1 + 34 A.2 + 8 A.2.4 + 4 A.2.5 + 22 A.2.6
  - 30 A.3 + 23 A.4 + 12 A.5 explain
  - +200 Wave 12 adversarial scenarios
- All Phase Art Ross gates green: Wave 11 (201) + A.0 parity (4)
  + A.2 executor (11) + A.3 career (10) + A.4 league (11) +
  A.5 explain (12) + Wave 12 (200) + the 5-role checkpoint
  review review-action items (12) all closed.

### Real bugs surfaced + fixed during the phase

- 5 silent-placeholder fixes from the 5-role A.2 checkpoint review
- 1 from Wave 12: `rookie-season>=N` numeric routing
- 1 from A.2.6: planner's SlidingWindow render was stale

### Deferred to v0.20.1+

- Cross-surface parity tests (CLI / web / TUI all parse identically)
- `--strict` flag wired through to error before any fetch
- Per-season sharded `BoxscoreIndex` with LRU cap (today's
  `IcelinesProvider` walks the full manifest)
- Criterion benchmark for `EVER` cold/warm budgets (≤8s / ≤2s)
- Surface swap (replace `parse_filter_expr` with `parse_query`
  on every `--filter` site, not just sliding-window)
- `query career` integration with cross-league atoms
- `SeasonAxis::Playoff` partition in the executor

## v0.19.1 — 2026-05-06

Headline: **3 production filter bugs surfaced by Wave 11 (200
adversarial scenarios) + Wave 10 UX polish. ~1 855 → 2 056 tests.**

### Fixed
- **Goalie compound filter rewrite ate boundary characters.**
  `icelines query goalies --filter "gp>=10 AND sv%>=0.9"` was
  silently corrupting the input to `goalie-games>=10 ANDAsv%>=0.9`
  (later just `ANDsv%>=0.9` after a partial fix). Root cause:
  `goalie_filter_rewrite_expr` had a bare `continue` inside an
  inner `for kw in ["AND","OR","NOT"]` loop where `continue 'outer`
  was needed; after matching a keyword and advancing `i`, the
  outer while-loop kept executing with the stale `c` captured at
  the top, then re-pushed it into the next atom. Compounded with
  `flush_atom` only preserving trailing whitespace (not leading)
  around the rewritten core. Both fixed; any compound goalie
  filter now parses correctly.
  ([icelines-cli/src/commands/query.rs](icelines-cli/src/commands/query.rs))
- **Bio atoms broken when wrapped in outer parens.**
  `--filter "(age<=24 AND p>=10)"` failed with
  `unknown stat key "age"` because `extract_bio` didn't recurse
  into a single-paren-wrapped expression — the catalog parser
  then saw `age` (which isn't a catalog stat). Added
  `peel_outer_parens` helper + recursive `extract_bio_into`.
  ([icelines-query/src/lib.rs](icelines-query/src/lib.rs))
- **`query leaders --week`/`--month` silently dropped `--filter`.**
  The dispatcher routed to `run_windowed_leaders(top, sort, json)`
  ignoring `filters`. Added a loud rejection at the dispatch
  boundary pointing the user at `icelines favorites --range week`
  for the populated path; full filter wiring will land in Phase
  Art Ross. ([icelines-cli/src/main.rs](icelines-cli/src/main.rs))

### Polished (from Wave 10)
- `icelines favorites --date 2014-10-08` empty-state now echoes the
  date back in the header (was: only populated state showed it).
- `icelines data-status` documented in `COMMANDS.md` (shipped in
  Foster +2, undocumented).
- Three global-flag long_about strings (`--no-live`, `--no-dashboards`,
  `--no-setup`) were 200-390 chars each on a single line in non-TTY
  output; restructured into shorter paragraphs so `--help` lines stay
  under 130 cols when piped.
- Bare `icelines` (no args) intentionally prints a friendly landing
  and exits 0 (deliberate UX, documented in Wave 10 #025).
- One unused-import warning cleaned up in `favorites_view.rs`.

### Tests
- **Wave 11 — 201 filter-grammar adversarial scenarios** across 10
  sections: boolean precedence + associativity, atom-op stress,
  bio + stat interplay, windowed atom precedence, paren / whitespace
  edges, conflicting / tautological predicates, goalies subcommand
  rewrites, alias coverage, pathological inputs (deep nesting, long
  chains, Unicode, scientific notation), output truthfulness
  (commutativity, De Morgan's laws, inclusion-exclusion).
  ([icelines-cli/tests/persona_wave11.rs](icelines-cli/tests/persona_wave11.rs))
- **Wave 10 — 100 UX consistency + truthfulness scenarios** across
  8 sections: K2.4 envelope shape, exit-code consistency, error
  message format, date / team format consistency, output stream
  discipline, COMMANDS.md ↔ binary parity, CLAUDE.md ↔ binary parity,
  `--help` quality (no dev jargon, line-width caps, examples
  present). ([icelines-cli/tests/persona_wave10.rs](icelines-cli/tests/persona_wave10.rs))
- New L0 unit tests for `peel_outer_parens` + recursive `extract_bio`
  paths in `icelines-query`.

### Note
The Wave 11 fixes set the stage for **Phase Art Ross** (next): a
unified query architecture with sliding-window streak atoms, career
aggregates across all 38 bundled seasons, cross-league career-history
atoms, on-demand data fetch driven by the query plan, and a
`--explain` view of the plan tree.

## v0.13.0 — 2026-05-03

Headline: **38 seasons bundled in (1987-88 → 2025-26), Reports overlay,
boolean filter grammar, and ~1720 tests across 4 persona-scenario waves.**
Binary grew 23 MB → 57 MB to fit the full historical era.

### Added
- **L.7b 38-season bundle.** `BUNDLED_SEASONS` now covers every NHL
  season from 1987-88 forward except the 2004-05 lockout. Refactored
  `bundled.rs` to a table-driven layout (228 lookup-table entries,
  one macro per season). Mario Lemieux's 1992-93 (218.7 Pts/82) and
  Wayne Gretzky's 1987-88 (190.9 Pts/82) are queryable from a fresh
  binary with no `data install` needed.
- **Phase Reports — `R` overlay in TUI.** Toggleable Tier-1 reports
  (realtime / timeonice / goalsForAgainst / goalie-advanced /
  goalie-savesByStrength). Disabled reports drop their columns from
  career tables, sort pickers, and query results. Persists to
  `~/.icelines/config.toml`. New `Config::reports` field +
  `ReportToggles::is_stat_visible(stat)` gate. Removes the noisy
  "Missing data: realtime" banner.
- **UX.1 — Lazy career loader on player card open.** `app.repo` LRU
  cap bumped 8 → 80. Opening a player card fans out across all 38
  bundled seasons, pulling that player's career into the repo. ~50 ms
  per first open, cached after. McDavid surfaces 11+ regular seasons,
  Gretzky 12, Crosby 18+, Ovechkin 18+.
- **UX.2 — `[/]` discoverability hint** in Queries title bar.
- **UX.3 — Tab unconditionally cycles screens.** Pre-UX.3, Tab
  toggled section expand/collapse on Queries (trapped users on the
  Stats tab). Section toggle moved to `o`. New tests pin the rebind.
- **Gaps.1 — Short filter aliases.** `--filter "g>=50"` works as
  `goals>=50`. Aliases: `g`/`a`/`p`/`pts`/`s`/`gp`/`ppg`/`gpg`/`apg`/
  `+/-`/`pim`/`pen`/`blk`/`tk`/`gv`/`mis`/`fow%`/`pace`/`sv%`/`sv`/
  `ga`/`sa`/`w`/`l`/`so`/`ot`. Filter keys are case-insensitive.
- **Gaps.2 — `query player --seasons N`.** Full bundled-history
  career arc on the CLI. Default 38 = full history.
- **Gaps.3 — `query compare --seasons N`.** Multi-season head-to-head
  with each player's career arc printed alongside.
- **Gaps.4 — Goalie filter rewrite.** `query goalies --filter "gp>=15"`
  rewrites `gp`→`goalie-games` before parsing; `starts`→`goalie-starts`.
  Error messages hint goalie-specific keys.
- **Gaps.5 — `query player` accepts goalies.** `query player Patrick Roy`
  resolves now (chains skater + goalie bios).
- **Gaps.6 — Cross-bundled name lookup.** `query player Wayne Gretzky`
  resolves without `--season` via `resolve_player_id_by_name` walking
  bundled bios + lazy career fan-out.
- **Filter.OR — Boolean filter grammar.** `--filter` now accepts AND /
  OR / NOT / parens. Recursive descent parser, precedence NOT > AND > OR.
  Bare atoms still route through `stat_filters` for normalization;
  compound expressions go to new `expr_filters`. Multiple `--filter`
  flags ANDed at top level. 19 new L0 tests.
- **`icelines docs` subcommand.** Embeds `COMMANDS.md` via
  `include_str!()` so the full command reference ships inside the
  binary. No internet needed to learn the CLI.
- **`COMMANDS.md`** — single-page command reference with every
  subcommand, examples, the alias table, the filter grammar BNF, and
  the TUI keybind matrix.
- **Rich `--help` long_about** for top-level + `query leaders`,
  `query player`, `query compare`, `query goalies`. Examples,
  alias hints, and filter grammar inline in `--help` output.
- **400 persona-scenario tests** across 4 waves
  (`persona_scenarios.rs`, `persona_wave2.rs`, `persona_wave3.rs`,
  `persona_wave4.rs`). Cover: historical seasons, multi-filter
  patterns, lazy career loading, Reports overlay, goalie filter
  rewrite, JSON/CSV output, bundle integrity, robustness, edge cases.

### Changed
- **Workspace tests: ~1720** (up from ~1275). 400 new persona scenarios
  + 19 filter-expr tests + Reports / UX / Gaps coverage.
- **Binary size: 57 MB** (up from 23 MB). 33 historical seasons + 5
  current = 38 total at ~1 MB / season bundled JSON.
- **Player loading API**: legacy `PlayerRepository::new(store, season).load_all()`
  references in CLAUDE.md replaced with the actual
  `icelines_fetch::stats_loader::load_into_repo(season, season_type, store)`
  surface.
- **`load_into_repo` LRU cap**: 8 → 80 windows so historical fan-outs
  don't evict the active season.
- **`from_cli_key` is case-insensitive** and accepts the alias map.

### Fixed
- "Missing data: realtime" banner removed — was noise, not signal.
  Phase Reports overlay handles per-report visibility properly.
- NHL API breaking change: `pim` removed from `/skater/realtime`.
  Schema field made `Option<u32>` with `#[serde(default)]`.
- 19951996 / 19961997 unbundled-season tests swapped to 20042005
  (lockout, never bundled) to remain truly unbundled after L.7b.

### Docs
- **CLAUDE.md** — refreshed AI-instruction surface. Removed misleading
  references to deleted `PlayerRepository`, "5 seasons bundled",
  "338 tests", and the cancelled proof / DASHBOARD-SPEC integration.
  Added sections on the Reports overlay, lazy career loader,
  short-alias rule, goalie filter rewrite, Filter.OR grammar.
- **README.md** — bundled count 5 → 38, test count 338 → 1720.
  New sections: catalog filter grammar with alias table, multi-season
  player/compare examples, TUI keybind reference (R, y, Shift+P, o,
  `[`, `]`, `/`).
- **COMMANDS.md (new)** — single-page reference designed for AIs and
  new users. Embedded into the binary via `icelines docs`.

## Unreleased

### Changed
- Phase 8j (rev): Native sparklines, proof_lib back to dev-only.
  `proof:chart` directives don't compose inside `proof:region` bodies
  (filed at design/proof-bug-report.md), so the dashboard compositor
  was wrapping plain text we already lay out cheaply with ratatui.
  - New `tui::sparkline` module (~80 lines, zero new deps) renders
    Unicode block sparklines `▁▂▃▄▅▆▇█` from a `&[f64]`.
  - `tui::dashboard_panel` rewritten to build lines natively. Identity
    + counting stats + bundled history trend, in 14 lines of panel.
  - Players with 5 bundled seasons get two sparklines + a latest-season
    anchor (e.g., `25-26 → G 48 Pts 138`). Players with one season show
    that season's row. Players with no bundled history get the pace
    fallback. `if a player has less than 5 we can just show the seasons
    they have` — done.
  - proof_lib + tempfile demoted from runtime back to dev-deps. The
    smoke test (`tests/proof_lib_smoke.rs`) keeps the integration
    paved if we re-introduce proof for site dashboard generation.
  - `--dashboards` flag remains as the opt-in toggle for the panel.
  - 9 new L0 tests in `tui::sparkline` (empty input, single value,
    constant series, increasing walk, bucket-when-overflow, negatives,
    real McDavid trend shape, width-clamps-to-input). Dashboard panel
    tests rewritten for the native renderer; total 622 → ~625 tests.
- Phase 8j (cont.): Real player stats in the dashboard panel + CI-ready
  proof pinning.
  - `tui::dashboard_panel` now compiles a per-player proof source
    (name, team, position, G/A/Pts/+/-/PP-Pts/Shots, GP/PPG/Pts-82
    rate stats) and caches by `nhl_id`. Each player's compile happens
    once and the rendered lines are reused on every subsequent frame.
    Long names truncate with an ellipsis; missing rate stats render
    as em-dashes so the layout never collapses.
  - Player screen pulls per-player lines via `lines_for_player(p)`
    instead of the static placeholder.
  - Output stripper now unwraps proof's `<!-- proof:compiled -->`
    markers and ` ```dashboard ` code-fence wrapper so the panel
    shows just the rendered region content.
  - **CI fix**: switched `proof_lib` from `path = "../../proof"` to
    `git = "...", rev = "9c5d456e"`. icelines release builds no
    longer need proof + mdpath checked out as siblings; cargo fetches
    them transitively from GitHub. Local fast-iteration preserved
    via a gitignored `.cargo/config.toml` with `[patch]` overrides
    pointing at the sibling repos. Template at
    `.cargo/config.toml.example`. Updated `design/proof_lib.md` to
    document the pattern.
  - Companion proof commit `9c5d456e` pins mdpath the same way.
  - 8 new L0 tests in dashboard_panel (build-source content, real
    stats render, cache-by-nhl_id, em-dash for missing fields,
    name-truncation helper, plus three strip-unwrap tests covering
    the proof:compiled scaffolding). 619 tests workspace-wide.
- Phase 8j: Proof-compiled dashboard panel — opt-in TUI feature flag.
  - `proof_lib` is now a runtime dependency of icelines-cli. The CLI
    binary always carries the proof code so toggling the flag at
    runtime needs no rebuild. Pinned by local path while pre-1.0.
  - New `--dashboards` global CLI flag, `ICELINES_DASHBOARDS=1` env
    var, and `dashboards = true` config key — same precedence pattern
    as the existing `--no-live` flag (CLI > env > config > default).
    Off by default while the integration matures.
  - `tui::dashboard_panel` module compiles a baked-in
    `*.dashboard.source.md` template via `proof_lib::compile_file`
    (disk roundtrip via `tempfile::tempdir()`, cached on first frame
    via `Arc<Mutex<Option<Vec<String>>>>`). Compile failures fall back
    to a single `[dashboard error]` line — never panics out of render.
  - Player detail screen (`tui::screens::player`) splits to three
    panes when the flag is on AND screen width ≥ 100 cols: headshot
    | stats | dashboard. Below the threshold the layout is unchanged.
  - 4 L0 precedence tests (matches the live-feeds shape), 5 L0
    panel-compile tests (compile + cache + front-matter strip + error
    fallback), 2 L0 player-screen render-guard tests, and 2 L2
    subprocess tests covering `--help` documentation and global flag
    acceptance. 611 tests workspace-wide, all green.
- Phase 8f.9: User schemes load from `~/.icelines/schemes/*.toml`.
  Closes the long-standing Phase-2 TODO. `scheme list` now shows user
  schemes alongside builtins (labelled `user`); `scheme show NAME`
  resolves user schemes first so a `~/.icelines/schemes/yahoo-standard.toml`
  cleanly overrides the builtin. Malformed user files are skipped with
  a warning rather than breaking the listing — `scheme show` still
  errors loud on a malformed exact-name match. SkaterWeights and
  GoalieWeights gain `#[serde(default)]` so partial schemes (only set
  the stats you score) parse without listing every field. 5 L0 tests
  cover the round-trip, override priority, builtin fallback, malformed
  skip, and empty-dir paths (using a process-global Mutex to serialize
  HOME-env mutations).
- Phase 8f.8: `icelines data verify [SEASON|--all]` checks SHA-256
  hashes of installed bundle files against a manifest written at
  install time. Catches partial downloads and post-install tampering.
  `data install` now writes `manifest.json` next to bios.json /
  stats.json (and playoffs.json when present) covering each file's
  SHA-256, season ID, and a versioned schema. Verify reports `✓` per
  clean bundle, `✗` with named mismatches when a file changes, and
  `?` for legacy bundles installed before this manifest existed.
  `--all` walks every installed season. New `to_hex()` helper avoids
  pulling in the `hex` crate. 6 L0 tests (file_sha256, manifest
  roundtrip, tamper detection, missing-file detection, no-manifest
  fallback) + 3 L2 subprocess tests (no-install hint, tampered
  bundle exit, clean bundle success).
- Phase 8f.7: `icelines scheme from-csv` now supports ESPN, Sleeper,
  and Fantrax CSVs in addition to Yahoo. Each platform has a dialect
  with `signatures` (signature columns for auto-detection) and
  `stat_cols` (column → normalized stat-key map). Auto-detection picks
  the dialect with the most signature hits; ties break in declaration
  order (Yahoo first, preserving Phase-5 behavior on ambiguous CSVs).
  New `--platform yahoo|espn|sleeper|fantrax` flag overrides
  auto-detection. Unrecognized headers error with a `--platform` hint.
  Output now includes the detected platform plus column-to-key
  mappings (`G (P) → goals`). New `scheme_dialects` module with 11 L0
  tests + 5 L2 subprocess tests covering autodetect, override,
  unknown-platform, and unrecognized-format paths.
- Phase 8f.6: `icelines group export/import/rename` for portable groups.
  - `group export NAME [--out PATH]` writes one group's members + metadata
    to JSON (default stdout, `--out file.json` for a file). Wire format
    is stable + versioned for future migrations.
  - `group import PATH [--as NEWNAME]` reads back a previously-exported
    JSON file and recreates the group with all members; `--as` lets
    users clone a group under a new name without editing the file.
  - `group rename OLD NEW` updates the group name, carrying members
    via a deferred-FK transaction (sqlite's `defer_foreign_keys = ON`).
    Same-name is a noop; collision errors with a clear message.
  - GroupDb gains `rename_group`, `add_members_bulk`, and
    `group_description` helpers backing the new commands. 5 L1 db tests
    + 3 L2 subprocess tests (export → import roundtrip, rename
    moves members, export-to-stdout).
- Phase 8f.5: `icelines scheme show NAME --source` prints the scheme as
  pretty JSON instead of the human-readable table. Useful for copy/paste,
  diffing two schemes, or piping into `jq`. The default (no flag) still
  emits the readable layout. Scheme already derived Serialize so the
  change is minimal. 1 L2 test verifies valid JSON with name + skater +
  goalie fields.
- Phase 8f.4: `--season YYYYZZZZ` flag on `query leaders/player/compare`.
  Pins the query to a specific bundled season instead of the current one
  — `icelines query leaders --season 20242025 --top 10` shows last
  season's leaders without changing config. Validates against
  `icelines_fetch::BUNDLED_SEASONS` (currently 2021-22 → 2025-26) and
  rejects unknown seasons with a copyable hint listing the bundled IDs.
  Mutually exclusive with `--seasons N` (the multi-season aggregate);
  combining the two errors with a clear explanation. New
  `load_all_players_for_season(Option<&str>)` helper backs all three
  query commands. 3 L0 validator tests + 5 L2 subprocess tests
  (success, unbundled-error, conflict-error, player + compare paths).
- Phase 8c: Historical playoffs bundle. New `playoffs_bundle` module
  defines the `PlayoffsBundle` JSON schema (rounds → series → per-game
  results with optional goal scorers) and a `to_bracket()` conversion
  that drops cleanly into the existing TUI render path. `bundled::
  load_playoffs(season)` resolves installed bundle first then the
  binary-embedded copy. Hand-authored `data/seasons/19931994/
  playoffs.json` ships as the first fixture — full 4-round NYR Cup run
  with per-game results for the Cup Final. `tui::playoffs` now consults
  bundled data before any network call; historical seasons never hit
  the live API. `render_series_body` renders the per-game log when
  present (Game N · date · home N–N away · series-after) and falls
  back to the existing "X game(s) played" hint for live-API series.
  Closes the `Per-game scores + scorers ship with playoffs.json (v2)`
  TODO. 11 L0 tests in `playoffs_bundle` + `bundled`, 5 L0 tests in
  TUI cache + render paths, 2 L1 integration tests covering the full
  load → convert → render chain.
- Phase 8f.2 + 8f.3: snapshot prune + diff
  - `icelines snapshot prune --keep N [--dry-run]` keeps the newest N
    sealed snapshots per tier and deletes the rest. Active snapshot is
    always preserved; drafts are excluded from the keep count. Pair with
    `snapshot gc` to reclaim chunk space. 5 L0 + 2 L2 tests.
  - `icelines snapshot diff <A> <B>` compares two chunked snapshots and
    reports player-level changes (added / removed / changed bios /
    changed stats). O(n) hash-set diff via the chunked layout — exact
    and fast. Legacy snapshots error with a hint to run `rebuild
    --chunked` first. 4 L0 + 1 L2 tests.
- Phase 8f.1: live-feeds toggle — `--no-live` global CLI flag,
  `ICELINES_NO_LIVE` env var, and `live = false` config key all suppress
  NHL API fetches in Scores / Schedule / Playoffs / boxscore + the auto-
  refresh timer. Precedence: CLI > env > config > default(on). When
  disabled, each live tab renders an explicit "Live feeds disabled —
  re-enable with …" message via the standard error path. 4 L0 precedence
  tests + 2 L2 (flag accepted globally, `--help` documents it).
- Phase 8d: `icelines export md <shape>` — writes deterministic markdown
  tables with YAML front-matter for proof DASHBOARD-SPEC consumption.
  Five shapes shipped: `leaders`, `team`, `depth`, `compare`, `roster`.
  `fantasy` and `series` are stubbed with deferred messages (need
  FantasyDb glue and historical playoffs.json respectively). Output
  goes to `~/.icelines/reports/{shape}.md` by default; pass `--out -`
  for stdout. 13 L0 tests + 3 L2 subprocess tests. `export-markdown.md`
  spec status flipped from `Planned` → `Implemented (partial)`.
- Phase 8h: Chunked snapshot store — content-addressed per-player chunks
  with SHA-256 deduplication. New `icelines-fetch::chunkstore` module
  (put/get/exists/delete with sharded layout); `SnapshotStore` extended
  with `write_chunked_stats`, `read_chunked_stats`, `is_chunked`, refs
  table (`chunkrefs.json`), `gc_chunks`, `recompute_refs`,
  `rebuild_chunked` (legacy → chunked migration). Two new CLI ops:
  `icelines snapshot rebuild --chunked <name>` and
  `icelines snapshot gc [--dry-run]`. `bundled::load_*_with_fallback`
  prefers chunked active snapshot then falls back to legacy → bundled.
  25 new tests (12 ChunkStore + 11 chunked snapshot + 2 L2). Storage
  reduction: ~10–15× for daily-cadence snapshots over a season.
- Phase 8b: Scores auto-refresh — live Scores tab polls every 30s. New
  `should_auto_refresh` pure function + `App::tick_auto_refresh` driven
  from the TUI event loop. "Updated Xs ago" indicator in the Scores
  title. Timer arms on tab entry / `t` jump to today, disarms on date
  change, never fires on past dates. 8 L0 + 2 render tests.
- Phase 8a: Test catch-up for previously-shipped features. 27 new tests:
  - **Scouting** (`commands/scouting.rs`) — extracted `validate_format` +
    pure `render_report() -> String`; 7 L0 tests cover all 3 formats,
    section presence, low-GP path; 3 L2 subprocess tests verify exit
    codes and JSON parseability.
  - **Admin overlay** (`tui/app.rs`, `screens/misc.rs`) — 5 L0 keystroke
    tests (capital `F` toggle, Esc closes, Tab blocked, lowercase f
    untouched) + 4 L0 render tests (Idle / Downloading / Done / Error
    phases). Added `InstallState::force_phase` for deterministic test
    drives.
  - **Headshot rendering** (`tui/headshot.rs`) — extracted
    `pixels_to_braille()` + `DOT_X` / `DOT_Y` / `THRESHOLD` constants;
    8 L0 tests cover braille bit layout, threshold contract, cache
    round-trip, loading/error placeholders, Arc-shared clone semantics.
- 10 new specs in `design/specs/` covering previously-homeless features:
  group-management, fantasy-leagues, data-bundles, site-generation,
  scouting-reports, scheme-customization, snapshot-operations,
  tui-admin-overlay, export-markdown (planned), headshot-rendering
  (reference). Specs INDEX updated.
- Phase 7c gap-fix: Scores tab — date navigation (`←/→`), `d` date picker
  (ISO or `MM/DD`), `t` back-to-today, per-game boxscore detail
  (goals/assists, goalies, series context for playoffs), per-date and
  per-boxscore caches
- Phase 7e: Playoffs tab — list-style bracket from `/v1/playoff-bracket/{year}`,
  per-round navigation, per-series detail with summary and "if needed" game
  hints, off-season / error states, `r` retry
- Phase 7d: Schedule tab — weekly view with date navigation, team and matchup
  search (`/SEA`, `/NYR WSH`), team-season detail and head-to-head matchup
  views, per-week + per-team caches with `r` retry, `t` jump to today
- TUI guide (`docs/guides/06-tui.md`) — covers Phase 7a–7e: six-tab nav,
  season time-travel, Scores, Schedule with search and matchups, Playoffs
  bracket and series detail
- `icelines query compare --comps` — contract comparable finder (in progress)
- Season data expansion to 2000-01 (in progress)

---

## v1.0.0 — 2026-04-26

IceLines v1: migrated from C:\src\NHL\fantasy-tracker to C:\src\icelines.
Clean repo structure matching proof/mdpath conventions.

### Architecture
- 4-crate Rust workspace: icelines-core, icelines-fetch, icelines-site, icelines-cli
- 5 seasons bundled in binary (20212022–20252026, ~4.3MB total)
- PlayerRepository — single authoritative data loading API
- 338 tests: L0 unit, L1 integration, L2 system + mock NHL API fixture

### Data pipeline
- NHL API client: bios, stats, realtime, rosters, contracts, schedule
- MoneyPuck xG/CF%/FF% integration (silo'd, optional)
- NHL realtime stats: hits, blocked_shots, giveaways, takeaways, PIM
- Snapshot store with SHA-256 integrity, provenance chain, tiered architecture
- Contract data: expiry_year, expiry_type (UFA/RFA/ELC)

### Player model
- 50+ fields covering all-situations, PP, SH, shot metrics, physical, bio, draft, contract
- Multi-season aggregate (`--seasons N`) across bundled history
- Y/Y improvement sort (`--sort improvement`)
- Duplicate player dedup (NHL API emits multiple rows for traded players)

### Commands
- `icelines fetch` — rosters, stats, realtime, positions, contracts, moneypuck
- `icelines query leaders/player/compare` — 30+ sort metrics, percentiles, JSON/CSV
- `icelines fantasy` — SQLite leagues/teams, scoring, trade simulation, axum HTTP server
- `icelines rank/team/players/history/project/scouting/mates/peers/class/compare`
- `icelines group/scheme/snapshot/data/tui/tonight/schedule`
- `icelines build/serve/deploy` — mkdocs static site

### Repo process
- CLAUDE.md — session context, crate ownership, rules
- CODEBASE.md — where to write code, full module map
- design/ — specs, plans, invariants, pitfalls
- docs/ — generated output, team pages
- .roles/ — 8 domain review roles
- design/plans/INDEX.md, design/specs/INDEX.md
