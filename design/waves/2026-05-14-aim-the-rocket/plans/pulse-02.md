# Pulse 02 - Scoring ViewModel Contracts

## Goal

Turn the Pulse 01 scoring-data inventory into typed contracts and a first
manifest-backed projection so later Rocket Richard pulses can build scoring
reports without renderer-local scoring math or a second cache path.

## Governing roles

- **tape**: official NHL play-by-play remains the row source; keep game, team,
  shooter/scorer, goalie, period/time, and score-state identity intact.
- **edge**: preserve missing coordinates and participants as `Option<T>`; do not
  normalize rink orientation or infer danger buckets yet.
- **wire**: reuse `DataKind::PlayByPlay` and `DataStore`; do not introduce a new
  scoring cache.
- **bench**: parser fixtures must cover every supported attempt family, and the
  provider must have a tempdir manifest round-trip.

## Owned scope

1. Add shared scoring contracts in `icelines-core`.
2. Extend `icelines-fetch` play-by-play parsing to project goal, shot-on-goal,
   missed-shot, and blocked-shot events.
3. Add a provider helper that reads manifest-backed raw play-by-play and returns
   scoring event inputs.
4. Add L0/L1 tests for official-shaped events and missing optional fields.
5. Update this wave's status/evidence.

## Non-goals

- No web, TUI, CLI, or API route for scoring reports.
- No claimed xG model, shot-danger model, or rink-orientation normalization.
- No third-party scraping or betting/lineup ingestion.
- No changes to existing record/streak behavior beyond preserving current
  `PlayByPlayGoal` and `PlayByPlayPenalty` parsing.

## Implementation result

- Added `ScoringEventInput`, `ShotEventKind`, `ShotLocation`,
  `ScoringEventSummary`, and first scoring ViewModel shells in
  `icelines-core::view_model::scoring`.
- Extended `parse_play_by_play` to preserve scoring attempt families while
  retaining the existing goal/penalty projections used by records.
- Added `icelines-fetch::scoring_provider::load_scoring_event_inputs` to read
  `DataKind::PlayByPlay` manifest entries and project scoring events from raw
  NHL JSON.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-fetch --quiet`
- [x] `cargo clippy -p icelines-core -p icelines-fetch -- -D warnings`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-14-aim-the-rocket design\waves\PHASES.md --errors-only`
