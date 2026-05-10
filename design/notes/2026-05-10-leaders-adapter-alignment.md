# Leaders Adapter Alignment

**Date**: 2026-05-10
**Phase**: Ted Lindsay closeout

## Current State

Leaders now has three different projection levels:

- Web HTML and web JSON build `LeadersView` in `icelines-web`.
- CLI text output builds `LeadersView` through `leaders_view_from_results`.
- CLI JSON and CSV now serialize from `LeadersView` while preserving the
  existing field names and precision.
- TUI Stats/Queries still executes selection/search over `PlayerView`, but the
  results table now renders identity and primary metric cells from
  `LeadersView`.

That means the feature is behaviorally useful, but not yet fully uniform.

## Contract To Close The Gap

All leaders-capable surfaces should eventually consume the same row contract:

- identity: rank, player id, display name, team, position;
- primary metric: stable key, label, value, unit, precision;
- secondary metrics: GP, goals, assists, points, and any exported compatibility
  fields such as PPG, points-per-82, and goals-per-82;
- context: season, season type, active filters, sort key, and top limit;
- empty/source state: no rows, missing source, and partial source state.

## Ordered Migration

1. Extend `LeadersView` construction to carry the compatibility metrics needed
   by existing CLI JSON/CSV without changing those wire shapes. Complete.
2. Move CLI JSON/CSV serialization to read from `LeadersView`, preserving
   existing field names and numeric precision. Complete.
3. Add a fixture test that compares CLI JSON and CSV row identity for the same
   query arguments. Complete.
4. Move TUI Stats/Queries row rendering behind a `LeadersView` adapter while
   keeping the interactive filter/sort controls in TUI state. Complete for
   rendering; execution/selection still uses `PlayerView`.
5. Promote the leaders row in `surface-parity.md` from partial only after CLI
   JSON/CSV and TUI Stats no longer project their own row identity.

## Pitfalls

- Do not change CLI JSON/CSV field names as part of the adapter migration.
- Do not force TUI controls into the ViewModel; controls are surface state, rows
  are platform state.
- Do not remove direct catalog sort support until `LeadersView` can represent
  the selected primary metric key and value exactly.
- Preserve existing display precision: CLI text, CSV, web JSON, and TUI can
  format the same value differently, but they should not disagree on row order
  or identity for the same query.
