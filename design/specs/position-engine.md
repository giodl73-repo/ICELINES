# IceLines Position Engine — Specification

**Version**: 0.1  
**Date**: 2026-04-25  
**Status**: Draft — pre-implementation

---

## 1. Overview

The position engine derives each player's primary position and multi-position eligibility
entirely from NHL API data. There is no Yahoo CSV dependency for position data.

Two sources feed the engine:

1. **Bios API `positionCode`** — the player's registered position (C, L, R, D, G). Used as
   the authoritative primary position when boxscore history is insufficient.
2. **Boxscore API `position` field** — the position at which a player actually appeared in
   each individual game this season. Aggregated across all games to determine how a player
   is actually deployed, which may differ from their registered position.

The output of the position engine is a `PositionProfile` per player:
- `primary_position`: the position they play most often this season
- `eligible_positions`: all positions meeting the multi-eligibility threshold
- `appearance_counts`: raw counts per position across all boxscores processed

---

## 2. Data Sources

### 2.1 Bios API — `positionCode`

```
GET https://api.nhle.com/stats/rest/en/skater/bios
    ?cayenneExp=seasonId={SEASON}%20and%20gameTypeId=2&limit=100&start={N}
```

Response field: `positionCode` — one of `"C"`, `"L"`, `"R"`, `"D"`, `"G"`.

This is the player's registered position with the league. Used as:
- The initial `primary_position` before boxscore data is available
- The fallback `primary_position` when a player has fewer than 5 boxscore appearances
  this season (too small a sample to override registration)

### 2.2 Boxscore API — per-game `position` field

```
GET https://api-web.nhle.com/v1/gamecenter/{GAME_ID}/boxscore
```

Response structure (relevant excerpt):

```json
{
  "playerByGameStats": {
    "awayTeam": {
      "forwards": [
        {
          "playerId": 8478402,
          "name": { "default": "Connor McDavid" },
          "position": "C",
          "toi": "21:43",
          "shifts": 25,
          "faceoffWinningPctg": 0.571
        }
      ],
      "defense": [...],
      "goalies": [...]
    },
    "homeTeam": { ... }
  }
}
```

The `position` field in `forwards` and `defense` arrays reflects the position at which the
player was actually deployed in that specific game by their coach. A center who occasionally
plays left wing will appear as `"L"` in those games.

**Game IDs** are obtained from the player's game log, which is available via the player
landing API:

```
GET https://api-web.nhle.com/v1/player/{PLAYER_ID}/game-log/{SEASON}/2
```

This returns a list of game IDs for all regular-season games the player appeared in.

---

## 3. Position Code Mapping

The NHL API uses single-letter codes. IceLines normalizes these to the `Position` enum defined
in `icelines-core`:

| API code | IceLines `Position` | Display label |
|----------|---------------------|---------------|
| `C`      | `Position::Center`  | C             |
| `L`      | `Position::LeftWing`| LW            |
| `R`      | `Position::RightWing` | RW          |
| `D`      | `Position::Defense` | D             |
| `G`      | `Position::Goalie`  | G             |

Any code not in this table causes a schema validation error (logged and the player is
flagged for manual review, not silently dropped).

---

## 4. Primary Position Algorithm

The primary position is the position at which the player appeared in the most games this
season, derived from the boxscore `position` field.

```
primary_position = argmax(appearance_counts)
```

Tie-breaking rule: if two positions have equal appearance counts, prefer the position
that matches the bios API `positionCode`. If neither matches `positionCode`, prefer
the position with the higher typical scoring rate (C > LW > RW > D).

**Minimum sample fallback**: If the player has fewer than 5 boxscore appearances this season
(e.g., they are newly recalled or the season is very early), the `positionCode` from the bios
API is used directly as `primary_position` and no multi-position eligibility is computed.

---

## 5. Multi-Position Eligibility Algorithm

A player is eligible at a second (or third) position if they have appeared there in at least
20% of their total games this season.

```
threshold = 0.20 × total_games_played
eligible_positions = { pos : appearance_counts[pos] >= threshold }
```

The `primary_position` is always included in `eligible_positions` regardless of appearance
count.

**Examples:**

| Player         | Appearances          | Total GP | 20% threshold | primary | eligible         |
|----------------|----------------------|----------|---------------|---------|------------------|
| C. McDavid     | C=70                 | 70       | 14            | C       | [C]              |
| L. Draisaitl   | C=45, L=25           | 70       | 14            | C       | [C, LW]          |
| B. McMann      | L=78                 | 78       | 15.6 → 16     | LW      | [LW]             |
| N. Ehlers      | L=40, R=10, C=5      | 55       | 11            | LW      | [LW, RW]         |
| R. Nurse       | D=65                 | 65       | 13            | D       | [D]              |

Threshold is compared against raw appearance count using integer comparison:
`count >= ceil(0.20 × total_gp)`. The `ceil` avoids floating-point edge cases at the boundary.

**Counting rule**: Each game in which the player appears in the boxscore contributes exactly
one count to the position under which they are listed in that game's boxscore. If a player
switches positions mid-game (e.g., a forward who takes a penalty-kill shift on defense), only
the listed boxscore position is counted — the boxscore API does not expose per-shift position
granularity.

---

## 6. `PositionProfile` Type

Defined in `icelines-core`:

```rust
/// Derived position data for a player from actual game deployment this season.
pub struct PositionProfile {
    pub player_id: u32,
    pub season: u32,
    /// Position the player appeared at most frequently this season.
    pub primary_position: Position,
    /// All positions meeting the 20% threshold, always includes primary_position.
    pub eligible_positions: Vec<Position>,
    /// Raw per-position appearance counts across all boxscores processed.
    pub appearance_counts: HashMap<Position, u32>,
    /// Total games processed (may be less than GP if some boxscores are not yet cached).
    pub games_processed: u32,
    /// True if this profile used the bios positionCode fallback (sample too small).
    pub is_fallback: bool,
}
```

---

## 7. Integration with Depth Chart Builder

The depth chart builder in `icelines-core` uses `PositionProfile` exclusively — it does not
consult Yahoo eligible positions, which are no longer part of the data model.

When assigning players to line slots:
1. The builder first attempts to place the player in a slot matching their `primary_position`.
2. If no slot is available for `primary_position`, the builder checks `eligible_positions`
   for an open slot.
3. If no eligible slot exists, the player is placed in `DepthChart::unplaced`.

This is a direct replacement for Yahoo's "Eligible Positions" column. The difference is that
Yahoo eligibility is based on games played in the prior season or a fixed roster window,
whereas IceLines eligibility is derived from the current season's actual deployment.

---

## 8. `icelines fetch positions` Subcommand

```
icelines fetch positions [OPTIONS]

Options:
  --season <YEAR>    Season in YYYYZZZZ format [default: current]
  --player <ID>      Fetch and recompute for a single player ID only
  --refresh          Invalidate cache and re-fetch all boxscores
  -v, --verbose      Show per-player progress and appearance counts
```

**Behavior:**

1. Load all player IDs from the cached bios (`~/.icelines/cache/stats/{SEASON}/bios.json`).
   If bios are not cached, exit with an error directing the user to run `icelines fetch stats`.

2. For each player, fetch their game log to obtain game IDs for this season:
   ```
   GET https://api-web.nhle.com/v1/player/{PLAYER_ID}/game-log/{SEASON}/2
   ```

3. For each game ID in the player's game log, fetch the boxscore:
   ```
   GET https://api-web.nhle.com/v1/gamecenter/{GAME_ID}/boxscore
   ```
   Boxscores are shared across players — if game G has 40 players from both teams,
   processing game G once populates position data for all 40. The fetch layer
   deduplicates game IDs across players and fetches each boxscore once.

4. For each player, aggregate appearance counts across all their boxscore games and
   compute the `PositionProfile` using the algorithm in §4 and §5.

5. Write the `PositionProfile` to cache (§9).

**Progress reporting:** When `-v` is passed, print one line per player:
```
[8478402] Connor McDavid — 70 games processed — primary: C — eligible: [C]
[8481528] Leon Draisaitl — 70 games processed — primary: C — eligible: [C, LW]
```

**Exit codes:**
- `0` — all players processed successfully
- `1` — one or more players could not be resolved (listed at the end of output)

---

## 9. Cache Layout

```
~/.icelines/cache/
  positions/
    {SEASON}/
      {PLAYER_ID}.json        # Serialized PositionProfile
  boxscores/
    {SEASON}/
      {GAME_ID}.json          # Full boxscore response (shared across all players in game)
```

**TTL policy:**
- Boxscores for completed games never expire — a finished game does not change. Once cached,
  a boxscore file is never re-fetched unless `--refresh` is passed.
- Boxscores for games played today or in the future are not cached (they may still be live
  or not yet played).
- `PositionProfile` files are recomputed from cached boxscores on each `fetch positions` run.
  They do not have an independent TTL; they are always derived from boxscore data.

**Staleness detection:** If the player's game log contains a game ID not present in the
boxscore cache, that game is fetched. If it is present, it is read from disk. The position
profile is then recomputed from all available boxscores, including newly fetched ones.

---

## 10. Non-Goals

- **Live game position tracking.** Position data is only fetched from completed games.
  In-progress games are not included.
- **Shift-level granularity.** The engine counts boxscore-listed position per game,
  not per shift. Intra-game position switching is not tracked.
- **Historical season position history.** The engine operates on the current season only.
  Prior-season deployment data is not used for multi-position eligibility.
- **Goalie deployment tracking.** Goalies are identified by position `G` in the boxscore
  and included in the `appearance_counts` map, but `PositionProfile` for goalies is not
  used by the depth chart builder or any current command.
