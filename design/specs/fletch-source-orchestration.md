# ICELINES FLETCH Source Orchestration

ICELINES has a larger source surface than ROUTE or BISECT: some inputs are single public HTTP objects, while others are paged, expanded from schedules, or expanded from active player sets. The migration boundary is therefore incremental.

FLETCH owns neutral source-byte acquisition for stable single-object URLs currently routed through the migration slice: team roster JSON and MoneyPuck CSV. ICELINES keeps ownership of snapshot sealing, active snapshot pointers, manifest shards, stale/TTL decisions, fetch locks, parsing, event-stream writes, and hockey-domain validation.

Paged NHL stats reports, ESPN transaction windows, player landing batches, boxscore/play-by-play batches, contracts, and career history remain adapter-required until FLETCH has generic pagination, batch expansion, and rate-limit primitives that match ICELINES semantics.

The non-mutating command is:

```text
icelines fetch fletch-sources --season 20252026 --type regular --gate
```

It writes `data/fletch-source-handoff.csv` and fails the gate only on registry or ICELINES handoff review failures. Adapter-required rows are expected inventory, not failures.

Execution migration scope:

- `icelines fetch rosters` acquires each team roster source object through FLETCH, then ICELINES parses `RosterResponse`, writes the rosters snapshot, and seals it.
- `icelines fetch money-puck` acquires the MoneyPuck CSV through FLETCH, then ICELINES parses/derives `moneypuck.json` and seals the snapshot.
- Paged stats reports, ESPN transaction windows, player landing batches, boxscore/play-by-play batches, contracts, and career history stay adapter-required.
