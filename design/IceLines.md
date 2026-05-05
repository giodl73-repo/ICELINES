# IceLines — App Plan

**Version**: 1.0
**Date**: 2026-05-01
**Status**: Active — first written 2026-05-01 after a 30-spec audit

---

## What IceLines is

IceLines is a single-binary NHL hockey analytics + fantasy tool. It runs locally on
one user's machine. Four surfaces share one engine:

- **TUI** (the default — `icelines` with no arguments) — full-screen `ratatui` UI with
  7 tabs (League, Stats, Goalies, Scores, Schedule, Groups, Playoffs). Primary surface.
- **CLI** — 28 composable commands (`query leaders`, `team EDM`, `scouting McDavid`,
  `fantasy standings`, `export md`, ...). For scripting and one-shot lookups.
- **Static site** (`build` / `serve` / `deploy`) — mkdocs-Material site, one team
  page per NHL team plus a ranked index. The "share with non-CLI users" surface.
- **HTTP server** (`fantasy serve`) — axum on `127.0.0.1`, JSON API + HTML dashboard
  for fantasy league management.

All four surfaces produce the same output for the same data state along the
canonical view path (depth chart, query, scouting, fantasy scoring,
`export md`, HTTP `/api/team/<abbr>/roster`). Surface-specific affordances
exist (TUI admin overlay, fantasy SQLite, transactions UI); only the data +
computation path is required to converge.

## Who uses it

A single user on their own machine. Specifically:

- A fantasy hockey manager curating watchlists, evaluating trades, projecting rest-
  of-season points, and running their own scoring scheme.
- A hockey-data-curious analyst exploring 38 seasons (1987-88 through 2025-26 minus
  the 2004-05 lockout) — depth charts, peer cohorts, cross-team line value, draft
  class performance.
- An NHL fan watching live scores, tracking the playoff bracket, and reading the
  league-wide transactions feed.

There is no auth, no cloud, no multi-tenant. There is no "IceLines.com." All data
lives under `~/.icelines/`.

## Feature × surface portfolio

A new feature in IceLines should ship on **every applicable surface** (CLI / TUI /
Web) unless there's a documented reason otherwise. The shared engine
(`icelines-core` for data + filters, `icelines-query` for grammar, `icelines-fetch`
for I/O) means the marginal cost of a third surface is presentation, not logic.

When a feature spec is written, declare its surface coverage upfront:

```
Surface coverage:  CLI ✅   TUI ✅   Web ✅
```

If any surface has ❌, add a one-line `Reason:` in the spec. Acceptable reasons
include: deliberate scope (fantasy is local-only today), data-source constraint
(Edge speed has no public API), or "deferred to phase X" with a tracked plan.

### Current portfolio (as of 2026-05-05, post-v0.13.0 + planned Phase Lady Byng)

| Feature | CLI | TUI | Web |
|---|---|---|---|
| Skater leaderboard | ✅ `query leaders` | ✅ `tui stats` *(LB)* | ✅ `/leaders` |
| Goalie leaderboard | ✅ `query goalies` | ✅ `tui goalies` *(LB)* | ✅ `/goalies` |
| Player card | ✅ `query player <name>` | ✅ `tui player <name>` *(LB)* | ✅ `/player/:id` |
| Team depth | ✅ `team EDM` | ✅ `tui team EDM` *(LB)* | ✅ `/team/:abbrev` |
| Compare | ✅ `query compare A B` | ✅ `tui comps <name>` *(LB)* | ✅ `/compare?a=…&b=…` |
| League rankings | ✅ `rank` | ✅ `tui league` *(LB)* | ✅ home preview |
| Tonight's scores | ✅ `tonight` | ✅ `tui scores` *(LB)* | ✅ `/scores` |
| Schedule | ❌ *(Lester Patrick)* | ✅ `tui schedule` *(LB)* | ✅ `/schedule` |
| Playoffs | ❌ *(Lester Patrick)* | ✅ `tui playoffs` *(LB)* | ✅ `/playoffs` |
| Transactions | ❌ *(Lester Patrick)* | ✅ `tui transactions` *(LB)* | ✅ `/transactions` |
| Docs reference | ✅ `docs` | ❌ *(Lester Patrick — overlay)* | ✅ `/docs` |
| Fantasy | ✅ `fantasy …` | (deep links via groups) | ❌ *(deferred — Phase Frank Selke when revisited)* |

*(LB)* = ships in Phase Lady Byng (per `plans/2026-05-05-phaseLadyByng-tui-experiences.md`).

The four ❌ rows in the planned-LB column are scheduled for **Phase Lester Patrick**
(see `plans/2026-05-05-phaseLesterPatrick-cli-parity.md`) — `icelines schedule`,
`icelines playoffs`, `icelines transactions`, plus an in-TUI docs overlay. The
fantasy-on-web ❌ is intentionally deferred (single-user local-only fantasy is
the v1 stance; the web surface stays read-only for analytical features).

## What the v1.0 surface is

The 28 commands working end-to-end against the post-Hart data model. Specifically:

- **Analytics**: `query` subcommands (leaders/player/compare/similar), plus
  top-level `rank`, `players`, `class`, `peers`, `compare`, `history`, `mates`,
  `scouting`, `project`, `team`, `trade`
- **Fantasy**: `fantasy {league-create / team-{create,add,drop,show,list} / standings
  / trade / serve}`, `scheme {list,show,fromcsv}`
- **Live**: `tonight`, `schedule`, `transactions`, plus the TUI Scores/Schedule/Playoffs tabs
- **Data ops**: `fetch all`, `data {install,list,remove}`, `snapshot {list,show,use,
  verify,delete}`, `group {create,add,show,list,delete}`
- **Output**: `build`, `serve`, `deploy` (mkdocs), `export md <shape>`

Plus the TUI: every tab functional, season time-travel via `y` working across all
screens, fantasy server stable, ASCII headshots rendering.

## Where we are (2026-05-01)

**Shipped** (verified by running the release binary against bundled data):
- All four surfaces functional. `query leaders --top 5` returns Kucherov 140.3, McDavid
  138.0; `team EDM` renders the depth chart correctly; `scouting McDavid` produces the
  full 8-section report; `export md leaders` writes the front-matter + table.
- Hart phase data normalization in progress: 5c.0 through 5c.5 done (PlayerFilter,
  DepthChart, scouting, query, fantasy, export all migrated to PlayerView).
- 1,020 tests across L0/L1/L2 tiers, all green.
- 38-season time-travel via `data install`.
- Phase Vezina (goalies) shipped.
- Phase Selke (transactions) shipped.
- Phase 8h chunked snapshots shipped.

**In progress**:
- [Hart.5c.6 — TUI App restructure](plans/2026-05-01-phaseHart-5c-6-tui-restructure.md)
  to own `StatsRepository` directly (instead of long-lived `Vec<Player>`).
  The biggest single sub-phase; sub-spec at v0.3.
- Hart.5c.7 — final delete of legacy `Player`/`Goalie` types. Cleanup pass.

**Specced but not implemented**:
- [Hart.6 — per-player playoff stats](plans/2026-05-01-phaseHart-6-playoff-data.md).
  Specced at v0.2. Without it, season-type toggle in the TUI returns
  `LoadError::MissingBundle` for playoffs.

## Where we're going

In order:

1. **Finish Hart** — 5c.6 (TUI), 5c.7 (delete legacy types), then 6 (playoff data).
   This closes the data normalization loop and enables historical playoff analysis.
2. **Stabilize** — once Hart lands, every spec in `design/specs/` either matches the
   running build or is explicitly marked deferred. Rebuild the docs index. Cut a v1.0
   release.
3. **Pick from backlog** — `INDEX.md` has 10 backlog items. After Hart, the highest-
   value next moves are probably (in some order): goalie fantasy scoring (data is
   already there post-Vezina; just wire it up), `mates` / `peers` against real shift
   data, fantasy daily delta scoring, NHL Edge skating speed (parked on data
   availability).

## Non-goals (where IceLines does not go)

- **Cloud / multi-user**: no accounts, no sync, no backend. Single-user local tool.
- **Real-time push / mobile**: no websocket pushes; no mobile companion. Polling at
  30-second intervals on the Scores tab is the closest thing to live.
- **Live game prediction**: pace projections are descriptive, not predictive.
  No betting odds, no win probability.
- **Tier 4-6 data sources** (Natural Stat Trick scraping, social signals, beat media
  RSS): documented in `data-sources.md` as future, never built. Nothing in v1 depends
  on them.
- **proof / mdpath / DASHBOARD-SPEC integration**: cut 2026-05-01.
  `design/specs/dashboard-engine.md` and `design/specs/export-markdown.md` references
  to proof are stale.
- **NHL Edge skating speed**: parked indefinitely (no public JSON endpoint).
- **Cap-hit / contract value modeling**: no cap math. Contract data is expiry-year +
  expiry-type only (UFA/RFA/ELC).
- **Trade-deadline rumor aggregation**: not in scope. The transactions feed shows
  what already happened, not what might.

## When does work stop

When:
1. Hart is complete (5c.6, 5c.7, 6 — three commits / sub-phases away).
2. The 30 specs in `design/specs/` are all marked `Implemented` or explicitly
   `Deferred` with a tracked plan or backlog row.
3. `INDEX.md` and `ARCHITECTURE.md` reflect reality.
4. CI is green, all tests pass, the release binary exercises every surface.

That's v1.0. After v1.0, work moves to the backlog as user-driven feature requests.

## Pointer to architecture

This document is the **what + who + where**. The **how** (system architecture, data
spine, query model, persistence layers, lifecycle) lives in `ARCHITECTURE.md`. The
**features** (28 commands and 7 TUI tabs in detail) live in `design/specs/`.
