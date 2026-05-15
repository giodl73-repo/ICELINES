# Finish Inventory and Proxy Contract

## Purpose

Measure the Finish extends Rocket Richard from "what happened in a game" to
"what does this player's finishing run mean?" This inventory locks the available
inputs and the first IceLines-owned shot-quality proxy before any trend or proxy
implementation starts.

## Source boundaries

| Source | Cache kind | Current reader | Available for this wave | Non-goal |
|---|---|---|---|---|
| Official NHL play-by-play | `DataKind::PlayByPlay` | `icelines_fetch::scoring_provider` | goals, shots on goal, missed shots, blocked shots, optional coordinates, shooter/scorer IDs, goalie-in-net ID, situation code, shot type, score state | no third-party xG, no scraped chance buckets |
| Official NHL boxscore | `DataKind::Boxscore` | `icelines_fetch::streaks_provider` | per-player game lines with goals, assists, points, team, opponent, date | no shot streaks until shot counts are joined from play-by-play |
| Manifest/cache loader | game-cache `boxscore` / `play-by-play` artifacts | `icelines_fetch::game_cache` | cache warming through CLI/Admin mutation paths; scoring aliases map to `PlayByPlay` | no GET-backed fetch or second scoring cache |

## Scoring-event fields available now

`ScoringEventInput` already carries the fields needed for player scoring trend
rows and an owned location proxy:

- identity: `game_id`, `event_id`, optional `date`;
- event family: `kind` (`goal`, `shot-on-goal`, `missed-shot`, `blocked-shot`);
- timing/context: `period`, `period_type`, `time_in_period`, optional
  `situation_code`;
- team/player IDs: optional event-owner team ID/abbrev, shooter ID, scorer ID,
  blocker ID, goalie-in-net ID;
- location/detail: optional `x_coord`, optional `y_coord`, optional zone code,
  optional shot type, optional reason, optional home-team defending side;
- score state: optional away/home score at the event.

Null semantics are important: missing coordinates mean "the NHL payload did not
provide a location", not "outside", "low danger", or zero distance. Missing
shooter/scorer IDs mean the player cannot be matched for that event without a
separate resolver; the proxy must not guess.

## Streak and game-line fields available now

`PlayerGameLineInput` is boxscore-derived and currently contains:

- `game_id`, optional `date`, player ID/name, team, opponent;
- goals and assists;
- derived points (`goals + assists`).

`PlayerStreaksView` and `TeamPlayerStreaksView` already compute current and
longest goal, assist, and point streaks from game lines. Team leaderboards pick
the best player per metric by longest streak, then current streak, then player
name.

Shot streaks are not available from `PlayerGameLineInput` yet. They should be
implemented by aggregating `ScoringEventInput` per player/game, not by inferring
from season totals.

## First owned shot-quality proxy

The first proxy should be named **IceLines inside-shot proxy**. It is a
descriptive location bucket, not expected goals.

### Coordinate normalization

For any scoring event with both coordinates present:

1. Use the offensive-end absolute x-distance convention:
   `distance_ft = sqrt((89 - abs(x_coord))^2 + y_coord^2)`.
2. Preserve the raw coordinates in the event row.
3. Mark rows with either missing coordinate as `unknown`, not low quality.

This convention avoids overclaiming rink orientation. It treats either attacking
end symmetrically and is sufficient for inside/outside pressure language.

### Buckets

| Bucket | Distance rule | Hockey language |
|---|---:|---|
| `crease` | `distance_ft <= 10` | crease / net-front finish |
| `inside` | `10 < distance_ft <= 25` | inside chance |
| `slot` | `25 < distance_ft <= 40` | slot / middle-distance look |
| `outside` | `distance_ft > 40` | outside attempt |
| `unknown` | missing `x_coord` or `y_coord` | location not provided |

These names are intentionally not "high danger", "medium danger", or
"expected goals". Future waves can add a calibrated IceLines model, but this
wave only owns distance bands.

## Trend rows to add after this pulse

Player scoring trends should be ViewModel-owned rows built from
`PlayerScoringProfileView.events`:

- recent window labels: last 3 games, last 5 games, last 10 games, season loaded;
- attempts, unblocked attempts, shots on goal, goals;
- shot percentage (`goals / shots_on_goal`) when SOG > 0;
- inside-shot proxy counts and unknown-location counts;
- source-state fields: games/events loaded and whether play-by-play coverage is
  partial.

Do not call these projections or predictions. Preferred copy: "recent volume",
"conversion", "inside looks", and "location coverage".

## Later pulse split

The original wave split had streak leaderboards before shot-quality proxy
implementation. The inventory shows that goal/assist/point streaks are already
served by boxscore game lines, while shot streaks need play-by-play aggregation.
The better split is:

1. Pulse 02: implement pure core proxy structs/functions with known-value tests.
2. Pulse 03: add player scoring trend rows using the proxy.
3. Pulse 04: extend streak leaderboards with shot/attempt streaks from
   play-by-play aggregation.
4. Pulse 05: surface parity and closeout.

## Required tests before implementation ships

- **Proxy known values**:
  - `(89, 0)` -> 0 ft -> `crease`;
  - `(79, 0)` -> 10 ft -> `crease`;
  - `(69, 0)` -> 20 ft -> `inside`;
  - `(54, 0)` -> 35 ft -> `slot`;
  - `(0, 0)` -> 89 ft -> `outside`;
  - missing `x` or `y` -> `unknown`.
- **Negative x symmetry**: `(-69, 0)` and `(69, 0)` land in the same bucket.
- **Player matching**: goals match by `scoring_player_id` and shots by
  `shooting_player_id`; missing IDs do not match.
- **Streak inputs**: shot streaks must be derived from per-game event
  aggregation, with zero-attempt games breaking the streak.
- **Source-state**: loaded play-by-play with zero matching events is `Complete`;
  missing play-by-play is `Unavailable`/missing source state.

## Role review

- **scout**: the proxy language is hockey-readable but avoids lineup, odds, and
  xG claims.
- **edge**: missing coordinates and IDs stay explicit, and no event is assigned
  a quality bucket by default.
- **bench**: bucket thresholds are enumerated as hand-calculated fixtures.
- **wire**: all reads stay on existing `DataKind::PlayByPlay`/`Boxscore`
  manifests; cache warming remains mutation-backed.
