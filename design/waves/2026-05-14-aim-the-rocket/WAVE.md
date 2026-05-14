---
wave: aim-the-rocket
date_open: 2026-05-14
status: active
source: hockey-stat-site benchmark and user request to turn missing major-site features into the next award phase
---

# Aim the Rocket

## Mission

Open Phase Rocket Richard by proving which scoring-intelligence data IceLines
can own from official NHL sources and existing local caches before building
shot maps, scoring reports, or daily scoring-intel surfaces.

## Award Fit

The Rocket Richard Trophy goes to the NHL's leading goal scorer. This phase is
about explaining goals and scoring pressure: who shoots, who scores, who creates
danger, which teams generate chances, and what a user should know before
tonight's games.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Event source truth | Inventory official NHL play-by-play shot/goal fields and current IceLines parser gaps. | Scrape Natural Stat Trick, HockeyViz, Daily Faceoff, or proprietary model pages. |
| Scoring contracts | Define the first ViewModel/data contracts for game, team, player, and tonight scoring intelligence. | Put scoring calculations directly in web/TUI renderers. |
| Cache path | Reuse the manifest-backed game cache and DataStore as the data-loading spine. | Add a second ad-hoc cache for scoring reports. |
| Product waves | Split Rocket Richard into follow-on waves for cache loading, game/team reports, tonight intel, player scoring profiles, and projections. | Mix salary-cap, natural-language query, or prospect phases into Rocket. |

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Scoring data inventory | done | `SCORING-DATA-INVENTORY.md`; `plans/pulse-01.md`; `panels/rocket-01-review/` |
| 02 - Scoring ViewModel contracts | done | `plans/pulse-02.md`; `icelines-core/src/view_model/scoring.rs`; `icelines-fetch/src/scoring_provider.rs` |
| 03 - Shot-event cache loader | planned | Depends on Pulse 02 contracts |
| 04 - Game/team scoring reports | planned | Depends on shot-event cache loader |
| 05 - Tonight scoring intelligence | planned | Depends on scoring reports and favorites cache path |
| 06 - Player scoring profiles and projections | planned | Depends on game/team scoring report primitives |

## Role Notes

- **tape**: official NHL play-by-play is the row source; MoneyPuck can enrich
  existing advanced summaries, but no third-party scraping is allowed.
- **edge**: coordinates, goalie IDs, and shooter IDs must be optional at the
  model boundary; missing fields are source-state, not fake zeros.
- **wire**: use the existing manifest/DataStore path for raw play-by-play bytes
  and expose cache/load failures explicitly.
- **bench**: every parser extension needs L0 known-shape fixtures plus L1
  tempdir round-trips through DataStore.

## Current Result

Pulse 01 confirms that the official NHL play-by-play payload contains enough
fields to support IceLines-owned shot/chance reporting: shot-on-goal, missed
shot, blocked-shot, and goal events carry event IDs, period/time, situation
code, team owner, shooter/scorer IDs, goalie IDs where applicable, coordinates,
zone code, and shot type. Current IceLines code persists raw play-by-play in
the manifest-backed `DataStore`, but the typed projection currently keeps only
goals and penalties. Rocket should extend that projection rather than introduce
a new data spine.

## Next

Generate Pulse 03 for shot-event cache loading/report source states on top of
the typed scoring-event projection.
