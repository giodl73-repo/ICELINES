# Pulse 05 - Records Surface Parity Refresh

## Goal

Make the new event-backed records discoverable and visible across the report
catalog, CLI docs, TUI handoff hints, and web/API records routes.

## Implementation

1. Updated `icelines report list` notes and text examples.
2. Updated clap long help and `COMMANDS.md`.
3. Updated TUI player/team records hints and command-bar flashes to point at
   metric-aware records routes.
4. Added metric query support to web/API records routes:
   - `/records/player/:id?metric=teams-scored-against`
   - `/records/player/:id?metric=goalies-scored-against`
   - `/records/player/:id?metric=fight-opponents`
   - `/records/team/:abbrev?metric=players-scored-against-team`
   - `/records/team/:abbrev?metric=goalies-beaten-by-team`
   - `/records/team/:abbrev?metric=fight-opponents-by-team`
5. Updated `design/specs/surface-parity.md`.

## Gates

- `cargo test -p icelines-web l1_records_player_metric_query_selects_fight_opponents`
- `cargo test -p icelines-cli records_team_handoff_flashes`
- `cargo check -p icelines-cli -p icelines-web`
- `proof check design\waves\2026-05-13-trace-the-events design\specs\surface-parity.md COMMANDS.md --errors-only`

## Result

The records family is aligned across CLI discovery, CLI execution, TUI hints,
and web/API routes. The wave now has a complete event-backed records slice.
