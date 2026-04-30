# IceLines Specs Index

Specs define what we're building. Each spec covers one feature or screen.

**Status legend:**
- `Implemented` — matches the running build
- `Implemented (partial)` — core is built; spec notes gaps
- `Draft` — design is complete; not yet built
- `Planned` — placeholder; spec not yet written

---

## Core Engine Specs

| Spec | Status | Summary |
|------|--------|---------|
| [rust-cli.md](rust-cli.md) | Implemented | 4-crate workspace, CLI commands, fetch pipeline |
| [data-sources.md](data-sources.md) | Implemented | NHL API endpoints, bundled data, MoneyPuck silo |
| [data-bundles.md](data-bundles.md) | Implemented | `data install/list/remove`, GitHub Releases, storage layout |
| [cache-model.md](cache-model.md) | Implemented | Snapshot store, tiers, integrity, provenance |
| [snapshot-operations.md](snapshot-operations.md) | Implemented | `snapshot list/show/use/verify/delete` CLI |
| [fantasy-scheme.md](fantasy-scheme.md) | Implemented | Scheme engine, weights, compute_fantasy_score |
| [scheme-customization.md](scheme-customization.md) | Implemented | `scheme list/show/fromcsv` CLI, user vs built-in |
| [fantasy-leagues.md](fantasy-leagues.md) | Implemented | Fantasy SQLite schema, scoring, trade eval, axum server |
| [group-management.md](group-management.md) | Implemented | Player watchlists: SQLite, CRUD, TUI g/f keys |
| [player-analysis.md](player-analysis.md) | Implemented | PlayerFilter, similarity search, career arc |
| [scouting-reports.md](scouting-reports.md) | Implemented | 8-section player report (terminal/markdown/json) |
| [projection-engine.md](projection-engine.md) | Implemented | pace/regressed/composite modes, age_factor |
| [position-engine.md](position-engine.md) | Implemented | PositionResolver, boxscore eligibility |
| [query-engine.md](query-engine.md) | Implemented | query leaders/player/compare, 30+ sort metrics |
| [site-generation.md](site-generation.md) | Implemented | mkdocs build/serve/deploy, Tera-free template |
| [export-markdown.md](export-markdown.md) | Planned | `export md` for proof DASHBOARD-SPEC bridge |
| [goalies.md](goalies.md) | Draft | Goalies as first-class players: GoalieStats schema, GoalieRepository, dedicated TUI tab, team-card slots, query goalies, fantasy goalie scoring |
| [test-strategy.md](test-strategy.md) | Implemented | L0/L1/L2 tiers, mock NHL API fixture |

---

## TUI Specs

| Spec | Status | Summary |
|------|--------|---------|
| [tui.md](tui.md) | Implemented | v1 as-built: 8 tabs, all current screens, key bindings |
| [tui-v2.md](tui-v2.md) | Implemented | v2 redesign: 6 tabs (Phase 7a–e shipped) |
| [tui-admin-overlay.md](tui-admin-overlay.md) | Implemented (basic) | `F` overlay, install status, planned `:` prompt |
| [depth-chart.md](depth-chart.md) | Implemented (partial) | Cross-team line value rankings + team depth grid |
| [scores.md](scores.md) | Implemented | Live game scores, date navigation, game detail (Phase 7c + gap-fix) |
| [schedule.md](schedule.md) | Implemented | Season schedule, team filter, matchup search (Phase 7d) |
| [playoffs.md](playoffs.md) | Implemented | Bracket, series tracker (Phase 7e); historical bundles deferred |
| [season-timetravel.md](season-timetravel.md) | Implemented | Global season picker, 38-season navigation (Phase 7b) |
| [headshot-rendering.md](headshot-rendering.md) | Implemented (reference) | Braille dither algorithm for player headshots |
| [dashboard-engine.md](dashboard-engine.md) | Draft | proof DASHBOARD-SPEC integration (deferred) |

---

## Reading Order

**To understand the current build**: `tui.md` → `depth-chart.md` → `query-engine.md`

**To understand what's being built next**: `tui-v2.md` → `scores.md` → `schedule.md` → `playoffs.md` → `season-timetravel.md`

**To understand the data layer**: `data-sources.md` → `cache-model.md` → `rust-cli.md`

---

## Spec Health

Last audit: 2026-04-28

| Issue | Spec | Action needed |
|-------|------|---------------|
| tui.md was pre-implementation, now stale | tui.md | ✓ Updated to reflect v1 as-built |
| No spec for depth chart algorithm | — | ✓ Created depth-chart.md |
| No spec for new live screens | — | ✓ Created scores.md, schedule.md, playoffs.md |
| No spec for time-travel feature | — | ✓ Created season-timetravel.md |
| 8 homeless features (groups, fantasy, etc.) | — | ✓ Created 7 new specs + 3 reference docs |
| Phase 7 implementation done | tui-v2.md, scores.md, etc. | ✓ Status flipped from Draft to Implemented |
