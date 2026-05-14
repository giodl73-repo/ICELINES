# Rocket.1 - Scoring Data Inventory

## Purpose

Ground Phase Rocket Richard in data IceLines can legally and reliably own:
official NHL play-by-play JSON already fetched through `NhlApiClient` and
persisted in the manifest-backed game cache.

## Current IceLines state

| Area | Current state | Implication |
|---|---|---|
| Raw source | `NhlApiClient::fetch_play_by_play_with_raw(game_id)` fetches `/v1/gamecenter/{id}/play-by-play` and returns raw JSON. | Scoring reports can reuse the existing official source. |
| Persistence | `game_cache::GameCacheArtifact::PlayByPlay` stores raw JSON under `DataKind::PlayByPlay`. | No new cache spine is needed. |
| DataStore | `DataStore::load_play_by_play_raw(DataKey::Game(GameId))` reads persisted raw play-by-play. | Providers can project shot events from cached raw bytes. |
| Typed parser | `parse_play_by_play` currently projects only `goals` and `penalties`. | Rocket must add shot-event projections before UI work. |
| Existing consumers | `records_provider` uses play-by-play goals for goalie-beaten records and penalties for fight records. | New shot parsing must preserve existing records behavior. |

## Official NHL play-by-play fields verified

Checked against official NHL web API sample:

```text
https://api-web.nhle.com/v1/gamecenter/2025020001/play-by-play
```

The first-period event sample includes these event families and detail keys:

| Event type | Detail keys observed |
|---|---|
| `goal` | `xCoord`, `yCoord`, `zoneCode`, `shotType`, `scoringPlayerId`, `scoringPlayerTotal`, `assist1PlayerId`, `assist1PlayerTotal`, `assist2PlayerId`, `assist2PlayerTotal`, `eventOwnerTeamId`, `goalieInNetId`, `awayScore`, `homeScore`, highlight clip fields |
| `shot-on-goal` | `xCoord`, `yCoord`, `zoneCode`, `shotType`, `shootingPlayerId`, `goalieInNetId`, `eventOwnerTeamId`, `awaySOG`, `homeSOG` |
| `missed-shot` | `xCoord`, `yCoord`, `zoneCode`, `reason`, `shotType`, `shootingPlayerId`, `goalieInNetId`, `eventOwnerTeamId` |
| `blocked-shot` | `xCoord`, `yCoord`, `zoneCode`, `blockingPlayerId`, `shootingPlayerId`, `eventOwnerTeamId`, `reason` |
| `faceoff` | `eventOwnerTeamId`, `losingPlayerId`, `winningPlayerId`, `xCoord`, `yCoord`, `zoneCode` |
| `hit` | `xCoord`, `yCoord`, `zoneCode`, `eventOwnerTeamId`, `hittingPlayerId`, `hitteePlayerId` |
| `takeaway` / `giveaway` | `xCoord`, `yCoord`, `zoneCode`, `playerId`, `eventOwnerTeamId` |
| `penalty` | `xCoord`, `yCoord`, `zoneCode`, `typeCode`, `descKey`, `duration`, participant IDs, `eventOwnerTeamId` |

Common top-level fields on play events include:

- `eventId`
- `periodDescriptor.number`
- `periodDescriptor.periodType`
- `timeInPeriod`
- `timeRemaining`
- `situationCode`
- `homeTeamDefendingSide`
- `typeCode`
- `typeDescKey`
- `sortOrder`

## Metrics IceLines can compute from official events

| Metric family | Feasibility | Notes |
|---|---|---|
| Shots on goal | Ready after parser extension | Count `shot-on-goal` plus `goal` by team/player. |
| Goals | Already parsed narrowly; needs coordinate/shot fields added | Preserve current goalie-beaten behavior. |
| Shot attempts / Corsi | Ready after parser extension | Count `goal`, `shot-on-goal`, `missed-shot`, `blocked-shot`. |
| Fenwick / unblocked attempts | Ready after parser extension | Count `goal`, `shot-on-goal`, `missed-shot`. |
| Blocked shots for/against | Ready after parser extension | `blocked-shot` carries shooter, blocker, event owner. |
| Period splits | Ready after parser extension | `periodDescriptor.number` exists on events. |
| Strength/situation splits | Requires explicit decoder | `situationCode` is present; a decoder/test matrix must be specified before product claims. |
| Shot location tables | Ready after parser extension | `xCoord`/`yCoord` are present, but optional in the model. |
| Rink plot | Feasible | Requires coordinate normalization and home-team defending-side handling. |
| Danger buckets | Feasible as IceLines-owned proxy | Can bucket by coordinate distance/angle, but must not claim Natural Stat Trick or MoneyPuck proprietary parity. |
| xG | Deferred | Existing MoneyPuck season summaries exist, but event-level xG is not currently an IceLines-owned model. |
| Deserve-to-win meter | Deferred | Requires a documented model and should not imply MoneyPuck equivalence. |

## Contract recommendations

Add core inputs in a Rocket follow-up pulse:

- `ScoringEventInput`
- `ShotEventKind` (`Goal`, `ShotOnGoal`, `MissedShot`, `BlockedShot`)
- `ShotLocation`
- `GameScoringReportView`
- `TeamScoringProfileView`
- `PlayerScoringProfileView`
- `TonightScoringIntelView`

Add fetch/provider projection in `icelines-fetch`:

- parse raw `DataKind::PlayByPlay` into shot events
- preserve existing `PlayByPlayGoal` and `PlayByPlayPenalty` behavior
- keep unknown/missing event fields as `Option<T>`
- expose source-state for missing play-by-play cache

## Constraints

1. No third-party scraping for Natural Stat Trick, HockeyViz, Daily Faceoff, or
   proprietary model pages.
2. No betting odds ingestion in Rocket Richard.
3. No event-level xG claim until IceLines has an owned model/spec or a documented
   allowed source.
4. No renderer-local scoring math; computation belongs in core/fetch ViewModels
   and providers.
5. No fake zeros for absent coordinates, goalie IDs, shooter IDs, or situation
   fields.

## Suggested next pulse

**Pulse 02 - Scoring ViewModel contracts**

Define the core data types and parser extension tests without adding web/TUI
surfaces yet. The exit gate should prove that official NHL shot, goal, missed,
and blocked-shot fixtures parse into typed `ScoringEventInput` rows with stable
IDs, optional coordinates, team IDs, player IDs, period/time, and situation code.
