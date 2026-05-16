# WIRE/FORGE Review - Match the Week

## Findings

- WIRE: cached finalized daily rows are the right source boundary. Weekly
  aggregation must preserve missing cache and unfinalized-game warnings by date.
- FORGE: the weekly contract belongs in `icelines-core`; SQLite and cache
  orchestration belong in `icelines-fetch`; CLI/web/TUI should remain adapters.
- Schedule mutation should start in CLI/FantasyDb. Web/dashboard GET routes must
  remain read-only.

## Required Pulse Constraints

- Do not add live NHL calls while building a matchup week.
- Do not mutate matchups through GET routes.
- Do not introduce a second fantasy scoring formula in surface code.
- Refuse invalid team IDs/names at the schedule API boundary with explicit
  errors.
