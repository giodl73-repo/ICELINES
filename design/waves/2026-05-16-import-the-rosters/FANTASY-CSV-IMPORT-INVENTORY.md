# Fantasy CSV Import Inventory

Pulse 01 reviewed the current fantasy database, Yahoo CSV parser, import-related
spec truth, and fantasy surfaces to turn the Tier 3 "Yahoo league CSV roster
import" backlog item into executable pulses.

## Existing Rails

| Rail | Current state | Reuse decision |
|---|---|---|
| Fantasy persistence | `FantasyDb` stores leagues, teams, a marked user team, normalized roster rows, and local matchup rows in SQLite. | Add a bulk roster import API beside existing manual create/add operations; preserve the current tables unless a proven status/audit field is needed. |
| Manual roster setup | CLI supports `fantasy league-create`, `team-create`, `team-use`, `team-add`, `team-drop`, `team-show`, standings, gaps, simulation, daily, and matchup commands. | Import should reduce manual setup, not replace the commands or change their semantics. |
| Imported roster consumers | Poach, roster gaps, simulation, daily delta, and weekly matchups already read FantasyDb rosters to mark ownership or score local fantasy teams. | Imported rows should feed these existing consumers through the same normalized roster tables. |
| CSV boundary | `icelines-fetch/src/csv_loader.rs` parses Yahoo eligibility CSVs with header-name validation, UTF-8 BOM stripping, flexible rows, and tests for missing columns/diacritics. | Reuse the CSV crate/BOM/header-validation pattern; extend or add a parser for roster ownership instead of indexing columns by position. |
| Data-source truth | `design/specs/data-sources.md` says Yahoo CSV is optional and not authoritative for stats, photos, or NHL team. | Keep roster import scoped to fantasy membership/eligibility metadata; never read Yahoo stats into rankings/scoring. |
| ViewModel parity | `FantasyLeagueView`, `FantasyRosterGapView`, `FantasySimulationView`, `FantasyDailyDeltaView`, and `FantasyMatchupWeekView` already define fantasy read/product outputs. | Add a dedicated import preview/result contract so surfaces do not invent local summaries. |

## Gaps

| Gap | Impact | Pulse |
|---|---|---|
| No import result ViewModel | CLI/TUI/web would otherwise report different totals, warnings, unresolved rows, and mutation status. | 02 |
| No roster CSV parser | Existing `load_csv_eligibility` reads player eligibility, but not fantasy-team/owner membership needed to populate `fl_teams` and `fl_roster`. | 03 |
| No bulk FantasyDb import transaction | Manual commands can create/add one row at a time, but cannot atomically preview/apply a league export. | 03 |
| No dry-run/apply CLI surface | Users still have to recreate Yahoo rosters with repeated `team-create` and `team-add` commands. | 04 |
| TUI/web dashboard can only hand off fantasy management | Import needs explicit handoff/deferred browser behavior without GET mutation or local-file ambiguity. | 04 |
| Docs still frame Yahoo CSV as eligibility-only | Users need updated truth for roster import while preserving "Yahoo stats are ignored." | 05 |

## Decisions

- "Yahoo roster CSV import" means a user-supplied local file that maps players to
  fantasy teams/owners for one IceLines fantasy league.
- The first import surface should be CLI-first with `--dry-run` and `--json` so
  users can inspect changes before writing SQLite rows.
- The import contract should count created teams, updated rosters, skipped rows,
  unresolved player names, duplicate ownership conflicts, and missing/unknown
  columns.
- Player matching should use normalized names, with team/position hints reported
  when available. Ambiguous identity is a warning/error row, not a silent match.
- Applying an import should be idempotent for repeated runs of the same CSV: team
  and roster rows converge instead of duplicating.
- Schedule import, keeper status, salary, waiver priority, private Yahoo API
  auth, and automatic remote refresh are out of scope.

## Proposed CSV Contract

Pulse 03 should support a documented alias table rather than one fragile Yahoo
header spelling. The minimum logical fields are:

| Logical field | Examples of accepted headers | Required | Purpose |
|---|---|---|---|
| Player name | `Player`, `Name`, or `First Name` + `Last Name` | yes | Normalize and persist roster membership. |
| Fantasy team | `Fantasy Team`, `Team Name`, `Rostered By`, `Owner Team` | yes | Create/find the FantasyDb team. |
| Owner | `Owner`, `Manager` | no | Populate team owner when available. |
| NHL team | `Team`, `NHL Team` | no | Diagnostic/disambiguation hint only; not stat truth. |
| Eligible positions | `Eligible Positions`, `Positions` | no | Optional metadata/diagnostic; not required for roster membership. |

## Stop Conditions

- Stop if the only available implementation would read Yahoo stat columns into
  player rankings, fantasy points, or projections.
- Stop if a row cannot identify a fantasy team and the design would silently add
  it to a default team.
- Stop if browser/dashboard import would mutate state through GET.
- Stop if tests require a real Yahoo export, live Yahoo API access, or a user's
  real `~/.icelines/icelines.db`.
- Stop if duplicate ownership or ambiguous normalized names would be accepted
  without diagnostics.
