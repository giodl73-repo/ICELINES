# Fantasy pickup sequence v1

`fantasy_pickup_sequence.v1` is the shared Monday-Sunday acquisition-planning
contract for IceLines surfaces and private downstream consumers such as PUCK.

The contract contains league/team/week identity, evaluation time, readiness,
the acquisition budget, one primary sequence, materially distinct fallbacks,
daily legal-start coverage, search bounds, warnings, and a material
fingerprint. Generation time and elapsed runtime do not affect that
fingerprint.

Each move preserves stable player keys and display labels, exact UTC effective
time, league-local date, add/drop pair, conditional status, marginal active
value, and newly usable dates. Each sequence preserves pre/post roster
fingerprints and reserve remaining. A zero-move sequence is always eligible.

All player rates and objective components must be finite. Every retained prefix
is checked for acquisition budget, ownership, waivers, locks, reserve-slot and
standard-roster capacity, then evaluated through the canonical daily lineup
builder. Raw scheduled games are not credited when a player would remain on the
bench. Category plans without matchup posture are provisional.

The planner is bounded, not globally optimal. `beam_width`, evaluated-state
count, and `truncated` disclose that boundary. Consumers must display readiness
and warnings and must not describe projected value as a probability or promise.

Decision capture is a separate mutation. Migration 020 stores the exact JSON
projection as immutable text; duplicate league/team/fingerprint capture is
idempotent. Manager rationale remains private by default, while later outcomes
and corrections append new rows rather than updating history.
