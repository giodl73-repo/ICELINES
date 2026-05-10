# Phase Messier — execution plan v0.2

**Status**: Active - Messier.1-.6 first pass implemented
**Spec**: `design/specs/phase-messier-overview.md` v0.2
**Review note**: `design/notes/2026-05-08-phaseMessier-roles-review.md`
**Target release**: v0.24.0
**Estimated**: 6 sub-phases × ~half-day–1 day each (Messier.1 + .6 are larger)
**Test budget**: +95 planned; baseline must be re-measured after Phase Jennings.

---

## Hard preflight

Phase Messier MUST NOT begin until Phase Jennings and Phase Campbell close. The
test budget above is the older v0.23.5 estimate; the baseline count must be
re-measured after Jennings and refreshed here if it differs.

Required Jennings exit state:

- `cargo check --workspace` green.
- `cargo test --workspace --no-fail-fast` compiles and runs.
- The `Config` struct-literal drift class has a structural fix
  (`Default`, `test_default`, or equivalent builder).
- Phase Campbell has created `design/specs/platform-contracts.md` and the
  initial ViewModel contract types for Leaders/TeamDepth/Goalies.
- `design/plans/INDEX.md` records the measured post-Jennings baseline.
- This plan's cumulative test counts are refreshed if the measured baseline
  differs from the older estimate.

Reason: Messier is a cross-screen refactor. KEEL/BENCH reject starting a
multi-commit migration from a broken full-suite baseline.

---

## Platform contracts consumed

Messier consumes `design/specs/platform-contracts.md` this way:

- **Data context**: filter caches and ViewModels include active
  `(season, season_type)` and source/completeness state.
- **Query/filter intent**: TUI shortcuts and cmdbar kv grammar lower to one
  typed filter/sort state.
- **ViewModel**: TeamDepth and Goalies filter state eventually render through
  Campbell ViewModels rather than screen-local row shapes.
- **Surface parity**: CLI/web parity remains follow-up work for Lester Patrick
  and Ted Lindsay, but the behavior is described in shared contract terms.
- **Visual language**: chrome/hint rows expose semantic tokens and active
  filters; renderer-specific styling stays out of core filter logic.

---

## Role review gates

| Role | Messier gate |
|---|---|
| HART | `RosterFilterState` and `FilterCache` must invalidate on an explicit semantic key: `(season, season_type, repo_generation, filter_signature)`. Pointer identity is not a cache key. |
| KEEL | Messier is TUI-first. Any CLI/web parity created by the new grammar is documented as a follow-up for Lester Patrick/Ted Lindsay, not silently promised here. |
| TAPE | Nationality/country filters read identity/bio fields only; missing bio data excludes rows rather than defaulting to a country. |
| FORGE | New filter types make invalid state unrepresentable; no `unwrap()` in production paths except documented impossible invariants. |
| PACE | Filter cost claims are measured or marked estimates; render path must not call expensive query matching on every frame. |
| BENCH | Every keybind added to a screen has L0/L1 coverage and a chrome/hint-row assertion. |
| EDGE | Tests cover GP=0/BelowThreshold, no country, bad country, duplicate kv keys, and repo swap invalidation. |
| WIRE | AI prompt v2 remains a translation hint only; deterministic parser validates any command before execution. |
| SCOUT | Goalie Starter/Backup language documents the GP-share heuristic and does not imply true coaching deployment. |
| GLASS | Per-screen chrome/hint rows are updated in the same sub-phase as behavior; no hidden keybinds. |

---

## Decisions resolved (post user decision A-E)

- **A**: rename `c` → `n` (nationality)
- **B**: Goalies role-class via GP-share-of-team-minutes (≥60%)
- **C**: invest in type modeling — `CountryCode([u8;3])`, typed
  `RosterKvArgs`, `ForcedColumns: bitflags!`
- **D**: 95-test realistic budget
- **E**: filter-chain memoization in Messier.1

---

## Sub-phase Messier.1 — RosterFilterState extraction + types + memoization

### Files

- **NEW**: `icelines-cli/src/tui/filter_state.rs` — types module
- **NEW**: `icelines-cli/src/tui/filter_state/cache.rs` (or inline) — `FilterCache`
- **MODIFY**: `icelines-cli/src/tui/mod.rs` — register module
- **MODIFY**: `icelines-cli/src/tui/screens/team.rs` — embed `filters: RosterFilterState`; rename `TeamPosFilter` → `PosFilter`; replace `country_filter: Option<&'static str>` → `Option<CountryCode>`; replace `force_hits_column: bool` → `forced_columns: ForcedColumns`
- **MODIFY**: `icelines-cli/src/tui/app.rs` — `pub team: TeamScreenState` (no shape change)
- **NEW**: `icelines-cli/tests/messier_1_parity_snapshot.rs` — insta golden harness
- **NEW**: `icelines-cli/benches/filter_chain.rs` — criterion perf check (advisory unless Jim Gregory makes benches blocking)
- **MODIFY**: `icelines-cli/Cargo.toml` — add `bitflags`; add `insta` and `criterion` as dev-dependencies only. Do not add `enumset` unless implementation proves `bitflags` insufficient.

### Code sketch

```rust
// tui/filter_state.rs
use std::sync::Arc;
use icelines_core::stats_repository::PlayerView;
use icelines_query::QueryPlan;

// ── CountryCode newtype (FORGE #1) ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CountryCode([u8; 3]);

#[derive(Debug, Clone, thiserror::Error)]
pub enum CountryCodeError {
    #[error("country code must be 3 ASCII letters; got {0:?}")]
    NotThreeAscii(String),
}

impl CountryCode {
    pub fn parse(s: &str) -> Result<Self, CountryCodeError> {
        let trimmed = s.trim();
        if trimmed.len() != 3 || !trimmed.is_ascii() {
            return Err(CountryCodeError::NotThreeAscii(s.to_owned()));
        }
        let mut bytes = [0u8; 3];
        for (i, b) in trimmed.as_bytes().iter().enumerate() {
            bytes[i] = b.to_ascii_uppercase();
        }
        Ok(Self(bytes))
    }
    pub fn as_str(&self) -> &str {
        // Safe — constructor enforces ASCII.
        std::str::from_utf8(&self.0).unwrap_or("???")
    }
    pub const CAN: CountryCode = CountryCode([b'C', b'A', b'N']);
    pub const USA: CountryCode = CountryCode([b'U', b'S', b'A']);
    pub const SWE: CountryCode = CountryCode([b'S', b'W', b'E']);
    pub const FIN: CountryCode = CountryCode([b'F', b'I', b'N']);
    pub const RUS: CountryCode = CountryCode([b'R', b'U', b'S']);
    pub const CZE: CountryCode = CountryCode([b'C', b'Z', b'E']);
    pub const SVK: CountryCode = CountryCode([b'S', b'V', b'K']);
}

pub const COUNTRY_CYCLE: &[CountryCode] = &[
    CountryCode::CAN, CountryCode::USA, CountryCode::SWE,
    CountryCode::FIN, CountryCode::RUS, CountryCode::CZE,
    CountryCode::SVK,
];

// ── PosFilter / GoalieRoleFilter (FORGE #8) ──────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PosFilter {
    #[default] All, Forwards, Defense, C, LW, RW, LD, RD,
}

impl PosFilter {
    pub fn next(self) -> Self { /* cycle */ }
    pub fn matches(self, abbrev: &str) -> bool { /* shared predicate */ }
    pub fn label(self) -> &'static str { /* "All"/"F"/.../"RD" */ }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GoalieRoleFilter {
    #[default] All, Starters, Backups,
}

// ── ForcedColumns (FORGE #9) ─────────────────────────────────

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct ForcedColumns: u8 {
        const HITS   = 0b00000001;
        const BLOCKS = 0b00000010;
        const TOI    = 0b00000100;
        const SAVES  = 0b00001000;
    }
}

// ── RosterFilterState + memoization (PACE #1) ────────────────

#[derive(Debug, Clone, Default)]
pub struct RosterFilterState {
    pub pos_filter: PosFilter,
    pub country_filter: Option<CountryCode>,
    pub min_gp: u32,
    pub forced_columns: ForcedColumns,
    pub free_filter: Option<Arc<QueryPlan>>,
    cached: Option<FilterCache>,
}

#[derive(Debug, Clone)]
struct FilterCache {
    season: SeasonId,
    season_type: SeasonType,
    repo_generation: u64,
    filter_signature: u64,
    filtered_pids: Vec<icelines_core::identity::PlayerId>,
}

impl RosterFilterState {
    /// Phase Messier — invalidate the memoized cache on input.
    /// Called by the keybind handlers and cmdbar dispatcher.
    pub fn invalidate(&mut self) {
        self.cached = None;
    }

    /// Phase Messier — compute (or reuse cached) filtered pid
    /// list. Caller passes active season/type and repo_generation to detect
    /// repo swaps and time-travel. Pointer addresses are never used as cache
    /// identity.
    pub fn filter<'a>(
        &mut self,
        views: &'a [PlayerView<'a>],
        season: SeasonId,
        season_type: SeasonType,
        repo_generation: u64,
    ) -> &[icelines_core::identity::PlayerId] {
        let filter_signature = self.compute_filter_signature();
        if !matches!(&self.cached, Some(c)
            if c.season == season
                && c.season_type == season_type
                && c.repo_generation == repo_generation
                && c.filter_signature == filter_signature)
        {
            self.cached = Some(FilterCache {
                season,
                season_type,
                repo_generation,
                filter_signature,
                filtered_pids: self.compute_filtered_pids(views),
            });
        }
        &self.cached.as_ref().unwrap().filtered_pids
    }

    fn compute_filtered_pids(&self, views: &[PlayerView<'_>]) -> Vec<_> {
        // Single-pass filter (PACE #2)
        views.iter()
            .filter(|v| self.pos_filter.matches(v.position().abbreviation()))
            .filter(|v| match self.country_filter {
                None => true,
                Some(cc) => v.identity.bio.nationality_code
                    .as_deref()
                    .and_then(|s| CountryCode::parse(s).ok())
                    .map(|got| got == cc)
                    .unwrap_or(false),
            })
            .filter(|v| v.gp() >= self.min_gp)
            .filter(|v| match &self.free_filter {
                None => true,
                Some(plan) => /* eval via Constraint::matches with EvalCtx */,
            })
            .map(|v| v.id())
            .collect()
    }

    fn compute_filter_signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.pos_filter.hash(&mut h);
        self.country_filter.hash(&mut h);
        self.min_gp.hash(&mut h);
        self.forced_columns.bits().hash(&mut h);
        self.free_filter
            .as_ref()
            .map(|p| p.stable_signature())
            .hash(&mut h);
        h.finish()
    }
}

// ── Typed RosterKvArgs (FORGE #2 / EDGE #6) ──────────────────

#[derive(Debug, Clone, Default)]
pub struct RosterKvArgs {
    pub sort: Option<String>,           // resolved per verb
    pub pos: Option<PosFilter>,
    pub country: Option<CountryCode>,
    pub min_gp: Option<u32>,
    pub forced_columns: ForcedColumns,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum KvParseError {
    #[error("unknown key {key:?} — try sort, pos, country/nationality, min-gp, hits")]
    UnknownKey { key: String },
    #[error("duplicate key {0:?}")]
    DuplicateKey(String),
    #[error("kv pair after positional only — got {token:?}")]
    PositionalAfterKv { token: String },
    #[error("invalid {key} value {raw:?}: {reason}")]
    InvalidValue { key: &'static str, raw: String, reason: String },
}

pub fn parse_roster_kv(tokens: &[Token]) -> Result<RosterKvArgs, KvParseError> {
    /* per-key dispatch with duplicate detection */
}
```

### Migration shim (FORGE #39)

In Messier.1's commit, `team.rs` keeps `pub use PosFilter as TeamPosFilter;`
so all Adams.10/.12 tests pass without rename churn. The shim is removed
in a Messier.1-followup commit before Messier.2 lands.

### Insta parity harness (BENCH #5)

```rust
// icelines-cli/tests/messier_1_parity_snapshot.rs
use insta::assert_snapshot;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[test]
fn messier_1_team_screen_renders_unchanged() {
    // Set up app with bundled fixture (frozen — Wayne Gretzky era,
    // see CLAUDE.md fixture pattern).
    let app = build_canonical_app();
    let backend = TestBackend::new(160, 50);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| crate::tui::screens::render(f, &app)).unwrap();
    let buf = buffer_to_string(term.backend().buffer());
    assert_snapshot!("team_screen_default", buf);
}
```

Snapshot file `messier_1_parity_snapshot__team_screen_default.snap`
checked in. CI compares post-refactor output byte-for-byte.

### Perf L0 (PACE acceptance)

```rust
// icelines-cli/benches/filter_chain.rs (criterion)
fn bench_filter_chain(c: &mut Criterion) {
    let views: Vec<PlayerView> = build_700_player_fixture();
    let mut state = RosterFilterState {
        pos_filter: PosFilter::Forwards,
        country_filter: Some(CountryCode::CAN),
        min_gp: 20,
        ..Default::default()
    };
    c.bench_function("filter_chain_n700", |b| {
        b.iter(|| {
            state.invalidate();
            black_box(state.filter(black_box(&views), 1));
        })
    });
}
```

Acceptance: median time recorded against the canonical fixture. The old target
is <= 1ms, but the gate is "measured and not accidentally render-path
expensive" until Jim Gregory decides whether benches are blocking.

### Gauntlet

- All Adams.10/.12 tests pass with the new struct shape (via shim).
- `cargo build` clean; clippy clean for new module.
- Insta snapshot diff = 0 bytes.
- Criterion bench records median and fixture size; <= 1ms remains the target,
  but CI blocking status is deferred to Jim Gregory.
- No new warnings in icelines-cli.

### Acceptance

Pure refactor + cache layer. Behavior identical to v0.23.5 by insta.

### Closeout so far

- Added `icelines-cli/src/tui/filter_state.rs` with `CountryCode`,
  `PosFilter`, `ForcedColumns`, `RosterFilterState`, semantic filter
  signatures, and a cache shell keyed by `(season, season_type,
  repo_generation, filter_signature)`.
- Migrated Team screen state to embed `RosterFilterState` while preserving the
  existing `s`/`p`/`c`/`h` behavior and render output semantics.
- Kept `TeamPosFilter` as a test-only compatibility alias while production code
  uses the shared `PosFilter`.
- Verified with focused bin tests for Messier filter-state, Team L0 behavior,
  Team keybind L1 behavior, `cargo fmt --check`, and
  `cargo check -p icelines-cli --bin icelines`.

---

## Sub-phase Messier.2 — Goalies adopts standard matrix

### Files

- **MODIFY**: `icelines-cli/src/tui/screens/goalies.rs` — embed
  `filters: RosterFilterState`, add `role_filter: GoalieRoleFilter`,
  add `role_threshold_for_team: HashMap<TeamAbbr, GpSharePoint>`.
- **MODIFY**: `icelines-cli/src/tui/app.rs` — add `p`/`n`/`h`/`f` arms.

### Goalies role-class (decision B)

GP-share-of-team-minutes, ≥60% threshold:

```rust
fn compute_role_threshold(team_goalies: &[&PlayerView<'_>]) -> Option<GpSharePoint> {
    let total_team_gp: u32 = team_goalies.iter().map(|v| v.gp()).sum();
    if total_team_gp == 0 { return None; }
    let threshold = (total_team_gp * 60) / 100;
    Some(GpSharePoint { team_gp_threshold: threshold })
}

// Render-time predicate
fn matches_role(role: GoalieRoleFilter, view: &PlayerView, threshold: u32) -> bool {
    match role {
        GoalieRoleFilter::All => true,
        GoalieRoleFilter::Starters => view.gp() >= threshold,
        GoalieRoleFilter::Backups => view.gp() < threshold,
    }
}
```

Threshold computed once on screen entry (or `repo_swap`); cached per
team.

Chrome title shows the threshold:
`Goalies · pos=Starters(GP≥27) · nationality=CAN`.

### Code sketch (handler)

```rust
} else if self.screen == Screen::Goalies && c == 'p' {
    self.goalies.role_filter = self.goalies.role_filter.next();
    self.goalies.filters.invalidate();
    self.selected = 0;
    self.status = format!("Goalies role: {}", self.goalies.role_filter.label());
} else if self.screen == Screen::Goalies && c == 'n' {
    cycle_country(&mut self.goalies.filters);
    self.selected = 0;
    self.status = format!("Goalies nationality: {}", country_label(&self.goalies.filters));
} else if self.screen == Screen::Goalies && c == 'h' {
    self.goalies.filters.forced_columns.toggle(ForcedColumns::SAVES);
    self.status = format!("Goalies Saves col: {}", on_off(&self.goalies.filters.forced_columns, ForcedColumns::SAVES));
} else if self.screen == Screen::Goalies && c == 'f' {
    /* open free-form filter overlay — same as Stats's f */
    self.show_free_filter_for(Screen::Goalies);
}
```

### Test budget (BENCH #18 calibrated)

- 6 L0 (cycles + chrome + threshold computation)
- 6 L1 (each keybind dispatch end-to-end)
- 2 L1 (insta golden Goalies-default + Goalies-with-filters)
- 2 L0 (`forced_columns::SAVES` toggle invariants — idempotent
  toggle, no dedup needed because bitflags)

Total: 16

### Acceptance

Press `:goalies` Enter, then `p` `p` `p` cycles All → Starters →
Backups → All. Press `n` cycles countries. Press `h` toggles Saves
column. Press `f` opens free-form overlay. Per-screen hint row shows
all six.

### Closeout so far

Implemented in the first Messier pass:

- `GoaliesState` owns `RosterFilterState` and `GoalieRoleFilter`.
- `p`, `n`, and `h` handlers cycle role, nationality, and forced saves
  column state; chrome and footer expose the active state.
- Goalie render, ViewModel construction, and Enter navigation use the same
  filtered row set.
- Starter/backup remains the documented GP-share heuristic, not a deployment
  claim.
- `f` pre-fills the MDI command bar with `goalies ` so the shared KV grammar
  is the free-form path for goalie filters.

---

## Sub-phase Messier.3 — Stats `n` country shortcut + cmdbar parity

### Files

- **MODIFY**: `icelines-cli/src/tui/screens/queries.rs` — extend
  chrome with `n`.
- **MODIFY**: `icelines-cli/src/tui/app.rs` — add Char('n') arm for
  Stats screen.
- **MODIFY**: `icelines-cli/src/tui/command.rs` — `:stats country=X`
  and `:stats nationality=X` both lower to Art Ross atom.

### Code sketch

```rust
// app.rs handler
} else if self.screen == Screen::Queries && c == 'n' {
    self.queries.mode = QueryMode::FilterEdit;
    self.queries.filter_text = "nationality=".to_owned();
    self.status = "Type a 3-letter country code, Enter to apply".to_owned();
}

// command.rs (Messier.6 lands the full kv path; Messier.3 stubs it)
fn lower_stats_kv_to_atom(args: &RosterKvArgs) -> String {
    let mut atoms: Vec<String> = Vec::new();
    if let Some(cc) = args.country {
        atoms.push(format!("nationality={}", cc.as_str()));
    }
    if let Some(min_gp) = args.min_gp {
        atoms.push(format!("gp >= {min_gp}"));
    }
    if let Some(pos) = args.pos {
        if pos != PosFilter::All {
            atoms.push(format!("pos={}", pos.label()));
        }
    }
    atoms.join(" AND ")
}
```

### Round-trip test (EDGE #4 acceptance)

```rust
#[test]
fn l1_messier_3_stats_kv_lowers_to_same_ir_as_typed() {
    let mut app1 = fresh_mdi_app();
    type_cmd(&mut app1, ":stats country=CAN");
    submit(&mut app1);

    let mut app2 = fresh_mdi_app();
    app2.screen = Screen::Queries;
    app2.handle(Action::Char('n'));   // opens FilterEdit with "nationality="
    for c in "CAN".chars() {
        app2.handle(Action::Char(c));
    }
    app2.handle(Action::Enter);

    // Both produce an Art Ross plan with one Bio atom for nationality=CAN
    assert_eq!(plan_signature(&app1.queries.filter_plan),
               plan_signature(&app2.queries.filter_plan));
}
```

### Test budget

- 4 L0 (`lower_stats_kv_to_atom` for each kv combination)
- 4 L1 (`n` shortcut behavior, `:stats country=` lowering, round-trip
  IR equality, error path for bad country)
- 2 L1 (insta golden Stats-with-nationality-filter)

Total: 10

### Acceptance

`n` on Stats opens FilterEdit pre-filled. Chrome row advertises
`n=nationality`. Cmdbar `:stats country=CAN` produces identical
plan IR. AI fallback can emit either form.

### Closeout so far

Implemented in the first Messier pass:

- Stats `n` opens FilterEdit prefilled with `nationality=`.
- Query screen chrome advertises `n=nation`.
- `stats nationality=CAN pos=LW min-gp=20` lowers through the deterministic
  command parser to the Art Ross filter expression
  `nationality=CAN AND pos=LW AND gp>=20`.

---

## Sub-phase Messier.4 — Depth position + nationality + free-form

### Files

- **MODIFY**: `icelines-cli/src/tui/screens/depth.rs` — embed
  `filters: RosterFilterState`, add `p`/`n`/`f` keybinds.
- **MODIFY**: `icelines-cli/src/tui/app.rs` — handler arms.

### Code sketch

```rust
// depth.rs
pub struct DepthScreenState {
    pub filters: RosterFilterState,
}

pub fn chrome(mode: ScoringMode, state: &DepthScreenState) -> ScreenChrome {
    let title = format!(
        "Depth · scoring={} · pos={} · {}",
        mode.label(),
        state.filters.pos_filter.label(),
        country_chip(&state.filters)
    );
    let keybinds = vec![
        KeyHint::new("s", "toggle scoring"),
        KeyHint::new("p", "cycle pos"),
        KeyHint::new("n", "cycle nation"),
        KeyHint::new("f", "free filter"),
        KeyHint::new("↑↓", "select"),
        KeyHint::new("Enter", "team chart"),
    ];
    ScreenChrome { title, keybinds }
}
```

### Test budget

- 4 L0 (cycles + chrome + filter state defaults)
- 4 L1 (each keybind dispatch + filtered result count)
- 2 L1 (insta golden default + with-filters)
- 2 L0 (cache invalidation on `s` (scoring) — should NOT invalidate
  filter cache; verify)

Total: 12

### Acceptance

8 → 12 tests. Per-screen hint row shows the new keybinds. Scoring
toggle remains independent of filter chain.

### Closeout so far

Implemented in the first Messier pass:

- `App` owns `depth_filters: RosterFilterState`.
- Depth chrome exposes scoring, position, and nationality state.
- `p` and `n` handlers mutate typed depth filters and reset selection.
- League and team depth computations consume filtered player views before
  computing team strength.
- `f` pre-fills the MDI command bar with `depth ` so the shared KV grammar is
  the free-form path for Depth filters.

---

## Sub-phase Messier.5 — Favorites sort + nationality + free-form

### Files

- **MODIFY**: `icelines-cli/src/tui/screens/favorites.rs` — add
  state struct, sort/filter logic, expanded chrome.
- **MODIFY**: `icelines-cli/src/tui/app.rs` — handler arms.

### Code sketch

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FavoritesSort {
    #[default] RecentlyAdded,
    Name,
    Position,
    Team,
}

pub struct FavoritesScreenState {
    pub sort: FavoritesSort,
    pub filters: RosterFilterState,
}
```

### Test budget

- 4 L0 (sort cycle + chrome)
- 6 L1 (each keybind dispatch + sort ordering correctness)
- 2 L1 (insta golden empty-favs / with-favs)

Total: 12

### Acceptance

`s` cycles RecentlyAdded → Name → Pos → Team → wrap. `p` / `n` / `f`
standard. Sort changes order deterministically.

### Closeout so far

Implemented in the first Messier pass:

- `FavoritesScreenState` adds sort and `RosterFilterState`.
- Favorites chrome advertises sort, position, nationality, and free-form
  controls.
- App handlers for `s`, `p`, `n`, and `f` update state and status.
- Fallback member-list rendering now honors `sort=Recent`, `sort=Name`, and
  `sort=Kind`.
- Live `FavoritesView` player rows and fallback member-list player rows now
  apply position, nationality, and min-GP filters by resolving row player IDs
  against active `PlayerView`s. Team favorites remain visible because roster
  filters describe players, not teams.
- `f` pre-fills the MDI command bar with `favorites ` so the shared KV grammar
  is the free-form path for Favorites filters.

---

## Sub-phase Messier.6 — Cmdbar verb-kv grammar + AI prompt v2

### Files

- **MODIFY**: `icelines-cli/src/tui/command.rs` — extend
  `parse_command` with positional + kv form.
- **MODIFY**: `icelines-cli/src/tui/command.rs` — typed `RosterKvArgs`
  in `Command` variants.
- **MODIFY**: `icelines-cli/src/tui/command.rs` — `execute_command`
  per-verb `apply_kv`.
- **MODIFY**: `icelines-cli/src/ai.rs` — `SYSTEM_PROMPT_VERSION = "v2"`,
  prompt landmarks updated, 4 new few-shot examples.
- **MODIFY**: `icelines-cli/src/tui/persona_jack_adams.rs` — add
  scenarios for `:goalies sort=gaa min-gp=20`, `:team EDM pos=LW
  nationality=CAN`, error paths.

### Grammar implementation

```rust
// Quoted-string lexer reused from IN/LIKE path
fn tokenize_with_quotes(s: &str) -> Vec<Token> { /* existing */ }

// Generic kv parser used by all verbs
fn parse_roster_kv_after_positional(
    tokens: &[Token],
) -> Result<RosterKvArgs, KvParseError> {
    let mut args = RosterKvArgs::default();
    let mut seen_kv = false;
    let mut seen_keys: HashSet<&'static str> = HashSet::new();

    for tok in tokens {
        match tok {
            Token::Bare(s) if !s.contains('=') => {
                if seen_kv {
                    return Err(KvParseError::PositionalAfterKv {
                        token: s.to_owned(),
                    });
                }
                // Caller has already consumed positional args;
                // this is an unrecognized positional modifier.
                return Err(KvParseError::UnknownKey { key: s.to_owned() });
            }
            Token::Bare(s) | Token::Quoted(s) => {
                let (key, value) = s.split_once('=')
                    .ok_or_else(|| KvParseError::InvalidValue {
                        key: "(missing equals)",
                        raw: s.to_owned(),
                        reason: "expected key=value".to_owned(),
                    })?;
                seen_kv = true;
                if !seen_keys.insert(canonical_key(key)) {
                    return Err(KvParseError::DuplicateKey(key.to_owned()));
                }
                apply_kv_to_args(&mut args, key, value)?;
            }
        }
    }
    Ok(args)
}

fn apply_kv_to_args(args: &mut RosterKvArgs, key: &str, raw: &str) -> Result<(), KvParseError> {
    match canonical_key(key) {
        "sort" => args.sort = Some(raw.to_owned()),
        "pos" => args.pos = Some(PosFilter::parse_loose(raw)?),
        "country" | "nationality" => {
            args.country = Some(CountryCode::parse(raw)
                .map_err(|e| KvParseError::InvalidValue {
                    key: "country",
                    raw: raw.to_owned(),
                    reason: e.to_string(),
                })?);
        }
        "min-gp" | "min_gp" => args.min_gp = Some(raw.parse().map_err(|_| KvParseError::InvalidValue {
            key: "min-gp",
            raw: raw.to_owned(),
            reason: "expected non-negative integer".to_owned(),
        })?),
        "hits" => match raw {
            "on" => args.forced_columns |= ForcedColumns::HITS,
            "off" => args.forced_columns &= !ForcedColumns::HITS,
            other => return Err(KvParseError::InvalidValue {
                key: "hits", raw: other.to_owned(),
                reason: "expected on or off".to_owned(),
            }),
        },
        // … more keys
        unknown => return Err(KvParseError::UnknownKey { key: unknown.to_owned() }),
    }
    Ok(())
}

fn canonical_key(k: &str) -> &str {
    match k {
        "country" => "country",  // alias for nationality on non-Stats
        "nationality" => "country",
        "min_gp" => "min-gp",
        other => other,
    }
}
```

### AI prompt v2 (WIRE)

```rust
// ai.rs
pub const SYSTEM_PROMPT_VERSION: &str = "v2";

// In default_system_prompt(), add:
//   - kv form examples ("sort=gaa", "min-gp=20", "nationality=CAN")
//   - Disambiguating few-shots for screen-targeted vs stat-query intent

#[test]
fn l0_messier_6_system_prompt_v2_landmarks() {
    let s = default_system_prompt();
    for landmark in &[
        "sort=gaa",
        ":goalies",
        "min-gp=20",
        "nationality",
        "pos=LW",
    ] {
        assert!(
            s.contains(landmark),
            "v2 prompt missing landmark {landmark:?}"
        );
    }
}

#[test]
fn l0_messier_6_system_prompt_version_is_v2() {
    assert_eq!(SYSTEM_PROMPT_VERSION, "v2");
}
```

### Test budget (BENCH #19 calibrated)

- 12 L0 — `parse_roster_kv` (success + each error variant + quoted
  values + canonical key aliases)
- 10 L1 — per-verb dispatch: `:goalies`, `:team`, `:stats`,
  `:depth`, `:favorites`, each happy + error path
- 5 L0 — `apply_kv_to_args` per key
- 2 L0 — AI prompt v2 landmarks + version
- 4 L1 — persona harness deltas (existing scenarios that exercised
  `s` on multiple screens get new assertions for the standardized
  matrix)

Total: 33

### Acceptance

Power user drives every per-screen filter dimension from cmdbar.
AI fallback gains kv form. SYSTEM_PROMPT_VERSION="v2" in single
commit. Phase Jennings records the measured pre-Messier baseline; Messier
adds the planned +95 tests on top of that measured count.

### Closeout so far

Implemented in the first Messier pass:

- `RosterKvArgs` and `parse_roster_kv` parse typed `sort`, `pos`, `country` /
  `nationality`, `min-gp`, `hits`, and `saves` keys with duplicate-key and
  invalid-value errors.
- `Command::GoaliesKv` applies sort, min-GP, nationality, position, and
  explicit saves-column state to the Goalies screen.
- `Command::DepthKv`, `Command::FavoritesKv`, and `Command::TeamKv` apply the
  same typed KV state to the existing screen states.
- `stats ...` KV input lowers to a validated Art Ross query expression.
- `SYSTEM_PROMPT_VERSION` is now `v2`; the prompt includes roster KV examples
  and `:goalies sort=gaa min-gp=20` landmarks while still requiring canonical
  output without a leading colon.
- Jack Adams persona scenarios now exercise `goalies`, `depth`, `favorites`,
  and `team EDM` KV flows.
- Favorites live-stat rows now apply roster filters via `EntityRef::Player`
  -> active `PlayerView` lookup instead of display-string inference.

---

## Risks (post-review v0.2)

1. **Bitflags vs EnumSet** — picked `bitflags` (zero new dep
   beyond what already exists in icelines-core). EnumSet was
   FORGE's first suggestion; bitflags is equivalent for our 8
   columns max. Mitigation: documented inline.

2. **CountryCode UTF-8 invariant** — `as_str()` does a runtime
   `from_utf8`. Since `parse()` enforces ASCII, this is always
   `Ok(_)`. Mitigation: `as_str()` uses `unwrap_or("???")`
   defensive default, never panics.

3. **`free_filter` Arc costs** — each filter clone increments
   atomic refcount. At ~10fps, that's 10 atomics/sec per screen.
   Negligible. Mitigation: documented; don't clone in render path
   (use `as_ref()`).

4. **Insta snapshot churn** — `messier_1_team_screen_renders_unchanged`
   fails if anyone touches Team rendering. That's the *point* —
   intentional changes update the snapshot via `cargo insta review`.
   Mitigation: documented in CONTRIBUTING.md (need to add).

5. **`canonical_key` aliasing** — `country` ↔ `nationality` makes
   error messages ambiguous (`unknown key "nationality"` vs
   `unknown key "country"`). Mitigation: error preserves user's
   original spelling; canonicalization is internal-only.

6. **AI prompt cache invalidation** — bumping version mid-phase
   would cause 6 cache misses across Messier.1-6. Locked: bump
   only in Messier.6 commit (single miss).

7. **Goalies threshold ambiguity at season start** — when total
   team GP = 0, `compute_role_threshold` returns None; predicate
   degrades to `All`. Documented; surfaced in chrome title as
   `pos=Starters(early-season)`.

8. **Dev dependency creep** — Messier uses `insta` and `criterion` as
   dev-dependencies only. Snapshot tests are blocking; benches are advisory
   until Jim Gregory sets CI policy. `bitflags` is the only planned runtime
   dependency.

---

## Acceptance for v0.24.0 ship

Inherits from spec acceptance. Plus:

- Plan v0.2 reviewed by user before Messier.1 commits.
- Messier.1 lands as a separate commit with insta snapshot baseline
  + criterion bench.
- Each subsequent Messier.X lands as its own commit; v0.24.0
  releases when Messier.6 closes.
- COMMANDS.md gets a unified per-screen keybind table.
- CHANGELOG.md gets the cumulative v0.24.0 entry.

---

## Test budget v0.2 summary

The original v0.23.5 estimate was 1051 pre-Messier tests and 1146 after
the planned +95 additions. Phase Jennings now owns the measured baseline;
refresh the cumulative column before Messier.1 starts.

| Sub-phase | Tests added | Cumulative |
|---|---|---|
| Pre-Messier | — | TBD after Jennings |
| Messier.1 | +12 | baseline + 12 |
| Messier.2 | +16 | baseline + 28 |
| Messier.3 | +10 | baseline + 38 |
| Messier.4 | +12 | baseline + 50 |
| Messier.5 | +12 | baseline + 62 |
| Messier.6 | +33 | baseline + 95 |
| **Total** | **+95** | **baseline + 95** |
