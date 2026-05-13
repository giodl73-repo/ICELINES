---
wave: hart-normalizes-the-core
pulse: 01
date: 2026-05-13
status: backfilled
kind: provenance
---

# Pulse 01 - Provenance and Shipped Slices

## Evidence Commits

| Commit | Evidence |
|---|---|
| `1f719a16` | Phase Hart.0 trophy naming, normalization plan, survey lock |
| `b7425d25` | normalized types in `icelines-core` |
| `fa5fde04` | `StatsRepository` + `PlayerView` + LRU + repo swap |
| `cc3375ee` | stats loader, `LoadOutcome`, parallel run |
| `1855ed90` | `flat_view_legacy` adapter for consumer migration |
| `686a8ce2` | Hart.4.1 test foundation |
| `034aa0cd` | `PlayerFilter::apply` / `apply_views` parity pin |
| `b945d5f7` | `DepthChartSlot` and depth consumers |
| `83a7f209` | export migrated to `PlayerView` |
| `7bcdc5fe` | Hart.5c.6 additive TUI repo migration foundation |
| `3b003e73` | delete legacy fields, widen schedule cache key |
| `90235851` | playoff schema/API `gameTypeId` parameterization |
| `50c5a54b` | TUI playoff toggle and nav marker |

## Carry Forward

- Use ViewModels for new surfaces instead of reviving legacy player structs.
- Keep season type explicit in new query/data contracts.
