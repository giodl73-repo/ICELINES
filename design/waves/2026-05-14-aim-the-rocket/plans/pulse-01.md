# Pulse 01 - Scoring Data Inventory

## Goal

Confirm which scoring-intelligence features Phase Rocket Richard can build from
official NHL play-by-play data and existing IceLines cache architecture.

## Work completed

1. Inspected current `PlayByPlay`, `PlayByPlayGoal`, and `PlayByPlayPenalty`
   structs in `icelines-fetch/src/nhl_api.rs`.
2. Inspected `parse_play_by_play`, `records_provider`, `DataStore`, `Manifest`,
   and `game_cache` to confirm the current raw-data path.
3. Queried an official NHL play-by-play sample for game `2025020001` to verify
   shot/goal coordinate fields.
4. Wrote `SCORING-DATA-INVENTORY.md`.
5. Reviewed the result through `tape`, `edge`, `wire`, and `bench`.

## Result

Rocket Richard is viable from the existing official NHL play-by-play path:

- raw play-by-play is already fetchable and persistable in `DataKind::PlayByPlay`
- current typed projection only keeps goals and penalties
- official shot events include IDs, period/time, situation code, team owner,
  shooter/scorer IDs, goalie IDs where applicable, coordinates, zone code, and
  shot type
- next work should extend typed event projection and ViewModels before any UI

## Gates

- `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-14-aim-the-rocket design\waves\PHASES.md --errors-only`

## Follow-up pulse

Generate **Pulse 02 - Scoring ViewModel contracts**:

1. Add `ScoringEventInput` and shot-event enums in `icelines-core`.
2. Extend `parse_play_by_play` or add a provider projection for shot events in
   `icelines-fetch`.
3. Add L0 parser tests for goal, shot-on-goal, missed-shot, blocked-shot, and
   missing-coordinate cases.
4. Add L1 DataStore/tempdir test for persisted raw play-by-play to scoring-event
   projection.
5. Do not add web/TUI routes until the contracts are reviewed.
