# Pulse 03 - Records ViewModels and CLI surface

## Goal

Implement records logic once in library code and expose it through a canonical
CLI surface.

## Proposed CLI

```bash
icelines records player "Andre Burakovsky" --metric teams-scored-against
icelines records player "Andre Burakovsky" --metric goalies-scored-against --json
icelines records player "Andre Burakovsky" --metric fight-opponents --csv
icelines records team SEA --metric player-goals-against
```

## Deliverables

- Extend parsed goal rows with scorer ids before exposing player goal records.
- `PlayerRecordsView` / `TeamRecordsView` in core or fetch-owned ViewModel code.
- `icelines records player/team` CLI.
- CSV/JSON output and report-list catalog promotion from `planned` to
  `available`.

## Metric order

1. `teams-scored-against` / `players-scored-against-team`.
2. appearance head-to-head counts from boxscore player ids.
3. `goalies-scored-against` after goalie-on-ice event data is available.
4. `fight-opponents` after penalty/fighting participant event data is available.

## Gates

- L0 ViewModel tests.
- L2 records CLI tests.
