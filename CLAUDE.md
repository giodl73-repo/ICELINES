# IceLines — Claude Code Context

**Project**: IceLines NHL analytics + fantasy/workbench platform
**Binary**: `icelines` (Rust CLI/TUI plus web dashboard/API)
**Repo root**: `C:/src/ICELINES/`
**Working dir: `C:/src/ICELINES/` (workspace root)

---

## ⚠ Git identity — verify BEFORE every commit

**This repo's commits go to the `giodl73-repo` GitHub account, NOT the work account.**

Required local config (already set):
- `user.name = giodl73-repo`
- `user.email = giodl73@gmail.com`

**Before any `git commit`, run:**
```bash
git config user.email
```
If it returns anything other than `giodl73@gmail.com` (especially anything `@microsoft.com`), STOP. Do not commit. Set it:
```bash
git config user.email giodl73@gmail.com
git config user.name "giodl73-repo"
```

If git ever prints `Your name and email address were configured automatically based on your username and hostname` after a commit, that means the local config is missing — the commit just got authored as the auto-resolved identity (which is `giodl@microsoft.com` on this machine). Fix immediately, then `git commit --amend --reset-author --no-edit` to fix the most recent commit, or filter-branch for a chain.

This applies to **every commit, every session**. Do not assume the config persists; verify each session.

---

## Crate ownership — where to write code

| What you're adding | Crate | Why |
|--------------------|-------|-----|
| Data types, Player struct, filters, scheme scoring, projections | `icelines-core` | Pure logic, no I/O, no network |
| NHL API fetch, snapshot store, bundled data, MoneyPuck, aggregate | `icelines-fetch` | All I/O and data loading |
| Web HTML/API routes, templates, server state | `icelines-web` | Browser/API adapters |
| CLI commands, argument parsing, TUI | `icelines-cli` | Thin UI layer only |
| Deferred mkdocs/static site generator | `icelines-site` | Historical/deferred site-only concerns |

**Rule**: Business logic belongs in `icelines-core` or `icelines-fetch`, never in `icelines-cli`. CLI commands call library functions — they don't compute anything themselves.

**Crate dependency chain** (lower can't import higher):
```
icelines-core
icelines-query  (depends on icelines-core)
icelines-fetch  (depends on icelines-core, icelines-query)
icelines-web    (depends on core/query/fetch)
icelines-site   (deferred static-site generator)
icelines-cli    (thin command/TUI/server launcher)
```

## VTRACE specification baseline

`docs/vtrace/` is the governing mission, requirements, design, interface,
verification, validation, work-package, review, and change-control baseline.
Use it before implementation work, and keep feature claims aligned with
`design/specs/surface-parity.md`.

---

## Common commands

```bash
# Build
cargo build                          # debug
cargo build --release -p icelines-cli  # release binary

# Test
cargo test                           # all crates
cargo test -p icelines-core          # one crate
cargo clippy -- -D warnings          # must be clean
cargo fmt --check                    # must be clean

# Run (after release build)
target/release/icelines.exe query leaders --pos C --top 10
target/release/icelines.exe fantasy league-create "My League"

# Release gate
powershell -ExecutionPolicy Bypass -File scripts/release-smoke.ps1
# Full checklist: design/release-checklist.md

# proof — documentation linting and guide compilation
# proof is a sibling repo at C:/src/proof/. Build it from there:
#   cd C:/src/proof && cargo build
# Binary lands at C:/src/proof/target/debug/proof.exe (Windows) or .../proof (Unix).
# scripts/build-guides.sh resolves the binary automatically.
C:/src/proof/target/debug/proof check .            # lint all markdown
C:/src/proof/target/debug/proof check . --errors-only   # errors only
bash scripts/build-guides.sh                       # compile src/guides/ → docs/guides/
bash scripts/build-guides.sh --check               # validate without writing
```

---

## Key constants and files

- **Current season**: `icelines_core::CURRENT_SEASON = 20_252_026` — change here each October, nowhere else
- **Bundled data**: `src/icelines-fetch/src/bundled.rs` — **38 seasons** (1987-88 through 2025-26, except 2004-05 lockout) embedded via table-driven `include_bytes!()`. `BUNDLED_SEASONS` is the full list; `MODERN_BUNDLED_SEASONS` is the 5-season subset that carries the full Tier-1 report suite.
- **Player loading**: always use `icelines_fetch::stats_loader::load_into_repo(season, season_type, store)` → returns `LoadOutcome { repo: StatsRepository, missing }`. Never reach into snapshot store directly from a command; legacy repository loaders were deleted in Hart.5b1.
- **Per-player career fan-out**: `icelines_fetch::stats_loader::load_player_career_into_repo(repo, pid)` walks every bundled season and merges that player's bios+stats into the repo. Used by the TUI lazy loader (UX.1) and the CLI's historical-name fallback in `query player`/`compare`.
- **Resolve historical name**: `icelines_fetch::stats_loader::resolve_player_id_by_name(name)` walks bundled bios+goalies for a partial name match. The CLI uses this so `query player Wayne Gretzky` resolves without `--season`.
- **Snapshot store**: `~/.icelines/snapshots/` — never hardcode paths, use `Config::load()?.snapshot_dir()`
- **Config**: `~/.icelines/config.toml` — carries `[reports]` section (`realtime`/`timeonice`/`goals_for_against`/`goalie_advanced`/`goalie_saves_by_strength` toggles). Persisted by the TUI Reports overlay (R key) via `Config::save_reports()`. **Phase Foster** added a `[sync]` section + `[sync.capabilities]` matrix (6 capabilities × 3 modes; `shifts=off` enforced). Read/write via `icelines config get/set/list/reset` or `Config::save_sync()`. Defaults — `stats=league`, `scores_schedule=league`, `transactions=favorites`, `boxscores=favorites`, `shifts=off` (locked), `career_history=favorites`.
- **SQLite DB**: `~/.icelines/icelines.db` — shared by GroupDb and FantasyDb
- **Repo LRU cap**: `load_into_repo` constructs `StatsRepository::with_lru_cap(80)` so a downstream lazy career fan-out (38 seasons × 2 types ≈ 76 windows) doesn't evict the active season. The legacy default of 8 only covered current-era queries.

---

## Testing tiers

| Tier | Location | Rule |
|------|----------|------|
| L0 unit | `#[cfg(test)]` inside each `.rs` file | Pure logic, no I/O, microseconds |
| L1 integration | `src/icelines-fetch/tests/` | Real structs, no network, tempdir |
| L2 system | `src/icelines-cli/tests/system_tests.rs` | Invokes compiled binary as subprocess |

**Every new feature needs at least L0 tests. New commands need L2 tests.**

The mock NHL API fixture is at `src/icelines-fetch/tests/mock_nhl_api.rs` — use `httpmock` there, not in L0 tests.

---

## Architecture rules

1. **No live network calls in tests** — all L1/L2 tests use bundled data or httpmock
2. **No season literals** — use `CURRENT_SEASON` / `CURRENT_SEASON_STR`, not `"20252026"`
3. **Dedup players by nhl_id** — the NHL bios API emits multiple rows for traded players; `StatsRepository::upsert_*` deduplicates but stay alert (the bundle itself can carry duplicates — see persona_wave3 p238)
4. **Option<T> for all nullable API fields** — `shooting_pct`, `toi_per_game_sec`, `faceoff_win_pct`, `realtime.pim` etc. are null in real data
5. **MoneyPuck is silo'd** — all MoneyPuck code lives in `icelines-fetch/src/moneypuck.rs`; removing it only requires deleting that file and the Option fields on Player
6. **CLI commands are async** — all `run()` functions are `async fn` dispatched by `tokio::main`
7. **Filter aliases live in `StatId::from_cli_key`** — `g`→`goals`, `p`→`points`, `gp`→`games`, `ppg`→`points-per-game`, `blk`→`blocked-shots`, `tk`→`takeaways`, `gv`→`giveaways`, `pen`→`pim`, `+/-`→`plus-minus`, `sv%`→`save-pct`, plus uppercase-insensitive (`HITS`→`Hits`). Adding a new alias is one match arm; the alias must resolve to an existing `cli_key()`.
8. **Goalie filter rewrite** in `query goalies` — `gp`/`games` rewrites to `goalie-games`, `starts` to `goalie-starts` BEFORE `parse_filter` runs. See `goalie_filter_rewrite` in `icelines-cli/src/commands/query.rs`.
9. **`age` is a flag, not a StatId** — surfaced as `--age-min N` / `--age-max N` on `query leaders`. Don't try to add it to the catalog; it's bio data, not a stat.
10. **Filter.OR — boolean filter grammar** — `parse_filter_expr` returns a `FilterExpr` (Atom / And / Or / Not). The CLI tries the boolean parser first; bare atoms route to `PlayerFilter::stat_filters` (preserves Min+Min normalization), compound expressions route to `PlayerFilter::expr_filters`. `apply_views` ANDs both. To extend the grammar, edit the recursive descent in `parse_filter_expr` (precedence: NOT > AND > OR). Tests live alongside in `stats_catalog.rs::tests::l0_filter_expr_*`.

---

## What's been built

- `icelines fetch` — NHL API data pipeline (bios, stats, realtime, rosters, contracts)
- `icelines query leaders/player/compare/goalies` — full query engine: 30+ sort metrics, `--filter` catalog grammar with short aliases, `--seasons N` aggregate (1-38), historical name resolution, percentiles, JSON/CSV export
- `icelines fantasy` — full fantasy league (SQLite, scoring, trades, axum HTTP server)
- `icelines rank/team/players/history/project/scouting/mates/peers/compare/class`
- `icelines export md` — markdown data tables (Phase 8d, shipped)
- `icelines x` — quick CSV/JSON export of any report shape (mirrors leaders/goalies/rank/players/class/history/peers/compare/transactions)
- `icelines tui` — ratatui interactive dashboard. Six tabs (League / Depth / Stats / Goalies / Scores / Schedule + Playoffs + Transactions overlays). Key bindings: `Tab`/`Shift+Tab` cycle screens, `y` season picker, `R` Reports overlay, `Shift+P` season-type toggle, `o` section toggle on Queries, `[` / `]` cycle career-table presets, `/` open sort picker
- **Reports overlay (R)** — toggles which Tier-1 reports populate columns (realtime, timeonice, goalsForAgainst, goalie-advanced, goalie-savesByStrength). Persists to `~/.icelines/config.toml`. Disabled reports drop their columns from career tables / sort picker / query output. See `design/specs/stat-catalog.md` and `Reports.1`-`Reports.7` in this codebase.
- **Lazy career loader (UX.1)** — opening a player card fans out across all 38 bundled seasons, pulling that player's career into the repo. ~50 ms per first open, cached after.
- **38-season bundle (L.7b)** — `BUNDLED_SEASONS` covers 1987-88 → 2025-26. Binary 56 MB. Adding a season: drop files into `data/seasons/YYYYZZZZ/` and add the row to each lookup table in `bundled.rs`.
- **Phase Calder — multi-league career history**: `icelines fetch career --bundled-seasons 5` walks the NHL landing endpoint for ~1,650 active-roster pids and writes `~/.icelines/career_history.json` (~30 MB compact). Surfaced on player card (CLI + TUI + Web) as the "Pre-NHL career" section, on scouting reports as the "Development arc" line, and via `icelines query career --league OHL --season 20142015` cohort leaderboards (CLI + `/career` web route + `/api/v1/career` JSON twin). Data path: `icelines-core::career_history` (types) + `icelines-fetch::career_landing` (parser, batch fetcher, store). Intentionally NOT bundled — see commit `72c851bd` for the lazy-vs-bundle tradeoff.
- **Phase Foster — favorites dashboard, time-travel, unified data layer**: Five sub-phases shipped on top of a new data architecture (Foster.0).
   - **F.0 — Data architecture**: `icelines-core::entity::EntityRef` (stringly-typed `player:8478402` / `team:EDM` / `game:2025020001`); `icelines-core::freshness` (`Freshness` + `Clock` trait + `MockClock` for tests); sharded `Manifest` per `DataKind` under `~/.icelines/data/manifest/<kind>.json` with version-floor refusal + atomic tmp+rename writes; `DataStore` routing manifest → bundle → lazy fetch → `NotInstalled` (with stderr banner on lazy fetch); snapshot read-shim for `~/.icelines/snapshots/` (`data/seasons/` always wins); migration 006 collapses `group_members.{kind, player_normalized}` into a single `entity_ref` column (idempotent); typed capability matrix in `[sync.capabilities]` (6 caps × 3 modes; `shifts=off` enforced with the literal BENCH-H3 error); `icelines config get/set/list/reset` and `icelines setup [--accept-defaults]` wizard.
   - **F.1 — Time axis**: `--date YYYY-MM-DD` on `tonight` and `schedule`; `Shift+D` opens a shared TUI date picker overlay on Tonight/Schedule (Playoffs reuses the season picker); web `/scores?date=` and `/schedule?date=` accept the same convention.
   - **F.2 — Favorites dashboard schemas**: `icelines-core::favorites` with distinct `SkaterNightLine` / `GoalieNightLine` schemas (SCOUT B1), `DnpReason` enum, `gate_finalized` (drops NHL API mid-game zero-defaults), `primary_goalie` multi-goalie picker (SCOUT H5); `icelines-core::timeframe::Timeframe { Day, Week, Month, Season }` with `range(date)`; `icelines favorites [--date] [--range] [--group] [--json]` CLI surface (renders empty-state today; per-night lines wire in F.3+).
   - **F.3 — EventStream + boxscore-event upsert**: SQLite migration 007 with PK `(date, entity_kind, entity_key, event_kind, event_id)` and dedup via `INSERT … ON CONFLICT DO UPDATE`; `icelines-core::event_stream` ships frozen v1 payload schemas + event_id formatters (e.g. trade dedup via alphabetic team sort); `icelines fetch boxscore [--date] [--for-favorites] [--dry-run]` writes a `score` event per game.
   - **F.4 — Sync engine**: non-blocking `launch_eager_sync(Arc<DataStore>) → Option<mpsc::Receiver<SyncEvent>>` (returns `None` under `ICELINES_TEST_MODE=1`); each refresh runs in `spawn_blocking` so sync HTTP doesn't pin the executor; `icelines fetch sync [--dry-run] [--force]` CLI surface walks the manifest via `enumerate_stale`.
   - **F.5 — Windowed filter atom**: `WindowedAtom` extends the filter grammar to `<stat-key>[.<window>]<op><value>` (e.g. `g.week>=10`); `query career --week / --month` rejected with the literal EDGE B2 error.
   - Test budget: ~167 new tests across the five sub-phases.
- **Phase Art Ross — query system rewrite (centerpiece)**: Unified `parse_query → Constraint::matches` pipeline replacing the legacy `parse_filter_expr → FilterExpr::matches` path. Lives in `icelines-query::{plan, parser, executor, planner, sliding_window, data_provider, input, errors, tokenizer}`. Five sub-phases shipped:
   - **A.0 — IR + planner**: n-ary `Constraint::All(Vec)/Any(Vec)/Not(Box)` IR + typed `Predicate { Scalar, Member, Pattern, Range }` (shape-by-construction). `parse_query(FilterInput) -> Result<QueryPlan, Vec<ParseError>>` is the front door. `DataProvider` trait owned by query (the dependency-inversion seam). `EvalCtx` is `!Send`-pinned via compile_fail doctest.
   - **A.1 — Grammar expansion**: `<` `>` `!=` `IN (...)` `NOT IN` `BETWEEN x AND y` `LIKE "pat"` `~` `!~` operators. New bio atoms: `pos=C`, `team=EDM`, `team.any=EDM`, `draft-round<=2`, `draft-overall<=10`, `birth-state=ON`, `nationality=USA`, `rookie-season>=20212022`. Quoted strings inside IN/LIKE.
   - **A.2 — Sliding-window atoms**: `<stat>.last<N><unit>` where unit is `g`/`d`/`w`/`m`. Optional scope modifiers `.allteams` / `.career`. `WindowPolicy::{RequireFull, AllowPartial, AllowPartialAbove(N)}`. Mid-season-trade aware. `IcelinesProvider::fetch_game_lines` walks the boxscore manifest.
   - **A.3 — Historical EVER + AT-age**: `p.career>=500` / `p.streak>=15` / `g.any10g>=5 EVER` / `g.seasons-with>=5`. Optional `AT age<=N` modifier (scalar or `BETWEEN`-form). Intra-season only, axis-typed, lockout 2004-05 skipped per spec. HR Feb-1 age convention via existing `compute_age`.
   - **A.4 — Cross-league career atoms**: `league=OHL` / `league NOT IN (NHL)` / `league.tier=Junior`. Stat-aggregate 3-dot keys: `p.career.junior>=200`, `p.career.nhl>=500`, `p.career.ohl>=300`. `IcelinesProvider::fetch_career_history` reads `~/.icelines/career_history.json`. Uses canonical Phase Calder `LeagueTier` classification.
   - **A.5 — `--explain` flag**: `icelines query leaders --filter X --explain` prints the parsed plan tree + data requirements without running the query. Pair with `--json` for the `explain.v1` envelope (frozen v1).
   - **Hybrid CLI wiring**: filters whose IR contains `SlidingWindow`/`CareerAggregate`/`CareerLeague` (`Constraint::needs_provider() == true`) route through the new pipeline; legacy filters continue through `parse_filter_expr → apply_views` unchanged. v0.19.1 query results are preserved bit-for-bit.
   - **Test budget**: ~424 new tests across the phase. All Phase Art Ross gates green: Wave 11 (201) + A.0 parity (4) + A.2 executor (11) + A.3 career (10) + A.4 league (11) + A.5 explain (12) + Wave 12 (200 adversarial scenarios on the new grammar).
- **Phase Conn Smythe — live playoff tracking**: Three sub-phases on top of Foster's rails.
   - **C.1 — Series momentum**: `icelines-core::series_momentum::SeriesMomentum` schema (leader, OT count, last_result, home_advantage in 2-2-1-1-1 format) + `icelines-fetch::series_momentum_builder::compute_series_momentum` projection from `PlayoffSeries`. CLI surface: `icelines playoffs --series A [--season YYYYZZZZ]`. Renders summary line + last-game result + next-game venue.
   - **C.2 — Cup-run player narratives**: `icelines-core::playoff_run::PlayoffRunSummary` schema with skater + goalie aggregates (W-L-OTL, SV%, GAA, shutouts). `icelines query leaders --playoff` walks the Boxscore manifest filtering on `gameType=3` and aggregates by PID. JSON envelope mirrors `query leaders --week`.
   - **C.3 — Live game tracking surface**: `icelines-core::live_game::LiveGameDetail` schema + new web `/game/:id` route. Live HTML page with scoreboard, goalie table (PID-linked to player cards), goal summary, top-5 skater rows per team, auto-refresh meta-tag every 30s when `state ∈ {LIVE, CRIT, PRE}`.
- **Phase Norris — TUI architecture refactor (v0.21.0)**: Pure internal refactor extracting per-screen state structs out of the 3,800-line App god-object. No keybind change, no UX delta. Six state structs now live alongside their renderers:
   - `tui::screens::queries::QueriesState` (Norris.1, 17 fields) — the centerpiece, holds all `query_*` / `sort_picker_*` / `career_table_preset` state.
   - `tui::screens::schedule::ScheduleScreenState` (Norris.2, 8 fields) — week + caches + search filter. Suffixed `Screen` to disambiguate from the existing `tui::schedule::ScheduleState` load-state enum.
   - `tui::screens::transactions::TransactionsState` (Norris.3, 8 fields) — rows + filters + cursor. App field is `app.txs` (not `app.transactions`) to avoid substring overlap with the legacy `transactions_*` field names.
   - `tui::screens::goalies::GoaliesState` (Norris.4, 3 fields) — sort + min_gp + cursor.
   - `tui::screens::playoffs::PlayoffsScreenState` (Norris.4, 3 fields) — cache + round + series.
   - `tui::screens::misc::TonightScreenState` (Norris.4, 4 fields) — caches + date sentinel + cursor.
   - **Cross-screen state stays on App**: screen discriminator, repo, active_season, status, overlays (show_help/show_admin/show_season_picker/show_reports_overlay/group_picker_*), reports config, picker scaffolding (`scores_picker_*` + `picker_target` are shared between Tonight and Schedule).
   - **Test pattern**: each `<Screen>State` ships with ~10 L0 default-contract tests in its module's `norris_state_tests` + 3 L1 sequencing tests in `tui::screens::app_snapshot_tests`. No L2 (TUI is interactive; subprocess can't drive keystrokes). Test growth: 705 → 749 (+44 across the phase).
   - **Trophy fit**: best defenseman, "foundational, structural, both-ends work" per the picking guide. Spec: `design/specs/phase-norris-overview.md`. Plan: `design/plans/2026-05-07-phaseNorris-tui-state-extraction.md`.
- **Phase Masterton — TUI chrome + standalone mode + Screen trait scaffold (v0.22.0)**: Two user-facing features + scaffolding for a deeper future refactor.
   - **M.1 — declarative chrome**: each main screen exports a `chrome()` accessor returning `ScreenChrome { title, keybinds }`. The shell renders a consistent header (tabs + right-aligned title at ≥120 cols) + footer (keybind chips + GLOBAL_KEYBINDS, with transient flash priority). Replaces the imperative `app.status = "..."` pattern for permanent state hints. Lives in `tui/chrome.rs`.
   - **M.2.1 — Screen trait scaffold**: `Screen` trait (`type State`, `handle/render/chrome`), `ScreenAction` enum (Continue/Quit/Push/Pop/Replace/OpenOverlay/Flash), `OverlayKind` enum (Help/Admin/SeasonPicker/Reports/Docs/DatePicker/GroupPicker — cross-screen overlays only), `AppContext<'_>` (split-borrow context), `App::dispatch` interpreter, `App::make_context` factory. Lives in `tui/screen.rs`. Module-level `#[allow(dead_code)]` because the deep per-screen migration (Masterton.2.2-2.7) is deferred — see CHANGELOG v0.22.0 for honest framing.
   - **M.3 — `--standalone` flag**: `icelines tui <surface> --standalone` locks the TUI to one screen. Tab/Shift+Tab no-op; tab strip hidden; per-screen keybinds + cross-screen overlays still work. Implemented as `App::locked_screen: Option<Screen>` (pragmatic approach — gets the user-facing feature without requiring the deep Screen-trait migration). Examples: `icelines tui goalies --standalone`, `tui scores --standalone`, `tui transactions --standalone`.
   - **Trophy fit**: Bill Masterton — perseverance, dedication to hockey, long-term unglamorous infrastructure. Same defensive character as Norris. Spec: `design/specs/phase-masterton-overview.md`. Plan: `design/plans/2026-05-08-phaseMasterton-tui-screen-trait.md`. Test growth: 763 → 803 (+40 across the phase).
- `docs/vtrace/` — VTRACE specification baseline and work-package evidence spine
- `icelines serve` — axum web dashboard/API; mkdocs CLI entry points were removed
- ~2050 tests across L0/L1/L2 + 4 persona-scenario waves (`persona_scenarios.rs` + `persona_wave2.rs/wave3/wave4`) plus `persona_foster.rs` (30 scenarios) including mock NHL API fixture

## Pending (see design/plans/INDEX.md)
- NHL EDGE skating-speed summary stats remain blocked on a supported public endpoint. Goal-level movement is separately available through Gamecenter landing `pptReplayUrl` and `icelines fetch goal-visualizer`; do not conflate the two sources.
- Fantasy daily delta scoring

---

## Roles

Eight domain roles in `.roles/` review from different angles:
- **scout** — player analysis correctness
- **tape** — data pipeline integrity  
- **forge** — Rust code quality and safety
- **edge** — query engine and filter logic
- **bench** — test coverage and quality
- **glass** — TUI and rendering
- **pace** — performance and algorithmic efficiency
- **wire** — API contracts and schema evolution

Run `/review-specs` to invoke all roles on a spec or implementation.

---

## Documentation surface for users + AIs

If a user / future AI lands on this repo and needs to learn the CLI, the canonical entry points are:

1. **`COMMANDS.md`** — single-page reference: every subcommand with examples, the catalog `--filter` grammar, the short alias table (`g`/`p`/`gp`/`ppg`/`blk`/...), the TUI keybind matrix.
2. **`README.md`** — installation + usage primer with copy-paste examples.
3. **`icelines --help`** — clap auto-generated help; top-level + per-subcommand `long_about` carries inline examples.
4. **`design/specs/stat-catalog.md`** — the StatId catalog spec with all 108 stats, categories, units, report sources.

When updating the CLI surface (new flag, new subcommand, new keybind, new alias) update `COMMANDS.md` AND the relevant clap `long_about` in the same change. The release artifact ships with `--help` text; if it's not in --help or COMMANDS.md, the user has no way to discover it.
