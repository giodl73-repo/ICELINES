# Phase Hart — Data Model Normalization

**Status**: Draft v0.3 — incorporates WIRE / TAPE / EDGE / GLASS spec reviews
plus BENCH (test machinery) and FORGE (Rust soundness) plan reviews
**Date**: 2026-04-30
**Trophy**: Hart (league MVP — touches every consumer)
**Spec**: this document is both spec and plan
**Target**: v0.13.0
**Replaces**: design/plans/2026-04-30-phaseS-season-type.md (the season-type
work falls out of this phase for free; the Phase S plan stays as
historical record but is no longer the active path)

---

## Goal

Replace the flat-Player / parallel-Goalie data model with a fully
normalized one keyed by **(player_id, season, season_type)**. Stats,
team stints, advanced metrics, and goalie-specific fields all hang off
that primary key. PlayerIdentity is stored once per player, ever —
not duplicated per season.

Every recent feature (goalies, time-travel, transactions, season-type)
has wedged itself into the flat snapshot shape with awkward parallel
fields. That cost is non-recoverable until the model itself is fixed.
Phase Hart fixes it.

After Hart:
- Adding preseason is one match arm on `SeasonType`.
- Career history is `repo.career(id)` — derivable, not a separate type.
- Goalies and skaters share infrastructure; goalie fields are an
  `Option<GoalieSeasonStats>` on `SeasonStats` rather than a parallel
  species.
- Mid-season trades produce a `TeamStint` vec, not implicit "current
  team + comma-string of stops."
- The renderer asks `repo.season(id, season, type)` and gets a
  consistent shape every time.

---

## Why now

Three forcing functions:

1. **Phase S would have shipped a workaround.** The accessor pattern
   (`Player::active_stats(SeasonType)`) compiled around the flat shape
   without paying down the debt. Every future feature would hit the
   same wall and add another `Option<XStats>` to Player.

2. **Goalie duplication has compounded.** `GoalieRepository`,
   `goalie_repository.rs`, `Goalie::full_name`, `Goalie::team`,
   `Goalie::name_normalized` — all parallel to Player. A normalized
   model collapses them.

3. **The TUI's per-season views are getting awkward.** `app.players`
   and `app.goalies` are loaded for one (season, type); time-travel
   reloads. With per-(season, type) keying, time-travel becomes a
   selector change, not a reload.

---

## Non-goals

- Persistence to SQLite. The in-memory `StatsRepository` is enough for
  our scale (~1000 players × 5 seasons × 2 types = 10K rows). SQLite
  can come later if the bundled data grows.
- Live-fetch refresh during a TUI session. Phase Hart is a model
  change; live refresh stays Phase Hart-out-of-scope.
- Breaking the bundled JSON format on disk. The existing
  `bios.json` / `stats.json` files stay shape-compatible. Phase Hart
  rewrites only the in-memory model and the loader path.
- Schema migrations of the on-disk SnapshotStore. Same reason — Phase Hart
  is structural, not file-level.
- Public API stability for downstream tools. `icelines-core` is internal
  to this workspace; consumers all live in this repo.

---

## The new data model

### Identity (one per player, ever) — TAPE+EDGE-revised

```rust
// icelines-core/src/model.rs

/// Stable NHL player ID. The natural primary key — unique across
/// trades, retirements, name changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerId(pub u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerIdentity {
    pub id:              PlayerId,
    pub full_name:       String,
    pub name_normalized: String,
    /// Truly-stable canonical CDN headshot
    /// (`assets.nhle.com/mugs/nhl/default/{nhl_id}.png`). Season-agnostic
    /// — disk cache keyed by nhl_id only. Per-season+team URLs computed
    /// at render time from `SeasonStats.team_stints.last()`.
    pub headshot_canonical_url: Option<String>,
    pub bio:             PlayerBio,
}

/// `position` and `sweater_number` are NOT on PlayerIdentity (EDGE):
/// position re-categorizations (Marchand C→LW, Boeser RW→LW, hybrid
/// emergency-goalie scenarios) and sweater changes are per-season facts.
/// They live on `SeasonStats.position` and `SeasonStats.sweater_number`.
///
/// `is_goalie` is NOT a field anywhere (TAPE): derived per-row from
/// `SeasonStats.goalie.is_some()`. Avoids the David-Ayres / Scott-Foster
/// emergency-backup-goalie edge case where a non-goalie career suits up
/// in net for one game.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerBio {
    pub birth_date:       Option<String>,    // YYYY-MM-DD
    pub birth_country:    Option<String>,    // ISO-3
    pub nationality_code: Option<String>,
    pub height_in_inches: Option<u32>,
    pub weight_lbs:       Option<u32>,
    pub draft_year:       Option<u16>,
    pub draft_round:      Option<u8>,
    pub draft_overall:    Option<u16>,
    pub shoots_catches:   Option<String>,    // "L" | "R"
    pub rookie_season:    Option<String>,    // YYYYZZZZ
}
```

### Identity merge policy (TAPE)

A new fetch for an existing `PlayerId` runs through
`PlayerIdentity::merge_with(&mut self, incoming: PlayerIdentity)`:

- **Most-recent-non-null-wins with sanity floors**, NOT raw most-recent-wins.
  Reject obviously-bogus incoming values:
  - `weight_lbs < 100` or `> 350` → keep prior.
  - `height_in_inches < 60` or `> 84` → keep prior.
  - `birth_date` change with prior non-null → keep prior (birth dates are immutable).
  - `draft_year` / `draft_round` / `draft_overall` change → keep prior.
  - `rookie_season` change → keep prior.
  - `shoots_catches` is naturally immutable; mismatch → keep prior + warn.
- **PlayerId reissue detection** (EDGE): if incoming has a different
  `rookie_season` than persisted (and both are non-null), the loader
  errors with `LoadError::LikelyIdReissue { id, prior_rookie_season,
  incoming_rookie_season }` rather than silently overwriting.

Merge policy lives in `model.rs` as a `pub(crate)` impl, tested at L0
with proptest: prior-good + current-bad merges to prior-good.

### Stats (one per `(player_id, season, season_type)`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonStats {
    pub player_id:   PlayerId,
    pub season:      Season,
    pub season_type: SeasonType,

    /// Per-season fact (EDGE: moved off PlayerIdentity).
    /// Marchand 2017-18=C, 2018-19=LW round-trips correctly.
    pub position:        Position,

    /// Per-season fact. Sweater can change across teams.
    pub sweater_number:  Option<u32>,

    /// One stint per team played for this season+type. ≥1 always.
    /// Mid-season trades produce >1 stints. Sorted chronologically by
    /// `started`, with stable tie-break by `team` abbrev when `started`
    /// is None for both (TAPE: nondeterministic ordering would silently
    /// change which team `view.team()` returns).
    pub team_stints: Vec<TeamStint>,

    /// Aggregated across stints. Sum-equals invariant holds in fixtures;
    /// real API rows can mismatch by ±1 GP on game-of-trade. Loader
    /// policy (TAPE): trust API totals (matches NHL.com display),
    /// recompute stints to match by clamping the last stint, log a
    /// `tracing::warn` so drift is visible.
    pub totals:      StatTotals,

    /// Realtime stats (hits, blocks, takeaways, giveaways) when the
    /// realtime endpoint has been fetched. None during cold-start.
    /// `pub(crate)` to force consumers through `PlayerView::hits()`-style
    /// accessors that return Option at the leaf (WIRE: prevents the
    /// tired `.unwrap()`).
    pub(crate) realtime:    Option<RealtimeStats>,

    /// MoneyPuck advanced (xG, CF%, FF%, xGF%) when MoneyPuck has been
    /// fetched and joined. `pub(crate)` for the same reason.
    pub(crate) advanced:    Option<AdvancedStats>,

    /// Populated when this row's player suited up as a goalie this
    /// season+type (matches the TAPE-revised "is_goalie is derived"
    /// policy). None for skaters.
    pub goalie:      Option<GoalieSeasonStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamStint {
    pub team:    TeamAbbr,
    pub started: Option<String>,
    pub ended:   Option<String>,
    /// Stint-scoped subset. Sum-equals across stints produces totals
    /// (with the loader-policy clamping noted above).
    pub gp:      u32,
    pub goals:   u32,
    pub assists: u32,
    pub points:  u32,
    /// Per-stint goalie counts (EDGE: traded-mid-season goalies lose
    /// per-team GS/W/L without this). None for skaters.
    pub goalie:  Option<GoalieStintStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GoalieStintStats {
    pub games_started: u32,
    pub wins:          u32,
    pub losses:        u32,
    pub ot_losses:     Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatTotals {
    pub gp:                 u32,
    pub goals:              u32,
    pub assists:            u32,
    pub points:             u32,
    pub plus_minus:         i32,
    pub pim:                u32,
    pub shots:              u32,
    pub shooting_pct:       Option<f32>,
    pub toi_per_game_sec:   Option<u32>,
    pub pp_goals:           u32,
    pub pp_points:          u32,
    pub gwg:                u32,
    pub faceoff_win_pct:    Option<f32>,
    pub pace_score:         Option<PaceScore>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct RealtimeStats {
    pub hits:          u32,
    pub blocked_shots: u32,
    pub takeaways:     u32,
    pub giveaways:     u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct AdvancedStats {
    pub xg:        Option<f64>,
    pub xg_per_60: Option<f64>,
    pub cf_pct:    Option<f64>,
    pub ff_pct:    Option<f64>,
    pub xgf_pct:   Option<f64>,
}

/// Goalie season-aggregate. Lives ON SeasonStats, not as a parallel
/// species. `qualified_for(SeasonType)` carries the 15/4 threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalieSeasonStats {
    pub games_started:         u32,
    pub wins:                  u32,
    pub losses:                u32,
    pub ot_losses:             Option<u32>,
    pub ties:                  Option<u32>,
    pub shots_against:         u32,
    pub goals_against:         u32,
    pub saves:                 u32,
    pub save_pct:              Option<f32>,
    pub goals_against_average: Option<f32>,
    pub shutouts:              u32,
    pub time_on_ice:           u32,
}

impl GoalieSeasonStats {
    pub fn qualified_for(&self, season_type: SeasonType, gp: u32) -> bool {
        let min = match season_type {
            SeasonType::Regular => 15,
            SeasonType::Playoff => 4,
        };
        gp >= min
    }
}
```

### Mid-playoff-trade worked example (TAPE)

Vladimir Tarasenko, 2022-23 season (regular STL → NYR Feb 2023; playoff
NYR-only):

```text
SeasonStats { player_id: 8475765, season: 20222023, season_type: Regular,
              position: RW, sweater_number: Some(91),
              team_stints: [
                TeamStint { team: STL, gp: 38, goals: 10, ... },
                TeamStint { team: NYR, gp: 31, goals: 8,  ... },
              ],
              totals: { gp: 69, goals: 18, ... } }

SeasonStats { player_id: 8475765, season: 20222023, season_type: Playoff,
              position: RW, sweater_number: Some(91),
              team_stints: [
                TeamStint { team: NYR, gp: 7, goals: 1, ... },
              ],
              totals: { gp: 7, goals: 1, ... } }
```

Two rows, two season_types, separate stint vecs. Goalies traded mid-
playoffs use `TeamStint.goalie` to track per-stint W/L/GS.

### Stats (one per `(player_id, season, season_type)`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonStats {
    pub player_id:   PlayerId,
    pub season:      Season,
    pub season_type: SeasonType,

    /// One stint per team played for this season+type. ≥1 always.
    /// Mid-season trades produce >1 stints; the totals are aggregates.
    pub team_stints: Vec<TeamStint>,

    /// Aggregated across stints. The most-common UI shape — "their
    /// 2023-24 regular season."
    pub totals:      StatTotals,

    /// Realtime stats (hits, blocks, takeaways, giveaways) when the
    /// realtime endpoint has been fetched. None during cold-start.
    pub realtime:    Option<RealtimeStats>,

    /// MoneyPuck advanced (xG, CF%, FF%, xGF%) when MoneyPuck has been
    /// fetched and joined. None for goalies and for skaters when the
    /// CSV hasn't been pulled.
    pub advanced:    Option<AdvancedStats>,

    /// Populated for goalies only — Some when this row's player is a
    /// goalie (matches `PlayerIdentity.is_goalie`). None for skaters.
    pub goalie:      Option<GoalieSeasonStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamStint {
    pub team:    TeamAbbr,
    /// Span of dates this stint covers. Both Optional because the API
    /// rarely tells us. We persist what we know.
    pub started: Option<String>,
    pub ended:   Option<String>,
    /// Stint-scoped subset of StatTotals. Sum-equals across stints
    /// produces the SeasonStats::totals row.
    pub gp:      u32,
    pub goals:   u32,
    pub assists: u32,
    pub points:  u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatTotals {
    pub gp:                 u32,
    pub goals:              u32,
    pub assists:            u32,
    pub points:             u32,
    pub plus_minus:         i32,
    pub pim:                u32,
    pub shots:              u32,
    pub shooting_pct:       Option<f32>,
    pub toi_per_game_sec:   Option<u32>,
    pub pp_goals:           u32,
    pub pp_points:          u32,
    pub gwg:                u32,
    pub faceoff_win_pct:    Option<f32>,
    /// PaceScore is computed lazily from points + gp; cached here so
    /// repeated reads are O(1).
    pub pace_score:         Option<PaceScore>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealtimeStats {
    pub hits:          u32,
    pub blocked_shots: u32,
    pub takeaways:     u32,
    pub giveaways:     u32,
    // pim already lives in StatTotals
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdvancedStats {
    pub xg:        Option<f64>,
    pub xg_per_60: Option<f64>,
    pub cf_pct:    Option<f64>,
    pub ff_pct:    Option<f64>,
    pub xgf_pct:   Option<f64>,
}

/// Goalie-specific row. Lives ON SeasonStats, not as a parallel species.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalieSeasonStats {
    pub games_started:         u32,
    pub wins:                  u32,
    pub losses:                u32,
    pub ot_losses:             Option<u32>,
    pub ties:                  Option<u32>,
    pub shots_against:         u32,
    pub goals_against:         u32,
    pub saves:                 u32,
    pub save_pct:              Option<f32>,
    pub goals_against_average: Option<f32>,
    pub shutouts:              u32,
    pub time_on_ice:           u32,    // seconds
}

impl GoalieSeasonStats {
    /// Type-aware qualifying threshold. 15 GP for Regular, 4 GP for
    /// Playoff (a Cup-final losing starter still qualifies).
    pub fn qualified_for(&self, season_type: SeasonType) -> bool {
        let min = match season_type {
            SeasonType::Regular => 15,
            SeasonType::Playoff => 4,
        };
        // GP comes from the SeasonStats parent's totals — the caller
        // passes the right value or we expose a richer view (see below).
        // For now, derive from games_started.
        self.games_started >= min
    }
}
```

### `Projection` — tagged, like Phase S v0.3 (FORGE carryover)

```rust
pub enum Projection { Per82(f64), PerGame(f64) }
impl Projection {
    pub fn label(&self) -> &'static str { match self { Self::Per82(_) => "/82", Self::PerGame(_) => "/g" } }
    pub fn render(&self) -> String { /* "138.0/82" or "0.95/g" */ }
}
impl PaceScore {
    pub fn projected_for(&self, season_type: SeasonType) -> Projection {
        match season_type {
            SeasonType::Regular => Projection::Per82(self.pace_82),
            SeasonType::Playoff => Projection::PerGame(self.points_per_game()),
        }
    }
}
```

### Repository — WIRE+EDGE+TAPE-revised

```rust
// icelines-core/src/stats_repository.rs

/// Configurable cap. Default 8 (Season, SeasonType) windows resident at
/// once — well above the 5 bundled seasons × 2 types but bounded for
/// `--season 19951996` time-travel after live install (EDGE).
pub const DEFAULT_LRU_CAP: usize = 8;

pub struct StatsRepository {
    pub identities: HashMap<PlayerId, PlayerIdentity>,
    pub stats:      HashMap<(PlayerId, Season, SeasonType), SeasonStats>,
    pub contracts:  HashMap<PlayerId, PlayerContract>,

    // Derived indexes — rebuilt on upsert, not persisted independently.
    rosters_last_stint: HashMap<(Season, SeasonType, TeamAbbr), Vec<PlayerId>>,
    rosters_all_stints: HashMap<(Season, SeasonType, TeamAbbr), Vec<PlayerId>>,

    // LRU bookkeeping for (Season, SeasonType) windows. Identities never
    // evict — they're cheap and stable.
    window_lru: VecDeque<(Season, SeasonType)>,
    lru_cap:    usize,

    // EDGE: !Sync by construction so concurrent upserts can't tear the
    // roster indexes. Loader is documented single-threaded; render
    // path holds an immutable borrow.
    _not_sync:  std::marker::PhantomData<std::cell::Cell<()>>,
}

impl StatsRepository {
    pub fn new() -> Self { /* lru_cap = DEFAULT_LRU_CAP */ }
    pub fn with_lru_cap(cap: usize) -> Self { /* … */ }

    // ── Single-row lookups ──
    pub fn identity(&self, id: PlayerId) -> Option<&PlayerIdentity> { /* … */ }
    pub fn season(&self, id: PlayerId, s: Season, t: SeasonType) -> Option<&SeasonStats> { /* … */ }
    pub fn contract(&self, id: PlayerId) -> Option<&PlayerContract> { /* … */ }
    pub fn view(&self, id: PlayerId, s: Season, t: SeasonType) -> Option<PlayerView<'_>> { /* … */ }

    // ── Career iterators (TAPE: typed, never mixed) ──
    //
    // FORGE: explicit `'a` lifetime — `impl Iterator<…> + 'a` form
    // captures `&self` correctly on stable. `+ '_` would also work
    // but explicit is portable across edition + MSRV.
    pub fn career_regular<'a>(&'a self, id: PlayerId)
        -> Option<impl Iterator<Item = &'a SeasonStats> + 'a> { /* … */ }
    pub fn career_playoff<'a>(&'a self, id: PlayerId)
        -> Option<impl Iterator<Item = &'a SeasonStats> + 'a> { /* … */ }
    pub fn career_all<'a>(&'a self, id: PlayerId)
        -> Option<impl Iterator<Item = &'a SeasonStats> + 'a> { /* … */ }

    // ── League / roster iterators ──
    pub fn league<'a>(&'a self, s: Season, t: SeasonType)
        -> impl Iterator<Item = PlayerView<'a>> + 'a { /* … */ }
    pub fn skaters<'a>(&'a self, s: Season, t: SeasonType)
        -> impl Iterator<Item = PlayerView<'a>> + 'a { /* … */ }
    pub fn goalies<'a>(&'a self, s: Season, t: SeasonType)
        -> impl Iterator<Item = PlayerView<'a>> + 'a { /* … */ }

    /// Default roster: players whose LAST stint was on this team
    /// (= "current home"). The number that matches the lineup card today.
    pub fn team_roster(&self, team: &TeamAbbr, s: Season, t: SeasonType) -> Vec<PlayerView<'_>> { /* … */ }
    /// Analytical roster: every player who had ANY stint with this team.
    /// Used by trade/historical views.
    pub fn team_roster_all_stints(&self, team: &TeamAbbr, s: Season, t: SeasonType) -> Vec<PlayerView<'_>> { /* … */ }

    // ── Mutators (loader-only — note: not pub(crate) because the
    // loader lives in icelines-fetch; the crate boundary forces these
    // to be pub. Stats construction happens via `SeasonStatsBuilder`
    // — see FORGE adjustments below — so consumers can't accidentally
    // construct a SeasonStats with arbitrary realtime/advanced fields).
    pub fn upsert_identity(&mut self, identity: PlayerIdentity)
        -> Result<(), LoadError> { /* … */ }
    pub fn upsert_stats(&mut self, stats: SeasonStats)
        -> Result<(), LoadError> { /* invalidates roster indexes; updates LRU */ }

    // ── Atomic replacement (EDGE: avoid mid-render mutation) ──
    /// Swap the entire repository state. Returns the old repo via
    /// `mem::replace` so callers can drop, inspect, or roll back. All
    /// currently-borrowed PlayerViews are invalidated by the swap;
    /// render paths must drop borrows before calling. Borrow checker
    /// enforces this at compile time — see the `compile_fail` doctest.
    pub fn repo_swap(&mut self, new_repo: StatsRepository) -> StatsRepository {
        std::mem::replace(self, new_repo)
    }

    // ── Lazy-load contract (WIRE) ──
    //
    // Two flavors per FORGE: the TUI's render path is sync (no .await),
    // so it uses the blocking variant. CLI commands run inside #[tokio::main]
    // and use the async variant to cooperate with the runtime.

    /// Synchronous load — TUI render path. Blocks for ~50ms while
    /// reading off disk. LRU may evict LRU (season, type) when over cap.
    pub fn ensure_loaded(&mut self, s: Season, t: SeasonType)
        -> Result<(), LoadError> { /* … */ }

    /// Async wrapper — CLI / loader path. Internally `spawn_blocking`s
    /// the disk read so we don't stall the runtime.
    pub async fn ensure_loaded_async(&mut self, s: Season, t: SeasonType)
        -> Result<(), LoadError> { /* … */ }
}

/// Borrowed projection over the natural join. Render code never sees
/// raw structs — accesses go through PlayerView accessors.
pub struct PlayerView<'a> {
    pub identity: &'a PlayerIdentity,
    pub stats:    &'a SeasonStats,
    pub contract: Option<&'a PlayerContract>,
}

impl PlayerView<'_> {
    pub fn full_name(&self) -> &str { &self.identity.full_name }

    /// Per-season position (TAPE: lives on SeasonStats, not identity).
    pub fn position(&self) -> Position { self.stats.position }

    /// Last-stint team (= "current home" semantics matching today's UI).
    pub fn team(&self) -> Option<&TeamAbbr> {
        self.stats.team_stints.last().map(|s| &s.team)
    }
    /// GLASS: render-side helper — always returns a string, never
    /// pushes None-handling onto every render call site. Empty stints
    /// (impossible per invariant but defensive) → em-dash.
    pub fn team_display(&self) -> &str {
        self.team().map(|t| t.0.as_str()).unwrap_or("—")
    }
    /// True if traded mid-(season, type). Used by Player card to decide
    /// whether to render the per-stint sub-block (GLASS).
    pub fn was_traded_in_window(&self) -> bool {
        self.stats.team_stints.len() > 1
    }

    pub fn goals(&self)    -> u32 { self.stats.totals.goals }
    pub fn assists(&self)  -> u32 { self.stats.totals.assists }
    pub fn points(&self)   -> u32 { self.stats.totals.points }
    pub fn gp(&self)       -> u32 { self.stats.totals.gp }
    pub fn pace_score(&self) -> Option<&PaceScore> {
        self.stats.totals.pace_score.as_ref()
    }
    pub fn is_goalie(&self) -> bool { self.stats.goalie.is_some() }

    // WIRE: realtime/advanced read through Option-at-leaf accessors,
    // never `.unwrap()`.
    pub fn hits(&self)          -> Option<u32> { self.stats.realtime.as_ref().map(|r| r.hits) }
    pub fn blocked_shots(&self) -> Option<u32> { self.stats.realtime.as_ref().map(|r| r.blocked_shots) }
    pub fn takeaways(&self)     -> Option<u32> { self.stats.realtime.as_ref().map(|r| r.takeaways) }
    pub fn giveaways(&self)     -> Option<u32> { self.stats.realtime.as_ref().map(|r| r.giveaways) }
    pub fn xg(&self)            -> Option<f64> { self.stats.advanced.as_ref().and_then(|a| a.xg) }
    pub fn cf_pct(&self)        -> Option<f64> { self.stats.advanced.as_ref().and_then(|a| a.cf_pct) }
}
```

`PlayerView` is the **one type the UI sees**. The flat `Player` struct
of today goes away entirely.

### Loader — partial-fetch transparency (WIRE)

```rust
// icelines-fetch/src/stats_loader.rs

/// Result of populating the StatsRepository from a fetch / bundled load.
/// Surfaces partial conditions so callers can render a "MoneyPuck timed
/// out" banner instead of silently shipping advanced=None for everyone.
pub struct LoadOutcome {
    pub repo:          StatsRepository,
    /// Sources that didn't materialize. Empty vec = clean load.
    pub missing:       Vec<MissingSource>,
    /// Files we expected but didn't find (per-season bundled-file
    /// audit; EDGE: bios.json present, stats.json missing → expected
    /// SeasonStats rows missing).
    pub missing_files: Vec<String>,
    pub fetched_at:    String,
}

pub enum MissingSource {
    Realtime { season: String, season_type: SeasonType, reason: String },
    MoneyPuck { season: String, reason: String },
    Contracts { reason: String },
    GoalieStats { season: String, season_type: SeasonType, reason: String },
}

pub fn load_into_repo(
    season:      Season,
    season_type: SeasonType,
    store:       &SnapshotStore,
) -> Result<LoadOutcome, LoadError>;
```

CLI / TUI translate `MissingSource` variants into specific user-facing
banners; never collapse "MoneyPuck unavailable" with "snapshot missing."

### Schema versioning (WIRE)

`_meta.json` (Phase T struct) gains two version fields:

```rust
// icelines-fetch/src/snapshot.rs (extension of Phase T's SnapshotMetaFlags)
pub struct SnapshotMetaFlags {
    // ... Phase T fields preserved ...

    /// Bundled-JSON file format version. Loader validates: equal = OK;
    /// `incoming > known` = error with upgrade message; `incoming < known`
    /// = run a migrator to produce the in-memory shape.
    pub bundle_schema_version: u32,
    /// In-memory `StatsRepository` model version. Bump on every
    /// breaking change to the model (Phase Hart starts at 1).
    pub repository_version:    u32,

    /// From Phase S v0.3 (carried forward).
    pub playoff_phase:         icelines_core::model::PlayoffPhase,
    pub playoff_data_through:  Option<String>,
}
```

### Errors — split fetch vs load (FORGE-revised)

```rust
// icelines-fetch/src/error.rs — fetch-time only (already from Phase T)
pub enum FetchError {
    // ... existing variants ...
    SuspiciousEmpty { season: String, season_type: SeasonType },
}

// icelines-fetch/src/stats_loader.rs — load-time only.
// FORGE: I/O + Parse wrapped under a single `Bundle` variant so the
// public surface stays small; `#[from]` makes `?` ergonomic.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("season {season} not bundled in this build")]
    SeasonNotBundled    { season: String },
    #[error("season {season} has no {season_type:?} bundle")]
    MissingBundle       { season: String, season_type: SeasonType },
    #[error("likely PlayerId reissue: {id:?} prior_rookie={prior_rookie_season:?} incoming={incoming_rookie_season:?}")]
    LikelyIdReissue     { id: PlayerId, prior_rookie_season: Option<String>,
                          incoming_rookie_season: Option<String> },
    #[error("stats upserted before identity for player {id:?} at {season}/{season_type:?}")]
    StatsWithoutIdentity { id: PlayerId, season: String, season_type: SeasonType },
    #[error("bundle schema version {found} unknown (this binary supports up to {max_known})")]
    BundleSchemaUnknown { found: u32, max_known: u32 },
    /// Wraps I/O + parse failures so the public surface is one variant.
    #[error("bundle read/parse failure: {source}")]
    Bundle { #[from] source: BundleError },
}

#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Parse(#[from] serde_json::Error),
}
```

### FORGE soundness adjustments

A handful of small but load-bearing decisions called out separately so
each Hart sub-phase inherits the contract:

- **Cross-crate construction via `SeasonStatsBuilder`.** `pub(crate)`
  visibility on `SeasonStats.realtime` / `.advanced` would prevent the
  loader (in `icelines-fetch`) from setting them. Solution: keep fields
  `pub` on `SeasonStats`, but provide a builder that's the *recommended*
  construction path and the only thing the loader uses:
  ```rust
  pub struct SeasonStatsBuilder { /* … */ }
  impl SeasonStatsBuilder {
      pub fn new(player_id: PlayerId, season: Season, season_type: SeasonType,
                 position: Position) -> Self { /* … */ }
      pub fn with_team_stints(self, stints: Vec<TeamStint>) -> Self { /* … */ }
      pub fn with_totals(self, totals: StatTotals) -> Self { /* … */ }
      pub fn with_realtime(self, r: RealtimeStats) -> Self { /* … */ }
      pub fn with_advanced(self, a: AdvancedStats) -> Self { /* … */ }
      pub fn with_goalie(self, g: GoalieSeasonStats) -> Self { /* … */ }
      pub fn with_sweater_number(self, n: u32) -> Self { /* … */ }
      pub fn build(self) -> SeasonStats { /* validates invariants, sorts stints */ }
  }
  ```
  Reads still go through `PlayerView` accessors. Direct `pub` field
  reads outside `model.rs` and migrated allow-list are caught by the
  CI guard.

- **`StatsRepository: !Sync + !Send`.** `PhantomData<Cell<()>>` makes
  the type both `!Sync` AND `!Send` (the cell type is neither). For
  any background-loader path that wants `tokio::spawn(async move {
  repo... })`, callers must wrap the repo in `Arc<RwLock<_>>` at the
  call site. This trade-off is intentional: tearing the roster index
  via concurrent upsert is the nightmare we're avoiding. Documented in
  §Risks. Static assertion enforces:
  ```rust
  use static_assertions::assert_not_impl_any;
  assert_not_impl_any!(StatsRepository: Sync, Send);
  ```

- **`Season(pub u32)`** (existing) gains `#[serde(transparent)]` to
  ensure JSON output stays a bare number rather than `[20252026]`. This
  is a no-op behavior-wise IF the current derive emits `20252026`
  already, but Hart.0 verifies and locks. PlayerId is already specced
  with `#[serde(transparent)]`.

- **`#[serde(default)]` on every `Option<...>` field** added in Hart.1
  so bundles written before this phase (without the new field present)
  parse cleanly. Bumping `bundle_schema_version` only happens when a
  *non-Option* field is added.

- **`PlayerIdentity::merge_with(self, incoming: PlayerIdentity)`**
  takes `incoming` by value (loader owns the parsed identity; saves
  the String clones).

- **PlayerView lifetime discipline.** `PlayerView<'a>` is *render-scoped
  only*; it MUST NOT be a field of any struct outliving a frame.
  Doc-comment on `PlayerView` enforces socially; a future linting layer
  could enforce mechanically (e.g. a sealed marker trait). For now,
  Hart.0 surveys for any current Player-storage patterns that would
  port to PlayerView-storage and flags them in the migration checklist.

---

## Migration strategy

The risk on a phase this size is a long-running broken `main`. Strategy:

### Parallel-run approach

1. **Hart.1 + Hart.2** add the new types alongside the old ones. Old
   `Player` / `Goalie` / `PlayerRepository` / `GoalieRepository` stay
   functional. Tests still pass.
2. **Hart.3** populates a new `StatsRepository` from the same fetcher
   path that populates the old repos. Both populated for one merge cycle.
3. **Hart.4** rewrites consumers one screen / command at a time. Each
   commit migrates a slice (e.g. "Goalies tab → StatsRepository") with
   tests for that slice. Main stays green throughout.
4. **Hart.5** deletes the old `Player` flat struct, `Goalie` type,
   `PlayerRepository`, `GoalieRepository` once every consumer has moved.
5. **Hart.6** captures + bundles playoff data using the new shape (the
   season-type-from-Hart freebie).

This is multiple commits, each independently green. No single PR drops
~80 call sites at once.

### Compatibility shim

For Hart.4's transition window, expose:

```rust
impl StatsRepository {
    /// Flat-Player view of the current season. Reproduces today's
    /// PlayerRepository::load_all() output. Deleted in Hart.4j (last
    /// consumer migration), NOT Hart.5 (EDGE: leaves Hart.5 to be
    /// pure-cleanup with no compile risk).
    #[deprecated(note = "use league(season, type) and PlayerView instead")]
    pub fn flat_view_legacy(&self, s: Season, t: SeasonType) -> Vec<Player> { /* … */ }
}
```

### CI gate — machine-checked, not aspirational (WIRE+BENCH+FORGE)

Three enforcement mechanisms:

1. **`.deprecated_budget` file** at the repo root, containing a single
   integer. CI runs:
   ```
   cargo build --message-format=json \
     | jq '[.[] | select(.reason == "compiler-message"
                         and (.message.message | test("deprecated"))) ] | length'
   ```
   and **fails on `actual != budget`** (NOT `actual <= budget` — BENCH:
   silent drift if a fix decreases count without updating the file).
   Each Hart.4* commit either decrements the budget along with the
   migration, or stays equal — never inequal. Hart.4j ends at 0.

2. **`migrated.txt`** at the repo root, one module path per line. Each
   Hart.4 commit appends one line. The sentinel test
   `icelines-cli/tests/no_deprecated_in_migrated.rs` reads this file at
   compile time and emits `use` statements via macro expansion, with
   `#![deny(deprecated)]` at the top — any module re-introducing
   legacy-shim use fails compilation. FORGE: this is cleaner than a
   `build.rs` walking source paths because the gate is explicit and
   review-able in the same PR that adds the migration.

3. **`compile_fail` doctest** on `repo_swap`:
   ```rust
   /// ```compile_fail
   /// let mut repo = StatsRepository::new();
   /// let view = repo.view(...);  // borrows &repo
   /// repo.repo_swap(StatsRepository::new());  // requires &mut self
   /// drop(view);
   /// ```
   ```
   Confirms borrow-checker enforcement of the "drop borrows before
   mutating" contract; documents the rule next to the API.

Hart.5 deletes `flat_view_legacy` (already deleted in Hart.4j),
`migrated.txt`, and `.deprecated_budget` — they're scaffolding.

### Hart.4 commit ordering (EDGE)

Joining screens (player card has identity + stats + transactions +
groups; goalies tab has stats + filters + sort) migrate **last** within
their cluster. Cross-surface tests run between commits to assert name
parity — a player traded mid-season must show the same team string on
the player card and on a transactions row mentioning them.

Ordering:
- 4a: `query` / `players` / `rank` (no joins; narrow)
- 4b: `analysis` (class / peers / compare — pairwise joins, simple)
- 4c: `project` / `scouting` (single-player, deep accessor use)
- 4d: `fantasy` (joined with scheme; load_pools must produce the same
  Vec<PlayerView> shape)
- 4e: TUI Home / Team / Player card (largest single conceptual shift;
  per-stint sub-block is here)
- 4f: TUI Depth chart
- 4g: TUI Goalies (the parallel-Goalie collapse — biggest deletion)
- 4h: TUI Stats (Queries / Projections / Search)
- 4i: TUI Transactions player-card link
- 4j: icelines-site (mkdocs builder) + delete `flat_view_legacy`

---

## UI contracts pulled forward (GLASS)

These render-side rules are committed up-front so each Hart.4* sub-commit
inherits the same contract:

- **`view.team_display() -> &str`** is the render-side accessor. Every
  column / row label uses this. Empty-stint case → em-dash (`—`,
  single char, right-aligned safe). Renderers never branch on
  `view.team()`'s Option.
- **Team strip** (`team_roster`) defaults to last-stint-only. The
  number that today says "EDM — 23 players" stays 23 after Hart for
  unchanged rosters; players traded TO EDM mid-season appear, players
  traded FROM EDM disappear (matches "current home" mental model).
  Trade history view uses `team_roster_all_stints`.
- **Player card** renders one Stats block (totals) by default; if
  `view.was_traded_in_window()`, append a per-stint sub-block under
  the totals showing each TeamStint's gp/goals/assists/points and the
  team abbreviation. Single-team players see zero new chrome.
- **Four-state empty handling** carries forward from Phase S v0.3 AND
  applies to skater stat blocks, not just goalies:
  - `playoff_phase=NotStarted` → "Playoffs have not begun for {season}" banner
  - `Final` season + no SeasonStats row → "Did not appear in playoffs"
  - SeasonStats with totals.gp=0 → impossible (loader normalizes to "no row")
  - SeasonStats with gp>0 and points=0 → render normal stat block; zeros are real.
- **Time-travel visual cue**: the `y` season picker triggers a one-frame
  inverse-color flash on the season segment of the title bar so the
  user sees the action register. No "some loaded / some not" banner —
  bundled-data fall-through guarantees the 5 standard seasons are
  always present; `ensure_loaded` for off-bundle seasons blocks ~50ms
  with a status-line spinner.
- **Goalies tab keystroke contract** locked at Hart.4g acceptance:
  - `s` cycles sort: SV% / GAA / W / GP / Saves / SO (unchanged).
  - `m` cycles min-GP threshold: `[Default(qualified_for(active_type)),
    All, Custom]`. Default is type-aware (15 RS / 4 PO).
  - Columns unchanged: GP / W-L-OT / SV% / GAA / SO / Saves.
- **Career history**: `repo.career_regular(id)` /
  `repo.career_playoff(id)` typed iterators, never mixed (TAPE foul
  fix carried forward). Renderer uses `take(visible_rows)` — never
  `collect::<Vec<_>>()` — so 25-season `data install` users don't
  pay an unbounded allocation per render.

## Affected surfaces

Almost every file in `icelines-cli` and `icelines-core`. High-level:

| Area | Change |
|---|---|
| `icelines-core::model` | Player struct deleted; PlayerIdentity + SeasonStats + PlayerView added; Goalie struct deleted (its data moves into SeasonStats.goalie); TeamAbbr unchanged. |
| `icelines-core::filter` | PlayerFilter operates on `&PlayerView` instead of `&Player`. Same predicates, same semantics. |
| `icelines-core::scoring` | `compute_pace_score` operates on `&SeasonStats`. `sort_by_pace` takes `&mut [PlayerView]`. |
| `icelines-core::cross_team` | TeamStrength built from `repo.team_roster(...)` instead of player iter. |
| `icelines-core::scheme` | `compute_fantasy_score(view, scheme)` takes `&PlayerView`. |
| `icelines-core::history` | CareerSummary derived from `repo.career(id)` via iterator chain. The standalone type might survive as a UI-rendered shape; the load path goes away. |
| `icelines-fetch::repository` | Replaced by `StatsRepository` + a thin loader that pulls bios + stats + realtime + moneypuck and merges into the repo. |
| `icelines-fetch::goalie_repository` | Deleted — goalies are loaded by the same path, with `is_goalie=true` and `SeasonStats.goalie=Some(...)`. |
| `icelines-fetch::bundled` | `load_with_fallback(season, type)` returns a populated `StatsRepository`; same fall-through chain. |
| `icelines-cli::commands::*` | Every report command's loader / filter / output stage moves to the new shape. CSV/JSON output unchanged at the column level. |
| `icelines-cli::tui::*` | `app.players: Vec<Player>` becomes `app.repo: StatsRepository`. Each screen rewrites its iter to use `app.repo.league(season, type)` etc. |
| `icelines-site::*` | mkdocs site builder moves to PlayerView. |
| Bundled JSON files | Unchanged on disk. Loader transforms them into the new in-memory shape. |
| Snapshot store layout | Unchanged. |

---

## Sub-phases (review-revised)

- **Hart.0** — Survey & lock. **Completed 2026-04-30**. Workspace-wide
  grep over `**/*.rs` produced the inventory below. Allow-list locked
  into the BENCH ratchet test in Hart.1. Season serde shape verified
  bare-number — Hart.1's `#[serde(transparent)]` is a no-op stamp.
  See "Hart.0 inventory" section below for the full migration checklist.
  ~0.75 day → **done in 0.25 day** (faster than estimated; the survey
  was a single batched grep run).

- **Hart.1** — New types in icelines-core: PlayerIdentity (no
  `is_goalie`, no per-season fields), SeasonStats (with `position` +
  `sweater_number`), TeamStint + GoalieStintStats, StatTotals,
  RealtimeStats / AdvancedStats (`pub(crate)`), GoalieSeasonStats,
  Projection, identity merge-with-sanity-floors impl. Plus
  `fixtures::identity()` / `fixtures::stats()` builders. Comprehensive
  serde + merge-policy + reissue-detection tests. ~1.5 days
  (was 1 — added merge policy + fixture builders + sanity-floor tests).

- **Hart.2** — StatsRepository + PlayerView (with `team_display`,
  `was_traded_in_window`, Option-at-leaf accessors for hits/blocks/xG)
  + LRU + `repo_swap` + roster_last_stint / roster_all_stints indexes
  + typed career iterators (career_regular / career_playoff /
  career_all). `!Sync` static_assertion. Tested in isolation against
  synthetic fixtures. ~1.25 days (was 1 — added LRU, atomic swap,
  typed career, dual roster indexes).

- **Hart.3** — Loader: `load_into_repo(season, type, store)` returning
  `LoadOutcome { repo, missing, missing_files, fetched_at }`. Bios +
  stats + realtime + moneypuck + contracts merged, with partial-fetch
  surfaced via `MissingSource`. Single-fetch / two-materializers during
  parallel-run window. `_meta.json` extension for
  `bundle_schema_version` + `repository_version`. Both repository
  shapes (old + new) populated; equivalence tests assert no data loss.
  ~1.5 days.

- **Hart.4** — Consumer migration with deprecated-warning budget gate.
  Each commit decrements the budget; CI fails if it doesn't. Joining
  screens migrate last within their cluster (EDGE).
  - 4a: `query` / `players` / `rank` (no joins; narrow)
  - 4b: `analysis` (class / peers / compare — pairwise; simple)
  - 4c: `project` / `scouting` (single-player, deep accessor use)
  - 4d: `fantasy` (load_pools → Vec<PlayerView>; score_team typed)
  - 4e: TUI Home / Team / Player card (per-stint sub-block lands here;
    cross-surface name-parity test runs at end of commit)
  - 4f: TUI Depth chart (team_roster default + missed-playoffs hatch
    treatment carries forward from Phase S)
  - 4g: TUI Goalies (parallel-Goalie collapse — biggest deletion;
    `s` / `m` keystroke contract preserved per GLASS)
  - 4h: TUI Stats (Queries / Projections / Search)
  - 4i: TUI Transactions player-card link (cross-surface name parity
    test reruns)
  - 4j: icelines-site (mkdocs builder) **+ delete `flat_view_legacy`
    shim** (EDGE: Hart.5 stays pure-cleanup; deletion happens here at
    last-consumer migration). Budget = 0 at end of 4j.
  Each step ~0.5 day; total ~5 days.

- **Hart.5** — Delete `Player` flat struct, `Goalie` type,
  `PlayerRepository`, `GoalieRepository`. The shim was already
  deleted in 4j; Hart.5 is purely the dead-type sweep.
  Compile-clean; the `#![deny(deprecated)]` sentinel test in
  `icelines-cli/tests/no_deprecated_in_migrated.rs` covers every
  consumer. ~0.5 day.

- **Hart.6** — Capture playoff data via probe + `--capture-playoff-stats`,
  bundle alongside regular for the 5 supported seasons. Title-bar
  `[PLAYOFF]` reverse-video marker + `P` / `R` keystrokes + four-state
  empty handling on skaters AND goalies (GLASS extension), inverse-
  color flash on `y` time-travel, saved-query type pin (carrying
  forward Phase S v0.3 GLASS+EDGE asks). ~1.5 days.

**Total: ~12 days.** Was estimated 8-12 in conversation; reviews
push us to the upper end. The additions (merge policy + fixture
builders + LRU + repo_swap + typed career + LoadOutcome + budget gate)
all reduce future bug surface materially.

---

## Tests (review-revised)

### L0 (icelines-core)
- Identity / SeasonStats / TeamStint / StatTotals / GoalieSeasonStats /
  GoalieStintStats serde round-trip — every field, including all
  Optionals.
- **`identity_merge_sanity_floors`** (TAPE proptest): prior-good
  identity + incoming identity with `weight_lbs ∈ {0, 50, 99, 400}`
  or out-of-range height → keeps prior values. Birth-date / draft /
  rookie_season immutable: change attempt → keep prior.
- **`identity_merge_likely_id_reissue`** (EDGE): incoming with
  different `rookie_season` than prior → returns `LoadError::LikelyIdReissue`,
  does NOT silently overwrite.
- **TeamStint chronological order invariant**: `upsert_stats` sorts
  stints by `started`, stable tie-break by team abbrev. Proptest:
  shuffled input → stable output. Re-load round-trip preserves order.
- **TeamStint sum-equals invariant** with policy on mismatch (TAPE):
  proptest over arbitrary stint vecs verifies sum-equals; an L1 test
  with intentionally-mismatched fixture verifies loader trusts API
  totals + clamps last stint + emits warn.
- PlayerView accessors return the same value as direct field reads
  (now via repo lookup, not flat field).
- `Projection::Per82` ≠ `Projection::PerGame` compile test.
- `GoalieSeasonStats::qualified_for(SeasonType, gp)` — 15 RS / 4 PO.
- `PaceScore::projected_for(Playoff)` divide-by-zero proptest carries
  forward.
- **CI guard test (walkdir + regex)**: bans `Player {` literal
  construction outside fixture builders and `model.rs`. Allow-list
  surveyed in Hart.0.
- **`fixtures::identity()` / `fixtures::stats()` builders exist and
  compile-test** (BENCH/EDGE): every old `Player { … }` in tests gets
  replaced; ratchet test counts `Player {` literals workspace-wide,
  fails if non-decreasing per Hart.4 commit.
- **`StatsRepository: !Sync`** (EDGE): static_assertion that the type
  cannot cross thread boundaries via shared reference.
- **LRU eviction** (EDGE): with `lru_cap=2`, loading 3 (Season, Type)
  windows evicts the least-recently-used. Identities never evict.
- **`career_regular` typing** (TAPE): proptest over a repo with mixed
  Regular and Playoff rows for one player → `career_regular` returns
  ONLY Regular; `career_playoff` returns ONLY Playoff; `career_all`
  is the union. No mixing.
- **`career_*` Some-vs-None contract** (EDGE): unknown `PlayerId` →
  None for all three; known player with no stats → `Some(empty
  iterator)` for the matching method.

### L1 (icelines-fetch)
- StatsRepository populated from a captured fixture has expected
  counts (identities, stats, roster indexes both flavors).
- **Identity-without-stats / stats-without-identity invariant** (EDGE):
  upsert_stats for a player_id without an identity row returns
  `LoadError::StatsWithoutIdentity`. Loader test asserts the load
  order: identities first, stats second.
- Mid-season-trade fixture (Tarasenko 2022-23): player has two
  TeamStints in Regular row, one in Playoff row. Totals.goals across
  stints = totals.goals on each row.
- **Mid-playoff goalie trade**: traded goalie row carries per-stint
  GoalieStintStats; sum-equals to GoalieSeasonStats season-aggregate
  GS/W/L.
- Playoff fetch (Hart.6) populates `season_type=Playoff` rows and
  per-(season, type) roster indexes.
- Fetcher returns `TypedStats { season_type, rows }` (carryover).
- Pre-playoff empty / closed-season SuspiciousEmpty paths from Phase S
  carry forward.
- **`LoadOutcome` partial-fetch** (WIRE): mock loader where MoneyPuck
  fetch fails → `LoadOutcome.missing` contains `MoneyPuck { reason }`,
  repo populated with `advanced=None` for all rows. NOT a hard error.
- **Single-fetch / two-materializers** (WIRE): during the parallel-run
  window in Hart.3, an L1 test asserts only ONE HTTP call per player
  even though both repositories hydrate from it.
- **`bundle_schema_version` forward-compat** (WIRE): loader against a
  bundle stamped with a higher version than `MAX_KNOWN_VERSION` →
  `LoadError::BundleSchemaUnknown`. Lower version → migrator runs.
- **`team_roster` vs `team_roster_all_stints`** (TAPE+GLASS): mid-season-
  traded player appears in `all_stints` for both teams; appears in
  default `team_roster` only for last-stint team.
- **Atomic `repo_swap`** (EDGE): swap with a fresh repo invalidates
  borrows; compile-test ensures no `&mut` access while a `&` borrow
  is live.
- **Parallel-run equivalence** (Hart.3–4j): every L1 test asserts the
  old PlayerRepository.load_all().len() equals StatsRepository.
  league(season, Regular).count() for the same season. Catches any
  silent data loss during the parallel window.

### L2 (icelines-cli)
- Every existing L2 system test continues passing — CSV output
  byte-identical or sort-stable equivalent.
- New: `icelines query leaders --type playoff --season 20232024 --top 5
  --csv` → count + 2 anchor names asserted (carries forward Phase S).
- New: `icelines query goalies --type playoff --season 20232024` →
  goalies appear via `qualified_for(Playoff, gp)`.
- **Deprecated-warning budget gate**: each Hart.4* commit's CI run
  asserts `cargo build` deprecated-warning count ≤ committed budget.
  Hart.4j ends with budget = 0.
- **`#![deny(deprecated)]` sentinel** in
  `icelines-cli/tests/no_deprecated_in_migrated.rs`: imports every
  already-migrated screen module. Compile failure if a regression
  re-introduces legacy shim use.

### TUI L1
- Every existing TUI snapshot test (Goalies glyph contract,
  Transactions glyph contract) continues passing.
- New: League → Team → Player drill-down works against
  StatsRepository.
- New: Goalies tab loads from `repo.goalies(season, type)` — no
  parallel app.goalies state.
- New: time-travel to a bundled season switches active (season, type)
  in repo via `repo_swap`; **title-bar season segment flashes inverse
  on the same frame** (GLASS). For non-bundled seasons,
  `ensure_loaded` blocks with a status-line spinner.
- **Player card per-stint sub-block** (GLASS): traded player's card
  shows totals + per-stint sub-block; single-team player card has zero
  new chrome (snapshot test compares to today's render).
- **`team_display()` em-dash on empty** (GLASS): synthetic fixture
  with empty team_stints (impossible per invariant but defensive) →
  rendered string contains `—`.
- **Goalies tab `m` cycle three states** (GLASS): cycles through
  `Default(qualified_for(active_type))` / `All` / `Custom`. Default
  is type-aware: in Playoff mode min-GP threshold flips to 4.
- **Skater four-state empty contract** (GLASS): table-driven test
  through all four states for a skater PlayerView; rendered text
  matches per-state copy.

### BENCH-mandated test refinements (v0.3)

| Test | Phase | Refinement |
|---|---|---|
| `parallel_run_field_parity` | Hart.3 L1 | Tuple equality on `(team_str, gp, points, plus_minus)` for every player_id, NOT just `count()`. Catches stint-aggregation bugs that preserve cardinality. |
| `budget_file_tightness` | Hart.4 CI | Fails on `actual != budget` (equality, not ≤). Forces the budget file to update in the same PR as the migration. |
| `hart0_survey_completeness` | Hart.0 L0 | Walkdir re-runs the four greps from Hart.0 over the workspace; compares result set to the checked-in allow-list. Future commits can't add a `season_goals` read without updating either survey or allow-list. |
| `player_literal_ratchet_with_path_allowlist` | Hart.0 onward | Path-partitioned: `model.rs` and `*/fixtures.rs` exempt; everywhere else monotonic-decrease. New test files using `fixtures::stats()` don't move the count. |
| `cross_surface_team_string_parity` | Hart.4e + Hart.4i TUI L1 | Cell-extraction (`render_to_cells()` test helper) — assert team-column cell matches between player card and transactions row for the same traded player. NO full-screen char-by-char diff. |
| `mid_playoff_goalie_trade_synthetic` | Hart.1 L0 + Hart.3 L1 | Synthetic fixture (no real bundled goalie playoff trade exists in the 5 seasons). Two TeamStints with `goalie: Some(GoalieStintStats)`; sum-equals against GoalieSeasonStats. |
| `loadoutcome_partial_fetch_<variant>` | Hart.3 L1 | One parameterized case per `MissingSource` variant: realtime / contracts / goalie_stats / moneypuck. Each asserts the variant + that the corresponding accessor returns None. |
| `teamstint_ordering_none_started_tiebreak` | Hart.1 L0 proptest | Both stints with `started: None` → output sorted lexicographically by team. Round-trip stable. |
| `lru_evicted_window_reloads_correctly` | Hart.2 L0 | cap=2; load A, B, C (evicts A); load A again; assert `repo.season(_, A.0, A.1).is_some()` AND B is now LRU. Catches "cached as evicted" bug. |
| `no_deprecated_in_migrated.generated.rs` | Hart.4* (auto-built) | Generated from `migrated.txt` via macro expansion, NOT a `build.rs` script. Each Hart.4* commit appends one line to migrated.txt. |
| `repo_swap_invalidates_borrows` | Hart.2 doctest | `compile_fail` doctest holding a `&PlayerView` while calling `repo_swap`. Documents the contract next to the API; cargo test runs doctests. |
| `merge_policy_proptest` | Hart.1 L0 | Generator-based ranges (proptest strategies for valid weights/heights), NOT static fixtures. Out-of-range constants merge → keep prior. |

---

## Risks

1. **80+ call sites is a lot.** Mitigation: parallel-run + per-surface
   commits with tests. The deprecated-warning count is the gate.
2. **Performance regression on iterator hot paths.** The flat
   `Vec<Player>` allows zero-cost iteration; PlayerView is a thin
   borrowed projection but its accessors are method calls. Mitigation:
   benchmark Hart.4 milestones; if any hot path regresses >2x, inline
   the field read pattern at that site (PlayerView is open-fielded).
3. **Allocation growth.** Per-stint Vec, per-row PlayerView projection.
   At 10K rows, this is well under memory budget (~few MB), but worth
   measuring.
4. **Goalie collapse is surgical.** `Goalie::full_name` etc. were
   accessed from the TUI's Goalies tab as a parallel pool. After Hart,
   `repo.goalies(season, type)` returns `Vec<PlayerView>` with the same
   shape. The migration in Hart.4g is the largest single conceptual
   shift; estimate generously.
5. **PlayerView lifetime annotations.** Rust's borrow checker on
   long-lived TUI state may push back on `PlayerView<'a>` storage.
   Mitigation: TUI doesn't store views — it stores the repository and
   builds views per-render. Borrow lifetimes are scoped to one render
   pass.
6. **Reviewer churn during a 10-day phase.** Mitigation: the spec gets
   reviewed once (now); each Hart.4* sub-commit gets a one-screen
   FORGE/BENCH review only if it touches reliability or test coverage.

---

## Resolved questions

### Carried forward from Phase S reviews

| Q | Resolution |
|---|------------|
| `Projection` shape? | Tagged enum: `Per82` vs `PerGame`; mixing is a compile error. |
| `PlayoffPhase` location? | `icelines-core::model`. SnapshotMetaFlags imports it. |
| Pre-playoff empty? | `Ok(empty TypedStats)` for current-season-not-yet-playoffs; closed-season empty is `Err(SuspiciousEmpty)`. |
| Empty-overwrite refusal? | Refuse to overwrite a non-empty bundled `stats-playoff.json` with a fresh empty fetch unless `--allow-empty`. |
| Goalie qualified threshold? | `GoalieSeasonStats::qualified_for(SeasonType, gp)` — 15 / 4. |
| Fantasy with type=Playoff? | Hard-pin to Regular in v1; regression test asserts ignore-type-arg. |
| Saved queries break on type toggle? | Persist `season_type`; literal `"Saved as Regular — re-save?"` warning. |
| L2 fixture rot? | Lock against completed bundled seasons; count + 2 corrections-immune anchor names. |
| Cross-type `compare`? | Both args share a type; mixing impossible. |
| CI guard? | walkdir + regex (no shell grep); allow-list locked in Hart.0. |
| Fetcher type-tags data? | `TypedStats { season_type, rows }` envelope. |

### New from Phase Hart reviews

| Q | Resolution | Source |
|---|------------|--------|
| `is_goalie` on identity? | **Removed.** Derived per-row from `SeasonStats.goalie.is_some()` to handle emergency-backup goalies (Ayres, Foster). | TAPE |
| `position` and `sweater_number` location? | **Per `SeasonStats`**, not identity. Marchand C→LW, Boeser RW→LW, hybrid emergency goalies all round-trip correctly. | EDGE+TAPE |
| Identity bio-field merge? | Most-recent-non-null with sanity floors (weight ∈ [100, 350], height ∈ [60, 84], birth-date / draft / rookie-season immutable). | TAPE |
| PlayerId reissue detection? | `LikelyIdReissue` error when incoming rookie_season ≠ persisted. | EDGE |
| TeamStint ordering? | Chronological by `started`, stable tie-break by team abbrev. Sorted in `upsert_stats`; round-trip-stable. | TAPE |
| Stint-totals mismatch policy? | Loader trusts API totals (matches NHL.com); recomputes stints to match by clamping last; emits `tracing::warn`. | TAPE |
| `team_roster` semantics? | Default = last-stint team only ("current home"). New `team_roster_all_stints` for trade-history view. | GLASS+TAPE |
| `view.team()` Option burden on render? | `view.team_display()` returns `&str` with em-dash fallback; logic still uses Option-returning `team()`. | GLASS |
| Mid-season-trade Player card? | Totals block always; per-stint sub-block when `was_traded_in_window()` is true. Single-team players see no chrome change. | GLASS |
| Career history typed iter? | `career_regular` / `career_playoff` / `career_all` — separate methods; mixing impossible at the iterator level. Returns `Option<impl Iterator>` (None=unknown player, Some(empty)=known but no stats). | TAPE+EDGE |
| Two URL fields? | `PlayerIdentity.headshot_canonical_url` is season-agnostic CDN URL (disk cache key). Per-row season+team URL computed at render time. | TAPE |
| Loader return type? | `LoadOutcome { repo, missing, missing_files, fetched_at }` — partial fetch surfaces specific MissingSource variants. | WIRE |
| Realtime / advanced API ergonomics? | `pub(crate)` fields on SeasonStats. Reads through `view.hits()` / `view.cf_pct()` accessors that return `Option` at the leaf. | WIRE |
| Schema versioning of bundles? | `bundle_schema_version` + `repository_version` in `_meta.json`. Loader errors on unknown-too-high; runs migrator on too-low. | WIRE |
| Time-travel reload policy? | Eager-load all bundled seasons at startup. Off-bundle seasons (live `data install`) trigger `ensure_loaded()`, blocking ~50ms with status-line spinner. | WIRE |
| Title-bar visual cue on `y`? | One-frame inverse-color flash on the season segment of the title bar. | GLASS |
| LRU eviction? | (Season, SeasonType) windows; default cap 8; identities never evict. | EDGE |
| Atomic repo refresh? | `StatsRepository::repo_swap(new_repo)` — never in-place mutation while a render frame holds a borrow. | EDGE |
| Concurrent upsert safety? | `StatsRepository: !Sync` by construction (PhantomData<Cell<()>>); compile-time guarantee. | EDGE |
| `flat_view_legacy` deletion? | Hart.4j (last consumer migration), NOT Hart.5. Hart.5 is pure-cleanup. | EDGE |
| Identity-without-stats / vice versa? | `upsert_stats` errors with `StatsWithoutIdentity` if no identity; loader contract: identities first, stats second. | EDGE |
| Goalies traded mid-season? | `TeamStint.goalie: Option<GoalieStintStats>` carries per-stint GS/W/L; sum-equals against season aggregate. | EDGE |
| Deprecated-warning gate is real? | `.deprecated_budget` file at repo root + monotonically-decreasing CI check + `#![deny(deprecated)]` sentinel test importing every migrated module. | WIRE |
| Mid-playoff-trade worked example? | Tarasenko 2022-23 in spec: Regular row stints [STL, NYR], Playoff row stint [NYR]. Two SeasonStats rows. | TAPE |
| Hart.4 commit ordering? | Joining screens (player card, transactions link) migrate last; cross-surface name-parity test runs between commits. | EDGE |
| Iterator allocation in render? | Career history renderer uses `take(visible_rows)`; never `collect::<Vec<_>>()`. | GLASS |
| Skater four-state empty? | Same four-state contract from Phase S (NotStarted / DidNotAppear / 0-GP-impossible / GP>0-with-0-points) applies to skater stat blocks too. | GLASS |
| Goalies tab `m`-cycle? | Three-state: Default (= `qualified_for(active_type)`), All, Custom. Type-aware default. | GLASS |

### New from Phase Hart plan reviews

| Q | Resolution | Source |
|---|------------|--------|
| Iterator lifetime annotation? | Explicit `'a` on every method returning a borrowing iterator: `pub fn league<'a>(&'a self, ...) -> impl Iterator<Item = PlayerView<'a>> + 'a`. Portable across editions. | FORGE |
| `pub(crate)` cross-crate? | Doesn't work — loader is in `icelines-fetch`, fields in `icelines-core`. Solution: keep fields `pub`, add `SeasonStatsBuilder` as the recommended construction path. CI guard catches direct-field reads. | FORGE |
| `LoadOutcome` location? | `icelines-fetch::stats_loader`. Cross-crate dep (fetch → core for StatsRepository) is already established and DAG-safe. | FORGE |
| `merge_with` ownership? | Take incoming by value (loader owns it; saves String clones). | FORGE |
| `#[serde(default)]` on Optionals? | Yes — every Option<...> field added in Hart.1. Bump bundle_schema_version only on non-Option additions. | FORGE |
| `repo_swap` return type? | Returns the old repo via `mem::replace` — callers can drop, inspect, or roll back. | FORGE |
| `ensure_loaded` sync vs async? | Both: sync for the TUI render path (no .await), async wrapper using `spawn_blocking` for the CLI / loader. | FORGE |
| StatsRepository thread safety? | `!Sync + !Send` via `PhantomData<Cell<()>>` — the cell type negates both. Background loader path must wrap in `Arc<RwLock<_>>` at the call site. | FORGE |
| On-disk migration on Hart.5? | None needed. Bundled JSON files are source of truth; flat `Player` struct only existed in-memory. Loader transforms unchanged JSON into the new shape. | FORGE |
| `LoadError` shape? | I/O + Parse wrapped under `Bundle { source: BundleError }` for clean public surface. `#[from]` for `?` ergonomics. | FORGE |
| `Season` serde? | Add `#[serde(transparent)]` to keep JSON output as a bare number (Hart.0 verifies current behavior; locks it). | FORGE |
| `flat_view_legacy` deletion timing? | Hart.4j (last consumer migration). Hart.5 stays pure-cleanup. | EDGE+FORGE |
| Sentinel test enforcement? | `migrated.txt` at repo root; sentinel test reads it, generates `use` statements via macro expansion, with `#![deny(deprecated)]`. NOT a build.rs walking source paths. | FORGE+BENCH |
| Budget gate semantics? | Equality (`actual == budget`), not ≤. Forces the budget file to update in the same PR as the fix. | BENCH |
| Parallel-run validation strength? | Tuple equality on `(team_str, gp, points, plus_minus)` per player_id, not `count()`. | BENCH |
| Survey completeness? | `hart0_survey_completeness` test re-runs the greps and compares to checked-in allow-list. Catches missed call sites. | BENCH |
| Literal ratchet path partition? | `model.rs` and `*/fixtures.rs` exempt; everywhere else monotonic-decrease. | BENCH |
| Cross-surface parity test mechanism? | Cell extraction via `render_to_cells()` helper, NOT char-by-char screen diff. | BENCH |
| MissingSource variant coverage? | One parameterized test per variant (realtime / contracts / goalie_stats / moneypuck). | BENCH |
| `repo_swap` borrow-discipline test? | `compile_fail` doctest on the method, runs via `cargo test` doctest path. | BENCH |
| Merge policy test inputs? | Proptest generators (in-range `weight_lbs`, `height_in_inches`, valid ISO date), not static fixtures. | BENCH |

---

## Memory hooks

After Hart closes:
- Update `season_type_plan.md` to "subsumed by Phase Hart, shipped in v0.13.0."
- Add `normalization_model.md` capturing the (player_id, season, type)
  primary key and the PlayerView render shape for future-me.
- Update `design/phases.md` to mark Hart shipped.

---

## Hart.0 inventory (completed 2026-04-30)

Captured workspace-wide via `Grep` over `**/*.rs`. Numbers are occurrence
counts per file (a single line can hold one). The BENCH ratchet test
(`player_literal_ratchet_with_path_allowlist`) starts from these counts
and asserts monotonic-decrease per Hart.4* commit, with `model.rs` and
`*/fixtures.rs` exempt.

### `Player {` literal construction — 29 occurrences across 13 files

```
icelines-core/src/model.rs                        4    ← allow-list (struct definition + Display tests)
icelines-core/src/cross_team.rs                   2    ← test fixtures
icelines-core/src/depth_chart.rs                  2    ← test fixtures
icelines-core/src/filter.rs                       3    ← test fixtures
icelines-fetch/src/aggregate.rs                   1    ← migrate (Hart.3 loader)
icelines-fetch/src/player_builder.rs              2    ← deleted in Hart.5 (loader replaced)
icelines-fetch/tests/integration_phase2.rs        4    ← test fixtures (Hart.4 fixture builder migration)
icelines-cli/src/cli.rs                           1    ← migrate
icelines-cli/src/main.rs                          1    ← migrate
icelines-cli/src/commands/export.rs               2    ← migrate (Hart.4j)
icelines-cli/src/commands/query.rs                1    ← migrate (Hart.4a)
icelines-cli/src/commands/scouting.rs             3    ← migrate (Hart.4c)
icelines-cli/src/tui/dashboard_panel.rs           3    ← migrate + tests (Hart.4e)
```

### `Goalie {` literal construction — 11 occurrences across 5 files

```
icelines-core/src/model.rs                        2    ← allow-list (struct definition)
icelines-fetch/src/goalie_repository.rs           4    ← deleted in Hart.5 (whole module goes)
icelines-cli/src/tui/screens/goalies.rs           2    ← migrate (Hart.4g — biggest collapse)
icelines-cli/src/tui/screens/team.rs              2    ← migrate (Hart.4e — Team screen goalie strip)
icelines-cli/src/tui/dashboard_panel.rs           1    ← migrate (Hart.4e)
```

### `PlayerRepository` / `GoalieRepository` — 30 references across 9 files

```
icelines-fetch/src/repository.rs                  6    ← deleted in Hart.5 (whole module)
icelines-fetch/src/goalie_repository.rs           6    ← deleted in Hart.5 (whole module)
icelines-fetch/src/lib.rs                         1    ← re-export, removed in Hart.5
icelines-cli/src/commands/players.rs              6    ← migrate (Hart.4a)
icelines-cli/src/commands/query.rs                2    ← migrate (Hart.4a)
icelines-cli/src/commands/rank.rs                 2    ← migrate (Hart.4a)
icelines-cli/src/commands/team.rs                 2    ← migrate (Hart.4e)
icelines-cli/src/tui/app.rs                       2    ← migrate (Hart.4e)
icelines-site/src/builder.rs                      3    ← migrate (Hart.4j)
```

### `season_goals` / `season_assists` / `season_points` / `pace_score` direct reads — 187 occurrences across 30 files

These are the field reads that get banned outside the allow-list. The
volume is larger than the `Player {` count because each consumer reads
multiple fields per touch.

**Allow-list** (direct reads permitted post-Hart.5):
```
icelines-core/src/model.rs                  ← struct definition + accessor methods
icelines-core/src/scoring.rs                ← compute_pace_score on raw Player
                                              (replaced with SeasonStats in Hart.1;
                                               file moves but stays on the allow-list)
icelines-fetch/src/player_builder.rs        ← deleted in Hart.5 (loader rewrite)
icelines-fetch/src/aggregate.rs             ← deleted in Hart.5 (loader rewrite)
icelines-fetch/src/repository.rs            ← deleted in Hart.5
icelines-fetch/src/goalie_repository.rs     ← deleted in Hart.5
icelines-fetch/tests/mock_nhl_api.rs        ← test fixture builders
icelines-fetch/tests/integration_phase2.rs  ← test fixture builders
```

**Migrate in Hart.4** (everything else):
```
Hart.4a — narrow CLI:
  icelines-cli/src/commands/query.rs              35
  icelines-cli/src/commands/players.rs             1
  icelines-cli/src/commands/rank.rs                3

Hart.4b — analysis pair commands:
  icelines-cli/src/commands/analysis.rs            5

Hart.4c — single-player deep:
  icelines-cli/src/commands/project.rs             3
  icelines-cli/src/commands/scouting.rs           26    ← largest single file

Hart.4d — fantasy:
  icelines-cli/src/commands/fantasy.rs             3

Hart.4e — TUI Home / Team / Player card:
  icelines-cli/src/tui/app.rs                      7
  icelines-cli/src/tui/screens/player.rs           6
  icelines-cli/src/tui/screens/team.rs             2
  icelines-cli/src/tui/screens/comps.rs           10
  icelines-cli/src/tui/screens/misc.rs             7
  icelines-cli/src/tui/dashboard_panel.rs          5
  icelines-cli/src/tui/widgets/mod.rs              1
  icelines-cli/src/tui/screens/search.rs           1
  icelines-cli/src/render/terminal.rs              5

Hart.4f — Depth chart:
  icelines-cli/src/tui/screens/depth.rs            1

Hart.4g — Goalies (biggest collapse — parallel-Goalie deletion):
  (no season_* reads — Goalie has its own field set; covered by
   the `Goalie {` literal migration above)

Hart.4h — Stats:
  icelines-cli/src/tui/screens/queries.rs         13

Hart.4i — Transactions player-card link:
  (no season_* direct reads — already uses player names; just needs
   the link type swap)

Hart.4j — site + tonight + export:
  icelines-cli/src/commands/tonight.rs             3
  icelines-cli/src/commands/export.rs             26
  icelines-site/src/builder.rs                     1
  icelines-site/src/html.rs                        2

icelines-core (touched in Hart.1, mechanical):
  icelines-core/src/cross_team.rs                  5
  icelines-core/src/filter.rs                      3
```

### Season serde shape — verified bare number

`l0_hart0_season_serde_emits_bare_number` test asserts current behavior:
`serde_json::to_string(&Season(20252026)) == "20252026"`. Hart.1's
`#[serde(transparent)]` stamp is a no-op that locks this in the type
system.

### Allow-list summary (locked into BENCH ratchet test)

Direct field reads of `season_goals` / `season_assists` /
`season_points` / `pace_score` are permitted ONLY in:
- `icelines-core/src/model.rs` (definitions + accessors)
- `icelines-core/src/scoring.rs` (the pace-computation function)
- `icelines-fetch/tests/**/*.rs` (test fixture builders)
- Files marked for deletion in Hart.5 (transitional during the migration
  window; gone after Hart.5)

Everything else routes through `PlayerView::goals()` /
`PlayerView::points()` / `PlayerView::pace_score()` etc.
