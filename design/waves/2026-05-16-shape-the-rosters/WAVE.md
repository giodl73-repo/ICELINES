---
wave: shape-the-rosters
date_open: 2026-05-16
status: closed
source: Tier 3 backlog - Fantasy roster shape enforcement
---

# Shape the Rosters

## Mission

Add fantasy roster shape enforcement so local leagues can describe legal roster
composition, validate imported/manual rosters, and surface clear setup problems
without changing fantasy scoring math.

## Award Fit

This is a Frank J. Selke / Lady Byng fantasy-operations wave: defensive, rules
aware, and focused on preventing invalid local roster state from looking valid.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Inventory | Map current FantasyDb, import, CLI, TUI/dashboard handoffs, and docs gaps. | Implement browser-side roster mutation. |
| Core contract | Add pure roster-shape rules and validation ViewModel types. | Move scoring math out of existing scheme functions. |
| Persistence/import | Persist league roster-shape settings and validate dry-run/apply imports. | Treat Yahoo CSV position hints as NHL truth. |
| CLI/surfaces | Add command-copyable validation/config surfaces and read-only dashboard/TUI handoffs. | GET-backed mutation. |
| Docs/closeout | Update README, COMMANDS, surface parity, and backlog truth. | Add remote Yahoo API sync. |

## Operating Rules

- Business logic belongs in `icelines-core`; fetch/DB wires it, CLI/web/TUI only
  render or dispatch.
- Roster-shape enforcement must not change fantasy point totals.
- Yahoo CSV positions are hints only; canonical eligibility must come from the
  active player pool when available.
- Existing leagues must keep working through safe defaults/migrations.
- Mutations stay POST/CLI-backed; dashboard and TUI command bars may hand off or
  defer but must not mutate through GET.

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Roster shape inventory and pulse map | complete | `FANTASY-ROSTER-SHAPE-INVENTORY.md`; `plans/pulse-01.md`; `panels/wave-plan-review/` |
| 02 - Core roster shape contract | complete | `icelines-core/src/roster_shape.rs`; 6 L0 tests |
| 03 - FantasyDb shape persistence and import validation | complete | `fl_leagues.roster_shape`; import warnings from canonical positions; fetch L1 tests |
| 04 - CLI, TUI, and dashboard validation surfaces | complete | CLI show/set/validate; TUI/web-dashboard handoffs; read-only web JSON validation |
| 05 - Docs, regression gates, and closeout | complete | README/COMMANDS/surface parity/backlog truth updated; focused gates passed |

## Role Notes

- **bench**: validation needs fixtures for legal rosters, over-cap rosters,
  missing goalie slots, duplicates, imports with hints, and unknown players.
- **forge**: make roster-shape invalid states explicit typed rows/errors; avoid
  stringly slot math leaking into CLI handlers.
- **wire**: Yahoo CSV fields are external hints and must degrade to warnings, not
  become authoritative player eligibility.
- **glass**: surfaces must show roster-compliance status at a glance without
  burying recovery commands.

## Current Result

Pulse 01 opened the wave. Current storage tracks teams and normalized rostered
player names only; the scoring scheme has weights but no roster-slot rules; and
manual/import paths enforce duplicate ownership but not roster shape.

Pulse 02 added the pure core roster-shape contract. `RosterShape` and
`RosterShapeValidationView` can now validate canonical position inputs for legal
rosters, underfilled/overfilled slots, unknown players, duplicate rows, and
goalie/skater shape mismatches without touching scoring math.

Pulse 03 added FantasyDb roster-shape persistence through the default
`yahoo-standard` preset, exposed persisted roster validation helpers, and wired
Yahoo CSV imports to emit roster-shape warnings only from canonical player
positions supplied by the active player pool. Yahoo CSV position hints remain
diagnostics/fallback context and do not become NHL eligibility truth.

Pulse 04 added the user-facing validation/config surfaces without GET-backed
mutation: CLI `fantasy roster-shape`, `fantasy roster-shape-set`, and
`fantasy roster-shape-validate`; TUI command-bar handoffs to those commands and
the read-only API; web dashboard command parsing that rejects shape mutation; and
JSON `GET /api/v1/fantasy/roster-shape` validation for persisted FantasyDb
rosters.

Pulse 05 closed the wave. README, COMMANDS, the surface-parity matrix, and the
backlog index now document roster-shape setup/validation as shipped, with CLI as
the mutation surface and web/TUI dashboard paths as read-only handoffs or
deferrals. The wave and phase index records are closed after focused core/fetch,
CLI, and web roster-shape gates, proof, formatting, and whitespace checks.

## Next

Shape the Rosters is closed.
