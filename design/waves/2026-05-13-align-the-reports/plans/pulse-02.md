# Pulse 02 - Symmetric records data inventory

## Goal

Determine exactly which event-level sources are needed to support player/team
records such as teams scored against, goalies scored against, and fight
opponents.

## Deliverables

- Inventory available bundled game/boxscore/play-by-play fields.
- Decide whether goalie-on-ice and fighting participants can be recovered from
  current stores or require new fetch/store work.
- Define `PlayerRecordsView` and `TeamRecordsView` inputs.

## Gates

- Core/fetch L0 tests for any parser additions.
- No live network calls in tests.

## Result

Done. `RECORDS-DATA-INVENTORY.md` records the existing sources, feasible
metrics, missing event data, ViewModel input shapes, and implementation order.
No parser code changed in this pulse, so no new L0 parser tests were required.
