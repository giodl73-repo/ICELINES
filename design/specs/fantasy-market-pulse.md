# Fantasy Market Pulse — Specification

**Version**: 0.1\
**Date**: 2026-09-08\
**Status**: Implemented (core + CLI foundation)\
**Owner domain**: fantasy provider evidence

---

## Purpose

Turn repeated fantasy-platform player-pool observations into a transparent,
provider-neutral movement report. The report answers whether a player's
platform demand is rising, falling, mixed, steady, or not yet comparable. It
does not predict hockey performance and does not assign a currency or price.

Puck may combine this output with a private league's quiet-night value,
projected fantasy scoring, and YHO valuation. Those personal decisions do not
belong in IceLines.

## Surface coverage

Surface coverage: CLI ✅   TUI ❌   Web ❌

Reason: v0.1 establishes the shared typed ViewModel and deterministic CLI
text/JSON contract. TUI and Web are tracked follow-up renderers and must consume
the same ViewModel without recomputing movement.

## Input contract

`fantasy_market_snapshot.v1` is an immutable observation supplied as JSON:

```json
{
  "schema": "fantasy_market_snapshot.v1",
  "provider": "yahoo",
  "captured_at": "2026-09-08T14:00:00Z",
  "source_url": "https://example.invalid/player-pool",
  "players": [
    {
      "provider_player_id": "12345",
      "name": "Example Player",
      "team": "SEA",
      "positions": ["LW", "RW"],
      "xrank": 120.0,
      "adp": 145.2,
      "rostered_pct": 12.0,
      "drafted_pct": 3.0,
      "adds": 320,
      "drops": 90
    }
  ]
}
```

- `provider_player_id` is required, provider-scoped identity. Names are display
  data and are never join keys; duplicate names are valid.
- Metrics are optional because exports vary. Rank and ADP must be positive;
  percentage fields must be in `[0, 100]`; all floating-point values must be
  finite.
- A comparison requires equal providers and a prior capture time not later than
  the current capture.
- The snapshot is provider evidence. It never mutates canonical NHL identity,
  team stints, position, or season statistics.

## Output contract and methodology

`fantasy_market_pulse.v1` returns every current player, the prior capture time,
metric deltas when comparable, current net adds (`adds - drops`), and one
transparent direction:

- `rising`: at least one positive signal and no negative signal;
- `falling`: at least one negative signal and no positive signal;
- `mixed`: both positive and negative signals;
- `steady`: a prior row exists but there is no positive or negative evidence; or
- `new_observation`: no prior row exists for the provider player ID.

Positive signals are rostered/drafted percentage increases, improving ADP or
XRank (a lower number), and positive current net adds. Negative signals are the
opposites. Missing values are absent evidence, never zero. No weighted composite
score is calculated.

Rows sort by direction (`rising`, `mixed`, `new_observation`, `steady`,
`falling`), then rostered-percentage change descending, current net adds
descending, ADP improvement descending, name, and provider ID. This is a report
ordering rule, not a player-value ranking.

## CLI

```text
icelines fantasy market-pulse --current PATH [--previous PATH] [--json]
```

`-` may be used for one input only. v0.1 reads caller-supplied immutable files
and does not persist, fetch, scrape, or execute provider transactions.

## Acceptance criteria

1. Stable provider IDs, not names, join observations.
2. Duplicate IDs, invalid ranges, non-finite values, provider mismatch, and
   reversed capture time fail with actionable errors.
3. Missing prior rows and missing metrics stay explicit and do not create fake
   zero deltas.
4. Duplicate display names compare independently.
5. Rank/ADP direction correctly treats a lower number as improvement.
6. Mixed evidence remains mixed rather than being collapsed into an opaque
   score.
7. Text and JSON render the same core ViewModel.
8. Core remains pure and I/O-free; CLI owns file/stdin reading only.
9. Yahoo is supported as a provider label but no Yahoo-specific DTO or personal
   league valuation enters the core contract.
