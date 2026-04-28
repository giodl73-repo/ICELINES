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
| [cache-model.md](cache-model.md) | Implemented | Snapshot store, tiers, integrity, provenance |
| [fantasy-scheme.md](fantasy-scheme.md) | Implemented | Scheme engine, weights, compute_fantasy_score |
| [player-analysis.md](player-analysis.md) | Implemented | PlayerFilter, similarity search, career arc |
| [projection-engine.md](projection-engine.md) | Implemented | pace/regressed/composite modes, age_factor |
| [position-engine.md](position-engine.md) | Implemented | PositionResolver, boxscore eligibility |
| [query-engine.md](query-engine.md) | Implemented | query leaders/player/compare, 30+ sort metrics |
| [test-strategy.md](test-strategy.md) | Implemented | L0/L1/L2 tiers, mock NHL API fixture |

---

## TUI Specs

| Spec | Status | Summary |
|------|--------|---------|
| [tui.md](tui.md) | Implemented | v1 as-built: 8 tabs, all current screens, key bindings |
| [tui-v2.md](tui-v2.md) | Draft | v2 redesign: 6 tabs, merged screens, admin overlay |
| [depth-chart.md](depth-chart.md) | Implemented (partial) | Cross-team line value rankings + team depth grid |
| [scores.md](scores.md) | Draft | Live game scores, date navigation, game detail |
| [schedule.md](schedule.md) | Draft | Season schedule, team filter, matchup search |
| [playoffs.md](playoffs.md) | Draft | Bracket, series tracker, historical Stanley Cup runs |
| [season-timetravel.md](season-timetravel.md) | Draft | Global season picker, 38-season navigation |
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
| tui-v2.md needs review before implementation | tui-v2.md | Review with team before coding |
