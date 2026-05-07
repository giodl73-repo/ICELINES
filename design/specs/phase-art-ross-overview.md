# Phase Art Ross — Overview

**Trophy**: Art Ross Trophy (NHL points leader — fitting for the
phase that rebuilds the query system into a points-leader-class
flexible engine)
**Version**: 1.0 (initial)
**Date**: 2026-05-06
**Status**: Spec — pending 8-role review
**Plan**: `design/plans/2026-05-06-phaseArtRoss-overview.md`

---

## Vision in one paragraph

The query system is the part of IceLines that justifies the system
existing. NHL stats sites are everywhere; what's missing is one that
lets you ask *real* hockey questions naturally — "who has had 5 goals
in any 10-game window, ever, while under 25?" — and get an answer.
Phase Art Ross rebuilds the filter pipeline into a unified
parser → planner → executor architecture: one front door, one
intermediate representation, one evaluator. Atoms route by type
(bio / season-stat / sliding-window / career / cross-league); the
planner determines what data is needed and pulls on demand if
missing; the executor walks the player set once. New atom shapes
unlock streak windows over per-game data, "ever" queries across all
38 bundled seasons, and cross-league career filters. A `--explain`
flag prints the plan tree so the user can confirm interpretation.

This is the centerpiece. After Art Ross, the answer to "who else is
going to do as good a job on hockey queries" is "nobody, because the
combinatorial space we expose is bigger than what any of the major
sites do."

## Locked decisions

(Updated 2026-05-06 post 8-role review — items marked **R#** correspond to review-action numbers in the action list at the bottom.)

| Decision | Choice | Rationale |
|---|---|---|
| Query IR location | New `icelines-query::plan` module (extends the existing bridging crate) | Already lives between `icelines-core` and consumers — natural home for the planner |
| Single front-door API | `parse_query(input: FilterInput) -> Result<QueryPlan, Vec<ParseError>>` in `icelines-query` | Multi-error reporting (R4); accepts pre-decoded `FilterInput` so each surface owns its own decode boundary (R10) |
| IR shape | Tree of typed `Constraint` nodes with **n-ary** booleans: `Bio` / `SeasonStat` / `SlidingWindow` / `CareerAggregate` / `CareerLeague` / `All(Vec<C>)` / `Any(Vec<C>)` / `Not(Box<C>)` | N-ary collapses naturally during parse, makes `requirements()` a single fold, gives `--explain` flat trees (R2) |
| Predicate shape | `Predicate { Scalar(ScalarOp, ScalarValue), Member(MemberOp, Vec<ScalarValue>), Pattern(PatternOp, GlobPattern), Range(RangeBounds<f64>) }` — invalid states unrepresentable | `LIKE 5` / `g IN 20` fail at parse, not at evaluate (R5) |
| Unimplemented variants | `parse_query` rejects atoms whose variant ships in a later sub-phase with `ParseError::FeatureNotYet { atom, ships_in: "A.2" }` — variant exists in enum but cannot be constructed from user input until the relevant sub-phase | Zero panic-on-execute; A.2-A.4 light up by adding parser arms (R3) |
| Crate dependency contract | `EvalCtx` carries `dyn DataProvider` trait owned by `icelines-query`; impl in `icelines-fetch` injected by the surface (CLI/web/TUI). Library crates do not write to stderr — `DataProvider` yields `FetchEvent` items the caller renders | Preserves the layering chain; query → fetch via inversion (R1) |
| Decode boundaries | `FilterInput` enum with three pre-decoded variants: `FilterInput::Cli(String)` (clap-decoded), `FilterInput::Form(String)` (URL-decoded by the surface), `FilterInput::Tui(Vec<AtomFragment>)` (TUI builds the tree directly without round-tripping through string). All three converge on the same `parse_query` after decode | Hides nothing; each surface owns its decode (R10) |
| On-demand fetch | Planner emits `PlanRequirement`; the executor's `materialize` step pulls missing data via `DataProvider::ensure(req)`. **`--strict` is checked between `requirements()` and `materialize()` — strict-violating plans error before a single fetch.** | Saves the user's clock when data is missing (R12) |
| `--strict` mode | `StrictMode { Off, RejectPartialSeasons, RejectPartialWindows, RejectAll }`; CLI flag `--strict[=mode]` (bare `--strict` = `RejectAll`); config setting `strict = "off|partial-seasons|partial-windows|all"` | Explicit enum beats boolean for ambiguity (R12) |
| Sliding-window granularity | `g.last10g` (last N GP, **current season, current team stint, contiguous**), `g.last30d` / `last3w` / `last3m` (calendar windows of trailing days). Modifiers: `lastNg.career` (cross-season), `lastNg.allteams` (any team stint this season). | Hockey-natural defaults; modifiers expose finer slicing (R7, scout) |
| `team=` semantics | `team=EDM` matches **current stint only**; `team.any=EDM` matches any stint this season; `team.career=EDM` matches any stint ever | Mid-season-trade behavior must be explicit (R7, scout) |
| `WindowPolicy` (sliding atoms when GP < window) | `WindowPolicy { RequireFull, AllowPartial, AllowPartialAbove(N) }`. Default: `RequireFull` (player with 7 GP fails `last10g`); `[short-window: 7g]` marker emitted when `AllowPartial` and the window is short. | Spec the boundary explicitly; never silent-false (R7) |
| `EVER` semantics | `EVER` is a global modifier on `CareerAggrConstraint::AnyWindow(N)`. Walks every bundled season **except `LOCKOUT_SEASONS = [20042005]`** (skip, not partial-mark). Within a season, the window is **intra-season only** (does not cross season boundaries). Inherits `axis: SeasonAxis` (Regular / Playoff) from the constraint — does not mix regular + playoff games unless `axis = All`. | Lockout is "no data," not "partial data" (R6, edge); intra-season matches fan intuition (scout) |
| `AT age` slicing | Hockey-Reference convention: **age as of February 1 of the season's second year** (already implemented in `compute_age`). Feb 29 birthdays use Feb 28 in non-leap years. Missing `birth_date` produces `FilterEvalError::MissingBio { pid, field: "birth_date" }`, never silent-false. Per-game age is out of scope (deferred). | One convention; user-visible (R8) |
| `IN ()` / `LIKE` edges | Empty `IN ()` rejected at parse with `ParseError::EmptySet`. `LIKE "pattern"` always normalizes both pattern and target via NFD-strip + lowercase, so `LIKE "stutzle"` matches `Stützle`. `~` is a "contains" sugar with the same normalization. | Slafkovský / Kämpf / O'Reilly must be reachable via ASCII patterns (R9, edge) |
| `country=` vs `nationality=` | `country=CAN` matches `birth_country`; `nationality=CAN` matches `nationality_code`. Two distinct atoms — dual-citizens (Matthews/Eichel/Tkachuk-class) diverge between them. | Don't conflate passport with IIHF (scout) |
| `pos=` semantics | `pos=C` matches roster `position_code` (canonical primary). Per-game slot variation is a deployment question, not a query atom. | One axis; deployment lives elsewhere (scout) |
| `name LIKE` | Always matches `name_normalized` (NFD-stripped) — pattern auto-normalized (R9). | ASCII patterns reach Unicode names (scout) |
| `p.career.junior` defn | `LeagueTier::Junior` from `career_history.rs` — covers CHL three (OHL/WHL/QMJHL) plus USHL, Liiga U20, etc. Not a hardcoded list. | Tier classification is the right axis (scout) |
| Strict comparators | Add `<` and `>` (currently only `<=` and `>=`) plus `!=` (currently only `==`/`=`); typo hint: `<>` suggests `!=` | "under 25" should be `age<25`, not `age<=24` |
| Range sugar | `BETWEEN x AND y` on numeric atoms — strictly inclusive both sides. Numeric only; rejected on string atoms. | Replaces `g>=20 AND g<=40` with `g BETWEEN 20 AND 40` |
| Plan visibility | `--explain` flag on every query subcommand. JSON envelope is `{schema_version: "explain.v1", route, data: {plan_tree, requirements, estimated_cost}, meta}`. Frozen v1 — additive only; breaking changes ship `explain.v2` alongside (R11) | `event_stream::payloads` precedent — frozen-v1 discipline |
| `SlidingWindow` wire format | Internally-tagged JSON: `{"kind": "last_n_gp", "n": 10}` / `{"kind": "last_n_days", "n": 30}` etc. Same for `EVER`/`AT` modifiers. Canonical text round-trip preserved so `--explain` output pastes back as `--filter` input (R11) | Stable, single representation per type |
| Cost estimation | `estimated_row_cost: u64` lives in `meta`, not `data`. Units: **boxscore reads** (not seconds). Wall-clock estimate uses calibration constant `READS_PER_SEC` measured per build. Marked non-reproducible (R11) | Honest about what's a contract vs a hint |
| Cross-surface parity | CLI / web / TUI all parse via `parse_query(FilterInput)` and execute via the same planner. IR is roundtrip-serializable: `Constraint → String → parse_query → identical Constraint` | Three surfaces, one engine |
| Index lifecycle | `BoxscoreIndex` is **per-season sharded** (38 small maps); only the active season's shard is hot. Cross-season queries iterate season-by-season, dropping each shard after evaluation. Both `BoxscoreIndex` and `CareerHistoryIndex` participate in `repo_swap` invalidation. | Caps resident set at one season (~4-6 MB) instead of 240 MB flat (R14) |
| Career-history coordination | Single `CareerHistoryFetcher` shared across A.2/A.3/A.4 with `Semaphore(4)` cap on concurrent landing fetches; atomic tmp+rename writes to `~/.icelines/career_history.json`; backoff inherited from existing `career_landing.rs` batch fetcher | Three sub-phases must not issue independent NHL API calls (R18, tape) |
| Performance budget | Cold `g.any10g>=5 EVER` over the full bundle: ≤8 s; warm: ≤2 s. Criterion harness committed at A.3 closeout. Short-circuit on first satisfying season for `AnyWindow`/`SeasonsWith`; `LifetimeSum` walks all 38. Parallel fetches: `tokio::join_all` with `Semaphore(4)`. | Concrete numbers before A.0 ships (R13) |
| Backward compatibility | Every filter expression that parsed in v0.19.1 continues to parse and produce the same results, **including the FIXED behavior of the 3 Wave 11 bugs** (goalie compound rewrite, paren-wrapped bio atoms, --filter+--week loud rejection). Wave 11 is run against the new pipeline at every A.x acceptance gate | Don't break users; don't reintroduce fixed bugs |
| Eligible-season tier for sliding windows | Boxscore-driven sliding windows: 2021-22 onward (where Foster +3 boxscore persistence covers the season). Pre-2021-22 seasons fall back to season aggregate with explicit `[fallback: 19891990]` marker emitted at `MaterializedSet` build time (deterministic JSON). `--strict=RejectPartialSeasons` rejects fallbacks. | Old-season provenance is honest (R17, tape) |

## Sub-phase ordering

```
A.0 ── IR + planner skeleton (the foundation)
         │
         ├─→ A.1 ── Grammar expansion (new operators + new atoms)
         │
         ├─→ A.2 ── Sliding-window atoms (over per-game boxscore data)
         │
         ├─→ A.3 ── Historical "ever" + at-age slicing
         │
         ├─→ A.4 ── Career-history atoms (cross-league)
         │
         └─→ A.5 ── --explain + cross-surface parity
```

A.0 is foundational — nothing else compiles until the IR exists. A.1
through A.4 can ship in any order on top of A.0; recommended order
(A.1 → A.2 → A.3 → A.4) puts user-visible features first. A.5 is the
visibility/parity layer; ships last so it can describe everything
that came before.

## Out of scope (deferred)

- **Continuous-time TOI windows** — "5 goals in any 60-min TOI
  window" needs shift-level data (NHL `gameLog` shift logs); not
  bundled and would balloon the data spec. Phase candidate: future
  Lady Byng (skill expression).
- **xG sliding windows** — xG is season-aggregate today; per-game xG
  would need MoneyPuck per-event data. Defer until / if MoneyPuck
  per-game lands.
- **Predictive queries** — "who's likely to break out next?" — needs
  modeling, not data engineering.
- **Filter aggregation across players** — "find pairs of linemates
  with combined p.last10g>=15" — needs a join axis we don't have.
- **GROUP BY / HAVING** — SQL-style group aggregates ("group by
  country, show top scorer per country"). Useful but not the core
  ask; future King Clancy maybe.
- **Saved queries / aliases** — `icelines query save "young-elite"
  --filter "age<=24 AND ppg>=1.0"`. Polish; not core.

## Surface coverage matrix

| Capability | CLI | TUI | Web |
|---|---|---|---|
| Unified parser entry | every `--filter` route | sort picker + filter overlay | `/leaders?filter=` form |
| `--explain` plan tree | `query leaders --filter X --explain` | new `e` keybind on query results | `?explain=1` query param |
| Sliding-window atoms | `--filter "g.last10g>=5"` | filter overlay accepts windowed atoms | form accepts windowed atoms |
| Historical "ever" | `--filter "g.any10g>=5 EVER"` | filter overlay | form |
| Career-league atoms | `--filter "league=OHL AND p.season>=80"` | (TUI shows league dropdown) | form |
| Strict comparators | `<` / `>` / `!=` | accepted | accepted |
| Set membership `IN` | `country IN (CAN,USA,SWE)` | accepted | accepted |
| `LIKE` patterns | `name LIKE "Mc*"` | accepted | accepted |

## Architecture: parser → planner → executor

```
   ┌─────────────────────────────────────────────┐
   │  Surface decoders                            │
   │   CLI:  String (clap-decoded)                │
   │   web:  String (URL-decoded)                 │
   │   TUI:  Vec<AtomFragment> (built directly)   │
   │  → all wrapped as FilterInput                 │
   └─────────────────────┬───────────────────────┘
                         │ FilterInput
                         ▼
   ┌─────────────────────────────────────────────┐
   │  parse_query(FilterInput)                    │
   │  → Result<QueryPlan, Vec<ParseError>>        │
   │  ▸ tokenize once (CLI/web only)              │
   │  ▸ build typed Constraint tree (n-ary)       │
   │  ▸ route atoms by key into typed variants    │
   │  ▸ collect ALL parse errors with span info   │
   └─────────────────────┬───────────────────────┘
                         │ QueryPlan
                         ▼
   ┌─────────────────────────────────────────────┐
   │  plan.requirements()                         │
   │  ▸ seasons / reports / boxscore dates / pids │
   │  ▸ estimated cost (boxscore reads, in meta)  │
   └─────────────────────┬───────────────────────┘
                         │ PlanRequirement
                         ▼
                  ┌────────────┐
                  │ StrictMode │ ◄── --strict gate fires HERE,
                  │   check    │     before any fetch
                  └──────┬─────┘
                         │
                         ▼
   ┌─────────────────────────────────────────────┐
   │  materialize(plan, ctx: EvalCtx)             │
   │  ▸ ctx.provider.ensure(req) — DataProvider   │
   │    trait owned by icelines-query, impl in    │
   │    icelines-fetch                            │
   │  ▸ aggregates per-game into windowed totals  │
   │  ▸ emits FetchEvent stream the surface       │
   │    renders (no library-side stderr writes)   │
   └─────────────────────┬───────────────────────┘
                         │ MaterializedSet
                         ▼
   ┌─────────────────────────────────────────────┐
   │  execute(plan, set)                          │
   │  ▸ walk players once                         │
   │  ▸ short-circuit n-ary All/Any               │
   │  ▸ evaluate Constraint tree per player       │
   └─────────────────────┬───────────────────────┘
                         │ Vec<PlayerRow>
                         ▼
                  (CLI / web / TUI render)
```

## Constraint IR — typed atom variants

```rust
pub enum Constraint {
    Bio(BioConstraint),                    // age, draft, country, pos, team, ...
    SeasonStat(SeasonStatConstraint),      // g.season>=20 (today's atoms)
    SlidingWindow(SlidingWindowConstraint),// g.last10g>=5, g.last30d>=10
    CareerAggregate(CareerAggrConstraint), // p.career>=500, g.any10g>=5 EVER
    CareerLeague(CareerLeagueConstraint),  // league=OHL, league IN (OHL,WHL)
    All(Vec<Constraint>),                  // n-ary AND
    Any(Vec<Constraint>),                  // n-ary OR
    Not(Box<Constraint>),
}

pub enum Predicate {
    Scalar(ScalarOp, ScalarValue),         // Eq/Ne/Lt/Le/Gt/Ge × Number|Text
    Member(MemberOp, Vec<ScalarValue>),    // In/NotIn — empty list rejected at parse
    Pattern(PatternOp, GlobPattern),       // Like/NotLike — NFD-normalized both sides
    Range(RangeBounds<f64>),               // Between (inclusive); numeric only
}

pub enum ScalarOp { Eq, Ne, Lt, Le, Gt, Ge }
pub enum MemberOp { In, NotIn }
pub enum PatternOp { Like, NotLike, Contains, NotContains }

pub enum ScalarValue {
    Number(f64),
    Text(String),  // canonical NFD-normalized + lowercased
}

pub enum SlidingWindow {
    LastN_GP { n: u8, scope: WindowScope, policy: WindowPolicy },
    LastN_Days { n: u16 },                 // calendar
    LastN_Weeks { n: u8 },                 // calendar
    LastN_Months { n: u8 },                // calendar
}

pub enum WindowScope {
    CurrentTeamCurrentSeason,              // default
    AllTeamsCurrentSeason,                 // .allteams modifier
    Career,                                // .career modifier
}

pub enum WindowPolicy {
    RequireFull,                           // default — GP < n returns false
    AllowPartial,                          // GP < n uses min(n, GP)
    AllowPartialAbove(u8),                 // partial OK if GP >= threshold
}

pub enum SeasonAxis { Regular, Playoff, All }

pub enum StrictMode {
    Off,
    RejectPartialSeasons,                  // any [fallback: <season>] errors
    RejectPartialWindows,                  // any [short-window: Ng] errors
    RejectAll,                             // both
}
```

Each variant carries enough information for `requirements()` to know
what data needs to be loaded. `BioConstraint` needs only the
identity layer (already loaded). `SlidingWindowConstraint` needs
boxscores in the relevant date range. `CareerAggrConstraint` needs
career-history data for active-roster pids. The shape-by-construction
discipline (`Predicate::Member` carries `Vec<ScalarValue>`,
`Pattern` carries `GlobPattern`) makes invalid states unrepresentable
— `LIKE 5` and `g IN 20` fail at parse, not at evaluate.

## DataProvider trait — the dependency-inversion seam

```rust
// icelines-query::data_provider

pub trait DataProvider: Send + Sync {
    /// Ensure the data described in `req` is available locally.
    /// Yields FetchEvent items as work progresses; the surface
    /// (CLI / web / TUI) renders them — never the library.
    fn ensure(&self, req: &PlanRequirement) -> FetchStream;
}

pub enum FetchEvent {
    Started { units: u32, label: String },
    Progress { done: u32, total: u32 },
    Complete,
    Failed { reason: FetchError },
}
```

Implemented in `icelines-fetch::query_provider::IcelinesProvider`,
which wraps `DataStore` + `NhlApiClient`. The CLI's `tokio::main`
constructs one and injects via `EvalCtx`; the web's axum handler
constructs one in app state; the TUI builds one inside the
`LocalSet` event loop. `EvalCtx` is `!Send` (carries
`StatsRepository` references); the executor must run via
`spawn_local`. A compile_fail doctest pins this.

## Decode boundaries

```rust
// icelines-query::input

pub enum FilterInput {
    /// CLI: clap has already shell-decoded the string.
    Cli(String),
    /// Web: the surface URL-decodes form values before
    /// constructing this variant.
    Form(String),
    /// TUI: the user builds atoms incrementally via the
    /// filter overlay; the surface composes a Vec<AtomFragment>
    /// directly without round-tripping through a string.
    Tui(Vec<AtomFragment>),
}
```

Each surface owns its decode. The TUI variant skips the tokenizer
entirely — it constructs `Constraint` directly from the overlay's
typed widget state. Round-trip property: every `Constraint` tree
serializes back to a canonical string that re-parses to an
identical tree (tested in A.5).

## Sliding-window atom grammar

| Atom | Window axis | Meaning | Data needed |
|---|---|---|---|
| `g.last10g>=5` | GP-counted | Last 10 games played, **current season + current team stint, contiguous** | Current-season boxscore shard |
| `g.last10g.allteams>=5` | GP-counted | Last 10 GP this season across all stints | Current-season boxscore shard |
| `g.last10g.career>=5` | GP-counted | Last 10 GP across season boundaries (career-tail) | Current + previous season boxscore shards |
| `g.last30d>=10` | Calendar | Last 30 days (any team) | Current-season boxscore shard, last 30 days |
| `g.last3w>=8` | Calendar | Last 21 days | Current-season boxscore shard, last 21 days |
| `g.last3m>=20` | Calendar | Last 90 days | Current-season boxscore shard, last 90 days |
| `g.any10g>=5 EVER` | GP-counted, intra-season | Did the player EVER have a 10-GP window in any season with ≥5 G | All eligible seasons (2021-22+) per-shard; older seasons fall back to season aggregate |
| `g.any10g>=5 EVER AT age<=22` | GP-counted, intra-season, age-sliced | Same, but only seasons where the player was ≤22 at HR age-as-of-Feb-1 | Bios + eligible seasons |
| `p.streak>=15` | GP-counted, longest run | Longest point streak in any season | Eligible seasons' boxscore shards |
| `g.season>=50 AT age<=22` | Season totals at age | Seasons where the player had ≥50 goals while ≤22 | Bios + every season's totals |

**WindowPolicy** (default `RequireFull`): a player with GP < window
returns false. `g.last10g>=5 :allow-partial` uses min(N, GP); the
result row carries a `[short-window: 7g]` marker. `:above-3` allows
partial when GP ≥ 3.

**Eligible seasons for boxscore-driven sliding windows**: 2021-22
onward (Foster +3 boxscore persistence covers these). Pre-2021-22
seasons fall back to season aggregate with `[fallback: 19891990]`
marker. `--strict=RejectPartialSeasons` rejects fallbacks at the
StrictMode gate (between `requirements()` and `materialize()`).

**Lockout handling**: `LOCKOUT_SEASONS = [20042005]` is **skipped**,
not partial-marked — there's no data to fall back to.

## Backward compatibility plan

- Every existing `--filter` shape continues to work. The new parser
  recognizes the legacy grammar as a subset of the unified grammar.
- Bare-array `--json` shapes (legacy `query leaders`) keep their
  shape; new K2.4-enveloped routes (`leaders.windowed`,
  `leaders.playoff`, `leaders.career`) are opt-in via `--envelope`
  flag during the transition. v0.20.0 ships both; v0.21.0 deprecates
  the legacy shape; v0.22.0 removes it. Phase Art Ross only ships
  v0.20.0.
- All 2056 existing tests stay green. New atom forms get new tests;
  legacy atoms keep their existing tests.

## Test budget

Estimated ~150-200 new tests across the wave, broken down:
- A.0 IR + planner: 30 (constraint construction, requirement
  computation, plan tree rendering)
- A.1 grammar expansion: 40 (each new operator + each new atom)
- A.2 sliding windows: 40 (per-game aggregation correctness, GP-
  counted vs calendar axis, `--strict` rejection)
- A.3 historical "ever": 30 (cross-season fanout, at-age slicing,
  partial-data handling)
- A.4 career-league: 25 (league atom parsing, career fanout, cross-
  league sums)
- A.5 --explain + parity: 25 (explain output stability, CLI/web/TUI
  produce same answer for same input)

Plus a Wave 12 (filter combinations on the new grammar) at ~150
scenarios, modeled on Wave 11.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Parser rewrite introduces regressions in existing filters | Run all 2056 v0.19.1 tests against the new parser before any new feature lands; backwards-compat checks at A.0 closeout |
| Sliding-window aggregation slow on large boxscore set | Index boxscores by date in DataStore; lazy-load per-season; cap "EVER" queries at MAX_SEASONS_FANNED=38 with progress feedback |
| On-demand fetch surprises users with network calls | Stderr banner + `--no-fetch` flag to refuse the operation if any data is missing |
| Career data thin for old seasons | Document the boundary: career-landing for active rosters only (~1 650 pids); pre-NHL career arcs from `career_history.json`; explicit "no data" marker for retired pre-1990 players |
| Test time grows | Keep wave persona tests at L2 subprocess (existing pattern); new IR / planner unit tests at L0 |

## Open questions — resolved by 8-role review

1. **`EVER` syntax**: keyword form `... EVER` stands. Per scout, EVER
   is intra-season only (no boundary crossing), and per edge it
   inherits the constraint's `axis: SeasonAxis` rather than mixing
   regular + playoff games. Lockout seasons are skipped (not
   partial-marked).
2. **`--explain` output**: both the constraint tree AND the data
   requirements ship in v1. JSON envelope:
   `{schema_version: "explain.v1", data: {plan_tree, requirements,
   estimated_cost}, meta}`. Frozen v1; additive only.
3. **`query career` vs `--filter "league=OHL"`**: per scout +
   keel, both ship. `query career` is a cohort leaderboard surface
   (existing); `--filter "league=OHL"` lets `query leaders` filter
   players by their cross-league career. Defined as
   `LeagueTier::Junior` not a hardcoded list. Resolved before A.4.
4. **`AT age<=22` syntax**: modifier form stands per scout — more
   flexible than synthetic atoms. Convention: Hockey-Reference
   Feb-1-of-second-season-year (already in `compute_age`). Missing
   birth_date errors loudly; per-game age is out of scope.

## Performance budget

- Cold `g.any10g>=5 EVER` over the full 38-season bundle: **≤8 s**.
- Warm (BoxscoreIndex shard hot): **≤2 s**.
- Criterion harness committed at A.3 closeout; benchmark gates the
  release. `READS_PER_SEC` calibration constant measured per build.
- Sliding-window aggregation: O(N) per player where N = window size
  (capped at 90 for `last3m`).
- Short-circuit on first satisfying season for `AnyWindow` /
  `SeasonsWith`-with-threshold; `LifetimeSum` walks all 38.
- Parallel boxscore fetch when on-demand: `tokio::join_all` with
  `Semaphore(4)` — friendly to NHL API.
- BoxscoreIndex memory footprint: ~4-6 MB per season shard; only
  the active shard is hot for non-EVER queries; EVER iterates
  shard-by-shard, dropping each after evaluation. Resident-set
  ceiling: one season shard at a time, regardless of EVER scope.

## Index lifecycle

- `BoxscoreIndex` and `CareerHistoryIndex` participate in
  `repo_swap` invalidation when the user presses `y` to switch
  seasons (TUI) or `--season YYYYZZZZ` is changed (CLI/web).
- Per-season sharding: each `BoxscoreIndex` shard keyed by season,
  built on first access, dropped from cache when LRU bounds are
  exceeded (cap = 4 shards = ~24 MB).
- `CareerHistoryIndex` is mostly season-invariant (career data
  doesn't change with season switches), but the `at_age` slice
  depends on bios from the active season — that slice cache
  invalidates on `repo_swap`.
- Index rebuild trigger: manifest version bump (Foster's pattern).

## Connection to prior work

- **Phase Foster** built the data architecture (DataStore,
  Manifest, sync engine, boxscore persistence). Art Ross is the
  consumer Foster was built for.
- **Phase Conn Smythe** added playoff-specific queries; the
  `--playoff` filter axis becomes a `SeasonStatConstraint::Playoff`
  variant in the new IR.
- **Phase Calder** added career-history fan-out; the cached
  `~/.icelines/career_history.json` is the data source for A.4
  career-league atoms.
- **Wave 11** surfaced 3 production bugs in the legacy filter
  pipeline that motivated the rewrite. The new IR makes those classes
  of bugs structurally impossible (typed atoms can't string-corrupt
  each other).
