# Phase Art Ross — Implementation orchestrator

**Specs**: `design/specs/phase-art-ross-overview.md`
**Status**: Plan — orchestrator
**Date**: 2026-05-06

---

## Sub-phase ordering

```
A.0 (IR + planner skeleton) ──────────────────────────────────┐
                                                                │
A.1 (grammar expansion: <, >, !=, IN, BETWEEN, LIKE) ←─ A.0 ───┤
                                                                │
A.2 (sliding windows over per-game boxscores)        ←─ A.0 ───┤
                                                                │
A.3 (historical "ever" + at-age slicing)             ←─ A.2 ───┤
                                                                │
A.4 (career-league atoms across cross-league data)   ←─ A.0 ───┤
                                                                │
A.5 (--explain + cross-surface parity)               ←─ all ───┘
```

Critical path: **A.0 → A.1 → A.2 → A.3 → A.4 → A.5**.
A.1 / A.2 / A.4 can run in parallel after A.0; A.3 depends on A.2.

## Per-sub-phase test budget

(Updated post 8-role review — bench's coverage gaps absorbed.)

| Sub-phase | Tests | Focus |
|---|---|---|
| A.0 | 35 (15 L0 + 10 L1 + 10 L2) | IR construction, requirements walk, FilterInput decode boundaries, DataProvider trait, !Send compile_fail, IR roundtrip property test (proptest), Wave 11 adapter |
| A.1 | 40 (25 L0 + 8 L1 + 7 L2) | Each new operator + each new atom; one L0 per new ParseError variant |
| A.2 | 50 (20 L0 + 15 L1 + 15 L2) | Per-game aggregation, GP-counted vs calendar, WindowPolicy {RequireFull/AllowPartial/AllowPartialAbove} × {WindowScope}, --strict pre-materialize gate, fixture-driven correctness |
| A.3 | 35 (15 L0 + 10 L1 + 10 L2) | EVER axis-typing (regular vs playoff), lockout skip, intra-season-only, at-age slicing, fallback markers, Criterion benchmark |
| A.4 | 30 (15 L0 + 10 L1 + 5 L2) | League atom parsing, tier classification (`LeagueTier::Junior`), career fanout via fixture (no network), shared CareerHistoryFetcher coordinator |
| A.5 | 30 (10 L0 + 10 L1 + 10 L2) | Explain text + JSON envelope, golden snapshots (frozen Clock+bundle+READS_PER_SEC), surface parity coverage matrix |
| **Total Art Ross** | **220 tests** | (excluding Wave 12) |
| + Wave 12 | ~200 (L2 persona) | Filter combinations adversarial on new grammar — matches Wave 11's 201 |
| **Grand total** | **~420 new tests** | |

Plus all existing 2056 v0.19.1 tests must stay green at every
sub-phase acceptance gate.

## Pre-flight

- [x] v0.19.1 tagged — Wave 11 + Wave 10 polish committed
- [x] design/specs/phase-art-ross-overview.md drafted
- [x] design/plans/2026-05-06-phaseArtRoss-overview.md drafted (this file)
- [x] 8-role review (forge / edge / keel / wire / pace / bench / tape / scout) — all 8 returned GO-WITH-CHANGES
- [x] 18 review action items applied to spec + plan (see "Action items applied" table at the bottom)
- [ ] CLAUDE.md `[Phase Art Ross]` memory entry written (cross-session continuity)
- [ ] phases.md: Art Ross moves from Future to Active
- [ ] A.0 spawn

---

## A.0 — IR + planner skeleton

Goal: get the unified parser, n-ary IR, dependency-inversion seam,
and decode-boundary contract into place without changing any
existing user-visible behavior. After A.0, every existing
`--filter` shape parses through the new front door and produces
**identical results to v0.19.1, including the FIXED behavior of
the 3 Wave 11 bugs**. Nothing user-facing yet.

### A.0.1 — `Constraint` IR (icelines-query)

```rust
// icelines-query/src/plan.rs

pub enum Constraint {
    Bio(BioConstraint),
    SeasonStat(SeasonStatConstraint),
    SlidingWindow(SlidingWindowConstraint),  // parser rejects with FeatureNotYet until A.2
    CareerAggregate(CareerAggrConstraint),   // parser rejects with FeatureNotYet until A.3
    CareerLeague(CareerLeagueConstraint),    // parser rejects with FeatureNotYet until A.4
    All(Vec<Constraint>),                    // n-ary AND (NOT binary Compose)
    Any(Vec<Constraint>),                    // n-ary OR
    Not(Box<Constraint>),
}

pub struct BioConstraint {
    pub field: BioField,
    pub predicate: Predicate,                // shape-by-construction
}

pub struct SeasonStatConstraint {
    pub stat: StatId,
    pub predicate: Predicate,
    pub axis: SeasonAxis,                    // Regular | Playoff | All
}

pub enum BioField { Age, DraftYear, DraftRound, DraftOverall,
                    Height, Weight, Country, Nationality, Shoots,
                    Position, Team, TeamAny, TeamCareer,
                    BirthCity, BirthState, RookieSeason }

pub enum Predicate {
    Scalar(ScalarOp, ScalarValue),
    Member(MemberOp, Vec<ScalarValue>),      // empty list → ParseError::EmptySet
    Pattern(PatternOp, GlobPattern),         // NFD-normalized
    Range(RangeBounds<f64>),                 // numeric only
}

pub enum ScalarOp { Eq, Ne, Lt, Le, Gt, Ge }
pub enum MemberOp { In, NotIn }
pub enum PatternOp { Like, NotLike, Contains, NotContains }
pub enum SeasonAxis { Regular, Playoff, All }
```

### A.0.2 — `parse_query` front door + FilterInput

```rust
// icelines-query/src/input.rs

pub enum FilterInput {
    Cli(String),                             // clap-decoded
    Form(String),                            // surface URL-decoded
    Tui(Vec<AtomFragment>),                  // built directly, no string round-trip
}

// icelines-query/src/parser.rs

pub fn parse_query(input: FilterInput) -> Result<QueryPlan, Vec<ParseError>>;

pub struct QueryPlan {
    pub root: Constraint,
}

#[derive(thiserror::Error)]
pub enum ParseError {
    #[error("filter is empty")]
    EmptyInput,

    #[error("empty set in `{atom}` — `IN ()` is not valid")]
    EmptySet { atom: String },

    #[error("feature `{atom}` ships in {ships_in}; not yet supported")]
    FeatureNotYet { atom: String, ships_in: &'static str },

    #[error("predicate {predicate:?} is incompatible with {field}")]
    IncompatiblePredicate { field: String, predicate: String },

    // ... existing variants from FilterParseError
}
```

`parse_query` returns `Vec<ParseError>` (multi-error reporting) so a
5-atom filter with 3 errors surfaces all 3 in one round-trip. Each
error carries span info pointing at the offending atom.

For `FilterInput::Tui(Vec<AtomFragment>)`, the parser skips the
tokenizer entirely — the TUI overlay constructs typed `Constraint`
variants directly from widget state.

### A.0.3 — `PlanRequirement` (icelines-query)

```rust
pub struct PlanRequirement {
    pub seasons_needed: Vec<Season>,
    pub reports_needed: Vec<ReportKind>,
    pub boxscore_seasons_needed: Vec<Season>,    // shard granularity
    pub boxscore_date_range: Option<DateRange>,
    pub career_pids_needed: Vec<PlayerId>,
    pub eligible_for_strict: StrictEligibility,  // can --strict=RejectAll succeed?
}

pub struct StrictEligibility {
    pub all_seasons_have_boxscores: bool,
    pub all_pids_have_career_history: bool,
    pub fallback_seasons: Vec<Season>,            // seasons that would emit [fallback:]
}

// estimated_row_cost lives in meta only — non-reproducible debug hint
pub struct PlanCostEstimate {
    pub boxscore_reads: u64,
    pub estimated_seconds: f64,                   // boxscore_reads / READS_PER_SEC
}
```

### A.0.4 — `DataProvider` trait (icelines-query) + `EvalCtx`

```rust
// icelines-query/src/data_provider.rs

pub trait DataProvider {
    /// Ensure the data described in `req` is available locally.
    /// Returns a stream of FetchEvents the surface renders;
    /// library never writes to stderr/stdout.
    fn ensure(&self, req: &PlanRequirement) -> FetchStream;
}

pub struct EvalCtx<'a> {
    pub provider: &'a dyn DataProvider,
    pub repo: &'a StatsRepository,           // !Send — see compile_fail doctest
    pub strict: StrictMode,
    pub clock: &'a dyn Clock,                // injected per Foster.0 — testable
    _not_send: PhantomData<*const ()>,
}

pub enum FetchEvent {
    Started { units: u32, label: String },
    Progress { done: u32, total: u32 },
    Complete,
    Failed { reason: FetchError },
}

pub enum StrictMode {
    Off,
    RejectPartialSeasons,
    RejectPartialWindows,
    RejectAll,
}
```

The `IcelinesProvider` impl lives in `icelines-fetch::query_provider`.
CLI/web/TUI each construct one and inject into `EvalCtx`. Compile-
fail doctest pins `EvalCtx: !Send` for the `tokio::spawn` rejection.

### A.0.5 — `materialize` + StrictMode gate

```rust
pub fn materialize(plan: &QueryPlan, ctx: &EvalCtx) -> Result<MaterializedSet, MaterializeError>;

// Order of operations inside materialize:
//   1. plan.requirements()
//   2. StrictMode check — if strict-violating: error BEFORE any fetch
//   3. ctx.provider.ensure(req) — fetches missing data, emits FetchEvents
//   4. Build BoxscoreIndex shards as needed (per-season, dropped after use)
//   5. Aggregate per-game lines into windowed totals
//   6. Return MaterializedSet (carries [partial:] / [fallback:] markers)
```

### A.0.6 — Test fixtures (committed to repo)

- `tests/fixtures/boxscores/` — 3 seasons of curated boxscores
  covering: a traded player (mid-season EDM → DAL), an accented
  player (Slafkovský), a GP=0 player, a known 5-in-10 streak
  example with hand-verified expected values, a goalie with mixed
  decisions (W/L/OTL/Shutout).
- `tests/fixtures/career_history_sample.json` — small (~50 KB)
  career-history blob covering OHL/WHL/QMJHL/NCAA/SHL/KHL with
  tier classification + a player with no junior history.

These fixtures ship with the repo (tests/fixtures/ is checked in,
~200 KB total). All A.2/A.3/A.4 tests use these instead of the
network or the full bundle. The full bundle is exercised by Wave 12
end-to-end (which already runs against committed bundle data).

### A.0.7 — Adapter shim in `icelines-cli` and `icelines-web`

The CLI's `apply_views` and the web's `parse_filter_form` both wrap
their input in `FilterInput::Cli` / `FilterInput::Form`, call
`parse_query`, and walk the resulting plan. Legacy filter strings
must still produce identical results.

The TUI's filter overlay builds `FilterInput::Tui(Vec<AtomFragment>)`
directly from typed widget state — no string round-trip.

### A.0.8 — Wave 11 + 12 adapter

Wave 11's 201 persona scenarios live in `persona_wave11.rs` and
exercise the legacy filter pipeline. A.0 adds an adapter layer so
Wave 11 ALSO runs against the new pipeline (same input, same
expected results — including the FIXED behavior of the 3 Wave 11
bugs). Wave 12 (filter combinations on new grammar, ~200 scenarios
to match Wave 11's surface) ships at A.5 closeout.

### A.0 acceptance gate

- All 2056 v0.19.1 tests pass with the new pipeline.
- 30 new A.0 tests pass.
- Wave 11 (201 scenarios) runs against the new pipeline; if any
  regress, fix before merging.
- `EvalCtx: !Send` compile_fail doctest passes.
- Crate dependency graph clean: `cargo tree -p icelines-query`
  shows no upward dep on `icelines-fetch`.
- IR roundtrip property: random `Constraint` trees serialize to
  canonical strings that reparse identically (proptest, 1000 runs).

---

## A.1 — Grammar expansion

### A.1.1 — Strict comparators (`<`, `>`)

In `parse_filter`, add `<` and `>` to the OPS table. Order matters:
`<=`/`>=` already shadow them at the same position, so prefer the
2-char op when both match.

### A.1.2 — `!=`

Add `!=` as the not-equals comparator. Hint when user types `<>`
(SQL-style not-equals) suggesting `!=`.

### A.1.3 — `IN (...)` / `NOT IN (...)`

New atom shape: `<key> IN (a, b, c)` / `<key> NOT IN (a, b, c)`.
Tokenizer recognizes `IN` keyword (boundary-matched). Values comma-
separated, optional whitespace, optional quotes for strings.
Implementation: produces a `ConstraintValue::Set(Vec<String>)`.
Numeric atoms reject `IN` (use `BETWEEN` instead).

### A.1.4 — `BETWEEN x AND y`

`<key> BETWEEN x AND y` — strictly inclusive both sides.
Equivalent to `<key>>=x AND <key><=y` but cleaner. Numeric only.

### A.1.5 — `LIKE "pattern"`

Glob-style `*` wildcard, anchored to start+end (so `Mc*` matches
`McDavid` but not `MacDonald`; `*Mac*` matches both via substring).
Case-insensitive by default. Add `~` for "contains" sugar.

### A.1.6 — New bio atoms

Add to `BioField`:
- `position` (`pos=C`, `pos IN (C,LW,RW)`)
- `team` (`team=EDM`, `team IN (EDM,DAL,COL)`)
- `draft_round` (`draft-round<=2`)
- `draft_overall` (`draft-overall<=10`)
- `birth_state` (`birth-state=ON`)
- `rookie` (`rookie=true` — boolean atom; pulls `rookie_season`
  from `PlayerBio`)

### A.1 acceptance gate

- 40 new tests pass (the budget).
- Wave 11 still passes — backward compat preserved.
- COMMANDS.md gets a new "Operators" section documenting the
  expanded grammar.

---

## A.2 — Sliding windows over per-game boxscores

### A.2.1 — `SlidingWindowConstraint` IR

```rust
pub struct SlidingWindowConstraint {
    pub stat: StatId,                        // skater stats from boxscore
    pub window: SlidingWindow,
    pub predicate: Predicate,                // typed shape — see A.0.1
    pub axis: SeasonAxis,                    // Regular | Playoff | All
}

pub enum SlidingWindow {
    LastN_GP { n: u8, scope: WindowScope, policy: WindowPolicy },
    LastN_Days { n: u16 },
    LastN_Weeks { n: u8 },
    LastN_Months { n: u8 },
}

pub enum WindowScope {
    CurrentTeamCurrentSeason,                // default
    AllTeamsCurrentSeason,                   // .allteams modifier
    Career,                                  // .career modifier
}

pub enum WindowPolicy {
    RequireFull,                             // default — GP < n returns false
    AllowPartial,                            // GP < n uses min(n, GP)
    AllowPartialAbove(u8),                   // partial OK if GP >= threshold
}
```

### A.2.2 — Boxscore aggregation engine

```rust
pub fn aggregate_window(
    pid: PlayerId,
    window: &SlidingWindow,
    axis: SeasonAxis,
    shard: &BoxscoreShard,
) -> WindowResult;

pub enum WindowResult {
    Full(WindowTotals),
    ShortWindow { totals: WindowTotals, gp: u8 },  // emits [short-window: Ng]
    Empty,                                          // GP=0, atom returns false
}
```

For `LastN_GP` aggregates the trailing N games (sorted by date,
respecting `WindowScope` for team filtering). For calendar windows
filters by date first, then aggregates.

**Mid-season trade handling (R7, edge)**: `WindowScope::CurrentTeam-
CurrentSeason` (default) reads `team_stints` from the player's
identity, picks the most recent stint, and filters boxscores to
games where the player's team matched that stint's team. This is
the hockey-natural behavior for "last 10 EDM games."

### A.2.3 — `BoxscoreIndex` (per-season sharded)

```rust
pub struct BoxscoreIndex {
    shards: BTreeMap<Season, BoxscoreShard>,   // ~4-6 MB per shard
    lru: VecDeque<Season>,                     // cap = 4 shards
}

pub struct BoxscoreShard {
    by_pid: HashMap<PlayerId, Vec<GameRef>>,   // pid → games sorted by date
    season: Season,
    boxscore_count: u32,
}
```

- **Per-season sharding**: only the active season is hot; cross-
  season queries iterate season-by-season, dropping each shard.
- **LRU cap = 4 shards** — keeps resident set bounded at ~24 MB.
- **Eligible seasons for boxscore-driven evaluation**: 2021-22+
  (Foster +3 boxscore persistence covers these). Pre-2021-22:
  `MaterializedSet` falls back to season aggregate with
  `[fallback: <season>]` marker.
- **Cache invalidation**: index participates in `repo_swap` —
  switching seasons invalidates the active shard. Manifest version
  bump rebuilds the affected shard.

### A.2.4 — On-demand fetch via `DataProvider`

```rust
// In materialize():
let req = plan.requirements();
ctx.strict_check(&req)?;  // R12: BEFORE any fetch
ctx.provider.ensure(&req).await?;  // emits FetchEvents
```

`DataProvider::ensure` is implemented in
`icelines-fetch::query_provider::IcelinesProvider`:
- Identifies missing boxscore date ranges.
- Issues parallel fetches with `tokio::join_all` + `Semaphore(4)`
  (R-pace).
- Yields `FetchEvent::Progress { done, total }` items the surface
  renders (CLI: stderr banner; web: SSE; TUI: sync banner widget).

`--no-fetch` flag: refuses the operation if any data is missing
(produces `MaterializeError::DataMissing`).

### A.2.5 — `--strict` flag wiring

`--strict[=mode]` on every query subcommand:
- `--strict` (bare) → `StrictMode::RejectAll`
- `--strict=partial-seasons` → `StrictMode::RejectPartialSeasons`
- `--strict=partial-windows` → `StrictMode::RejectPartialWindows`
- `--strict=all` → `StrictMode::RejectAll`

Config: `strict = "off|partial-seasons|partial-windows|all"`.

The check fires between `requirements()` and the first
`provider.ensure()` call — strict-violating plans error before
any network I/O.

### A.2 acceptance gate

- 40 new tests (per the budget).
- Fixture-driven correctness:
  - `g.last10g>=5` with `WindowPolicy::RequireFull` returns false
    for a fixture player with GP=7.
  - `g.last10g>=5 :allow-partial` returns the right answer with
    `[short-window: 7g]` marker.
  - `team=EDM AND g.last10g>=5` for a fixture mid-trade player
    counts only EDM-stint games.
- Hand-verified bundle queries:
  - `--filter "g.last10g>=5"` finds players with 5+ goals in their
    last 10 games this season.
  - `--filter "p.last30d>=20"` finds 20+ points in last 30 days.
- `--strict=partial-seasons` rejects 2019-20 queries before any
  fetch.

---

## A.3 — Historical "ever" + at-age slicing

### A.3.1 — `CareerAggrConstraint` IR

```rust
pub struct CareerAggrConstraint {
    pub stat: StatId,
    pub aggregator: CareerAggregator,
    pub predicate: Predicate,
    pub axis: SeasonAxis,                    // EVER inherits this — does NOT mix
                                             //   regular + playoff unless axis=All
    pub at_age: Option<AgeBound>,
}

pub enum CareerAggregator {
    LifetimeSum,                             // p.career>=500 — walks all 38 seasons
    AnyWindow(u8),                           // g.any10g>=5 EVER — short-circuits on first hit
    LongestStreak,                           // p.streak>=15 — walks until current best > target
    SeasonsWith,                             // count of seasons matching threshold
}

pub struct AgeBound { pub max: Option<u32>, pub min: Option<u32> }
```

### A.3.2 — Cross-season fanout (locked semantics)

```rust
pub fn evaluate_ever(
    pid: PlayerId,
    c: &CareerAggrConstraint,
    ctx: &EvalCtx,
) -> Result<EvalResult, EvalError>;
```

**Locked rules (R6, edge + scout):**
1. `EVER` walks every bundled season **except `LOCKOUT_SEASONS = [20042005]`** (skip — no data, no partial-mark).
2. **Intra-season only**: the window does not cross season boundaries. Game 4 of 25-26 + last 6 of 24-25 do not form a 10-GP window.
3. **Axis-typed**: a season's regular-season games and playoff games are evaluated separately unless `axis = SeasonAxis::All`. So `g.any10g>=5 EVER` over `axis=Regular` (default) does not mix in playoff goals.
4. **Short-circuit**: `AnyWindow` returns true on the first satisfying season. `SeasonsWith` returns count; can short-circuit if comparator allows.
5. **Eligible-season fallback** (R17, tape): pre-2021-22 seasons fall back to season aggregate via `MaterializedSet`'s `fallback_seasons` set; the `AnyWindow` aggregator collapses to "did the season total cross a derived threshold" with explicit fidelity loss recorded.

### A.3.3 — At-age slicing (locked convention)

`AT age<=22` filters the season set BEFORE aggregation. Convention:
**Hockey-Reference Feb 1 of season's second year** (already in
`compute_age`). Feb 29 birthdays use Feb 28 in non-leap years.
Missing `birth_date` produces `EvalError::MissingBio { pid, field:
"birth_date" }` (R8, edge).

### A.3.4 — Fallback marker (deterministic)

When boxscores missing for a season:
- `MaterializedSet` records the season in `fallback_seasons`.
- Per-row `partial_seasons: Vec<Season>` field tracks which fallback
  seasons contributed to that row.
- JSON envelope: `meta.partial_seasons` is the union across rows
  (deterministic — sorted by season).
- `--strict=RejectPartialSeasons` errors at the StrictMode gate
  before materialize.

### A.3.5 — Performance (R13, pace)

- Criterion bench `bench_ever_query` committed at A.3 closeout.
- Budget: cold `g.any10g>=5 EVER` ≤8s, warm ≤2s.
- `READS_PER_SEC` calibration constant measured per build (one-time
  fixture pass at build time).
- Per-season iterator pattern: load shard → evaluate → drop shard.
  Resident-set ceiling: one shard at a time (~4-6 MB).

### A.3 acceptance gate

- 30 new tests.
- Criterion benchmark passes the cold/warm budgets.
- Hand-verified: `--filter "g.any10g>=5 EVER AT age<=22"` returns
  young-McDavid-class results from the 38-season bundle.
- `g.any10g>=5 EVER` with axis=Regular DOES NOT include
  playoff games (regression test).
- 2004-05 (lockout) is skipped, not partial-marked (regression test).

---

## A.4 — Career-league atoms

### A.4.1 — `CareerLeagueConstraint` IR

```rust
pub struct CareerLeagueConstraint {
    pub league: LeagueAtom,
    pub stat: Option<StatId>,                // None = "played in this league"
    pub aggregator: Option<CareerAggregator>,
    pub predicate: Option<Predicate>,
}

pub enum LeagueAtom {
    Code(String),                            // "OHL"
    InSet(Vec<String>),                      // OHL, WHL, QMJHL — empty rejected at parse
    Tier(LeagueTier),                        // Junior / Pro / College / International / Other
}
```

`p.career.junior` is defined as `LeagueTier::Junior` (R-scout) —
covers CHL three (OHL/WHL/QMJHL) + USHL + Liiga U20 + other
junior leagues classified by Phase Calder's tier mapping. Not a
hardcoded list.

### A.4.2 — `CareerHistoryFetcher` (shared coordinator)

Single `CareerHistoryFetcher` shared across A.2/A.3/A.4 with
`Semaphore(4)` cap on concurrent landing fetches. Atomic
`tmp+rename` writes to `~/.icelines/career_history.json` (Foster's
atomic-write idiom). Backoff inherited from existing
`career_landing.rs` batch fetcher (R18, tape).

```rust
// icelines-fetch::career_landing::fetcher

pub struct CareerHistoryFetcher {
    cache: Arc<RwLock<CareerHistoryStore>>,
    semaphore: Arc<Semaphore>,                // cap=4
    client: NhlApiClient,
}

impl CareerHistoryFetcher {
    pub async fn ensure_pids(
        &self,
        pids: &[PlayerId],
        events_tx: mpsc::Sender<FetchEvent>,
    ) -> Result<(), FetchError>;
}
```

Three sub-phases (A.2/A.3/A.4) share the SAME fetcher instance via
`DataProvider`. No independent fetches; rate-limit-friendly.

### A.4.3 — Career-history index

```rust
pub struct CareerHistoryIndex {
    by_pid: HashMap<PlayerId, Vec<CareerSeasonLine>>,
    by_league: HashMap<String, Vec<(PlayerId, Season)>>,
    by_tier: HashMap<LeagueTier, Vec<(PlayerId, Season)>>,
}
```

Built lazily on first A.4 query; dropped on `repo_swap` for the
`at_age` slice; otherwise persists for the binary lifetime
(career data is mostly season-invariant).

### A.4.4 — Aggregate atoms

- `p.career.junior>=200` — junior-tier career points.
- `p.career.nhl>=500` — NHL-only career points.
- `g.season.junior>=50` — any junior season with 50+ goals.
- `league=OHL AND p.season>=80 AND age<=18` — produces elite-
  junior cohort.
- `league IN (OHL,WHL,QMJHL)` — explicit league set.
- `league.tier=Junior` — same as `IN` over all junior leagues.

### A.4.5 — ESPN-style team-abbrev concern

The career-landing payload may use legacy abbreviations for old
seasons (PHX vs ARI vs UTA boundary). The team-resolution helper
must accept all three for the relevant seasons; the spec's
`league=` atom is league-keyed, not team-keyed, so this surfaces
only when combined with `team.career=` filters. Resolved by
reusing `icelines-core::model::TeamAbbr::canonicalize` which
already handles the relocations.

### A.4 acceptance gate

- 25 new tests.
- Fixture-driven: `tests/fixtures/career_history_sample.json`
  drives all L1 tests (no network).
- Hand-verified: `--filter "league=OHL AND p.season>=80 AND
  age<=18"` returns elite-junior production.
- Shared coordinator: A.2 + A.3 + A.4 in one query do not exceed
  4 concurrent NHL API requests.

---

## A.5 — `--explain` + cross-surface parity

### A.5.1 — `--explain` text output

```
$ icelines query leaders --filter "g.last10g>=5 AND age<=24" --explain

QUERY PLAN  (explain.v1)
══════════════════════════════════════════════════════════════════
All
├─ SlidingWindow(g, last10g{n=10, scope=current-stint, policy=require-full},
│                Scalar(>=, 5), axis=Regular)
│    ▸ requires: boxscores season=20252026 [2025-10-01 .. 2026-05-06]
│    ▸ estimated: 220 boxscores, ~8 500 reads
└─ Bio(age, Scalar(<=, 24))
     ▸ requires: bios (already loaded)

DATA REQUIREMENTS
══════════════════════════════════════════════════════════════════
✓ bios            season=20252026   (loaded)
✓ stats           season=20252026   (loaded)
↓ boxscore        season=20252026   (200 local, 18 missing)

Strict eligibility:
  ✓ all seasons have boxscores
  ✓ all pids resolve in identity layer
  No fallback seasons.

Estimated cost:    8 500 boxscore reads
Estimated runtime: 1.2 s  (cost / READS_PER_SEC=7000)
                          [non-reproducible — meta only]

Run with --no-fetch to skip the fetch step.
══════════════════════════════════════════════════════════════════
```

### A.5.2 — `--explain --json` envelope (frozen v1)

```json
{
  "schema_version": "explain.v1",
  "route": "leaders.explain",
  "data": {
    "plan_tree": { "kind": "all", "children": [...] },
    "requirements": {
      "seasons_needed": [20252026],
      "boxscore_seasons_needed": [20252026],
      "boxscore_date_range": { "start": "2025-10-01", "end": "2026-05-06" },
      "career_pids_needed": [],
      "fallback_seasons": []
    },
    "strict_eligibility": {
      "all_seasons_have_boxscores": true,
      "all_pids_have_career_history": true,
      "fallback_seasons": []
    }
  },
  "meta": {
    "estimated_cost": { "boxscore_reads": 8500, "estimated_seconds": 1.2 },
    "filter_input": "g.last10g>=5 AND age<=24",
    "schema_note": "explain.v1 frozen — additive only; v2 ships alongside"
  }
}
```

**Locked frozen-v1 discipline (R-wire)**:
- `data` keys are stable: `plan_tree`, `requirements`,
  `strict_eligibility`. New keys are additive only.
- `meta.estimated_cost` is non-reproducible — excluded from
  snapshot diffs.
- Internally-tagged JSON for IR variants (`{"kind": "...", ...}`).
- `#[serde(deny_unknown_fields)]` on every IR struct surfaced in
  the envelope.
- Breaking changes ship `explain.v2` alongside (route
  `leaders.explain` accepts a `version` query param).

### A.5.3 — Three new K2.4-enveloped routes

| Route | Replaces | Migration |
|---|---|---|
| `leaders.windowed` | `query leaders --week`/`--month` (currently bare-array) | Opt-in via `--envelope`; default in v0.21.0 |
| `leaders.playoff` | `query leaders --playoff` (Phase Conn Smythe added bare-array) | Opt-in `--envelope` v0.20.0; default v0.21.0 |
| `leaders.career` | NEW route — extension of `query career` shape, NOT replacement | `query career` keeps its own shape; `leaders.career` is a NEW surface for cross-league filters in `query leaders` |

`--envelope` flag during transition: opt-in v0.20.0; default v0.21.0
with `--no-envelope` escape; removed v0.22.0.

### A.5.4 — Cross-surface parity tests

`persona_query_parity.rs`: for each of **N canonical query
expressions**, where N = (10 ops × 6 atom families × 3 surfaces ≈
180 cells) — at least **one positive + one negative case per cell**
(R-bench). Verifies CLI / web / TUI produce identical result count +
identical first-10 player list + identical `--explain` plan tree.

### A.5.5 — Golden snapshot strategy (R-bench)

`--explain` golden snapshots live at `tests/snapshots/explain/`.
Determinism requirements:
- Frozen `Clock` injected into `EvalCtx` (Foster's `MockClock`).
- Frozen bundle version (test fixture pins
  `BUNDLED_SEASONS_VERSION`).
- Frozen `READS_PER_SEC` constant (test override via env var).
- `meta.estimated_cost.estimated_seconds` redacted in snapshots
  (already non-reproducible).
- `insta` for snapshot capture/compare.

### A.5.6 — Wave 12

`persona_wave12.rs`: ~200 scenarios (matching Wave 11's 201)
exercising every new operator × atom family × edge case discovered
in this 8-role review. Specific coverage:
- Each new `FilterParseError` variant gets an L0 test asserting it
  fires for the right input.
- Every new operator (`<`, `>`, `!=`, `IN`, `NOT IN`, `BETWEEN`,
  `LIKE`, `~`) tested in compound expressions.
- Mid-trade `team=` / `team.any=` / `team.career=` semantics.
- `EVER` axis-typing (regular vs playoff vs all).
- `LIKE` with Unicode names (Slafkovský / Kämpf / O'Reilly).
- Empty `IN ()` rejection.
- 38-season fanout correctness (lockout skip, eligible-season
  fallback, partial-season markers).

### A.5 acceptance gate

- 25 new A.5 tests.
- ~200 Wave 12 tests (committed at A.5 closeout).
- Surface parity coverage: ≥1 positive + ≥1 negative per
  (operator × atom_family × surface) cell.
- All 2056 v0.19.1 tests + Wave 11 (201) still green.
- `--explain` golden snapshots stable across 3 consecutive runs.
- Three K2.4-enveloped routes produce identical `data` to their
  bare-array counterparts (envelope-stripping equivalence).

---

## Closeout (after A.5)

- COMMANDS.md gets a major new "Query language" section.
- README.md gets a "Why IceLines queries are different" section
  with the three killer examples.
- CHANGELOG.md gets a v0.20.0 entry summarizing all 6 sub-phases.
- CLAUDE.md "What's been built" gets a Phase Art Ross bullet.
- Tag v0.20.0.
- Wave 12 (filter combinations on new grammar, 150 personas) ships
  in v0.20.1 as the bug-hunt validation pass.

## Open questions — RESOLVED by 8-role review (2026-05-06)

1. **`EVER` syntax**: keyword form `... EVER`. Intra-season only.
   Inherits axis from constraint. Skips `LOCKOUT_SEASONS = [20042005]`.
2. **`--explain` output**: both tree and requirements. JSON envelope
   `{schema_version: "explain.v1", data: {plan_tree, requirements,
   strict_eligibility}, meta: {estimated_cost}}`. Frozen v1.
3. **Career-league atoms vs `query career`**: both ship.
   `query career` is the cohort surface (existing); `--filter
   "league=OHL"` lets `query leaders` filter by cross-league career
   via `LeagueTier::Junior` (not hardcoded).
4. **`AT age<=22` syntax**: modifier form. Convention: HR Feb 1 of
   season's second year (already in `compute_age`). Missing birth_date
   errors loudly.

## Action items applied (R# = review-action number)

| R# | Item | Where applied |
|---|---|---|
| R1 | `DataProvider` trait + dependency inversion | A.0.4 |
| R2 | N-ary `All(Vec)` / `Any(Vec)` IR | A.0.1 |
| R3 | `ParseError::FeatureNotYet` (no panic placeholders) | A.0.2 |
| R4 | `Result<_, Vec<ParseError>>` multi-error | A.0.2 |
| R5 | `Predicate { Scalar / Member / Pattern / Range }` | A.0.1 |
| R6 | `EVER` axis-typed, intra-season, lockout-skip | A.3.2 |
| R7 | `WindowPolicy` enum + `team.any=` / `team.career=` | A.2.1, spec |
| R8 | HR Feb-1 age convention; missing-bio errors | A.3.3 |
| R9 | Empty `IN ()` rejected; `LIKE` NFD-normalized | spec |
| R10 | `FilterInput` enum with three pre-decoded variants | A.0.2 |
| R11 | `explain.v1` frozen JSON envelope; canonical wire | A.5.2 |
| R12 | `StrictMode` enum; pre-materialize gate | A.0.5, A.2.5 |
| R13 | Criterion bench; ≤8s cold / ≤2s warm | A.3.5 |
| R14 | Per-season sharded `BoxscoreIndex`; LRU cap=4 | A.2.3 |
| R15 | Committed test fixtures (boxscores + career sample) | A.0.6 |
| R16 | `--explain` golden snapshots; frozen Clock+bundle | A.5.5 |
| R17 | Eligible-seasons tier (2021-22+); fallback marker | A.2.3, A.3.4 |
| R18 | Shared `CareerHistoryFetcher` coordinator | A.4.2 |
