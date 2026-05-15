# Pulse 01 - Projection Inventory and Assumptions Contract

## Goal

Define the Sim the Spark projection/outlook contract before adding code. The
output should identify which descriptive pace fields IceLines already owns,
which Rocket scoring trend inputs can safely feed an outlook, and which
assumptions/tests must exist before player or team surfaces ship.

## Governing roles

- **pace**: projections are descriptive rate normalizations. State formulas,
  sample-size thresholds, tiebreakers, rounding, and confidence labels plainly.
- **scout**: outlook language must not imply betting value, lineup certainty, or
  proprietary expected-goals parity.
- **wire**: all inputs must come from loaded bundles, snapshots, manifests, or
  local stores. No GET route may trigger a live fetch.
- **bench**: inventory must name the known-value tests needed for every formula
  and threshold before implementation.

## Owned scope

1. Inspect existing projection/pace surfaces: `icelines project`, `rank`,
   `PlayerView::pace_82`, scoring trend rows, team season performance, and
   Rocket scoring reports.
2. Write `SPARK-INVENTORY.md` in this wave folder covering available inputs,
   assumptions, source-state behavior, formula non-goals, and first implementation
   split.
3. Amend `WAVE.md` pulse status or later pulse names if the inventory reveals a
   better sequence.
4. Do not add code beyond documentation/planning artifacts in this pulse.

## Candidate deliverable

`SPARK-INVENTORY.md` should cover:

- player pace fields already available from season stats;
- recent scoring trend fields available from `PlayerScoringTrendRow`;
- team goal-for/goal-against and recent-form inputs available from schedule,
  scores, and team season ViewModels;
- cache/source-state requirements for web/API and CLI/TUI surfaces;
- formulas that are allowed (`pace`, `range`, `descriptive outlook`) and banned
  claims (`odds`, `win probability`, proprietary xG equivalence);
- L0/L1/L2 tests required for implementation pulses.

## Role review notes

- **pace**: the inventory must decide whether the first player ViewModel uses
  season-to-date 82-game pace, rest-of-season pace, recent-window pace, or a
  side-by-side of those values. Do not leave formula names ambiguous.
- **scout**: any label such as "hot", "cooling", "on pace", or "regression"
  needs a plain hockey meaning and a caveat when source coverage is partial.
- **wire**: source-state must distinguish "no games loaded", "season stats
  loaded but no play-by-play", and "play-by-play loaded with zero events".
- **bench**: every future threshold should have a manual expected value example:
  low GP, exactly threshold GP, zero shots, zero goals, and tied pace rows.

## Gates

- [ ] `cargo fmt --check`
- [ ] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-sim-the-spark design\waves\PHASES.md --errors-only`
