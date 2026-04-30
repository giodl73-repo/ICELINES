# Goalies — Specification

**Version**: 0.1
**Date**: 2026-04-29
**Status**: Draft — not yet implemented
**Replaces**: implicit "skater-only" assumption baked into `repository.rs`,
`schema::SkaterStats`, and `Player`.

---

## Purpose

Restore goalies as first-class players in IceLines. Today
`PlayerRepository::load_all()` drops `RosterResponse.goalies` on the floor
(`repository.rs:109-110`), `SkaterStats` is the only stats schema, and
`commands/fantasy.rs` literally tells users "goalies are not supported".
This spec brings goalies back across the entire stack:

- their own data type (separate from skaters — no muddied `Player` fields);
- a dedicated TUI tab for league-wide goalie comparison;
- presence on team depth charts (G1 starter / G2 backup);
- query and rank commands that handle goalies natively;
- fantasy goalie scoring wired through the existing `scheme.goalie` weights;
- five seasons of bundled goalie data alongside `bios.json` / `stats.json`.

---

## Data model

### Decision: separate `Goalie` struct (not a `Player` extension)

Three options were considered:

| Option | Pros | Cons |
|--------|------|------|
| (A) Add `goalie_stats: Option<GoalieStats>` to `Player` | One list, unified iteration | `Player.season_goals` etc. are meaningless for goalies; bug magnet |
| (B) **Separate `Goalie` struct + `Vec<Goalie>` alongside `Vec<Player>`** | Type system prevents skater ops on goalies; clean schema separation | Every command touching "all players" has to opt into goalies |
| (C) `enum Skater \| Goalie` polymorphism | Single sum type | `match` clutter at every use site |

**Decision**: **(B) separate struct**. The cost (commands explicitly choose
which list) is a feature, not a bug — goalies have nothing to do with `+/-`
or pace projections, so commands accidentally including them in skater
sorts is a current source of "totally f-d" output. Position-aware commands
become explicit about which species they care about.

### `icelines-fetch::schema::GoalieStats`

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalieStats {
    pub player_id:        u32,
    pub games_played:     u32,        // GP
    pub games_started:    u32,        // GS — distinct from appearances
    pub wins:             u32,
    pub losses:           u32,
    pub ot_losses:        u32,
    pub shots_against:    u32,
    pub goals_against:    u32,
    pub saves:            u32,
    pub save_pct:         f32,        // 0.0..=1.0 (NHL API publishes "savePctg")
    pub goals_against_avg: f32,       // GAA
    pub shutouts:         u32,
    pub time_on_ice_sec:  Option<u32>, // total goalie minutes
}
```

Mirrors the NHL `/stats/rest/en/goalie/summary` shape. Fields nullable in the
API are `Option<…>` here; `save_pct` and `goals_against_avg` are computed
fields the API ships pre-calculated.

### `icelines-core::model::Goalie`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goalie {
    pub nhl_id:           u32,
    pub full_name:        String,
    pub name_normalized:  String,
    pub team:             TeamAbbr,
    pub stats:            Option<GoalieStats>,    // None for never-played
    pub bio:              GoalieBio,              // age, draft, height, catches…
    pub headshot_url:     Option<String>,
    pub sweater_number:   Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalieBio {
    pub birth_date:        Option<String>,
    pub birth_country:     Option<String>,
    pub nationality_code:  Option<String>,
    pub catches:           Option<String>,        // "L" | "R"
    pub height_in_inches:  Option<u32>,
    pub weight_lbs:        Option<u32>,
    pub draft_year:        Option<u16>,
    pub draft_round:       Option<u8>,
    pub draft_overall:     Option<u16>,
    pub rookie_season:     Option<u32>,
}
```

### Derived metrics (`icelines-core::scoring`)

No "pace projection" — goalies don't accumulate counting stats like skaters,
and rate stats (SV%, GAA) are already per-game by definition. Provide:

- `goalie_pace_qualified(g, min_gp)` — bool gate at the position-rank
  threshold (default `min_gp = 15`, matching NHL's leaderboard convention).
- `gsaa(g, league_avg_sv_pct)` — goals saved above average; computed from
  `(league_sv_pct - g.save_pct) * shots_against`. League average is derived
  from the loaded snapshot, not an external feed.
- `quality_starts(g)` — placeholder for when per-game goalie logs land
  (deferred — out of scope here).

---

## NHL API surface

Single endpoint — same `/stats/rest/en` host as skater stats, different path:

```
GET https://api.nhle.com/stats/rest/en/goalie/summary
    ?cayenneExp=seasonId={SEASON} and gameTypeId=2
    &start={offset}&limit=100
    &sort=[{"property":"savePctg","direction":"DESC"}]
```

Pagination identical to skater pipeline. Add `fetch_goalie_stats(season)` to
`NhlApiClient` mirroring `fetch_stats`. Goalie bios come from the same
`/v1/player/{id}/landing` endpoint we already use for skaters — the `position`
field there carries the discriminator.

---

## Snapshot + bundling

### Snapshot store

New tier `SnapshotTier::GoalieStats` mapped to `goalie-stats.json` in the
snapshot directory layout. Identical write/read semantics to the existing
`Stats` tier. `verify` walks it. Chunked snapshots gain a parallel
`goalies/` subdirectory; refs table tracks chunks the same way.

### Bundled data

Each season's `data/seasons/{YYYY}{YYYY+1}/` directory gains
`goalie-stats.json` alongside `bios.json` and `stats.json`.
`bundled::get_goalie_stats(season) -> Option<Vec<GoalieStats>>` mirrors
`get_stats`. Embedded for the same five current seasons; loaded for any
installed historical season.

`load_bios_with_fallback` style: `bundled::load_goalies_with_fallback`
follows the chunked → legacy → embedded chain.

### `GoalieRepository`

New struct alongside `PlayerRepository`:

```rust
pub struct GoalieRepository { ... }
impl GoalieRepository {
    pub fn new(store: SnapshotStore, season: impl Into<String>) -> Self;
    pub fn load_all(&self) -> Result<Vec<Goalie>, FetchError>;
    pub fn load_team(&self, abbrev: &str) -> Result<Vec<Goalie>, FetchError>;
}
```

Roster source: `RosterResponse.goalies` (currently dropped). Each entry has
the bio fields; stats are matched in by `nhl_id`.

---

## TUI: dedicated Goalies tab

A new tab in the main nav, sitting after **Stats** in the tab order:

```
  Home  ·  Stats  ·  Goalies  ·  Scores  ·  Schedule  ·  Playoffs  ·  Groups
```

### Layout

```
┌─── 2025-26 Goalie Leaders  ·  ↑↓ Enter ·  s sort  ·  m min-gp  ·  Esc back  ──┐
│ Sort: SV% (desc) · Min GP: 15                                                  │
│  #  Goalie                Team    GP    W-L-OT    SV%      GAA   SO   Saves   │
│ ── ───────────────────── ───── ────  ─────────  ──────  ──────  ──  ──────── │
│  1  Connor Hellebuyck     WPG    65    37-21-7   .921    2.05    7    1840   │
│  2  Sergei Bobrovsky      FLA    52    32-13-5   .918    2.18    5    1497   │
│  3  Igor Shesterkin       NYR    58    30-21-7   .917    2.22    6    1623   │
│  4  Linus Ullmark          OTT    44    24-15-3   .914    2.30    4    1210   │
│  …                                                                              │
└────────────────────────────────────────────────────────────────────────────────┘
```

### Keys

| Key | Action |
|-----|--------|
| `↑↓` | Move row cursor |
| `Enter` | Open goalie detail card |
| `s` | Cycle sort: SV% → GAA → W → GP → Saves → SO → SV% |
| `m` | Open min-GP picker (5 / 15 / 25 / 40) |
| `t` | Filter to selected team only (cycles through 32 + "all") |
| `Esc` | Back to previous tab |
| `y` | Season picker (same season-time-travel as elsewhere) |

### Goalie detail card (`Enter` from leaderboard)

Mirrors the skater player card layout but with goalie-specific fields:

```
┌─── Connor Hellebuyck  WPG  ·  G  ·  CAN/L  ·  Esc back ─────────────────────┐
│ ⠁⢀⣀⣤⣶⣿⣿⣿⣿⣿⣿⣿⣿⣿⡏    Record                                                 │
│ ⣷⡿⠿⠿⠿⠿⠿⢿⣿⣿⣿⣿⣿⣿⡗     GP        65       W-L-OT    37-21-7                  │
│ ⣿⠿⢋⡠⠞⠉⠀⢀⣽⣿⣿⣿⣿⡎     SO         7       Saves     1840                     │
│ …(headshot)…             SV%      .921     GAA       2.05                     │
│                          GSAA   +18.4     QS%       60.0%   [v2]              │
│                                                                                │
│                          Bio                                                   │
│                          Draft: 2012 R5 #130                                  │
│                          CAN  Catches: L                                      │
│                          Born 1993-06-19 in Commerce, MI                       │
└──────────────────────────────────────────────────────────────────────────────┘
```

When `--dashboards` is on, the right-hand panel renders:

```
┌─ Scout card ─┐
│ Hellebuyck   │
│  ·  WPG G    │
│              │
│ Last 5 seasons 22→26
│ SV%  ▆▄▁▇█    .918 → .921
│ GAA  ▂▄▆▁▁   2.49 → 2.05  (lower=better, colors inverted)
│ W    ▄▅▃█▆    37  → 37
│              │
│ Pos vs G: #1/64
│ ███████████░  top 4%  │
└──────────────┘
```

Notes:
- GAA sparkline reads "lower is better" — invert the colour convention so
  green = below median (good) and red = above median (bad).
- Pos rank uses SV% by default (configurable later via `s` toggle).

### Off-season state

If no goalie has played any games in the active season (pre-October), tab
shows: *"No 2025-26 goalie data yet — press y to browse historical seasons"*.

---

## Team-card integration

`Screen::DepthTeam(abbrev)` and `Screen::Team(abbrev)` are the two screens
that render a team's roster. Both currently show **forwards only on the
left, defensemen on the right**. Add a third column or a bottom strip for
goalies:

```
  CENTER       LW          RW          DEFENSE         GOALTENDING
  Crosby       Rust        Rakell      Karlsson        Jarry        62 .906
  Malkin       Hayes       Hagel       Letang          Ne. Nedeljkovic 18 .898
  …
```

Constraints:
- **Two slots typical** (G1 starter, G2 backup). Sort by `games_played`
  descending so the starter appears first.
- Display: `Lastname  GP  SV%`. Single-line, fits the existing column width.
- If a team has more than two goalies on the active roster (e.g., AHL
  call-up scenarios), show all of them; the panel grows vertically.
- No fit-class colouring (skater concept).

---

## Command surface

### New: `icelines query goalies`

```
icelines query goalies [--top N] [--sort sv-pct|gaa|wins|gp|saves|so]
                       [--team ABC] [--min-gp N] [--season YYYYZZZZ]
                       [--json | --csv]
```

| Flag | Default | Notes |
|------|---------|-------|
| `--top N` | `20` | |
| `--sort` | `sv-pct` | `gaa` reverses sort direction (lower better) |
| `--team` | none | filter to one team |
| `--min-gp` | `15` | NHL leaderboard convention; bypass with `--min-gp 0` |
| `--season` | current | reuses Phase 8f.4 logic; validates against bundled seasons |
| `--json` / `--csv` | off | structured export for scripting |

### Extended: `icelines rank --pos G`

Already-existing position filter accepts `G`. Today it returns nothing
because no goalies live in `app.players`. With this spec wired:

- `rank --pos G` returns the goalie leaderboard sorted by `sv_pct` by
  default, identical column set to `query goalies` but rendered at the
  `rank` command's terminal style.
- `rank --top N` works as today; `--scheme` is rejected with a hint to use
  `query goalies --sort` since fantasy scoring composes goalie + skater
  weights and a single-position sort doesn't apply.

### Extended: `icelines fantasy`

Scoring already supports goalies — `scheme.goalie` weights have been there
since Phase 1. Wire `compute_fantasy_score` to detect goalies:

```rust
// icelines-core::scheme

pub fn compute_goalie_fantasy_score(
    stats: &GoalieStats,
    weights: &GoalieWeights,
    min_gp: u32,
) -> Option<FantasyScore> { ... }
```

Fantasy team rosters become `Vec<RosterEntry>` where:

```rust
enum RosterEntry {
    Skater(NhlId),
    Goalie(NhlId),
}
```

`fantasy team-add` looks up the player by name across both pools and stores
the right discriminator. `fantasy standings` totals a team's score by
summing skater + goalie contributions.

Fantasy-leagues spec gets a follow-up amendment.

### Extended: `icelines fetch`

```
icelines fetch goalies [--season YYYYZZZZ]
icelines fetch all      # also fetches goalies — currently silent on them
```

`fetch all --chunked` writes goalie data into the chunked snapshot layout.

---

## App state

```rust
// App field — populated alongside `players` after the loader returns
pub goalies: Vec<Goalie>,

// New screen variants
Screen::Goalies,                      // league leaderboard
Screen::GoalieDetail(usize),          // index into `goalies`
```

`tui::loader::LoadState` extended to load both `Vec<Player>` and `Vec<Goalie>`
in one task. Background fetch matches the existing pattern for skaters.

---

## Phasing

Six commits, roughly sized:

| Phase | Commit theme | LOC | Outcome |
|-------|--------------|-----|---------|
| **G.1** | Schema + repo + bundled loader | ~400 | `GoalieStats`, `Goalie`, `GoalieRepository`, `bundled::get_goalie_stats`, snapshot tier. No UI yet — verify via L1 tests. |
| **G.2** | `fetch goalies` command | ~250 | New CLI subcommand; populates `~/.icelines/snapshots/.../goalie-stats.json`. |
| **G.3** | TUI Goalies tab | ~600 | New screen + leaderboard render; sort + min-gp + team filter; tab nav wires; goalie detail card. |
| **G.4** | Team-card integration | ~200 | Goalies appear on `DepthTeam` and `Team` screens. |
| **G.5** | `query goalies` + `rank --pos G` | ~350 | CLI parity with skater leaders. |
| **G.6** | Fantasy goalie scoring | ~400 | Wire `RosterEntry` enum, scheme weights, standings. Update fantasy spec. |
| **G.7** (optional) | Bundled-history sparklines | ~300 | Goalie panel SV%/GAA/W trends across the 5 bundled seasons; mirrors skater scout card. |

Bundling 5 seasons of goalie data (~50KB per season × 5 = ~250KB) is part
of G.1 — fetch the data once, commit the JSON files into
`data/seasons/{YYYY}{YYYY+1}/goalie-stats.json` so the binary ships with
them via `include_bytes!()`.

---

## Open questions

1. **Multi-team goalies in a season?** A goalie traded mid-season (e.g.,
   Linus Ullmark BOS → OTT in 2024-25) appears in two team rosters with
   split stats. NHL API publishes one row per team-stint. **Decision**:
   surface both rows in the leaderboard with the team as a discriminator;
   the player card aggregates with a "split" badge. Match the NHL.com
   convention.

2. **What's "qualified" for percentile rank?** Default `min_gp = 15`. For
   the dashboard panel's pos-vs-league bar, sub-15-GP goalies show no rank
   ("Pos rank unavailable — < 15 GP this season").

3. **Can we render a meaningful TOI/SO/Wins figure historically?** Going
   pre-2010 is fine for raw counting stats but SV% data quality drops
   before the 1980s. Document but don't gate.

4. **Combined leaderboards?** `icelines rank --top 10` currently returns
   skaters by pace. **Decision**: keep it skater-only; goalies live behind
   `rank --pos G` or the dedicated `query goalies` command. Mixed-position
   scoring requires a fantasy scheme — there's no apples-to-apples ranking
   between a 100-point skater and a .920-SV% goalie.

5. **MoneyPuck goalie metrics (xGA, GSAx)?** The MoneyPuck integration in
   `icelines-fetch::moneypuck` is silo'd to skater xG today. Bringing
   goalie metrics in-line is a follow-up after G.7.

---

## Test plan

- **L0** — schema parsing, GoalieStats round-trip, GAA/SV% formula
  validation, qualifying-GP gate, sort comparators.
- **L1** — `GoalieRepository::load_all` builds correct `Vec<Goalie>` from
  fixture roster + bundled stats; multi-team trade case; goalie missing
  from stats (rookie call-up) keeps row with `stats = None`.
- **L2** — `fetch goalies --dry-run` exits 0; `query goalies --top 5`
  returns 5 rows with `.9XX` SV% values; `rank --pos G` matches.
- **TUI L0** — Goalies tab renders with off-season message when no rows;
  with rows shows columns; sort cycle shifts ordering; team filter narrows;
  goalie detail card opens on Enter.

Total expected: ~50 new tests across the workspace (currently 636).

---

## Decisions log

1. **Separate `Goalie` struct, not `Player` extension** — clean schema,
   prevents skater ops on goalies.
2. **Dedicated Goalies tab in the TUI** — positioned after Stats so
   skater-first workflows stay one tap away.
3. **Two-slot goalie strip on team cards** — starters surface first by GP.
4. **`min_gp = 15` qualifying threshold** — matches NHL leaderboard.
5. **No "pace" metric** — sort directly by raw rate stats.
6. **Phased shipment** — G.1 lands the data plumbing alone; UI lands in
   G.3; sparklines deferred to G.7 so each commit is independently
   shippable + revertable.
