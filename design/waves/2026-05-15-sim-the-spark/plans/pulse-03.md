# Pulse 03 - Team Scoring Outlook ViewModel

## Goal

Add the core-owned team outlook ViewModel for Sim the Spark. The ViewModel
should describe team goals-for and goals-against pace from already-loaded,
caller-supplied schedule/score inputs, with nullable projected-finish fields
when remaining-game counts are unavailable.

## Governing roles

- **pace**: formulas must be explicit, use regular-season 82-game pace, and keep
  recent trend language descriptive.
- **scout**: labels must read as hockey context ("tracking toward",
  "recent pressure", "below sample floor") without certainty claims.
- **wire**: no live network calls. Inputs must be loaded data supplied by the
  caller; GET surfaces in later pulses may not mutate or fetch live NHL data.
- **bench**: add known-value L0 tests for pace math, missing remaining games,
  zero games, and partial-source disclosure.

## Owned scope

1. Add core ViewModel types for team scoring outlooks in `icelines-core`.
2. Accept loaded team game/summary inputs rather than fetching schedules or
   standings.
3. Build goals-for and goals-against rows with current totals, games played,
   per-game pace, 82-game pace, and optional projected finish.
4. Include source/completeness state for partial or missing loaded game inputs.
5. Export the types from the existing ViewModel module surfaces.
6. Add focused L0 tests only; defer web/CLI/TUI wiring to later pulses.

## Non-goals

- No web/API route wiring.
- No CLI/TUI command changes.
- No standings, playoff odds, win probability, betting, or xG model.
- No route handler or template-local math.
- No GET-backed live NHL schedule/score fetch.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo clippy -p icelines-core -- -D warnings`
