# Wave: VTRACE WP-006 Fantasy State Safety

## Goal

Execute `WP-006` as a controlled VTRACE implementation wave: fantasy read flows,
local SQLite/FantasyDb preservation, cache-read-only behavior for fantasy API
GET routes, and explicit deferral of unsupported Web mutations.

## Pulse table

| Pulse | Title | Status | Outcome |
|------:|-------|--------|---------|
| 01 | Fantasy API local-state read boundary | closed_with_risk | Selected fantasy JSON GET routes now refuse missing local FantasyDb state without creating `~/.icelines`, and daily/matchup missing-cache reads render source-state warnings without opening the writable data store. Route tests prove the selected read paths do not create local SQLite or data-cache state; broader VAL-007 read/mutation-deferral demo remains pending. |
| 02 | Fantasy existing-DB read-only boundary | closed_with_risk | Existing FantasyDb-backed Web GET reads now use a read-only SQLite open path after confirming `icelines.db` exists. Route tests prove selected fantasy gaps GET reads do not create SQLite WAL/SHM sidecar files; broader VAL-007 transcript and active-writer database semantics remain pending. |
| 03 | Poach imported-availability read-only boundary | closed_with_risk | Web poach imported-availability and watch read helpers now reuse the immutable read-only SQLite helper. Route tests prove selected `/api/v1/poach?availability=imported-available` GET reads do not create SQLite WAL/SHM sidecar files; active-writer database semantics remain pending. |
| 04 | VAL-007 transcript closeout | closed_with_risk | Focused fantasy decision-loop tests cover shared ViewModels, CLI/TUI handoffs, CLI L2 import/gaps/roster-shape/daily/matchup/export commands, Web dashboard mutation deferrals, Web fantasy routes, and Web poach routes. WP-006 closes with accepted active-writer and broader interactive-TUI risks. |

## Success criteria

- `WP-006` stays linked to `REQ-FANTASY-001`, `REQ-WEB-001`,
  `REQ-CODE-001`, `IF-VIEW-001`, `IF-WEB-001`, `VAL-007`,
  `EVID-CR-008`, and local-state preservation evidence.
- Fantasy read routes do not create missing local SQLite or data cache state.
- Existing FantasyDb-backed reads remain available when local state already
  exists.
- Unsupported Web mutations remain CLI/TUI handoffs or explicit refusals, not
  GET mutations.
- TRACKER submodule pointer updates remain separate from ICELINES child-repo
  implementation commits.

## Gate Status

Current gate: `closed_with_risk` after pulses 01-04.

This wave remains in progress. Pulse 01 closes selected JSON GET local-state and
cache-read-only boundaries for fantasy roster gaps, daily deltas, and weekly
matchups. Pulse 02 closes the selected existing-FantasyDb read-only open
boundary for Web GET reads. Pulse 03 closes the selected Web poach
imported-availability read-only SQLite boundary. Pulse 04 records the focused
`VAL-007` command/API transcript and closes WP-006 with accepted risk for
active-writer database semantics and broader interactive-TUI parity.
