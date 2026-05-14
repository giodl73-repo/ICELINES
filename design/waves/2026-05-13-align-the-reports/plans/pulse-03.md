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

- `PlayerRecordsView` / `TeamRecordsView` in core or fetch-owned ViewModel code.
- `icelines records player/team` CLI.
- CSV/JSON output and report-list catalog promotion from `planned` to
  `available`.

## Gates

- L0 ViewModel tests.
- L2 records CLI tests.
