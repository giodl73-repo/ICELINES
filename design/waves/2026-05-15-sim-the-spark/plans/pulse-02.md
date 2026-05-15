# Pulse 02 - Player Scoring Pace ViewModel

## Goal

Add the first core-owned Sim the Spark ViewModel for player scoring outlooks.
The ViewModel should describe season-to-date goal, point, and shot pace from
already-loaded `PlayerView` data, with nullable projected-finish fields when a
remaining-game count is supplied.

## Governing roles

- **pace**: formulas must be explicit, regular-season 82-game pace only, and
  nullable below `MIN_GP`.
- **scout**: output labels must be descriptive: "on pace", "tracking", and
  "below sample floor"; no betting or certainty language.
- **wire**: no live network calls. The ViewModel accepts loaded data and optional
  remaining-game counts from callers.
- **bench**: every formula and threshold must have known-value L0 tests.

## Owned scope

1. Add core ViewModel types for player scoring pace/outlook in `icelines-core`.
2. Build rows from `PlayerView` totals: goals, points, and shots.
3. Include sample status and nullable pace/final values.
4. Export the types from the existing ViewModel module surfaces.
5. Add focused L0 tests only; defer web/CLI/TUI wiring to later pulses.

## Non-goals

- No web/API route wiring.
- No CLI/TUI command changes.
- No regression, age-curve, betting, odds, or xG model.
- No route handler or template-local math.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo clippy -p icelines-core -- -D warnings`
