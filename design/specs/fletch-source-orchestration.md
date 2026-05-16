# ICELINES FLETCH Source Orchestration

ICELINES has a larger source surface than ROUTE or BISECT: some inputs are single public HTTP objects, while others are paged, expanded from schedules, or expanded from active player sets. The migration boundary is therefore incremental.

FLETCH owns neutral source-byte acquisition for stable single-object URLs and paged JSON reports currently routed through the migration slice: team roster JSON, MoneyPuck CSV, and NHL stats API paged report envelopes. ICELINES keeps ownership of snapshot sealing, active snapshot pointers, manifest shards, stale/TTL decisions, fetch locks, parsing, event-stream writes, and hockey-domain validation.

ESPN transaction windows, player landing batches, boxscore/play-by-play batches, contracts, and career history remain adapter-required until FLETCH has generic batch expansion and rate-limit primitives that match ICELINES semantics.

The non-mutating command is:

```text
icelines fetch fletch-sources --season 20252026 --type regular --gate
icelines fetch fletch-partitions --season 20252026 --type regular --gate
icelines fetch fletch-quivers --season 20252026 --type regular --gate
```

It writes `data/fletch-source-handoff.csv` and fails the gate only on registry or ICELINES handoff review failures. Adapter-required rows are expected inventory, not failures.
The partition command writes `data/fletch-query-partitions.json`, mapping ICELINES query-facing surfaces to FLETCH partition and rollup identifiers. It records which source fletches can already be acquired generically and which still need ICELINES adapters before a partition can become active.
The quiver command writes `data/fletch-query-quivers.json`, grouping query partitions into season/offline bootstrap bundle candidates. It is a handoff report, not a byte export or activation step.

Execution migration scope:

- `icelines fetch rosters` acquires each team roster source object through FLETCH, then ICELINES parses `RosterResponse`, writes the rosters snapshot, and seals it.
- `icelines fetch money-puck` acquires the MoneyPuck CSV through FLETCH, then ICELINES parses/derives `moneypuck.json` and seals the snapshot.
- `icelines fetch report` acquires NHL stats paged JSON through FLETCH's generic paged cacheline, then ICELINES writes the report envelope into its snapshot layout and remains responsible for typed loading and validation.
- `icelines fetch fletch-partitions --gate` projects query surfaces such as leaders, player, compare, goalies, windowed game-line queries, career queries, roster bios, and MoneyPuck advanced metrics onto durable partition/rollup IDs. FLETCH source cache presence is not treated as active query data; ICELINES sealed snapshots and active pointers remain the activation evidence.
- `icelines fetch fletch-quivers --gate` groups those partition rows into query bootstrap and enrichment quiver candidates while preserving the rule that ICELINES must parse, validate, seal, and activate snapshots before queries trust imported or staged bytes.
- ESPN transaction windows, player landing batches, boxscore/play-by-play batches, contracts, and career history stay adapter-required.
