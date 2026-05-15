---
wave: measure-the-finish
date_open: 2026-05-14
status: active
source: Rocket Richard Wave 1 closeout and the Phase Rocket Richard roadmap
---

# Measure the Finish

## Mission

Deepen Rocket Richard scoring intelligence from raw scoring-event reports into
player-level finishing context: shot volume trends, conversion trends, streak
leaderboards, and a deliberately owned shot-quality proxy that does not claim
third-party xG parity.

## Award Fit

The Rocket Richard Trophy rewards goal scoring. This wave asks whether a player
is merely finishing chances, creating pressure repeatedly, or riding a short
streak. The output should help a user answer: "who is driving scoring right now,
who is due for regression, and who is carrying the best finishing run?"

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Player scoring trends | Add ViewModel-owned trend rows for recent shot volume, goals, and conversion. | Put trend math in web templates or TUI renderers. |
| Streak leaderboards | Surface best goal, assist, point, and shot streaks at player/team levels where cache exists. | Infer missing game lines from season totals. |
| Shot-quality proxy | Specify and test an IceLines-owned distance/location proxy from official coordinates. | Claim Natural Stat Trick / MoneyPuck xG or scrape proprietary chance buckets. |
| Surface expansion | Extend web/API first, then CLI/TUI only where a ViewModel already exists. | Add projections or odds before the proxy contract is reviewed. |

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Finish inventory and proxy contract | planned | `plans/pulse-01.md` |
| 02 - Player scoring trend rows | planned | depends on Pulse 01 |
| 03 - Streak leaderboards | planned | depends on cached game-line/streak primitives |
| 04 - Shot-quality proxy implementation | planned | depends on Pulse 01 contract |
| 05 - Surface parity and wave closeout | planned | depends on Pulses 02-04 |

## Role Notes

- **scout**: trend language must read like hockey context, not a betting pick.
- **edge**: missing coordinates, shooter IDs, and partial game caches must remain
  explicit source-state or nullable data.
- **bench**: every trend/proxy threshold needs known-value tests from fixtures.
- **wire**: cache reads are GET-only; warming remains Admin/CLI POST or fetch.

## Current Result

Wave opened after Aim the Rocket completed and CI passed through Pulse 06.

## Next

Execute Pulse 01: inventory the current player scoring profile and streak data,
then write the first owned shot-quality proxy contract before implementing
trend/proxy math.
