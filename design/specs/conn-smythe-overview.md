# Phase Conn Smythe — Overview

**Trophy**: Conn Smythe Memorial Trophy (playoff MVP — "the player adjudged most valuable to his team during the playoffs")
**Version**: 1.0 (initial)
**Date**: 2026-05-06
**Status**: Spec — ready for implementation
**Plan**: `design/plans/2026-05-06-phaseConnSmythe-overview.md`

---

## Vision in one paragraph

Phase Foster shipped the rails — EventStream, sync engine, favorites
surfaces, capability matrix, per-night stat lines. Conn Smythe is
what those rails were built for: live playoff tracking. Three things
the user can't do today that they can after Conn Smythe:

1. **Series momentum**: open a playoff series and see the live
   state — "BOS leads 3-2 · won G5 in OT · FLA forced G6 at home" —
   instead of a static bracket.
2. **Cup-run player narratives**: a favorited player gets a "Playoff
   run: 12G 18A 30P in 14 GP" line on their card, and the Playoffs
   tab gets a leaderboard of postseason point producers.
3. **Live game surface**: in-progress games get a period-by-period
   detail view (pulled goalies, game-state, last goal, time
   remaining when the API exposes it).

NHL playoffs are mid-flight as of 2026-05-06 → the data is live and
the timing is perfect.

## Locked decisions

| Decision | Choice | Rationale |
|---|---|---|
| Playoff filter axis | `game_type=3` from the schedule endpoint | Same field already drives `is_playoff()` on `ScheduledGame` |
| Series identity | NHL API's series letter (A–H) + season | Stable across rounds; the endpoint already exposes it |
| Momentum window | Per-series, no aggregation across rounds | Round-by-round is the natural narrative unit |
| Cup-run aggregation | Read from Boxscore manifest entries with date ∈ playoff window | Reuses Foster +3 persistence + Foster +26 windowed-leaders shape |
| Live game polling | Existing 30s auto-refresh on Scores tab; Conn Smythe doesn't add a new poller | Don't multiply pollers; share the existing live channel |
| Surface coverage | CLI ✅ TUI ✅ Web ✅ on all three sub-phases | Phase Foster proved the three-medium discipline |

## Sub-phase ordering

```
C.1 ─── Series momentum
              │
              └─→ C.2 ─── Cup-run player narratives
                            │
                            └─→ C.3 ─── Live game tracking surface
```

C.1 is foundational — series state is what every other Conn Smythe
view needs. C.2 layers per-player aggregates on top of the playoff-
window filter. C.3 is the highest-visibility, ships last so it
composes against C.1 and C.2.

## Out of scope (deferred)

- **Conference / Cup Final probability models** — interesting but
  needs a stat-prediction component (xWin% etc.) that doesn't exist
  yet. Phase candidate: future Conn Smythe extension or a separate
  prediction phase.
- **Historical Cup-run leaderboards** — the bundled bios go back to
  1987, but per-game playoff stats only persist via Foster +3 going
  forward. Historical aggregates would need a backfill pass.
- **Series predictions / "x wins, y goals expected"** — same as
  above; modeling work, not data engineering.
- **Playoff-bracket auto-update from EventStream** — manageable, but
  the existing `fetch_playoff_bracket` endpoint already serves the
  bracket. EventStream-derived bracket is a polish item.

## Surface coverage matrix

| Capability | CLI | TUI | Web |
|---|---|---|---|
| Series momentum view | `icelines playoffs --series A` | Click into series on Playoffs tab | `/playoffs?series=A` |
| Cup-run leaderboard | `icelines query leaders --playoff [--top N]` | New tab section on Playoffs | `/playoffs?leaders=true` |
| Per-player playoff aggregate | `icelines query player NAME --playoff` (or part of card) | Player card sub-section | `/player/:id` (sub-section) |
| Live game surface | `icelines tonight --game GID --detail` | Click into game on Scores tab | `/game/:id` |
| Live banner during refresh | (CLI: stderr line) | Sync banner widget (Foster +10) | Visible in fetch_error template slot |

## Sub-phase summaries

### C.1 — Series momentum (~2 days)

`icelines-core::series_momentum` module with `SeriesMomentum` struct
(games_played, leader, lead_changes, OT-game count, last_result,
home_advantage). Pure projection from a series's score events.

`icelines playoffs --series A` reads + renders. TUI Playoffs tab gets
an Enter-to-detail flow into the momentum view. Web `/playoffs?series=A`
mirrors.

**Test budget**: 8 tests (4 L0 momentum projection + 2 L1 wire +
2 L2 system).

### C.2 — Cup-run player narratives (~2 days)

`compute_playoff_run(pid, season)` aggregates per-game lines from
the Boxscore manifest entries where `game_type=3` AND
`date ∈ (april–june of season_year+1)`. Returns
`PlayoffRunSummary { games, goals, assists, points, plus_minus,
toi_seconds, goalie_decisions }`.

Surfaces: player card "Playoff run" section, Playoffs tab Top-10
leaderboard, `query leaders --playoff` flag. Reuses Foster +26's
windowed-leaders code path.

**Test budget**: 6 tests (3 L0 aggregation + 1 L1 player-card
render + 2 L2 system).

### C.3 — Live game tracking surface (~3 days)

New TUI sub-screen + web `/game/:id` route showing the in-progress
detail: scoreboard with live period, goalie line (saves/SA), per-
period scoring summary, pulled-goalie indicator. Reuses
`NhlApiClient::fetch_boxscore` + the existing 30s auto-refresh
ticker on Scores tab.

`icelines tonight --game GID --detail` for the CLI surface (single-
shot text dump, no auto-refresh).

**Test budget**: 8 tests (3 L0 projection + 3 L1 web smoke +
2 L2 CLI).

## Total budget

- ~7 working days
- ~22 tests across the three sub-phases
- 1 closeout pass (C.6 — docs + persona pass), bundled into the
  final commit rather than a separate sub-phase

## Pre-flight checklist

- [x] Foster phase shipped (v0.16.0) — EventStream, sync engine,
      favorites, capability matrix all in place
- [x] Foster polish (v0.17.0, v0.18.0) — per-night stat lines,
      live slate, mid-day trades, windowed leaders all in place
- [x] NHL API probe — `/v1/playoff-bracket/{season}` confirmed
      working for current + bundled seasons
- [x] Manifest's `Boxscore` shard already keyed on `Game(GameId)`,
      no schema changes needed
- [ ] Phases.md update moving Conn Smythe from Future to Active
      (lands with C.0 commit)
- [ ] C.1 starts (series momentum)

## Cross-cutting open items

1. **Series identity drift** — NHL API has used both `series_letter`
   (A/B/C…) and `series_id` (numeric) over the years. Foster.1
   already handles the "playoff-bracket shape varies" case in
   `parse_game`. Conn Smythe queries by series_letter (the modern
   shape) and falls back to seasonal series-number mapping for
   historical bracket renders.
2. **Game vs series boundary** — a game that ends in OT is series-
   advancing only if it's the deciding game. The momentum projector
   reads `game_outcome.lastPeriodType` for OT/SO marking and the
   wins counts for series-state.
3. **Web `/game/:id` collision check** — current routes are
   `/playoffs`, `/scores`, `/schedule`. `/game/:id` is new and slots
   in cleanly between scores tab clicks and the team page.
4. **Live polling cadence** — the Scores tab already auto-refreshes
   every 30 seconds for live dates. C.3 reuses that ticker rather
   than introducing a new one. The game-detail surface shares the
   `last_auto_refresh` clock state.
