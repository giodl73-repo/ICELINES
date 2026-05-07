# Phase Conn Smythe — Implementation orchestrator

**Specs**: `design/specs/conn-smythe-overview.md`
**Status**: Plan — orchestrator
**Date**: 2026-05-06

---

## Sub-phase ordering

```
C.1 (series momentum) ─────────────────────┐
                                            │
C.2 (Cup-run player narratives) ←─ C.1 ─────┤
                                            │
C.3 (live game tracking surface) ←─ C.1+C.2 ┘
```

Critical path: **C.1 → C.2 → C.3**. C.6 closeout (docs + persona
pass) lands as part of the C.3 commit.

## Per-sub-phase plans

| Sub-phase | Test budget | Notes |
|---|---|---|
| C.1 | 8 (4 L0 + 2 L1 + 2 L2) | Series momentum |
| C.2 | 6 (3 L0 + 1 L1 + 2 L2) | Cup-run player narratives |
| C.3 | 8 (3 L0 + 3 L1 + 2 L2) | Live game tracking surface |
| **Total** | **22 tests** | + persona scenarios in closeout |

## Pre-flight

- [x] Foster v0.18.0 shipped — favorites, EventStream, sync engine,
      windowed leaders all live
- [x] `parse_game` already handles `game_type=3` discrimination
- [x] `parse_playoff_bracket` returns rounds with series letters
- [x] Boxscore manifest shard exists (Foster +3)
- [ ] phases.md: Conn Smythe moves from Future to Active mapping
- [ ] C.1 spawn

## C.1 — Series momentum

### C.1.1 — `SeriesMomentum` schema (icelines-core)

```rust
pub struct SeriesMomentum {
    pub series_letter: String,
    pub season: Season,
    pub round: u8,
    pub top_seed_abbrev: TeamAbbr,
    pub bottom_seed_abbrev: TeamAbbr,
    pub top_seed_wins: u8,
    pub bottom_seed_wins: u8,
    pub games_played: u8,
    pub games_remaining: u8,        // best-of-7 — series ends at 4
    pub leader: SeriesLeader,        // Top | Bottom | Tied
    pub last_result: Option<SeriesGameResult>,
    pub ot_games: u8,
    pub home_advantage: bool,        // true when next game is at higher seed
}

pub enum SeriesLeader { Top, Bottom, Tied }

pub struct SeriesGameResult {
    pub game_id: GameId,
    pub date: NaiveDate,
    pub winner: TeamAbbr,
    pub score: (u32, u32),  // (winner, loser)
    pub ot: bool,
}
```

L0 tests (4): empty (0-0), top-seed leads, bottom-seed leads, OT
game count tracked.

### C.1.2 — `compute_series_momentum(bracket, series_letter)` projection

Pure function: takes a `PlayoffBracket` (existing type from
`icelines-fetch::nhl_api`) and a series letter, walks the series's
games + their boxscore JSONs to build `SeriesMomentum`.

### C.1.3 — CLI surface

`icelines playoffs --series A [--season 20252026]` renders the
momentum view. Default season = active playoffs season. Output:

```
SERIES A — 1st Round  ·  EDM (1) vs LAK (2)
EDM leads 2-1  ·  3 games played, 4 remaining  ·  1 OT
Last game (G3, Apr 23): EDM 4-3 LAK (OT)
Next game: G4 at LAK (Apr 25)
```

L2 test (1): `--series` recognized, prints expected fields.

### C.1.4 — TUI integration

Playoffs tab: list view stays; **Enter on a series → momentum
detail view**. New `Screen::SeriesMomentum(letter)` that takes
the same render pattern as `SeriesDetail` but renders momentum
above the existing per-game detail.

L1 test (1): render smoke for the new sub-screen.

### C.1.5 — Web /playoffs?series=A

`/playoffs` already accepts `?season=`. Add `?series=A` —
when set, render the momentum block above the bracket.

L1 test (1): route accepts both params, returns 200.

### C.1.6 — JSON envelope

`/api/v1/playoffs?series=A` returns `SeriesMomentum` directly.
L2 test (1): JSON shape pinned.

## C.2 — Cup-run player narratives

### C.2.1 — `compute_playoff_run` (icelines-cli or fetch)

```rust
pub struct PlayoffRunSummary {
    pub player_id: PlayerId,
    pub games: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub plus_minus: i32,
    pub sog: u32,
    pub hits: u32,
    pub blocks: u32,
    pub toi_seconds: u32,
    // For goalies — populated only when player is in any GoalieLine.
    pub goalie_record: Option<GoalieRecord>,
}

pub struct GoalieRecord {
    pub wins: u32,
    pub losses: u32,
    pub ot_losses: u32,
    pub saves: u32,
    pub shots_against: u32,
    pub goals_against: u32,
    pub save_pct: f32,
}
```

Walks the Boxscore manifest entries with `game_type=3` (read from
the boxscore JSON's `gameType` field) AND date in the playoff
window for the season. Reuses Foster +26's loop pattern.

L0 tests (3): empty (no playoff games on disk), skater aggregation,
goalie aggregation.

### C.2.2 — Player card playoff section

CLI `query player NAME` adds a "Playoff run" line when the run is
non-empty. TUI player card (existing `Screen::PlayerById`) adds a
sub-section. Web `/player/:id` template adds a section.

L2 test (1): `query player NAME --playoff` exits 0 and surfaces
the run line.

### C.2.3 — Playoffs tab Top-10 leaderboard

Playoffs TUI tab gains a footer block: "Top playoff scorers —
1. McDavid 30P · 2. ..." Reads from the same compute path,
top-10 by points.

L1 test (1): render smoke for the leaderboard block.

### C.2.4 — `query leaders --playoff`

Foster +26's `query_window` gets a sibling: `query leaders --playoff`
narrows to game_type=3 + the playoff date window.

L2 test (1): flag recognized, output filtered.

## C.3 — Live game tracking surface

### C.3.1 — `LiveGameDetail` schema

```rust
pub struct LiveGameDetail {
    pub game_id: GameId,
    pub away: TeamAbbr,
    pub home: TeamAbbr,
    pub away_score: u32,
    pub home_score: u32,
    pub period: u8,                       // 1, 2, 3, OT (=4), SO (=5)
    pub period_label: String,             // "2nd", "OT", "Final/SO"
    pub time_remaining: Option<String>,    // "4:32" when API exposes
    pub goal_summary: Vec<GoalSummary>,
    pub starting_goalies: (Option<String>, Option<String>),
    pub goalie_pulled: bool,
    pub state: GameState,
}

pub struct GoalSummary {
    pub period: u8,
    pub time_in_period: String,
    pub team: TeamAbbr,
    pub scorer: String,
    pub strength: GoalStrength,           // EV / PP / SH
}
```

L0 tests (3): in-progress mid-game, finalized game, pulled-goalie
detection.

### C.3.2 — TUI game detail screen

Already exists in skeleton (`Screen::GameDetail(u64)`). C.3 fills
in the live-state rendering: period scoreboard, goal summary list,
goalie line, last-fetch indicator. Reuses the Scores tab's 30s
auto-refresh ticker.

L1 test (1): render smoke for live state.

### C.3.3 — Web /game/:id route

New route. Same template pattern as the team / player page. Shows
the LiveGameDetail; auto-refreshes via meta refresh tag (30s) when
state ∈ {LIVE, CRIT}.

L1 tests (2): /game/:id returns 200; finalized state doesn't include
auto-refresh meta.

### C.3.4 — CLI tonight --game GID --detail

Single-shot text dump of the LiveGameDetail. No auto-refresh.

L2 tests (2): valid game id renders, invalid game id clean error.

## Files added

```
icelines-core/src/series_momentum.rs        ~150 lines  (new)
icelines-core/src/playoff_run.rs            ~120 lines  (new)
icelines-core/src/live_game.rs              ~150 lines  (new — LiveGameDetail schema)
icelines-cli/src/commands/playoffs_series.rs ~120 lines  (new)
icelines-cli/src/commands/playoff_run.rs    ~100 lines  (new — query player --playoff helper)
icelines-cli/src/tui/screens/series_momentum.rs ~150 lines (new)
icelines-cli/src/tui/screens/live_game.rs   ~150 lines  (new — fills in GameDetail)
icelines-web/src/lib.rs                     +250 lines  (favorites + game routes)
icelines-web/templates/series_momentum.html ~80 lines   (new)
icelines-web/templates/game_detail.html     ~120 lines  (new)
icelines-cli/tests/conn_smythe_persona.rs   ~250 lines  (new — 10 personas)
design/specs/conn-smythe-overview.md        (already shipped in C.0)
```

## Acceptance for Conn Smythe

- All three sub-phases ship with their test budgets met
- `cargo test --workspace` green at each commit
- Persona suite (`conn_smythe_persona.rs`) covers each of the three
  sub-phases with ≥3 scenarios
- README + COMMANDS.md updated with playoff-tracking section
- CLAUDE.md "What's been built" gains a Phase Conn Smythe bullet
- Tag `v0.19.0` (or higher; depends on what intermediate tags land)
  cut on the final C.3 commit

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| `parse_playoff_bracket` shape drift across seasons | Med | Defensive parse already in place; test against current + 1989 + 2010 brackets |
| Boxscore manifest size grows fast during playoffs | Low | ~16 series × 7 games max = 112 boxscores per round × 4 rounds = 448 total. ~30KB each = ~13MB. Trivial. |
| Live polling double-fires across surfaces | Med | Reuse the existing Scores-tab ticker; don't add a new one |
| Series-letter mapping inconsistency | Med | Foster.1 already handles this in parse_game; Conn Smythe inherits |

## Out-of-plan items (deferred to post-Conn-Smythe)

- Cup-run probability / xWin% modeling
- Historical Cup-run aggregates (pre-Foster boxscores)
- Pre-game previews (the AI direction option from earlier roadmap)
