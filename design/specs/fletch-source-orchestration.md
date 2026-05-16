# ICELINES FLETCH Source Orchestration

ICELINES has a larger source surface than ROUTE or BISECT: some inputs are single public HTTP objects, while others are paged, expanded from schedules, or expanded from active player sets. The migration boundary is therefore incremental.

FLETCH owns neutral source-byte acquisition for stable single-object URLs, paged JSON reports, and already-expanded HTTP batches currently routed through the migration slice: team roster JSON, MoneyPuck CSV, NHL stats API paged report envelopes, Gamecenter boxscore/play-by-play bytes after ICELINES expands a date schedule into game IDs, player landing bytes after ICELINES expands the active/bundled player set, and ESPN transaction bytes after ICELINES expands a season into date windows. ICELINES keeps ownership of snapshot sealing, active snapshot pointers, manifest shards, stale/TTL decisions, fetch locks, parsing, event-stream writes, and hockey-domain validation.

ICELINES still owns every domain expansion rule; FLETCH only acquires the expanded URLs and records cache evidence.

The non-mutating command is:

```text
icelines fetch fletch-sources --season 20252026 --type regular --gate
icelines fetch fletch-partitions --season 20252026 --type regular --gate
icelines fetch fletch-quivers --season 20252026 --type regular --gate
icelines fetch fletch-cache-index --season 20252026 --type regular --gate
```

It writes `data/fletch-source-handoff.csv` and fails the gate only on registry or ICELINES handoff review failures. Adapter-required rows are expected inventory, not failures.
The partition command writes `data/fletch-query-partitions.json`, mapping ICELINES query-facing surfaces to FLETCH partition and rollup identifiers. It records which source fletches can already be acquired generically and which still need ICELINES adapters before a partition can become active.
The quiver command writes `data/fletch-query-quivers.json`, grouping query partitions into season/offline bootstrap bundle candidates. It is a handoff report, not a byte export or activation step.
FLETCH-backed ICELINES fetches upsert `cache-manifest.json` under the ICELINES FLETCH cache root. The cache-index command reads that manifest by default and writes `data/fletch-cache-index.json`, mapping `fletch.cache-index.v1` evidence back onto ICELINES registered source IDs.

Execution migration scope:

- `icelines fetch rosters` acquires each team roster source object through FLETCH, then ICELINES parses `RosterResponse`, writes the rosters snapshot, and seals it.
- `icelines fetch money-puck` acquires the MoneyPuck CSV through FLETCH, then ICELINES parses/derives `moneypuck.json` and seals the snapshot.
- `icelines fetch report` acquires NHL stats paged JSON through FLETCH's generic paged cacheline, then ICELINES writes the report envelope into its snapshot layout and remains responsible for typed loading and validation.
- `icelines fetch boxscore` and `icelines fetch play-by-play` let ICELINES expand the date schedule and favorite filters, then acquire the resulting Gamecenter HTTP set through FLETCH's generic batch cacheline before ICELINES parses, persists manifests, and writes event-stream records.
- `icelines fetch contracts` and `icelines fetch career` let ICELINES expand the player set, then acquire player landing HTTP bytes through FLETCH's paced generic batch cacheline before ICELINES parses contracts/career history and writes its snapshots/blobs.
- `icelines fetch transactions` lets ICELINES expand the season into ESPN date windows, then acquire each transaction window through FLETCH's generic batch cacheline before ICELINES parses schema drift, classifies prose, writes stale flags, and seals the snapshot.
- `icelines fetch fletch-partitions --gate` projects query surfaces such as leaders, player, compare, goalies, windowed game-line queries, career queries, roster bios, and MoneyPuck advanced metrics onto durable partition/rollup IDs. FLETCH source cache presence is not treated as active query data; ICELINES sealed snapshots and active pointers remain the activation evidence.
- `icelines fetch fletch-quivers --gate` groups those partition rows into query bootstrap and enrichment quiver candidates while preserving the rule that ICELINES must parse, validate, seal, and activate snapshots before queries trust imported or staged bytes.
- `icelines fetch fletch-cache-index --gate` maps the ICELINES-owned FLETCH cache manifest to compact cache-index evidence and reuses FLETCH's shared cache-index gate contract after mapping dynamic child cachelines back to registered ICELINES parent sources. Missing source rows are allowed because not every source is fetched on every run; unverified or unexpected rows fail the gate.
