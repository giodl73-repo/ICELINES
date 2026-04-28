# IceLines Data Sources — Full Specification

**Version**: 0.1  
**Date**: 2026-04-25  
**Status**: Draft

The goal of IceLines is the best possible lines analysis — not just fantasy points or PPG pace,
but the full picture of how a player performs, how they are deployed, and what the game actually
looked like around them. This spec defines every data source we compile, what it contributes,
and how it feeds into the depth chart engine.

---

## Data Tier Overview

| Tier | Source | Granularity | Latency | What It Answers |
|------|--------|-------------|---------|-----------------|
| 0 — Rosters | NHL API `/v1/roster` | Per-team | 48h TTL | Player universe, position, headshot URL |
| 1 — Fantasy | Yahoo CSV export | Season totals | Manual export | Fantasy-context positions, ownership |
| 2a — Core Stats | NHL API `/stats/rest` | Season totals + bios | Daily | GP, G, A, TOI, PPG, shots |
| 2b — Schedule | NHL API `/v1/schedule/{date}` | Per-date | 6h TTL (future), permanent (past) | Game schedule, results |
| 2c — Live Scores | NHL API `/v1/score/now` | Per-game live | 30s while active | In-progress scores, period, time |
| 2d — Boxscore | NHL API `/v1/gamecenter/{id}/boxscore` | Per-game | Permanent once final | Goals, goalies, key stats |
| 2e — Playoff Bracket | NHL API `/v1/playoff-bracket/{year}` | Per-series | Daily during playoffs | Bracket, series state, advancement |
| 3 — Shifts | NHL API `/shiftcharts` | Per-shift, per-game | Daily | Real line deployment, line partners, zone starts |
| 4 — Advanced | NHL Edge / Natural Stat Trick | Per-situation | Daily | Corsi, xG, HDCA, zone entries |
| 5 — Social | Reddit NHL / Twitter | Per-day | Real-time | Fan sentiment, injury rumors, line news |
| 6 — Beat Media | RSS / web scrape | Per-article | Real-time | Official line rushes, coach quotes, practice lines |

Tiers 0–3 are **primary** — they drive all rankings, depth charts, and live screens.  
Tiers 4–6 are **contextual** — they annotate, not override.

---

## Tier 2c — Live Scores (Detail)

**Endpoints used by Scores screen:**

| Endpoint | Purpose | Cache TTL | Cache Key |
|----------|---------|-----------|-----------|
| `GET /v1/score/now` | Today's live scores | 30s (active), stale-while-revalidating | `scores_today` |
| `GET /v1/schedule/{YYYY-MM-DD}` | Any date's schedule/results | Permanent for past dates; 6h for future | `schedule_{date}` |
| `GET /v1/gamecenter/{gameId}/boxscore` | Goal log, goalie stats | Permanent once game is `FINAL` | `boxscore_{gameId}` |
| `GET /v1/playoff-bracket/{year}` | Bracket + series state | 1h during playoffs; permanent after season | `bracket_{year}` |

**Schema validation**: All API responses are validated before parsing. Required fields:
- `game_id: u64` — reject if 0 or missing
- `game_type: u8` — must be 1 (pre), 2 (regular), or 3 (playoff); unknown values treated as 2
- `startTimeUTC: String` — ISO 8601; parse failure → show raw string, do not crash
- `seriesSummary` — optional; if missing for a game_type=3 game, show score without series context

**Graceful degradation**:
- Network timeout (>10s) → return stale cached data with `[stale Xm ago]` label
- Malformed JSON → skip the affected game, show others with `[1 game unavailable]` warning
- Empty response → show "No games found for this date" — not an error

**Retry policy**: Exponential backoff starting at 1s, max 3 retries, max delay 8s.

---

## Tier 0 — NHL Team Rosters (player universe)

**Endpoint**: `https://api-web.nhle.com/v1/roster/{TEAM}/{SEASON}`  
**Fetched by**: `icelines fetch rosters` (32 requests, one per team)  
**Cache TTL**: 48 hours  
**Auth**: None

This is the authoritative player universe. All 32 rosters are fetched at the start of
every season and after roster moves. There is no CSV dependency for the player list.

**What we get:**
- `id` — NHL canonical player ID (`u32`)
- `firstName`, `lastName` — player name (Unicode, used for display)
- `positionCode` — `L` / `R` / `C` / `D` / `G`
- `headshot` — real team-specific photo URL:
  `https://assets.nhle.com/mugs/nhl/{SEASON}/{TEAM}/{player_id}.png`
- `birthDate`, `birthCountry`, `shootsCatches`, `sweaterNumber`

**Pipeline:** `icelines fetch rosters` → `data/rosters.json` → `icelines-core::PlayerBio`

---

## Tier 1 — Yahoo Fantasy CSV (optional)

**Status**: **Optional** — IceLines works without it.  
**Path**: `data/fantasy.csv` (user-supplied, manual export)

The Yahoo CSV is useful for one thing: fantasy position eligibility. Yahoo designates
some players as eligible at multiple positions (e.g., Draisaitl as C and LW) based on
their usage. This multi-position eligibility is fantasy-specific and not in the NHL API.

**What we use (and only this):**
- `Eligible Positions` — multi-position fantasy eligibility (e.g., `C,LW,Util`)
- `First Name`, `Last Name` — for matching to NHL player ID if rosters are stale

**What we do NOT use:**
- Any stat columns (`G (P)`, `A (P)`, etc.) — all stats come from Tier 2 NHL API
- `Image` — all photos come from Tier 0 NHL roster API
- `Team` — authoritative source is NHL API `currentTeamAbbrev`

**Limitations:**
- Manual export — not automated
- Export frequency is user-controlled
- When not provided, position eligibility falls back to NHL API `positionCode`

**Pipeline:** `data/fantasy.csv` (optional) → `icelines-core::YahooEligibility`

---

## Tier 2 — NHL API Core Stats

**Endpoint**: `https://api.nhle.com/stats/rest/en/`  
**Cache TTL**: 24 hours  
**Auth**: None (public)

### 2a — Skater Bios (GP + demographics)

```
GET /skater/bios?cayenneExp=seasonId={SEASON}&gameTypeId=2&limit=100&start={N}
```

**Fields used:**
- `gamesPlayed` — denominator for all pace calculations
- `playerId` — NHL canonical player ID (used for photo URL and future joins)
- `skaterFullName` — for name matching (with Unicode normalization)
- `currentTeamAbbrev` — team at time of fetch (may differ from Yahoo CSV after trades)
- `positionCode` — L/R/C/D for validation against Yahoo position eligibility

**Fields stored:** All of the above plus `birthDate`, `nationalityCode`, `shootsCatches`

### 2b — Skater Summary Stats

```
GET /skater/summary?cayenneExp=seasonId={SEASON}&gameTypeId=2&limit=100&start={N}
```

**Fields used:**
- `goals`, `assists`, `points` — cross-validation against Yahoo CSV
- `pointsPerGame` — NHL-computed PPG (sanity check against our projection)
- `timeOnIcePerGame` — TOI in seconds (line role validation)
- `ppGoals`, `ppPoints` — power play production
- `shGoals`, `shPoints` — shorthanded production
- `shots`, `shootingPctg` — shooting efficiency

### 2c — Skater Time On Ice

```
GET /skater/timeonice?cayenneExp=seasonId={SEASON}&gameTypeId=2
```

**Fields used:**
- `evTimeOnIce` — even strength TOI (true line role signal)
- `ppTimeOnIce` — power play TOI
- `shTimeOnIce` — penalty kill TOI

**Key insight:** `evTimeOnIce / gamesPlayed` is a stronger line deployment signal than points
alone. A player with 18:00 ES-TOI is on the first two lines regardless of point totals.

---

## Tier 3 — Shift Data (NHL API)

**Endpoint**: `https://api.nhle.com/stats/rest/en/shiftcharts`  
**Cache TTL**: 7 days (historical), 6 hours (current season)  
**Auth**: None

```
GET /shiftcharts?cayenneExp=gameId={GAME_ID}
```

Shift data is the ground truth of how coaches actually deploy players. It tells us:
- Which players were on the ice at the same time (line partners)
- Which zone they started in (offensive vs defensive zone starts)
- How long each shift was
- How many shifts per game (deployment frequency)

### What We Compute From Shifts

**Line partners:** For each forward, find the two other forwards most frequently on the ice
simultaneously. This gives the actual deployed line, not the presumed one. A player listed as
LW may be deployed as C when another player is injured.

**Zone start percentage:** `offensive_zone_starts / (offensive_zone_starts + defensive_zone_starts)`.
High ZS% (>55%) indicates offensive deployment. Low ZS% (<45%) indicates shutdown role.
This contextualizes PPG — a player with 0.70 PPG and 42% ZS% is more valuable than one with
0.70 PPG and 62% ZS%.

**Shift length distribution:** Average shift length (seconds) by line. First-line players
average 40–50 second shifts at high frequency. Fourth-line players average 30–35 second shifts
less frequently.

**Average shift rank:** For each player, their average shift sequence number in each period
(shifts 1–4 in a period = first rotation, 5–8 = second rotation). This directly identifies
line groupings without relying on coaching press releases.

### Schema

```rust
pub struct Shift {
    pub player_id: u32,
    pub game_id:   u64,
    pub period:    u8,
    pub shift_num: u16,
    pub start_sec: u32,   // seconds from period start
    pub end_sec:   u32,
    pub duration:  u16,   // seconds
    pub team_abbr: String,
}
```

### Derived Fields Per Player (Season)

```rust
pub struct ShiftProfile {
    pub player_id:          u32,
    pub games_with_shifts:  u32,
    pub avg_shift_sec:      f32,
    pub shifts_per_game:    f32,
    pub zone_start_pct:     Option<f32>,  // None if data unavailable
    pub top_line_partners:  Vec<(u32, f32)>,  // (player_id, co-ice_fraction)
    pub avg_ev_toi_seconds_per_game: u32,  // stored as INTEGER SECONDS, not minutes.
                                            // Display layer: divide by 60 for "MM:SS".
                                            // Threshold comparisons: 16:00 = 960, 10:00 = 600.
}
```

### Shift-Adjusted Line Score

The shift profile feeds into a **shift-adjusted line score** that can override pure PPG ranking
when deployment context clearly places a player higher or lower:

- If `avg_ev_toi_per_game` > 16:00 for a forward → Line 1–2 floor regardless of PPG
- If `avg_ev_toi_per_game` < 10:00 → Line 3–4 cap regardless of PPG
- `zone_start_pct` < 40% with PPG > 0.70 → flag as "shutdown star" (different value profile)
- Strong line partner signal (>60% co-ice with an identified top liner) → line assignment inherited

---

## Tier 4 — Advanced Stats (Natural Stat Trick)

**Source**: `https://www.naturalstattrick.com/`  
**Method**: HTML scrape (no official API)  
**Cache TTL**: 24 hours  
**Status**: Planned (Phase 2)

**Fields we want:**
- `CF%` — Corsi For percentage (shot attempt possession)
- `xGF%` — Expected goals for percentage
- `HDCF%` — High-danger chance percentage
- `xGA/60` — Expected goals against per 60 (defensive metric)
- `RelCF%` — Corsi relative to team (isolates individual contribution)

**Why it matters:** A player with 0.60 PPG but 56% xGF% is driving play. A player with
0.60 PPG but 44% xGF% is riding linemates. PPG alone cannot distinguish these.

**Integration:** Advanced stats annotate the depth chart card — they do not change rankings.
A player marked ↑ BURIED with high xGF% gets an additional "possession driver" tag.

---

## Tier 5 — Social Signal (Reddit NHL)

**Source**: Reddit `/r/hockey`, `/r/nhl`, individual team subreddits  
**Method**: Reddit API (public read access)  
**Cache TTL**: 4 hours  
**Status**: Planned (Phase 3)

**What we extract:**
- **Injury reports:** Posts mentioning `[player name] + (day-to-day|IR|injured|out)` with high upvote counts
- **Line rush reports:** Posts from beat reporters (verified flair) mentioning line combinations
- **Trade rumors:** Posts mentioning `[player name] + (trade|deal|moved)` from credible sources

**Schema:**
```rust
pub struct SocialSignal {
    pub player_id:    u32,
    pub signal_type:  SignalType,  // Injury | LineNews | TradeRumor
    pub source:       String,
    pub confidence:   f32,         // 0.0–1.0 based on upvotes + source credibility
    pub fetched_at:   DateTime<Utc>,
    pub text_excerpt: String,
}
```

**How it surfaces:** Social signals appear as annotations on team pages — a small icon and
tooltip on a player cell. They do NOT change rankings automatically. The analyst sees the signal
and decides.

---

## Tier 6 — Beat Media Line Rushes

**Sources**: Team-specific beat reporters (Daily Faceoff, The Athletic, team sites)  
**Method**: RSS feed scraping  
**Cache TTL**: 2 hours  
**Status**: Planned (Phase 3)

Daily Faceoff publishes line combinations for every team before each game, sourced directly
from morning skate observations. This is the authoritative "what is coach actually doing today"
signal.

**Fields:**
- Reported line combinations (LW-C-RW groupings)
- Date/time (valid for the next game only)
- Source reporter

**Integration:** If a practice line combination differs from our computed depth chart by 2+
lines for a player, flag the discrepancy. This is the "coach override" signal.

---

## Data Pipeline Architecture

```
Yahoo CSV          NHL API Bios    NHL API Stats    Shift Charts
    │                   │               │                │
    ▼                   ▼               ▼                ▼
YahooRecord     ────► PlayerBio    SkaterStats      ShiftProfile
    │                   │               │                │
    └───────────────────┴───────────────┴────────────────┘
                                │
                         PlayerRecord           ◄─── Social / Beat (annotations)
                                │
                        ┌───────┴────────┐
                        │                │
                  PaceScore          ShiftScore
                        │                │
                        └───────┬────────┘
                          CompositeScore
                                │
                    ┌───────────┴───────────┐
                    │                       │
              DepthChart              FitClass
                    │                       │
                    └───────────┬───────────┘
                          LineCard (HTML)
                                │
                         GitHub Pages
```

---

## Composite Scoring (Phase 1 → Phase 2 evolution)

### Phase 1 (current): Pure PPG Pace
```
score = (G + A) / GP * 82  +  (G / GP * 82) * 0.001
```

### Phase 2: PPG + TOI Weighted
```
toi_factor = clamp(evTOI_per_game / 18.0, 0.6, 1.2)
score      = ppg_82 * toi_factor  +  gpg_82 * 0.001
```
A player with 22:00 ES-TOI gets a 1.22× multiplier. A player with 10:00 ES-TOI gets 0.67×.
This corrects for defensive forwards who produce in limited minutes and offensive passengers
with inflated PPG from elite linemates.

### Phase 3: Full Composite
```
score = 0.60 * ppg_82_toi_adjusted
      + 0.20 * xGF_per60_normalized
      + 0.10 * zone_start_adjusted_factor
      + 0.10 * shift_rank_signal
```
Weights are provisional — calibrated against scout review of "obvious" line rankings
(McDavid is L1, every team's clear 4th liner is L4). PACE and SCOUT must agree before
any weight change ships.

---

## Data Storage Layout

```
~/.icelines/
├── cache/
│   ├── gp/
│   │   └── 20252026/
│   │       └── {player_id}.json       # bios API response
│   ├── stats/
│   │   └── 20252026/
│   │       └── {player_id}.json       # summary stats response
│   ├── shifts/
│   │   └── 20252026/
│   │       └── {game_id}.json         # full shiftchart response
│   └── social/
│       └── {YYYY-MM-DD}/
│           └── reddit.json            # signal dump
└── db/
    └── icelines.db                    # SQLite: derived PlayerRecord, ShiftProfile
```

All cache files include a `fetched_at` timestamp. `icelines fetch --refresh` clears and
re-fetches. `icelines fetch --stale` shows what is older than its TTL.

---

## Invariants

- **DI-10**: A PlayerRecord with GP < MIN_GP has no pace score — it is `None`, not 0.0
- **DI-11**: ShiftProfile is only created for players with ≥ 5 games of shift data
- **DI-12**: Social signals never modify `PaceScore` or `FitClass` — annotations only
- **DI-13**: A CompositeScore is reproducible: same inputs produce same output (no randomness)
- **DI-14**: Cache miss for any player is logged as a warning; missing player data is `None`, never silently 0

---

## Non-Goals

- **Real-time data**: IceLines is not a live dashboard. The minimum meaningful cadence is daily.
- **Game prediction**: We analyze deployment and production; we do not predict future performance.
- **Contract/salary data**: Out of scope — this is hockey analytics, not cap management.
- **Playoff-specific analysis**: Regular season only for now; playoff context is a different beast.
- **Player comparison across eras**: All rankings are within the current season only.
