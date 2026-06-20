# Signals discovery gate

Date: 2026-06-20

## Decision

Phase Rangers may add a Signals discovery lane, but the first implementation
must be a **roster discovery matrix**, not a leaderboard, `StatId` promotion,
filter key, or analytics-cache metric family.

Recommended command shape for the next implementation pulse:

```powershell
icelines signals roster --team NYR
icelines signals roster --team NYR --json
```

If the current positional `icelines signals "<player>"` command shape makes a
subcommand awkward, use an additive command name such as
`icelines signals-roster --team NYR` instead. Do not break the existing player
Signals command.

## Why roster matrix, not leaderboard

The first Signals are descriptive and scorer-biased. Ranking players by a single
Signal would make the surface read like a quality leaderboard even with careful
copy. A roster matrix keeps the purpose narrower:

- show which players have full, partial, or missing evidence;
- let users discover which player cards deserve inspection;
- preserve methodology and limitation copy;
- avoid implying team deployment, prediction, betting, injury, or autonomous
  coaching conclusions.

## Required product copy

Every roster discovery encoding must include this meaning:

- Signals are descriptive derived metrics built from existing stat inputs.
- Unavailable Signals mean required evidence is missing or below threshold, not
  zero-value truth.
- This is not a prediction, betting edge, injury signal, deployment
  recommendation, player-quality grade, or autonomous coaching decision.
- The matrix is an inspection aid; use player Signals cards or Markdown packets
  for full methodology and limitations.

## Required fields

Each row must carry:

- player id and name;
- team, position, games played, active season, and season type;
- one cell per current Signal with value or unavailable state;
- evidence tier per Signal;
- missing-input summary per Signal;
- row-level disclosure if any Signal is partial or missing.

JSON may use a new additive envelope such as `signals-roster.v1`. Text output
must render unavailable values as `unavailable`, never `0.00`.

## Required tests

- L0: renderer preserves `unavailable` and evidence tiers for full, partial, and
  missing rows.
- L0 or L1: team filter returns only the requested team and does not rank across
  the whole league by Signal value.
- L2: offline CLI subprocess for `NYR` proves disclosure copy, evidence-tier
  text, and no zero-filled unavailable values.
- Optional JSON L2: `--json` envelope includes schema name/version and row
  evidence without dropping missing-input summaries.

## Explicit non-promotions

This gate does not allow:

- stable `StatId` rows;
- `query leaders --sort` or `--filter` integration;
- public cross-team Signal leaderboards;
- analytics cache publication;
- Web/TUI parity claims;
- team confidence, deployment, injury, betting, or coaching recommendation copy.

Those require their own later gate and evidence.
