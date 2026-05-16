# WIRE/FORGE Review - Score the Day

## Findings

- WIRE: cached finalized boxscores are the right source boundary. Missing or
  unfinalized data must be explicit source state/warnings.
- FORGE: scoring belongs in `icelines-core`; SQLite/cache orchestration belongs
  outside core; CLI/web/TUI should remain thin adapters.
- The first wave should stay read-only. Persisted daily history can wait until
  the read contract proves useful.

## Required Pulse Constraints

- Do not add live NHL calls to `fantasy daily`.
- Do not add GET-backed mutations on web/dashboard routes.
- Do not introduce a second fantasy scoring formula in surface code.
