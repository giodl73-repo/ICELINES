---
wave: trace-the-events
date_open: 2026-05-13
status: closed
source: residual records gaps after align-the-reports
---

# Trace the Events

## Mission

Validate and ingest NHL play-by-play event participants so IceLines can add
richer individual records, especially goalies a player scored against and fight
opponents, without guessing from aggregate goalie or PIM totals.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Event source | Prove which play-by-play fields identify scorers, goalies in net, penalty actors, and fight opponents. | Infer goalie/fight records from current boxscore aggregate rows. |
| Fetch/cache | Add a typed play-by-play fetch/store path that can reuse the manifest-first data layer. | Rebuild the whole sync engine or require live network for records views. |
| Records | Extend `records` ViewModels and surfaces with validated goalie/fight metrics. | Put record math in CLI, TUI, or web renderers. |
| Surface alignment | Keep `report list`, `records`, player/team pages, and APIs in sync as each metric becomes available. | Add one-off report commands outside the records family. |

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Event participant source inventory | done | `EVENT-DATA-INVENTORY.md`; endpoint probes for goals, empty-net goals, and fights |
| 02 - Play-by-play fetch/store path | done | `icelines fetch play-by-play`; `DataKind::PlayByPlay`; parser/store tests |
| 03 - Goalies-scored-against records | done | `goalies-scored-against`; `goalies-beaten-by-team`; core/fetch/CLI/web tests |
| 04 - Fight-opponent records | done | `fight-opponents`; `fight-opponents-by-team`; reciprocal fighting-major dedup |
| 05 - Records surface parity refresh | done | `report list`; `COMMANDS.md`; TUI hints; metric-aware web/API records routes |

## Role notes

- **tape**: the event endpoint is the source of truth for event participants;
  aggregate goalie lines and PIM totals remain non-sources for individual
  opponent records.
- **wire**: the endpoint is external and schema-drifting; the fetch layer should
  preserve raw JSON and expose a narrow typed projection with optional fields for
  absent goalie/opponent ids.
- **forge**: keep pure grouping and dedup logic in `icelines-core`; network and
  manifest reads belong in `icelines-fetch`; CLI/web/TUI render only ViewModels.
- **edge**: empty-net goals have no goalie in net and must not count as a goalie
  beaten; fighting majors often appear as reciprocal penalty rows and must be
  deduped by a stable pair key.
- **glass**: once reliable, new metrics should appear through the existing
  records command/page family rather than requiring users to discover a new mode.

## Current Result

Pulse 01 validates that `/v1/gamecenter/{id}/play-by-play` carries the fields
needed for the remaining records family. Goals expose `scoringPlayerId` and,
when a goalie is present, `goalieInNetId`; empty-net goals can omit
`goalieInNetId`; fighting majors expose reciprocal `committedByPlayerId` /
`drawnByPlayerId` penalty rows.

Pulse 02 adds that cached play-by-play path. The fetch layer now has a narrow
`PlayByPlay` projection for goals and penalties, the manifest has a
`PlayByPlay` shard, and `icelines fetch play-by-play [--date YYYY-MM-DD]
[--for-favorites] [--dry-run]` persists raw event JSON under the data store.

The next pulse should derive goalie-beaten records from cached play-by-play
goals, counting only rows with explicit `goalieInNetId`.

Pulse 03 promotes goalie records. Player records can now count
`goalies-scored-against`; team records can count `goalies-beaten-by-team`.
Rows without `goalieInNetId` remain excluded instead of inferred.

Pulse 04 promotes fight records. Player records can now count
`fight-opponents`; team records can count `fight-opponents-by-team`.
Reciprocal fighting-major penalty rows are normalized to one fight pair before
the ViewModels count directed player/team rows.

Pulse 05 refreshes surface parity. `report list`, `COMMANDS.md`, TUI handoff
hints, and `/records/...?...metric=...` web/API routes now describe and expose
the full event-backed records slice.

## Closeout

The wave is closed. IceLines now has a cached play-by-play data path and a
complete first event-backed records set: teams scored against, goalies scored
against, players scored against team, goalies beaten by team, player fight
opponents, and team fight opponents. No metric is inferred from aggregate goalie
or PIM totals.
