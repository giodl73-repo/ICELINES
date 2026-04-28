# IceLines Fantasy Scheme — Specification

**Version**: 0.1  
**Date**: 2026-04-25  
**Status**: Draft

A fantasy scheme is a named, user-defined scoring formula that assigns a weight to each
statistical category. IceLines uses schemes to compute fantasy points per player, enabling
fantasy-context rankings alongside (not instead of) pure hockey metrics.

---

## Problem

Different fantasy leagues use different scoring rules. Yahoo H2H, ESPN standard, custom
rotisserie leagues, keeper leagues — each has its own weights. A player who scores 0.65 PPG
ranks the same in every league by hockey metrics, but their fantasy value varies enormously
depending on whether hits are worth 0.5 or 1.0 points, whether blocks count, whether
shorthanded goals carry a bonus.

IceLines needs to be scheme-agnostic. The analytics engine is always PPG-based (hockey truth).
The scheme layer adds the fantasy context on top, selectable per-command with `--scheme`.

---

## Scheme File Format

Schemes are TOML files in `~/.icelines/schemes/`.

```toml
# ~/.icelines/schemes/yahoo-standard.toml

[scheme]
name        = "Yahoo Standard"
description = "Standard Yahoo Fantasy Hockey points league"
version     = "1.0"
source      = "Yahoo"              # Yahoo | ESPN | Custom | CBS

# ── Skater stats ────────────────────────────────────────────────────────────
# Each key maps to a stat column in the NHL API or Yahoo CSV.
# Weight is fantasy points per unit of that stat.
# 0.0 = stat is available but not scored. Omit = stat not used.

[scoring.skater]
goals              = 3.0    # G
assists            = 2.0    # A (primary + secondary combined)
pp_goals           = 1.0    # PPG bonus (on top of goal value)
pp_assists         = 0.5    # PPA bonus (on top of assist value)
sh_goals           = 2.0    # SHG bonus
sh_assists         = 1.0    # SHA bonus
gwg                = 0.5    # Game-winning goals
hits               = 0.5    # Hits
blocks             = 0.5    # Blocked shots
plus_minus         = 0.0    # +/- (0 = available, not scored)
shots_on_goal      = 0.0    # SOG
takeaways          = 0.0    # TK
faceoff_wins       = 0.0    # FOW

# ── Goalie stats ─────────────────────────────────────────────────────────────
[scoring.goalie]
wins               = 5.0    # W
losses             = -2.0   # L
saves              = 0.15   # SV
goals_against      = -1.0   # GA
shutouts           = 4.0    # SHO
```

### All Scoreable Stats

The full set of stats IceLines can score, across all available data sources:

| Key | Stat | Source | Notes |
|-----|------|--------|-------|
| `goals` | Goals | NHL API summary | |
| `assists` | Assists (P+S) | NHL API summary | |
| `pp_goals` | Power play goals | NHL API summary | Bonus on top of `goals` |
| `pp_assists` | Power play assists | NHL API summary | Bonus on top of `assists` |
| `sh_goals` | Shorthanded goals | NHL API summary | Bonus on top of `goals` |
| `sh_assists` | Shorthanded assists | NHL API summary | Bonus on top of `assists` |
| `gwg` | Game-winning goals | NHL API summary | |
| `ot_goals` | Overtime goals | NHL API summary | |
| `hits` | Hits | NHL API realtime | |
| `blocks` | Blocked shots | NHL API realtime | |
| `shots_on_goal` | Shots on goal | NHL API summary | |
| `plus_minus` | Plus/minus | NHL API summary | |
| `takeaways` | Takeaways | NHL API realtime | |
| `giveaways` | Giveaways (negative) | NHL API realtime | Typically negative weight |
| `faceoff_wins` | Face-off wins | NHL API faceoff | Centers only |
| `toi_per_game` | Avg TOI (minutes) | NHL API TOI | Unusual but some leagues use it |
| `wins` | Goalie wins | NHL API goalie | Goalie only |
| `losses` | Goalie losses | NHL API goalie | Goalie only |
| `saves` | Saves | NHL API goalie | Goalie only |
| `goals_against` | Goals against | NHL API goalie | Goalie only, negative weight |
| `shutouts` | Shutouts | NHL API goalie | Goalie only |
| `save_pct` | Save percentage | NHL API goalie | e.g. 0.915 → multiply by weight |

---

## CLI Commands

### `icelines scheme new`

Interactive wizard to define a scheme.

```
icelines scheme new [OPTIONS]

Options:
  --name <NAME>      Scheme name [prompted if not provided]
  --source <SOURCE>  Yahoo | ESPN | Custom | CBS
  --copy <NAME>      Start from an existing scheme
```

Walks through each stat with current default value, lets user enter a weight.
Saves to `~/.icelines/schemes/{slug}.toml`.

---

### `icelines scheme from-csv`

Detect available stats from a Yahoo CSV export and generate a scheme template.
The CSV column headers reveal which stats the league is tracking — the scheme template
sets all detected stats to `0.0` (not scored) and prompts for weights.

```
icelines scheme from-csv <PATH> [OPTIONS]

Arguments:
  <PATH>         Path to Yahoo Fantasy Hockey CSV export

Options:
  --name <NAME>  Scheme name [default: derived from filename]
  --fill         Prompt for weights interactively after detection
```

**Stat detection from Yahoo CSV column headers:**

| CSV Column | Detected as |
|------------|-------------|
| `G (P)` | `goals` |
| `A (P)` | `assists` |
| `PPG (P)` | `pp_goals` |
| `PPA (P)` | `pp_assists` |
| `PPP (P)` | *(derived = pp_goals + pp_assists, not separately scored)* |
| `SHG (P)` | `sh_goals` |
| `SHA (P)` | `sh_assists` |
| `GWG (P)` | `gwg` |
| `HIT (P)` | `hits` |
| `BLK (P)` | `blocks` |
| `W (G)` | `wins` |
| `L (G)` | `losses` |
| `GA (G)` | `goals_against` |
| `SV (G)` | `saves` |
| `SHO (G)` | `shutouts` |

**Example output:**

```
Detected 15 scoreable stats from yahoo-465.l.1214-Players.csv

Generated template: ~/.icelines/schemes/yahoo-465.toml
Run `icelines scheme edit yahoo-465` to set weights, or
    `icelines scheme from-csv <PATH> --fill` to set weights now.
```

---

### `icelines scheme list`

```
icelines scheme list

Schemes:
  yahoo-standard    Yahoo Standard            (11 stats scored)
  yahoo-465         Yahoo 465.l.1214 League   (15 stats scored)
  espn-default      ESPN Default              (8 stats scored)
  custom-keeper     My Keeper League          (13 stats scored)
```

---

### `icelines scheme show <NAME>`

```
icelines scheme show yahoo-standard

SCHEME: Yahoo Standard
Source: Yahoo  |  Version: 1.0

Skater scoring:
  goals          3.0    pp_goals      1.0    sh_goals      2.0
  assists        2.0    pp_assists    0.5    sh_assists    1.0
  gwg            0.5    hits          0.5    blocks        0.5

Goalie scoring:
  wins           5.0    saves         0.15   shutouts      4.0
  losses        -2.0    goals_against -1.0

Unscored (available): plus_minus, shots_on_goal, takeaways, faceoff_wins
```

---

### `icelines scheme edit <NAME>`

Open the scheme TOML in `$EDITOR`. Validates on save.

---

## Integration with Other Commands

Any command that ranks or compares players accepts `--scheme`:

```bash
# Rank by fantasy points using yahoo-standard scheme
icelines rank --scheme yahoo-standard

# Team lineup with fantasy point totals per player
icelines team SEA --scheme yahoo-standard

# Filter players by minimum fantasy points pace
icelines players --scheme yahoo-standard --min-fpts 200

# Compare two players by fantasy points
icelines compare "Beniers" "Cooley" --scheme yahoo-standard

# Project rest-of-season fantasy points
icelines project "Jared McCann" --scheme yahoo-standard --mode regressed

# Draft class fantasy point ranking
icelines class 2022 --scheme yahoo-standard
```

When `--scheme` is provided, IceLines computes `fantasy_pts_per_game` alongside
`ppg_pace`. The ranking metric switches to fantasy points; the fit classification
thresholds are recalculated relative to the scheme's point distribution.

---

## Data Model

```rust
pub struct Scheme {
    pub name:        String,
    pub description: String,
    pub version:     String,
    pub source:      SchemeSource,
    pub skater:      SkaterWeights,
    pub goalie:      GoalieWeights,
}

pub struct SkaterWeights {
    pub goals:         f32,
    pub assists:       f32,
    pub pp_goals:      f32,
    pub pp_assists:    f32,
    pub sh_goals:      f32,
    pub sh_assists:    f32,
    pub gwg:           f32,
    pub ot_goals:      f32,
    pub hits:          f32,
    pub blocks:        f32,
    pub shots_on_goal: f32,
    pub plus_minus:    f32,
    pub takeaways:     f32,
    pub giveaways:     f32,
    pub faceoff_wins:  f32,
    pub toi_per_game:  f32,
}

pub struct GoalieWeights {
    pub wins:          f32,
    pub losses:        f32,
    pub saves:         f32,
    pub goals_against: f32,
    pub shutouts:      f32,
    pub save_pct:      f32,
}

pub enum SchemeSource {
    Yahoo,
    Espn,
    Cbs,
    Custom,
}

pub struct FantasyScore {
    pub total:     f32,
    pub per_game:  f32,
    pub breakdown: HashMap<String, f32>,  // stat → points contribution
}
```

---

## Scoring Engine

```rust
pub fn compute_fantasy_score(
    stats: &SkaterStats,
    weights: &SkaterWeights,
    gp: u32,
) -> Option<FantasyScore> {
    if gp < MIN_GP { return None; }

    let raw = weights.goals         * stats.goals         as f32
            + weights.assists       * stats.assists        as f32
            + weights.pp_goals      * stats.pp_goals       as f32
            + weights.pp_assists    * stats.pp_assists     as f32
            + weights.sh_goals      * stats.sh_goals       as f32
            + weights.sh_assists    * stats.sh_assists     as f32
            + weights.gwg           * stats.gwg            as f32
            + weights.hits          * stats.hits           as f32
            + weights.blocks        * stats.blocks         as f32
            + weights.plus_minus    * stats.plus_minus     as f32
            // ... all other stats
            ;

    Some(FantasyScore {
        total:    raw,
        per_game: raw / gp as f32,
        breakdown: build_breakdown(stats, weights),
    })
}
```

The `breakdown` field enables a per-stat contribution display:

```
FANTASY SCORE: Matty Beniers  (yahoo-standard · 82 GP)

Total: 179.0 pts  |  2.18 pts/gp

Breakdown:
  Goals (20 × 3.0)         60.0   33%
  Assists (30 × 2.0)        60.0   33%
  PP Goals (6 × 1.0)         6.0    3%
  PP Assists (5 × 0.5)        2.5    1%
  GWG (1 × 0.5)               0.5   <1%
  Hits (31 × 0.5)            15.5    9%
  Blocks (69 × 0.5)          34.5   19%
```

---

## Built-in Schemes

IceLines ships with three built-in read-only schemes:

| Scheme | Source | Key weights |
|--------|--------|-------------|
| `yahoo-standard` | Yahoo | G=3, A=2, +PPG=1, +PPA=0.5, HIT=0.5, BLK=0.5 |
| `espn-standard` | ESPN | G=6, A=4, +/-=2, PPP=2, SOG=1, HIT=1 |
| `simple-pts` | Custom | G=1, A=1 (pure hockey points, no bonuses) |

Users can `--copy` any built-in to create a customized version.

---

## Scheme Validation Rules (resolved from TAPE + EDGE blockers)

All scheme TOML files are validated on load and on save via `icelines scheme edit`. A scheme
that fails validation is rejected with an error message; no partial loading.

**Validation rules:**
1. `weight` values must be finite `f64` — `NaN` and `±Inf` are rejected
2. Zero weights are allowed (stat counts but has no score impact)
3. Negative weights are allowed (e.g., `losses = -2.0` for goalie losses)
4. Unknown stat keys are **rejected** with: `Unknown stat key 'custom_x'. Add to schema first.`
   (This prevents silent typos: `G (P)` vs `G(P)`)
5. `scheme.name` must not be empty and must not contain `/`, `\`, `:`, or `..`
6. `scheme.version` must be a valid semver string (e.g., `"1.0"`, `"2.3.1"`)

**CSV auto-detection (`icelines scheme from-csv`):**
- Columns not in the known stat mapping are logged as warnings, not errors
- A CSV with only 1 scoreable stat column is valid (produces a single-weight scheme)
- Column name `G (Pls)` (typo for `G (P)`) is flagged as: `Unrecognized column 'G (Pls)' — did you mean 'G (P)'?`

---

## Scheme Name Collision Rules (resolved from EDGE blocker)

User-created schemes are stored in `~/.icelines/schemes/` with the filename matching the
scheme name (slugified). Built-in schemes are read from the binary (not the filesystem).

**Collision policy:**
- User schemes that share a name with a built-in are stored under the `user/` namespace
  internally: `user/yahoo-standard` vs built-in `yahoo-standard`
- The CLI resolves `--scheme yahoo-standard` to the **user copy** if one exists, with warning:
  `Using custom 'yahoo-standard' — built-in version also exists. Use --scheme builtin/yahoo-standard to use the built-in.`
- User scheme filenames are slug-safe (lowercase, hyphens only, no `/`). The `user/` prefix
  is internal only; users reference schemes by slug, not path.

---

## Invariants

- **DI-20**: A `FantasyScore` is always computed from the same `Scheme` version — scheme version is stored alongside the score in cache
- **DI-21**: Built-in schemes are read-only — editing them creates a user copy in `~/.icelines/schemes/`
- **DI-22**: `fantasy_pts_per_game` is always `None` when `gp < MIN_GP`, never 0.0
- **DI-23**: The `breakdown` map sums to within 0.001 of `total` (floating point tolerance)
- **DI-24**: Fantasy mode uses the current scheme (v1.0) for all seasons including historical. No scheme versioning per season. (See `season-timetravel.md` §Fantasy Scoring Mode)
