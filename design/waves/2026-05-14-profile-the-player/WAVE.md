---
wave: profile-the-player
date_open: 2026-05-14
status: closed
source: user request for comprehensive player screens including records, streaks, and NHL awards
---

# Profile the Player

## Mission

Make the player experience coherent: a user should know every screen a player
can have, what question each screen answers, which data source powers it, and
which CLI/TUI/web surface exposes it.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Player screen taxonomy | Define the complete player screen set and navigation model. | Keep adding one-off player commands without a screen map. |
| Records and streaks | Place individual records and streaks as first-class player tabs/screens. | Hide them only in query filters. |
| NHL awards | Treat awards as a player "Trophy Case" screen backed by `/player/{id}/landing.awards`. | Invent awards from season stats or scrape unvalidated pages. |
| Surface parity | Align CLI commands, TUI screens/hints, web routes, and API twins. | Put player-specific computation directly in renderers. |

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Player screen map | done | `PLAYER-SCREEN-MAP.md`; awards source probe |
| 02 - Player records TUI screen | done | `Screen::PlayerRecordsById`; `r` from player card; `:records player <name>` opens records screen |
| 03 - Player streaks screen | done | `PlayerStreaksView`; `icelines streaks`; `/player/:id/streaks`; `Screen::PlayerStreaksById` |
| 04 - Player awards Trophy Case | done | `PlayerAwardsView`; `icelines awards`; `/player/:id/awards`; `Screen::PlayerAwardsById` |
| 05 - Player navigation polish | done | player-card hub hints; web player links; `:scouting player`; `:mates player` |

## Role notes

- **glass**: a player should feel like a hub with predictable tabs/screens, not
  a set of unrelated commands.
- **tape**: NHL awards must come from the landing endpoint `awards[]`, not from
  inferred stat leaders.
- **edge**: streaks require explicit game/event rows; do not infer streaks from
  season totals.
- **forge**: player screen data belongs in core ViewModels and fetch providers;
  TUI/web/CLI render those models.
- **wire**: awards endpoint shape is external and optional; parser should keep
  missing awards as an empty Trophy Case, not a failed player card.

## Current Result

Pulse 01 defines the player screen taxonomy. A comprehensive player should have
10 first-class screens:

1. Overview
2. NHL career table
3. Game log
4. Records
5. Streaks and windows
6. Awards / Trophy Case
7. Scouting report
8. Peers and comparisons
9. Mates and deployment
10. Fantasy/watch context

The NHL landing endpoint already carries `awards[]` with trophy names and
season rows, so awards are viable as a data-backed screen once parsed. Pulse 02
adds a dedicated TUI player records screen that renders all current player
record metrics from the shared `PlayerRecordsView`. Pulse 04 adds the Awards /
Trophy Case ViewModel and CLI/web/TUI surfaces backed by landing `awards[]`.
Pulse 03 adds the Streaks screen from cached boxscore game rows, with CLI, TUI,
web, and API surfaces sharing `PlayerStreaksView`. Pulse 05 polishes the player
card into the navigation hub for records, awards, streaks, scouting, compare,
groups/favorites, and fantasy/watch handoffs.

## Closeout

The wave is closed. IceLines now has a documented 10-screen player taxonomy,
first-class records, awards, and streaks player surfaces, and player cards act
as hubs into the shipped CLI/TUI/web/API routes without duplicating computation
outside shared ViewModels and fetch providers.
