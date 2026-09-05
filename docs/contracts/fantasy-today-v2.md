# Fantasy Today v2 consumer contract

`icelines fantasy today --json` emits `fantasy_today.v2`. The same owned,
read-only contract is returned by `GET /api/v2/fantasy/today` and rendered by
the TUI and Web cockpit. `GET /api/v1/fantasy/today` remains the compatibility
projection and deliberately omits v2-only decision semantics.

## Stable PUCK handoff

PUCK may ingest these public fields without copying IceLines implementation
details:

- `schema`, `context`, `state`, `material_fingerprint`, and
  `next_decision_deadline_utc` identify the evaluated decision state;
- `decisions.primary_decision` and `decisions.alternatives` provide ordered
  action, legality, firmness, matchup impact, deadline, and evidence age;
- `decisions.transaction_candidate` provides the optional bounded add/drop
  screen, including modeled delta, usable starts, waiver timing, population,
  acquisition cost and remaining budget, truncation, and evaluation elapsed
  time;
- `lineup`, `goalies`, `acquisitions`, `quiet_nights`, and `matchup` provide the
  supporting league-neutral summaries;
- `readiness`, `evidence`, `warnings`, `decisions.candidate_state`, and
  `decisions.candidate_recovery_command` explain unavailable or provisional
  conclusions.

Consumers should reject unknown major schema versions, preserve unknown fields,
and use `material_fingerprint` rather than `generated_at` to detect a materially
changed recommendation. Evidence timestamps describe the state used for the
answer; they are not permission to refresh a source or execute a transaction.

## Boundary

The command performs no network access or fantasy-platform mutation. The
bounded transaction row is a fast remaining-schedule screen that reuses the
canonical weekly pickup score. Its reasons point to `icelines fantasy pickups
--top 5` for exhaustive lineup optimization. A missing candidate is not advice
to transact.

Personal rosters, sentimental preferences, watch lists, manager decisions, and
season history belong in PUCK. IceLines exports the generic decision contract
but does not automatically write a journal, upload data, or import PUCK state.

## Request and surface parity

The local assembler resolves an exact league and user team, then carries the
requested stats season **and season type** through the stats load and returned
context. Missing assistant rules and missing schedule cache are typed failures
with recovery commands; stale or partial saved matchup snapshots are rejected
and disclosed rather than silently reused.

CLI, TUI, and Web render the shared `FantasyTodaySurfaceDecision` projection.
It is the canonical surface subset for primary decision, alternatives,
deadline, firmness, legality, matchup impact, evidence age, and decision
fingerprint. The public sealed fixture
`icelines-core/tests/fixtures/fantasy_today_surface_decision.v1.json` is consumed
by all three surface test suites.
