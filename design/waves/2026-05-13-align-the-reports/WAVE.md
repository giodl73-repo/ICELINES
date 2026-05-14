---
wave: align-the-reports
date_open: 2026-05-13
status: active
source: user request to align query/report/export CLI surfaces and plan individual records
---

# Align the Reports

## Mission

Make it obvious how a user generates every IceLines report, and reserve one
clean path for future symmetric player/team records instead of scattering new
one-off commands.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| CLI discovery | Add a report catalog that points to canonical query/x/export/report doors. | Move every existing command in one pulse. |
| Records family | Define player/team record examples and data requirements. | Pretend records are implemented before event-level data exists. |
| Screen alignment | Plan future Player Records / Team Records screens that reuse the records ViewModels. | Put record computation directly in TUI or web handlers. |

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Report surface inventory and CLI catalog | done | `REPORT-SURFACE-INVENTORY.md`; `icelines report list`; `COMMANDS.md` |
| 02 - Symmetric records data inventory | done | `plans/pulse-02.md`; `RECORDS-DATA-INVENTORY.md` |
| 03 - Records ViewModels and CLI surface | planned | `plans/pulse-03.md` |
| 04 - Player/team records screens | planned | `plans/pulse-04.md` |

## Role notes

- **edge**: `query` remains the filter/question surface; `records` should not
  be shoehorned into stat filters.
- **wire**: `report list --json` is the machine-readable catalog contract.
- **glass**: future records screens should be visible from player/team cards.
- **tape**: fight opponents and goalie-scored-against records need event-level
  source data; do not infer them from season aggregates.
- **forge**: records logic belongs in core/fetch ViewModels, not CLI rendering.

## Current Result

Pulse 01 adds `icelines report list` as the discovery layer. It does not rename
existing commands; it explains which door to use today and marks symmetric
records as planned so the future implementation has a canonical home.

Pulse 02 completed the records data inventory. The first implementable records
slice is `teams-scored-against` / `players-scored-against-team`, but it still
needs goal scorer ids parsed from persisted boxscore raw JSON. Goalie-scored-
against and fight-opponent records require play-by-play/event participant data
and must not be inferred from aggregate goalie or PIM rows.
