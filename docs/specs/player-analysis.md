# IceLines Player Analysis — CLI Specification

**Version**: 0.1  
**Date**: 2026-04-25  
**Status**: Draft  
**Depends on**: `docs/specs/rust-cli.md`, `docs/specs/data-sources.md`

The IceLines CLI is not just a depth chart generator. It is the primary interface for player
analysis, draft class comparison, peer group tracking, linemate discovery, and historical
context. Everything the original NHL Analysis Platform did as a web app, IceLines does as
composable CLI commands with structured output.

---

## Design Principle: Everything Composable

Every command produces structured output (table, JSON, or CSV with `--json`/`--csv` flags).
This means commands can pipe into each other and into other tools:

```bash
icelines players --pos C --age-max 23 --json | jq '.[] | .name'
icelines class 2022 --pos C | icelines rank --stdin
icelines peers McDavid --json > peer_group.json
```

---

## Command Reference

---

### `icelines players` — List & Filter Players

```
icelines players [OPTIONS]

Filter Options:
  --team <ABBREV>         Filter by NHL team (e.g. SEA, NYR)
  --pos <POS>             Position: C, LW, RW, D, G
  --age-min <N>           Minimum age (as of season start)
  --age-max <N>           Maximum age
  --years-min <N>         Minimum years in NHL (seasons played)
  --years-max <N>         Maximum years in NHL
  --nationality <CODE>    ISO country code (CAN, USA, SWE, FIN, RUS, CZE, SVK...)
  --region <REGION>       Broader region: North America, Europe, Scandinavia
  --draft-year <YEAR>     Draft year (e.g. 2021)
  --draft-round <N>       Draft round (1, 2, 3...)
  --draft-pick-max <N>    Drafted at or before this overall pick
  --undrafted             Only undrafted players
  --rookie                Only players in their first NHL season
  --handedness <L|R>      Shooting/catching hand
  --ppg-min <F>           Minimum PPG pace (e.g. 0.60)
  --ppg-max <F>           Maximum PPG pace
  --gp-min <N>            Minimum games played
  --toi-min <F>           Minimum avg ES-TOI per game (minutes)

Output Options:
  --sort <FIELD>          Sort by: ppg (default), goals, assists, age, draft
  --limit <N>             Max rows [default: 25]
  --json                  JSON output
  --csv                   CSV output
```

**Examples:**

```bash
# All U23 North American centers, sorted by PPG pace
icelines players --pos C --age-max 23 --region "North America" --sort ppg

# All first-round picks from 2020-2022 who are playing this season
icelines players --draft-year 2020 --draft-round 1 --gp-min 20
icelines players --draft-year 2021 --draft-round 1 --gp-min 20
icelines players --draft-year 2022 --draft-round 1 --gp-min 20

# Swedish defensemen with >16:00 ES-TOI per game
icelines players --pos D --nationality SWE --toi-min 16

# Undrafted players producing at a 0.50+ PPG pace
icelines players --undrafted --ppg-min 0.50
```

**Terminal output:**

```
PLAYERS — C · Age ≤23 · North America  (23 players)

 Rank  Player               Team  Age  Nat  GP    PPG    G/82  A/82  Draft
────────────────────────────────────────────────────────────────────────────
    1  Macklin Celebrini    SJ    19   CAN  74   1.24   49.5  52.2  24 R1#1
    2  Matty Beniers        SEA   22   USA  82   0.61   20.0  30.0  22 R1#2
    3  Logan Cooley         UTA   21   USA  68   0.58   18.2  29.5  22 R1#3
    4  Dylan Guenther       UTA   21   CAN  79   0.65   26.1  27.3  22 R1#9
```

---

### `icelines class` — Draft Class Analysis

Group all players from a given draft year, show how the class is performing as a cohort.

```
icelines class <YEAR> [OPTIONS]

Arguments:
  <YEAR>              Draft year (2018–current)

Options:
  --pos <POS>         Filter by position
  --round <N>         Filter by draft round
  --compare <YEAR>    Side-by-side with another draft class
  --years-in <N>      Show class at N years in (e.g. --years-in 3)
  --sort <FIELD>      ppg (default), goals, draft-pick
  --json
```

**Examples:**

```bash
# How is the 2022 draft class performing in their 3rd year?
icelines class 2022 --years-in 3 --pos C

# Compare 2021 and 2022 first-round center classes
icelines class 2021 --pos C --round 1 --compare 2022
```

**Terminal output:**

```
DRAFT CLASS 2022 — Centers · Year 3 in NHL

 Pick  Player               Team  GP    PPG    G/82  Note
────────────────────────────────────────────────────────────
   #1  Juraj Slafkovský     MTL   71   0.89   23.1  ★ elite
   #2  Simon Nemec          NJ    66   0.51   10.1
   #3  Logan Cooley         UTA   68   0.58   18.2
   #6  Matty Beniers        SEA   82   0.61   20.0
   #9  Dylan Guenther       UTA   79   0.65   26.1  ★ elite
  ...
CLASS MEDIAN PPG: 0.45  |  Class Hits (>0.70 PPG): 4/32
```

---

### `icelines peers` — Peer Group Comparison

Find a player's peer group — players with similar age, position, and draft era — and show
how they compare. This answers: "Is this player performing as expected for who they are?"

```
icelines peers <PLAYER> [OPTIONS]

Arguments:
  <PLAYER>              Player name (partial match OK)

Options:
  --by draft-class      Peers = same draft year ± 1 (default)
  --by age              Peers = ±1 year age window at same position
  --by pick-range       Peers = within 15 picks in same draft
  --size <N>            Peer group size [default: 10]
  --json
```

**Examples:**

```bash
# Where does Beniers rank among his draft class peers?
icelines peers "Matty Beniers"

# Where does Eberle rank among his age-matched RW peers?
icelines peers "Jordan Eberle" --by age
```

**Terminal output:**

```
PEERS OF MATTY BENIERS (SEA · C · Age 22 · 2022 R1#2)
Peer group: 2022 first-round centers ± 5 picks

 Rank  Player               Team  Age  GP    PPG    vs. Beniers
────────────────────────────────────────────────────────────────
    1  Logan Cooley         UTA   21   68   0.58   +0.03 ahead
  → 2  Matty Beniers        SEA   22   82   0.61   —
    3  Dylan Guenther       UTA   21   79   0.65   -0.04 behind
    4  Matvei Michkov       PHI   20   71   0.72   -0.11 behind
────────────────────────────────────────────────────────────────
Beniers rank: 2 of 4 peers  |  PPG percentile in group: 75th
```

---

### `icelines history` — Multi-Season Player History

Show a player's production across every NHL season they have played.

```
icelines history <PLAYER> [OPTIONS]

Options:
  --seasons <N>       How many seasons back [default: 5]
  --stat <FIELD>      Highlight stat: ppg (default), goals, assists, toi
  --pace              Normalize everything to 82-game pace
  --json
```

**Examples:**

```bash
icelines history "Jordan Eberle" --pace
icelines history "Connor McDavid" --seasons 10 --stat ppg
```

**Terminal output:**

```
JORDAN EBERLE — Career History (82-game pace)

 Season  Team  GP   G/82  A/82  PPG   TOI/G   Age
───────────────────────────────────────────────────
 25–26   SEA   80   27.1  56.6  0.69  16:42   35
 24–25   SEA   74   22.8  42.2  0.55  15:58   34
 23–24   SEA   69   28.1  51.2  0.63  17:02   33
 22–23   SEA   77   26.0  47.8  0.60  16:45   32
 21–22   SEA   67   30.1  54.1  0.67  17:10   31
───────────────────────────────────────────────────
Career pace avg: 0.63 PPG  |  Peak season: 25-26 (0.69)
```

---

### `icelines mates` — Linemate Analysis (Shift-Based)

Using shift data, identify who each player actually played with most frequently and how the
line combinations performed.

```
icelines mates <PLAYER> [OPTIONS]

Options:
  --top <N>           Top N line partners [default: 5]
  --min-shifts <N>    Minimum shared shifts to include [default: 50]
  --season <YEAR>     Season [default: current]
  --json
```

**Examples:**

```bash
# Who does Beniers actually play with most?
icelines mates "Matty Beniers"

# Who has Tolvanen been linemates with across the season?
icelines mates "Eeli Tolvanen" --top 8
```

**Terminal output:**

```
LINEMATES OF MATTY BENIERS (SEA · C)
Based on 1,247 tracked shifts this season

 Partner              Pos  Shared Shifts  ES-TOI Together  GF%  xGF%
──────────────────────────────────────────────────────────────────────
 Jordan Eberle        RW        412          148:22         55%   54%
 Eeli Tolvanen        LW        387          140:08         52%   53%
 Bobby McMann         LW        198           71:30         48%   51%
 Kaapo Kakko          RW        156           56:12         50%   49%
──────────────────────────────────────────────────────────────────────
Primary line: Tolvanen — Beniers — Eberle (387/412 shifts together)
```

---

### `icelines group` — Manage Peer Groups / Projects

Custom collections of players — draft classes, watchlists, trade targets, prospect pools.
Groups persist to `~/.icelines/groups.json`.

```
icelines group <SUBCOMMAND>

Subcommands:
  create <NAME> [--desc TEXT]         Create an empty group
  add <GROUP> <PLAYER>                Add a player to a group
  remove <GROUP> <PLAYER>             Remove a player
  list                                List all groups
  show <GROUP>                        Show group members with current stats
  delete <GROUP>                      Delete a group
  export <GROUP>                      Export group to CSV/JSON
  compare <GROUP1> <GROUP2>           Side-by-side group comparison
  auto --draft-year <Y> --pos <P>     Auto-populate from draft class + position
```

**Examples:**

```bash
# Create a "2022 C prospects" watchlist
icelines group create "2022-C-Class" --desc "2022 draft class centers"
icelines group auto "2022-C-Class" --draft-year 2022 --pos C

# Create a trade targets group
icelines group create "Trade Targets" --desc "SEA trade candidates"
icelines group add "Trade Targets" "Brady Tkachuk"
icelines group add "Trade Targets" "Devon Toews"
icelines group show "Trade Targets"

# Compare two groups
icelines group compare "2021-C-Class" "2022-C-Class"
```

**`show` output:**

```
GROUP: 2022-C-Class  (7 members)
"2022 draft class centers"

 Rank  Player               Team  Age  GP    PPG    G/82  Draft Pos
──────────────────────────────────────────────────────────────────────
    1  Matvei Michkov       PHI   20   71   0.72   26.0  R1#7
    2  Dylan Guenther       UTA   21   79   0.65   26.1  R1#9
    3  Matty Beniers        SEA   22   82   0.61   20.0  R1#2
    4  Logan Cooley         UTA   21   68   0.58   18.2  R1#3
    5  Juraj Slafkovský     MTL   22   71   0.89   23.1  R1#1
──────────────────────────────────────────────────────────────────────
Group median PPG: 0.65  |  Group GP median: 74
```

---

### `icelines compare` — Head-to-Head Player Comparison

```
icelines compare <PLAYER1> <PLAYER2> [OPTIONS]

Options:
  --pace              Normalize to 82-game pace (default)
  --raw               Show raw season totals
  --history <N>       Include last N seasons
  --json
```

**Example:**

```bash
icelines compare "Matty Beniers" "Logan Cooley"
```

**Output:**

```
HEAD-TO-HEAD COMPARISON  (82-game pace)

                         Matty Beniers     Logan Cooley
                         SEA · C · 22      UTA · C · 21
────────────────────────────────────────────────────────
 Draft                   2022 R1 #2        2022 R1 #3
 GP this season          82                68
 PPG                     0.61              0.58
 G/82                    20.0              18.2
 A/82                    30.0              29.5
 ES TOI/G                16:42             15:58
 Zone Start %            51%               56%
 Avg line (32 teams)     3.23              3.45
 Fit on own team         ~ solid           ~ solid
────────────────────────────────────────────────────────
Edge: BENIERS by 0.03 PPG; COOLEY by 5% Zone Start
```

---

### `icelines scouting` — Full Player Scouting Report

Combines bio, career history, pace stats, peer ranking, linemates, and fit classification
into a single formatted report.

```
icelines scouting <PLAYER> [OPTIONS]

Options:
  --format terminal (default) | markdown | json
  --out <FILE>        Write to file instead of stdout
```

**Output sections:**
1. Bio (age, nationality, draft, handedness)
2. Current season pace stats
3. Career trajectory (3-year pace trend)
4. Peer group rank (draft class percentile)
5. Linemate analysis (primary line partners)
6. Depth chart position on own team
7. Cross-team value (avg line on other 31 teams)
8. Fit classification and interpretation

---

## Data Model Extensions

These new commands require extending `icelines-core` with:

```rust
pub struct PlayerBio {
    pub player_id:     u32,
    pub full_name:     String,
    pub birth_date:    NaiveDate,
    pub nationality:   String,           // ISO-3166 alpha-3
    pub region:        Region,           // NorthAmerica | Europe | Scandinavia | Other
    pub shoots:        Hand,             // L | R
    pub draft:         Option<DraftInfo>,
    pub rookie_season: Option<Season>,
    pub league_bg:     LeagueBackground, // NCAA | OHL | WHL | QMJHL | SHL | Liiga | KHL | Other
}

pub struct DraftInfo {
    pub year:    u16,
    pub round:   u8,
    pub overall: u16,
    pub team:    String,
}

pub enum Region {
    NorthAmerica,  // CAN + USA
    Scandinavia,   // SWE + FIN + NOR + DEN
    CentralEurope, // CZE + SVK + AUT + SUI
    Russia,
    Other,
}

pub struct PlayerGroup {
    pub id:          uuid::Uuid,
    pub name:        String,
    pub description: Option<String>,
    pub created_at:  DateTime<Utc>,
    pub members:     Vec<u32>,         // player_ids
    pub tags:        Vec<String>,      // e.g. ["draft-class", "2022", "C"]
}

pub struct SeasonHistory {
    pub player_id: u32,
    pub seasons:   Vec<SeasonLine>,
}

pub struct SeasonLine {
    pub season:  Season,   // 8-digit YYYYZZZZ, e.g. Season(20252026)
    pub team:    String,
    pub gp:      u32,
    pub goals:   u32,
    pub assists: u32,
    pub ppg:     f32,      // points per game — stored as f32 for display
                           // NOTE: projection engine casts to f64 at its boundary;
                           // document the cast with a comment at the call site.
    pub toi_pg:  u32,      // avg TOI per game in SECONDS (not minutes).
                           // Display layer divides by 60. Internal math stays integer.
}
```

---

## New CLI Commands Summary

> **Note on "Release" vs plan "Phase"**: Release here refers to data tier readiness,
> not implementation plan milestones. Release 1 = NHL API Tiers 1+2 (rosters + stats).
> Release 2 = Tier 3 shift data. All commands in this spec are planned for the
> implementation plan's Phase 2+. The foundation plan (Phase 1) ships only `team` and `rank`.

| Command | Release | Key Flags |
|---------|---------|-----------|
| `icelines players` | 1 | --pos, --age, --nationality, --region, --draft-year, --ppg-min |
| `icelines class <YEAR>` | 1 | --pos, --round, --compare, --years-in |
| `icelines peers <PLAYER>` | 1 | --by draft-class\|age\|pick-range |
| `icelines compare <P1> <P2>` | 1 | --pace, --history |
| `icelines group <CMD>` | 1 | create, add, show, compare, auto |
| `icelines history <PLAYER>` | 1 | --seasons, --pace (from NHL career stats API) |
| `icelines mates <PLAYER>` | 2 | --top, --min-shifts (requires Tier 3 shift data) |
| `icelines scouting <PLAYER>` | 2 | --format, --out |

Release 1 = NHL API Tiers 1+2 (rosters, bios, season stats — no Yahoo CSV required)
Release 2 = add Tier 3 shift data for `mates` + enhanced `scouting`

---

## Filtering Engine Design

All filter commands go through a shared `PlayerFilter` builder in `icelines-core`:

```rust
pub struct PlayerFilter {
    pub teams:        Option<Vec<String>>,
    pub positions:    Option<Vec<Position>>,
    pub age_range:    Option<(u8, u8)>,
    pub years_range:  Option<(u8, u8)>,
    pub nationalities:Option<Vec<String>>,
    pub regions:      Option<Vec<Region>>,
    pub draft_years:  Option<Vec<u16>>,
    pub draft_rounds: Option<Vec<u8>>,
    pub pick_max:     Option<u16>,
    pub undrafted:    Option<bool>,
    pub rookie_only:  Option<bool>,
    pub ppg_range:    Option<(f32, f32)>,
    pub gp_min:       Option<u32>,
    pub toi_min:      Option<f32>,   // minutes per game
    pub handedness:   Option<Hand>,
}

impl PlayerFilter {
    pub fn apply(&self, players: &[PlayerRecord]) -> Vec<&PlayerRecord> { ... }
}
```

This single filter type is used by `players`, `class`, `peers`, `group auto`, and any
future command that needs player subsetting. New filter dimensions are added in one place.

---

## Persistence

Groups and custom data persist to `~/.icelines/`:

```
~/.icelines/
├── cache/           — API response cache (see data-sources.md)
├── db/
│   └── icelines.db  — SQLite: PlayerRecord, ShiftProfile, SeasonHistory
└── groups.json      — PlayerGroup definitions (portable, human-readable)
```

`groups.json` is intentionally plain JSON — users can hand-edit, commit to a dotfiles repo,
or share with others. Player IDs are NHL canonical IDs so groups are portable across CSV exports.
