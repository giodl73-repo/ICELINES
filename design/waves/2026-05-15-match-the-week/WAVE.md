---
wave: match-the-week
date_open: 2026-05-15
status: closed
source: Tier 3 backlog - Fantasy head-to-head matchup weekly
---

# Match the Week

## Mission

Add weekly fantasy head-to-head matchups: a local league schedule plus a dated
week view that totals cached, finalized daily fantasy scoring into matchup
results. The wave must reuse the Score the Day daily-delta contract instead of
forking fantasy scoring, and it must remain offline-testable.

## Award Fit

This is a Selke / Lady Byng product-utility wave: it turns the existing fantasy
league, daily scoring, and schedule/date rails into a clean matchup review loop
without pretending to be a proprietary Yahoo integration.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Matchup contract | Define a shared weekly matchup ViewModel in core. | Compute surface-local standings or official Yahoo results. |
| Schedule persistence | Add local FantasyDb matchup schedule rows. | Import private Yahoo schedules. |
| Data path | Aggregate cached finalized daily scoring across an ISO week. | Fetch live NHL data while scoring weekly matchups. |
| Surfaces | Add discoverable CLI/web/TUI handoff surfaces after the shared contract exists. | Add GET-backed mutations or a full TUI matchup screen before the contract proves out. |
| Closeout | Document commands, setup, data requirements, and gates. | Expand into playoffs, waivers, or keeper rules. |

## Operating Rules

- Weekly matchup scoring is descriptive: points earned by rostered players in
  cached, finalized game lines inside the selected week.
- Use `FantasyDailyDeltaView`, `Scheme`, and shared daily scoring adapters; do
  not fork category weights in CLI/TUI/web.
- Week boundaries use `Timeframe::Week` (ISO Monday through Sunday).
- Persist only the local matchup schedule in FantasyDb. Missing schedule rows,
  missing cache, and unfinalized games surface as explicit empty/source states.
- Mutations must be CLI or POST-backed only; dashboard/web GET routes remain
  read-only.
- Do not add live-network tests.

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Matchup inventory and pulse map | complete | `FANTASY-MATCHUP-INVENTORY.md`; `plans/pulse-01.md`; `panels/wave-plan-review/` |
| 02 - Core weekly matchup ViewModel | complete | `icelines-core/src/view_model/fantasy_matchup.rs`; `plans/pulse-02.md` |
| 03 - FantasyDb schedule and weekly builder | complete | `icelines-fetch/src/fantasy_db.rs`; `icelines-fetch/src/fantasy_matchup.rs`; `plans/pulse-03.md` |
| 04 - CLI, web, and TUI matchup surfaces | complete | `icelines-cli/src/commands/fantasy.rs`; `icelines-web/src/handlers/fantasy.rs`; `icelines-cli/src/tui/command.rs`; `icelines-web/src/dashboard_command.rs`; `plans/pulse-04.md` |
| 05 - Docs, regression gates, and closeout | complete | `README.md`; `COMMANDS.md`; `design/specs/surface-parity.md`; `design/plans/INDEX.md`; `plans/pulse-05.md` |

## Role Notes

- **pace**: weekly totals, ranking, matchup winner/tie rules, and ISO week
  boundaries must be explicit and descriptive.
- **bench**: fixture-driven tests must cover wins, losses, ties, byes, missing
  schedule, missing cache, and unfinalized games without network data.
- **wire**: missing cache and unfinalized daily rows must aggregate into weekly
  source-state/warnings instead of becoming zeros.
- **forge**: core owns the ViewModel/aggregation contract; fetch owns
  SQLite/cache orchestration; CLI/web/TUI remain thin adapters.
- **glass**: surfaces should show matchup score, opponent, week range, and source
  completeness without cluttering the first pass.

## Current Result

Match the Week is closed. The shipped path adds `FantasyMatchupWeekView`, local
`fl_matchups` schedule persistence, cached weekly aggregation over Score the
Day daily-delta results, CLI `fantasy matchup-set` / `fantasy matchup --date`,
JSON `/api/v1/fantasy/matchup?date=...`, and TUI/web-dashboard handoffs.
Missing schedule/cache and unfinalized games remain explicit empty/source-state
signals, not zero-shaped success.

## Closeout Gates

- `cargo fmt --check`
- focused matchup tests from Pulses 02-04
- proof on touched docs
