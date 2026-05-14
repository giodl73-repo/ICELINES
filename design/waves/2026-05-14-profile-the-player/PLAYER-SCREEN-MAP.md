# Player Screen Map

## Answer

Yes: **NHL awards should be a first-class player screen**. The screen should be
called **Awards** or **Trophy Case** and should be backed by the NHL landing
endpoint's `awards[]` array.

A complete IceLines player should have **10 player screens**. Some already
exist, some exist as commands/routes but not as dedicated TUI screens, and some
need new ViewModels.

## Current state

| Screen | TUI today | Web today | CLI today | Status |
|---|---|---|---|---|
| Overview | `PlayerById` player card | `/player/:id` | `query player <name>` | exists |
| NHL career table | embedded on player card | embedded on `/player/:id` | `history <player>`, `query player --seasons` | exists but should become a tab/section |
| Game log | not a player screen | game pages exist, not player game log | partial through boxscore-driven commands | planned |
| Records | hint only | `/records/player/:id?metric=...` | `records player <name> --metric ...` | web/CLI exists; TUI screen missing |
| Streaks and windows | not a player screen | not a player screen | query grammar can express streak filters | planned |
| Awards / Trophy Case | not modeled | not modeled | not modeled | planned |
| Scouting report | not a main player tab | `/scouting/:id` | `scouting <player>` | exists as separate surface |
| Peers and comparisons | `CompsById` exists | `/compare` exists | `peers`, `compare` | exists but not hubbed from player |
| Mates and deployment | not a player tab | not a player route | `mates <player>` | exists as CLI-only |
| Fantasy/watch context | favorites/watch surfaces, not player tab | favorites/watch pages, not player tab | `poach`, `watch`, `favorites` | planned as player context |

## Proposed 10-screen player system

| # | Screen | User question | Data source / ViewModel direction | First-class surfaces |
|---:|---|---|---|---|
| 1 | Overview | Who is this player right now? | existing `PlayerCardView` | TUI `PlayerById`, web `/player/:id`, CLI `query player` |
| 2 | Career | What did he do by season and league? | existing NHL career rows + Calder `CareerHistory` | TUI tab, web section/page, CLI `history` / `query player --seasons` |
| 3 | Game Log | What happened game by game? | cached boxscore/play-by-play rows | TUI tab, web `/player/:id/games`, API twin |
| 4 | Records | What symmetric records does he own? | `PlayerRecordsView` | TUI records screen, web `/records/player/:id?metric=...`, CLI `records player` |
| 5 | Streaks | What runs did he have? | game log/window ViewModel; no season-total inference | TUI tab, web `/player/:id/streaks`, CLI `streaks player` or `records player --metric streaks` |
| 6 | Awards / Trophy Case | What NHL awards and trophy seasons does he have? | new `PlayerAwardsView` from landing `awards[]` | TUI tab, web `/player/:id/awards`, API twin, CLI `awards player` |
| 7 | Scouting | What kind of player is he? | existing scouting report ViewModel | TUI tab/handoff, web `/scouting/:id`, CLI `scouting` |
| 8 | Peers / Compare | Who is he like, and how does he compare? | existing peers/compare ViewModels | TUI comps screen, web compare, CLI `peers` / `compare` |
| 9 | Mates / Deployment | Who does he play with and how is he used? | linemate/boxscore/deployment data | TUI tab, web `/player/:id/mates`, CLI `mates` |
| 10 | Fantasy / Watch | Should I roster, watch, trade, or poach him? | poach/favorites/watch/fantasy ViewModels | TUI tab/context panel, web player fantasy card, CLI `poach` / `watch` |

## NHL awards source

Validated against `GET https://api-web.nhle.com/v1/player/8478402/landing`.
The response includes an `awards` key.

Observed shape:

```text
awards[]:
  trophy.default: "Art Ross Trophy"
  trophy.fr: "Trophee Art Ross"       # optional
  seasons[]:
    seasonId: 20252026
    gameTypeId: 2
    gamesPlayed: 82
    goals: 48
    assists: 90
    points: 138
    plusMinus: 17
    pim: 44
    hits: 40
    blockedShots: 30
```

Awards are therefore viable as a real screen. They should not be inferred from
leaderboards because official award history includes voted trophies and playoff
awards such as Conn Smythe.

## Recommended build order

1. **Player Records TUI screen**: closes the newest gap and reuses existing
   `PlayerRecordsView`.
2. **Awards / Trophy Case**: adds a high-value player story screen from an
   already validated source.
3. **Streaks**: needs game-log/window semantics; build after player game log
   source shape is explicit.
4. **Game Log**: foundational for streaks, hot/cold form, and fantasy context.
5. **Player hub navigation**: link all screens from overview in TUI and web.

## Role decisions

- **glass**: use "player hub + tabs/screens" language. Do not make users guess
  whether awards live under `query`, `records`, or `scouting`.
- **tape**: awards are official endpoint data; streaks are not official season
  totals and must come from game rows.
- **edge**: awards can be empty for most players; empty Trophy Case is a valid
  complete result.
- **wire**: parser must accept missing `awards` or missing season stat fields.
- **forge**: add `PlayerAwardsView` in core and parser/provider in fetch before
  adding renderers.
