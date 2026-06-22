# Phase Canadiens Signals - Roster authority

Status: Closed

## Intent

Carry the new Signals source-authority contract into the team-scoped
`signals-roster` discovery lane without turning the roster matrix into a
leaderboard, filter surface, StatId family, or analytics-cache metric.

## Scope

- Add roster-level `source_authority` to the CLI `SignalsRosterView`.
- Expose the authority in `signals-roster.v1` JSON metadata and data.
- Print the shared authority label in text output.
- Keep row-level `PlayerSignalsView.source_authority` intact for each player.
- Preserve missing evidence as `unavailable`/`null`, not zero.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli signals_roster`
- `git diff --check`
