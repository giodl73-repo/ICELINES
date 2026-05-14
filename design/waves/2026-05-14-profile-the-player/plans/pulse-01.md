# Pulse 01 - Player Screen Map

## Goal

Answer how many screens a complete player should have and whether NHL awards
belong in that system.

## Work completed

1. Audited current player surfaces across CLI, TUI, web, and API.
2. Validated that the NHL landing endpoint exposes official `awards[]` data.
3. Wrote `PLAYER-SCREEN-MAP.md` with a 10-screen player taxonomy.
4. Opened follow-up pulses for records TUI, streaks, awards, and navigation.

## Result

A complete player should have **10 first-class screens**:

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

NHL awards are explicitly in scope as the **Awards / Trophy Case** screen. The
data source is `/v1/player/{id}/landing.awards[]`.

## Gates

- `proof check design\waves\2026-05-14-profile-the-player design\waves\PHASES.md --errors-only`
