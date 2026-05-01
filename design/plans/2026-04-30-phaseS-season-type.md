# Phase S — Season Type (Regular / Playoff)

**Status**: Draft v0.3 — incorporates WIRE/TAPE/EDGE/GLASS spec reviews
and BENCH/FORGE plan reviews
**Date**: 2026-04-30
**Spec**: this document is both spec and plan (scope is small enough)
**Target**: v0.12.0

---

## Goal

Stat-bearing surfaces (League / Depth / Stats / Goalies) gain a
**season type** scope so the user can toggle between regular-season
totals and playoff totals. Scores / Schedule / Transactions / Playoffs
are unaffected — they already handle game types per row.

User mental model:
- `y` (existing) = which **season year** (2021-22, 22-23, …, 25-26)
- `P` / `R` (new) = which **stat basis** (regular, playoff)

These are orthogonal axes. `y=23-24, P` shows you the 2024
Stanley Cup numbers; `y=25-26, R` is the default daily view.

Preseason is **deferred** — low signal, large in raw rows, rarely
queried. Easy to add later behind the same toggle.

---

## Affected surfaces

| Surface | Today | After Phase S |
|---|---|---|
| **League** (Home / Team / Player card) | regular-season totals | typed |
| **Depth** | regular pace_score → team strength | typed (with "missed playoffs" treatment) |
| **Stats** — Queries | filters on regular-season fields | typed (filters apply within type; saved-queries pin type) |
| **Stats** — Projections | regular pts/82 leaderboard | typed; playoff uses per-game absolute, not pts/82 |
| **Stats** — Search | regular | typed |
| **Goalies** | regular SV%/GAA, qualified=15 GP | typed; qualified threshold 4 GP for playoff |
| Scores | per-row game type | unchanged |
| Schedule | mixed | unchanged |
| Transactions | free-form prose | unchanged |
| Playoffs | game log + bracket | unchanged (already playoff-scoped) |
| Fantasy | regular weights | **hard-pinned to Regular in v1** (EDGE) — toggle does not affect fantasy scoring |
| `query` / `players` / `rank` | regular | gains `--type {regular\|playoff}` |
| `history` | aggregates whatever's loaded | **filters to Regular explicitly** (TAPE) |

---

## Data model (TAPE+FORGE revised)

### Approach: typed accessor on Player, not unprefixed fields

The breaking-change option (`Player.stats: HashMap<SeasonType, _>`)
blasts every fantasy/scoring/fitness consumer. The non-breaking option
(unprefixed `season_*` fields silently mean "regular") is a TAPE
foot-gun — every code path that forgets to branch shows regular numbers
when the user has toggled to Playoff.

**Compromise**: keep the existing fields literally meaning regular-season,
add `playoff_stats: Option<PlayoffStats>`, and **add a typed accessor**
that ALL consumers go through. Direct reads of `season_goals` etc.
outside an explicit allow-list are caught by a CI guard test.

### `ActiveStatsRef` — the typed accessor (FORGE: was undefined)

```rust
// icelines-core/src/model.rs

/// Borrowed view over either regular-season fields or PlayoffStats.
/// Methods abstract the species so consumers can write generic code
/// without dispatching on type for every read.
pub enum ActiveStatsRef<'a> {
    Regular(&'a Player),
    Playoff(&'a PlayoffStats),
}

impl ActiveStatsRef<'_> {
    pub fn goals(&self)        -> u32 { /* match … */ }
    pub fn assists(&self)      -> u32 { /* match … */ }
    pub fn points(&self)       -> u32 { /* match … */ }
    pub fn gp(&self)           -> Option<u32> { /* … */ }
    pub fn pace_score(&self)   -> Option<&PaceScore> { /* … */ }
    pub fn plus_minus(&self)   -> i32 { /* … */ }
    pub fn pim(&self)          -> u32 { /* … */ }
    pub fn shots(&self)        -> u32 { /* … */ }
    pub fn shooting_pct(&self) -> Option<f32> { /* … */ }
    pub fn season_type(&self)  -> SeasonType { /* match … */ }
}

impl Player {
    /// Regular always succeeds; Playoff returns None when no playoff data.
    pub fn active_stats(&self, t: SeasonType) -> Option<ActiveStatsRef<'_>> {
        match t {
            SeasonType::Regular => Some(ActiveStatsRef::Regular(self)),
            SeasonType::Playoff => self.playoff_stats.as_ref().map(ActiveStatsRef::Playoff),
        }
    }

    /// Future-compatible view (FORGE): exposes the same accessor surface
    /// as `ActiveStatsRef` so consumers route through one shape today.
    /// When we eventually break the schema and introduce `RegularStats`,
    /// this method's body changes but every caller stays identical.
    pub fn regular_stats(&self) -> ActiveStatsRef<'_> {
        ActiveStatsRef::Regular(self)
    }
}
```

`Goalie::active_stats(SeasonType)` parallel — same enum extended with
`Goalie(&Goalie)` and `GoaliePlayoff(&GoaliePlayoffStats)` arms (or a
parallel `ActiveGoalieStatsRef` — decide in S.1; both work).

### Player + Goalie field additions

```rust
// icelines-core/src/model.rs
pub struct Player {
    // ... existing regular-season fields preserved verbatim ...
    pub season_goals:   u32,
    pub season_assists: u32,
    pub season_points:  u32,
    pub pace_score:     Option<PaceScore>,
    // ...

    /// Playoff-tier stats. None when not loaded OR player had zero playoff GP.
    pub playoff_stats:        Option<PlayoffStats>,
    /// Comma-separated abbrevs when traded mid-playoffs. Optional shape
    /// matches the new Goalie::team_abbrevs Optional (FORGE: cleans the
    /// latent bug where empty != "didn't play").
    pub playoff_team_abbrevs: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayoffStats {
    pub gp:           u32,
    pub goals:        u32,
    pub assists:      u32,
    pub points:       u32,
    /// Per-game pace — see `Projection` below. Playoff samples are too
    /// small for /82 projections.
    pub pace_score:   Option<PaceScore>,
    pub plus_minus:   i32,
    pub pim:          u32,
    pub shots:        u32,
    pub shooting_pct: Option<f32>,
}
```

**Goalie schema fix (FORGE)**: `Goalie::team_abbrevs` becomes
`Option<String>` to match `playoff_team_abbrevs` and to disambiguate
"empty string" from "didn't play." Existing call sites updated in S.1
(mechanical). New `Goalie::playoff_team_abbrevs: Option<String>` and
`Goalie::playoff_stats: Option<GoaliePlayoffStats>`.

### `SeasonType` enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeasonType {
    Regular,  // gameTypeId = 2
    Playoff,  // gameTypeId = 3
    // Preseason deferred.
}

impl SeasonType {
    pub fn label(self)       -> &'static str { /* "regular" / "playoff" */ }
    pub fn label_short(self) -> &'static str { /* "RS" / "PO" */ }
    pub fn game_type_id(self) -> u32 { /* 2 / 3 */ }
}
```

**Note (FORGE)**: `min_goalie_gp` does NOT live on `SeasonType` — it's
goalie-specific data, lives on `Goalie::min_qualified_gp(season_type)`:

```rust
impl Goalie {
    pub fn min_qualified_gp(season_type: SeasonType) -> u32 {
        match season_type {
            SeasonType::Regular => 15,  // NHL Vezina-eligibility convention
            SeasonType::Playoff => 4,   // first-round losing starters qualify
        }
    }
    pub fn qualified(&self, season_type: SeasonType) -> bool {
        // Reads playoff_stats.gp if season_type == Playoff, else self.gp.
        // ...
    }
}
```

### `Projection` — tagged return type (FORGE)

`PaceScore::projected_for(SeasonType)` returns a tagged enum so
callers cannot silently mix scales (the "Hughes projects 180 pts in
playoffs" foot-gun becomes a type error):

```rust
// icelines-core/src/scoring.rs
pub enum Projection {
    /// Regular season: project 82-game total from current rate.
    Per82(f64),
    /// Playoff: per-game absolute (sample too small for /82).
    PerGame(f64),
}

impl PaceScore {
    pub fn projected_for(&self, season_type: SeasonType) -> Projection {
        match season_type {
            SeasonType::Regular => Projection::Per82(self.pace_82),
            SeasonType::Playoff => Projection::PerGame(self.points_per_game()),
        }
    }
}

impl Projection {
    pub fn label(&self) -> &'static str {
        match self { Self::Per82(_) => "/82", Self::PerGame(_) => "/g" }
    }
    /// Render as a labeled string ("48.0/82" or "0.95/g"). Always paired.
    pub fn render(&self) -> String { /* … */ }
}
```

A consumer that compares `Per82` to `PerGame` with `==` gets a type
error — structurally impossible to mix scales.

### `PlayoffPhase` lives in `icelines-core` (FORGE)

Shared between core (Player rendering / status text) and fetch (snapshot
persistence). Per the dependency-graph rule, lives at the lowest crate:

```rust
// icelines-core/src/model.rs
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayoffPhase {
    #[default]
    NotStarted,
    InProgress,
    Final,
}
```

`SnapshotMetaFlags` (Phase T struct in icelines-fetch) imports it.

---

## Fetcher (WIRE+FORGE-revised)

### `gameTypeId` parametrization, type travels with data

The fetcher returns a typed envelope so the `season_type` is carried
alongside the rows from S.2 onward — caller cannot lose track:

```rust
// icelines-fetch/src/schema.rs
pub struct TypedStats {
    pub season_type: SeasonType,
    pub rows:        Vec<SkaterStats>,
}
pub struct TypedGoalieStats {
    pub season_type: SeasonType,
    pub rows:        Vec<GoalieStats>,
}

// icelines-fetch/src/nhl_api.rs
pub fn fetch_all_stats(&self, season: &str, season_type: SeasonType)
    -> Result<TypedStats, FetchError>;
pub fn fetch_all_goalies(&self, season: &str, season_type: SeasonType)
    -> Result<TypedGoalieStats, FetchError>;
```

Existing call sites pass `SeasonType::Regular` (no behavior change at
the boundary). `gameTypeId` numbering is unofficial — comment in
`nhl_api.rs` calls out the observed-but-undocumented nature so future
drift is loud (TAPE).

### Pre-playoff semantics (NEW per WIRE)

A 200 with empty `data: []` for `gameTypeId=3`:
- **Current season** (NHL season is the active live one): legitimately
  empty pre-playoff window. Returns `Ok(vec![])` AND surfaces a
  `PlayoffsNotStarted` log marker to the caller. T.3-style stale-flag
  gets `playoff_phase: NotStarted` in the meta file.
- **Historical season** (closed): empty is suspicious — closed seasons
  always have playoff rows. Returns `Err(FetchError::SuspiciousEmpty
  { season })` so caller doesn't silently overwrite a populated bundle
  with nothing.

### Empty-overwrite refusal (WIRE)

Mirrors Transactions' `EmptyResponseRefused`. Refuse to overwrite a
non-empty bundled `stats-playoff.json` with a fresh empty fetch unless
`--allow-empty` is passed. Critical for the rate-limit-returns-200/empty
edge case.

### Schema validation (WIRE)

`PlayoffStats` ON `deny_unknown_fields`. Playoff API responses
historically include `roundId` and `seriesId` fields the regular feed
lacks — when present, must be added to the struct rather than dropped.
Comment in `schema.rs` documents this expected divergence.

---

## Storage

### Bundled layout (WIRE-revised)

Each bundled season grows from 2 stats files to 4. Each file carries a
provenance envelope (parallel to Transactions):

```
data/seasons/20232024/
├── bios.json                           ← unchanged
├── stats.json                          ← regular skaters (existing)
├── stats-playoff.json                  ← new — { source, fetched_at, season_type, row_count, rows }
├── goalie-stats.json                   ← regular goalies (existing)
├── goalie-stats-playoff.json           ← new — same envelope
└── transactions.json                   ← unchanged (Phase T)
```

`bundled.rs` adds:
```rust
pub fn get_stats(season: &str, season_type: SeasonType)
    -> Option<StatsEnvelope>;
pub fn get_goalie_stats(season: &str, season_type: SeasonType)
    -> Option<GoalieStatsEnvelope>;
pub fn load_stats_with_fallback(season: &str, season_type: SeasonType,
    store: &SnapshotStore) -> Result<StatsEnvelope, FetchError>;
```

Existing `get_stats(season)` becomes a deprecated shim defaulting to
Regular.

### `_meta.json` extension

`SnapshotMetaFlags` (Phase T) gains:
```rust
pub playoff_phase: PlayoffPhase,
pub playoff_data_through: Option<String>,  // YYYY-MM-DD; analog of transactions_fetched_at

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum PlayoffPhase {
    #[default]
    NotStarted,
    InProgress,
    Final,
}
```

TUI marker reads from this — so users know "playoff data is from
yesterday's games" rather than guessing.

### Per-type dedup (TAPE)

`(nhl_id, season_type)` is the dedup key, not just `nhl_id`. A goalie
who started regular season on EDM, traded to PIT, then traded back to
EDM mid-playoffs has:
- regular: `team_abbrevs = "EDM,PIT"`
- playoff: `playoff_team_abbrevs = "PIT,EDM"`

Documented in S.3 of sub-phases.

### Fall-through contract (WIRE+FORGE)

Errors split between fetch-time and load-time concerns (FORGE):

```rust
// icelines-fetch/src/error.rs — fetch-time only
pub enum FetchError {
    // ... existing T.0–T.3 variants ...
    SuspiciousEmpty { season: String },   // closed-season + empty data array
}

// icelines-fetch/src/bundled.rs — load-time only (new module-local enum)
pub enum LoadError {
    SeasonNotBundled    { season: String },
    MissingBundle       { season: String, season_type: SeasonType },
    Io(std::io::Error),
    Parse(serde_json::Error),
}
impl From<FetchError> for LoadError { /* I/O + Parse pass-through */ }
```

`load_stats_with_fallback(season, type)` returns:
- `Ok(envelope)` — bundled or installed data found.
- `Ok(empty_envelope_with_NotStarted)` — current season AND
  type=Playoff AND playoff_phase=NotStarted. NOT an error.
- `Err(LoadError::MissingBundle { season, season_type })` — historical
  season with no playoff bundle. Distinct from `SeasonNotBundled` (no
  bundle of EITHER type for that season) per EDGE.
- `Err(LoadError::SeasonNotBundled { season })` — neither regular nor
  playoff bundle exists; season completely unknown.
- `Err(_)` — actual I/O / parse failures.

CLI / TUI translate each variant to a distinct user-facing message;
never collapse "playoffs haven't started" with "data is missing."

---

## TUI (GLASS-revised)

### Keystrokes — direct mode (resolved per user preference)

Two screen-scoped uppercase keys:

- **`P`** → switch to Playoff stats
- **`R`** → switch to Regular stats

Idempotent: pressing `P` while already in Playoff is a no-op.

GLASS flagged `P`/`R` as muscle-memory-risky given lowercase `p` is
the Queries↔Projections toggle. Mitigation: the keys ONLY fire on
the four stat-bearing tabs (League / Depth / Stats / Goalies). On
the Stats tab, lowercase `p` still toggles Projections; uppercase
`P` switches stat basis. Footer hint on these tabs reads
`P/R: stat basis  p: queries↔projections` (Stats tab) or just
`P/R: stat basis` (other three) so the distinction is visible.

**Pre-playoff guard**: if the user presses `P` and the active season
has `playoff_phase: NotStarted`, the keystroke is REFUSED — status
line shows `"Playoffs have not begun for {season}"`. No silent empty-
mode switch (EDGE).

### Selection reset on switch (GLASS)

Toggling type resets `selected = 0` / `goalie_selected = 0` /
`tx_selected = 0` etc. on the active screen. Index 7 in regular
GAA-sorted goalies is meaningless in playoff GAA-sorted goalies.

### Mode marker — glyph + reverse-video (GLASS)

Color-as-sole-encoding fails the WCAG charter. The active-mode
marker uses BOTH a glyph AND reverse-video styling:

```
Title bar layout (Regular):
   Goalies · sort: SV%  ·  s:sort  Esc:back

Title bar layout (Playoff):
  ⛸ PLAYOFF ⛸  Goalies · sort: SV% · 23-24 · through Apr 28 · ...
  ^^^^^^^^^^^^^ — reverse-video block, yellow on default-bg
```

The skate glyph (⛸ U+26F8) is the carrier; reverse-video block ensures
deuteranope users see shape + inversion. Right edge of title bar pins
`[PO 23-24]` permanently so the announcement isn't ephemeral (GLASS:
status line is too transient).

### Title-bar fits in 80 cols (GLASS)

When in Playoff mode, screen-specific verbose hints move to the bottom
footer. Keep the title at `⛸ PLAYOFF ⛸  Goalies  [PO 23-24]` — under
40 chars, leaves headroom even at narrow terminals.

### Empty handling — three states (GLASS)

| State | Render |
|---|---|
| `playoff_stats = None` AND `playoff_phase = NotStarted` | "Playoffs have not begun for {season}" — banner-card, not 32 empty rows |
| `playoff_stats = None` AND season is Final | "Did not appear in playoffs" |
| `playoff_stats = Some(stats)` with `gp = 0` | impossible (we set None instead) |
| `playoff_stats = Some(stats)` with `gp > 0` and `points = 0` | render normal stat block; **0/0/0 is real data**, do not hide it |

### Depth chart — "missed playoffs" treatment (GLASS+EDGE)

Today: Depth shows team strength bars. With type=Playoff, a non-
qualifier renders zero bar = visually identical to "bad team."

After Phase S: a team with `playoff_stats = None` for ≥80% of its
roster renders the bar as **hatched/dimmed** with literal text
`Missed playoffs (RS: {pts} pts, {seed}th)` overlaid. Clear visual
distinction from low-pace-but-qualified teams.

### Saved queries pin season type (EDGE)

`SavedQuery` record gains `season_type: SeasonType`. When loading a
saved query, the TUI restores that type along with the field state.
If user changes type after loading, status line warns
`"Saved as Regular — re-save?"` to prevent confusion when threshold
filters (GP > 40) silently produce zero playoff results.

---

## CLI

Every report command that scopes by season gains
`--type {regular|playoff}`, default `regular`:

```
icelines query leaders --type playoff --top 20
icelines query goalies --type playoff
icelines query player McDavid --type playoff
icelines players --type playoff --pos C
icelines rank --type playoff
icelines x leaders --type playoff
```

`compare` rejects mixed types (EDGE):
```
icelines compare McDavid Bedard --type playoff   # ok
icelines compare McDavid Bedard                  # implicit regular, ok
# No way to mix — both args always share the same type.
```

`fetch all` orchestration (WIRE):
- Runs Regular unconditionally.
- Attempts Playoff with failures **demoted to warnings** (not aborts) —
  playoff fetch failure mid-season must not poison the regular run.
- New `--types regular,playoff` flag for explicit control.
- Default behavior is `regular,playoff` so daily fetch keeps both fresh.

`history` aggregates Regular-only by default (TAPE — silent type
mixing is a foul). `--type playoff` available but uncommon.

`--top N` truncation (EDGE): `query leaders --top 100 --type playoff`
in early playoffs may return < 100 rows. Output never pads; warns to
stderr when truncated below requested top.

---

## Tests (BENCH-revised)

### L0
- `SeasonType` enum: parse, label, label_short, game_type_id.
- `Goalie::min_qualified_gp(season_type)` returns 15 / 4 (FORGE: not on `SeasonType`).
- `PaceScore::projected_for(SeasonType)` returns `Projection::Per82` for Regular and `Projection::PerGame` for Playoff.
- **`pace_projected_for_playoff_zero_gp_no_nan`** (BENCH proptest):
  any `gp ∈ 0..=28, points ∈ 0..=70` produces a finite f64 or `Projection::PerGame(0.0)`, never NaN/Inf.
- `Player::active_stats(SeasonType)`: returns Some(Regular) always; Playoff returns None when `playoff_stats=None`.
- `Player::regular_stats()`: future-compat accessor returns `ActiveStatsRef::Regular`.
- **`goalie_active_stats_returns_none_when_playoff_stats_none`** (BENCH).
- **`goalie_active_stats_some_for_regular_always`** (BENCH).
- `Player.playoff_stats` defaults to None; serde round-trip.
- `Goalie.team_abbrevs` is `Option<String>` (FORGE schema fix); serde round-trip.
- `BUNDLED_PLAYOFF_SEASONS` constant non-empty + every entry parses as 8-digit (BENCH analog of `TRANSACTIONS_EARLIEST_SEASON`).
- **CI guard test (FORGE: walkdir + regex, not grep)** —
  `tests/season_field_guard.rs`: walks `**/*.rs` under workspace,
  greps for word-boundary `\b(season_goals|season_assists|season_points)\b`,
  excluding `//` and `///` lines, and fails if any match falls outside
  the allow-list (populated in S.1 by surveying the workspace first;
  see "Sub-phases" below).
- **`ci_guard_ignores_doc_comments`** (BENCH): synthetic Rust file with
  `/// season_goals` in docs must not trip the guard.

### L1
- `PlayerRepository::load_all(season, type=Playoff)` against fixture.
- `GoalieRepository::load_all(season, type=Playoff)` parallel.
- **Synthetic mid-playoff-trade fixture** (BENCH):
  `tests/fixtures/playoff_traded_skater.json` with `nhl_id=99999` traded
  EDM→PIT mid-PO. Real-data dedup is brittle; synthetic is deterministic.
- Per-type dedup uses `(nhl_id, season_type)` as key — playoff and regular
  stints don't collide.
- Mock fetcher for `fetch_all_stats(season, Playoff)` hits `gameTypeId=3`;
  return type is `TypedStats { season_type: Playoff, rows }` (FORGE).
- Mock 200 + empty + current season → `Ok(empty TypedStats)` with
  `PlayoffsNotStarted` marker.
- Mock 200 + empty + historical season → `Err(SuspiciousEmpty)`.
- `load_stats_with_fallback` returns `LoadError::MissingBundle` distinct
  from `LoadError::SeasonNotBundled` (FORGE error split).
- History aggregator filters Playoff out when called without `--type`.
- `Goalie::qualified(season_type)` — 15 GP threshold for Regular, 4 GP
  for Playoff. Cup-final losing goalie qualifies (BENCH explicit check).
- `Projection::Per82(_)` cannot be compared `==` to `Projection::PerGame(_)`
  (FORGE compile-test confirmed at type-check, not runtime).
- **`fantasy_score_ignores_season_type_arg`** (BENCH regression-catcher):
  call `compute_fantasy_score(player_with_playoff_stats, SeasonType::Playoff)`,
  assert returned score equals score from `season_goals`/`season_points`
  (regular fields), NOT from `playoff_stats.points`.
- Saved-query record round-trips `season_type` field.

### L2
- Lock fixtures only against completed bundled seasons (`--season 20232024`
  not 25-26 in-progress) — TAPE+EDGE.
- **`query_leaders_2023_playoff_top_5_count_and_anchors`** (BENCH-revised
  from "set membership"): asserts exactly 5 rows AND that two correction-
  immune Cup-final scorers (e.g. McDavid, Stuetzle for the 2024 final)
  are present. Other 3 slots float, count is locked.
- `query goalies --season 20232024 --type playoff` returns goalies with
  playoff GP ≥ 4 (the new threshold).
- `compare McD Bed --type playoff` works; mixing types impossible (both
  args always share a type).
- `--type playoff` on a season with no bundled playoff data: exit non-zero
  with helpful "missing bundle" message (`LoadError::MissingBundle`),
  not silent empty.
- `query leaders --top 100 --type playoff` early-playoff: returns < 100
  rows AND warns to stderr.

### TUI L1 — four-state empty handling (BENCH: was three)
- Title bar shows `⛸ PLAYOFF ⛸` reverse-video marker AND `[PO 23-24]`
  right-edge label when `season_type=Playoff`.
- `P` keystroke on League/Depth/Stats/Goalies switches type and resets
  `selected = 0` (GLASS).
- `P` keystroke when `playoff_phase=NotStarted` is REFUSED with status-
  line message; no silent empty mode switch.
- **`R_keystroke_when_already_regular_is_noop`** (BENCH idempotency).
- **`P_keystroke_when_already_playoff_is_noop`** (BENCH idempotency).
- Stats tab: lowercase `p` still toggles Projections↔Queries; uppercase
  `P` switches type. Both work without conflict.
- **`playoff_empty_state_4_distinct_renderings`** (BENCH table-driven):
  - `playoff_phase=NotStarted` → "Playoffs have not begun for {season}" banner
  - `Final` season + `playoff_stats=None` → "Did not appear in playoffs"
  - `playoff_stats=Some(gp=0)` → impossible state (we set None instead);
    test asserts the conversion path always normalizes to None
  - `playoff_stats=Some(gp>0, points=0)` → render normal stat block
    with literal zeros (real data, not a placeholder)
- Depth chart: team that missed the playoffs renders hatched bar with
  "Missed playoffs ({pts} pts)" overlay, distinct from low-pace teams.
- **`saved_query_type_mismatch_emits_status_warning`** (BENCH literal):
  load Regular-saved query, switch to Playoff, assert status line
  contains literal `"Saved as Regular — re-save?"`.

---

## Sub-phases (revised — FORGE)

- **S.0** — **Allow-list survey** (FORGE: do this BEFORE the guard or
  CI breaks on commit 1). Workspace-wide `Grep` for `\b(season_goals|
  season_assists|season_points)\b` in `*.rs`. Paste the full file list
  into the plan as the explicit allow-list. ~0.25 day.
- **S.1** — `SeasonType` enum + `ActiveStatsRef` enum + `Projection`
  tagged enum + `Player::active_stats` / `regular_stats` accessors +
  `PlayoffStats` struct + Goalie `team_abbrevs` Optional refactor +
  `Goalie::min_qualified_gp(season_type)` + `Goalie::qualified(season_type)`
  refactor + `PlayoffPhase` enum in `icelines-core::model` + walkdir
  CI guard test + history aggregator pinned to Regular. ~1.5 days
  (was 1; FORGE additions).
- **S.2** — Fetcher: `fetch_all_stats(season, type) -> TypedStats`
  envelope + `fetch_all_goalies(season, type) -> TypedGoalieStats`
  parallel + new `FetchError::SuspiciousEmpty` variant + mock tests
  for pre-playoff-empty (current vs historical). ~0.5 day.
- **S.3** — Bundled storage with provenance envelope, per-type dedup,
  `load_stats_with_fallback` returning `Result<_, LoadError>` (FORGE
  error split), `SnapshotMetaFlags` extension for `playoff_phase`
  (sourced from `icelines-core`) + `playoff_data_through`. Synthetic
  mid-playoff-trade fixture (BENCH). Capture playoff stats for the
  5 bundled seasons via `cargo run --example probe_espn_seasons --
  --capture-playoff-stats`. ~1 day.
- **S.4** — CLI `--type {regular|playoff}` on every report command,
  history defaults to Regular, fetch-all orchestration (Playoff
  demoted to warning), compare same-type rejection, top-N truncation
  warning. L2 tests with completed-season fixtures only. ~1 day.
- **S.5** — TUI `P`/`R` keystrokes (pre-playoff guard, idempotent
  no-ops), `⛸ PLAYOFF ⛸` reverse-video marker, right-edge pinned
  `[PO 23-24]` label, selection reset, four-state empty handling,
  depth-chart "missed playoffs" treatment, saved-query type pin
  with literal warning message. ~1.5 days.

**Total: ~5.75 days.** Up from 5; the FORGE additions (allow-list
survey, `ActiveStatsRef`/`Projection`/`LoadError` types, Goalie
team_abbrevs refactor) add ~0.75 day for materially less future
bug surface. Still ships as v0.12.0.

---

## Out of scope (revised)

- **Preseason**: easy add later behind same toggle.
- **Career-history typing flag**: history aggregator now defaults to
  Regular explicitly (TAPE fix); user-visible `--type playoff` flag
  on history defers to a follow-up.
- **Fantasy playoff scheme**: Phase S **hard-pins** fantasy to Regular.
  A playoff-tuned weights table is a separate worthy phase.
- **Cross-type comparisons** ("regular PPG vs playoff PPG side by side"):
  `compare` rejects mixed-type explicitly; a paired view defers.

---

## Resolved review questions

| Q | Resolution |
|---|------------|
| `T` cycle vs `P`/`R` direct? | **`P`/`R` direct** per user preference. Glyph+reverse-video marker addresses GLASS muscle-memory concern. |
| Pre-playoff empty? | `Ok(empty TypedStats)` with `PlayoffsNotStarted` marker; closed-season empty is `Err(SuspiciousEmpty)`. |
| `season_*` field foot-gun? | `ActiveStatsRef<'_>` enum + `Player::active_stats(type)` accessor; `regular_stats()` future-compat view; walkdir CI guard banning direct reads outside the surveyed allow-list. |
| `ActiveStatsRef` shape? | Enum variant per type; accessor methods (`goals`, `assists`, `points`, `gp`, `pace_score`, ...) abstract dispatch (FORGE). |
| History silently mixes types? | History aggregator defaults to Regular-only; explicit `--type playoff` deferred. |
| Goalie qualified threshold? | `Goalie::min_qualified_gp(SeasonType)` → 15 / 4. Lives on `Goalie`, not on `SeasonType` (FORGE cohesion). |
| pts/82 nonsensical in playoffs? | `PaceScore::projected_for(SeasonType) -> Projection` tagged enum (`Per82` vs `PerGame`). Mixing scales is a type error (FORGE). |
| Fantasy with type=Playoff? | Hard-pin to Regular in v1; regression test asserts ignore-type-arg (BENCH). |
| Stale-data analog? | `SnapshotMetaFlags::playoff_data_through: Option<String>` shows in title bar. |
| Saved queries break on type toggle? | Persist `season_type`; literal `"Saved as Regular — re-save?"` status warning (BENCH). |
| Selection preserved across modes? | Reset to 0 on switch (GLASS). Idempotent no-op tested for both directions (BENCH). |
| L2 fixture rot? | Lock against completed seasons only; assert count + 2 corrections-immune anchor names (BENCH revision of "set membership"). |
| Cross-type compare? | `compare` requires both args share a type; mixing impossible. |
| Goalie `team_abbrevs` shape? | Refactor to `Option<String>` to match new `playoff_team_abbrevs` and disambiguate "didn't play" from empty string (FORGE). |
| Error placement? | Split: `FetchError::SuspiciousEmpty` (network); new `LoadError::{MissingBundle, SeasonNotBundled}` (storage). `From<FetchError>` bridges the I/O cases (FORGE). |
| `PlayoffPhase` crate? | Lives in `icelines-core::model` (lowest crate); `SnapshotMetaFlags` imports it (FORGE). |
| Fetcher returns type-tagged data? | `TypedStats { season_type, rows }` envelope from S.2 onward — type travels with data (FORGE). |
| CI guard portability? | walkdir + regex test in `tests/season_field_guard.rs`, NOT shell grep (FORGE). |
| CI guard allow-list? | Surveyed first (S.0); locked into the test in S.1 (BENCH+FORGE: avoid breaking CI on commit 1). |

---

## Memory hooks

After Phase S closes, update
`C:/Users/giodl/.claude/projects/C--src-ICELINES/memory/season_type_plan.md`
to "shipped in v0.12.0".
