# Phase Messier — TUI cross-screen filter/sort consistency

**Trophy**: Mark Messier Leadership Award. Fit: "every screen leads
by example with the same consistent UX." Filtering/sorting on every
player-list screen converges on a single mental model.

**Builds on**: Phase Norris (state extraction), Phase Masterton
(declarative chrome), Phase Jack Adams (MDI dashboard + cmdbar +
per-screen hint row).

**Status**: spec v0.2 — 2026-05-08. Updated post 6-role review
(`design/notes/2026-05-08-phaseMessier-roles-review.md`); 7 BLOCKING
items resolved + 5 user decisions A-E folded in.

---

## Headline

Phase Messier standardizes the keybind matrix across every player-list
screen, hoists per-screen filter state into a shared
`RosterFilterState`, and adds cmdbar verb-kv grammar parity so every
filter is drivable from natural language *and* keybinds *and* the AI
fallback.

After Messier, switching workspace screens never requires re-learning
keys.

## Precondition

Phase Messier starts only after Phase Jennings restores a green workspace
baseline and Phase Campbell establishes the platform contracts/ViewModel path.
Any cumulative test numbers in this spec or the execution plan are estimates
until Jennings closes.

---

## Locked decisions (post-review v0.2)

### 1. Standard keybind matrix

| Key | Concept | Notes |
|---|---|---|
| `s` | cycle sort | per-screen sort enum |
| `p` | cycle position filter | shared `PosFilter` enum (skater axis) |
| `n` | cycle nationality filter | renamed from `c` per GLASS review |
| `h` | toggle headline column (Hits/Saves) | independent of sort; resolved column shown in chip |
| `f` | open free-form filter overlay | Phase Art Ross grammar; ships across all player-list screens |
| `/` | open search bar (substring) | screen-local |
| `m` | min-GP threshold | where applicable |
| `r` | refresh | global, unchanged |

`c` is freed up. **`n` for nationality** matches the bio field name
(`nationality` in the StatId catalog) and dodges the spreadsheet/vim
muscle memory of `c` = column/copy/clear.

Stats screen exception: `s` stays "save" (deeply muscle-memory'd).
Sort on Stats remains the existing `/` 108-stat picker.

### 2. Type modeling (FORGE)

```rust
// icelines-cli/src/tui/filter_state.rs

/// Phase Messier — 3-letter ISO country code newtype.
/// Copy-cheap, fits in a register, free Eq/Hash. Replaces the
/// soundness-lie `Option<&'static str>` from spec v0.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CountryCode([u8; 3]);

impl CountryCode {
    /// Returns Err on non-ASCII or non-3-byte input. Used by
    /// cmdbar parser; cycle constants use a const_unwrap path.
    pub fn parse(s: &str) -> Result<Self, CountryCodeError> { ... }
    pub fn as_str(&self) -> &str { ... } // ASCII only, std::str::from_utf8 unwrap_or unreachable
}

/// Skater position class. Goalies own their own axis; `G` is
/// NOT a variant here (per FORGE issue #8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PosFilter {
    #[default] All, Forwards, Defense, C, LW, RW, LD, RD,
}

/// Goalie-axis filter — separate enum because the discriminator
/// is GP-share-of-team-minutes, not position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GoalieRoleFilter {
    #[default] All, Starters, Backups,
}

/// Headline column toggle set. EnumSet (per FORGE issue #9) —
/// O(1) toggle, no duplicates representable.
bitflags::bitflags! {
    pub struct ForcedColumns: u8 {
        const HITS   = 0b00000001;
        const BLOCKS = 0b00000010;
        const TOI    = 0b00000100;
        const SAVES  = 0b00001000;
        // 4 spare bits for future columns
    }
}

/// Shared roster-filter state. Embedded by every player-list
/// screen state struct.
#[derive(Debug, Clone)]
pub struct RosterFilterState {
    pub pos_filter: PosFilter,
    pub country_filter: Option<CountryCode>,
    pub min_gp: u32, // 0 = no filter
    pub forced_columns: ForcedColumns,
    pub free_filter: Option<Arc<icelines_query::QueryPlan>>,
    /// Memoization cache (PACE issue #1) — built lazily on first
    /// render after a state-update event invalidates it.
    cached: Option<FilterCache>,
}

/// Memoized filter result. Invalidated on input event (key
/// press, cmdbar submit, repo swap).
struct FilterCache {
    repo_generation: u64,
    plan_hash: u64,           // hash of (pos_filter, country, min_gp, free_filter)
    filtered_pids: Vec<PlayerId>,
}
```

### 3. Cmdbar verb-kv grammar (EDGE/WIRE)

Verbs that today take a positional arg (`team EDM`, `team EDM
season`) gain optional kv pairs after the positional. Disambiguation:

- A token containing `=` is a kv pair.
- A token NOT containing `=` after the positional arg is a
  recognized **positional modifier** (`season`, `playoff`, `regular`).
- Unknown bare positional tokens after the verb's positional arg
  flash an error.

```
// Grammar
verb := <ident>
positional := <token-without-equals>
positional-modifier := "season" | "playoff" | "regular"
kv-pair := <ident> "=" <value>
value := <bare-token> | <quoted-string>

input := verb (positional positional-modifier* kv-pair* | kv-pair*)

// Examples
:goalies                              // verb only
:goalies sort=gaa min-gp=20           // verb + 2 kv
:team EDM                             // verb + positional
:team EDM season                      // verb + positional + modifier
:team EDM season pos=LW country=CAN   // verb + positional + modifier + 2 kv
:team EDM pos=LW country=CAN          // verb + positional + 2 kv
:stats country=CAN                    // verb + 1 kv
:stats country="Czech Republic"       // verb + 1 quoted kv (rejected — wrong type, see §4)
```

**Repeated keys = error** (per EDGE issue #16). `:goalies sort=gaa
sort=wins` flashes `duplicate key "sort"`.

**Positional must precede all kv** (per EDGE issue #16).

**Quoted-string lexer reused** from the existing `command.rs` IN/LIKE
path. Escape `=` literally inside quoted values.

```rust
// Typed kv args, parsed once, no stringly-typed re-validation.
#[derive(Debug, Clone, Default)]
pub struct RosterKvArgs {
    pub sort: Option<SortToken>,        // "gaa" / "wins" / etc.
    pub pos: Option<PosFilter>,
    pub country: Option<CountryCode>,
    pub min_gp: Option<u32>,
    pub columns: ForcedColumns,         // pseudo-multi: hits=on, blocks=on
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum KvParseError {
    #[error("unknown key {key:?} — try: sort, pos, country, min-gp, hits")]
    UnknownKey { key: String },
    #[error("duplicate key {0:?}")]
    DuplicateKey(String),
    #[error("kv pair after positional only — got {token:?}")]
    PositionalAfterKv { token: String },
    #[error("invalid {key} value {raw:?}: {reason}")]
    InvalidValue { key: &'static str, raw: String, reason: String },
}
```

### 4. Stats screen kv path lowers to Art Ross atoms (EDGE)

Spec v0.1 had two paths to "filter Stats by country" — the cmdbar
kv path setting `RosterFilterState.country_filter` and the `f`
overlay setting `free_filter`. v0.2 collapses to one:

- On Stats, **kv pairs lower to Art Ross atoms** and write
  `free_filter` (an `Arc<QueryPlan>`).
- `country_filter` field is reserved for screens without an Art
  Ross overlay (Goalies, Depth, Favorites — Messier.2/4/5).
- The `n` shortcut on Stats opens the FilterEdit overlay
  pre-filled with `nationality=` (matches the StatId field name).
- `:stats country=CAN` and `:stats nationality=CAN` both lower
  to the same atom; cmdbar normalizes `country` → `nationality`
  for the Stats verb.
- `min-gp` lowers to a `gp >= N` atom on Stats, not a separate
  field (per EDGE issue #15).

This means the Stats-screen kv path, the `n` shortcut, and the `f`
free-form filter all produce identical IR. Result: AI fallback
output that emits either form is bit-identical.

### 5. Performance budget (PACE)

**Filter chain runs in the state-update path, not the render path.**

Render fns read `RosterFilterState.cached.as_ref().map(|c|
&c.filtered_pids)` and never call `Constraint::matches`. The cache
rebuilds:

1. On any cmdbar submit that mutates the screen's filter state.
2. On any keybind that mutates the screen's filter state (s/p/n/h/f).
3. On `repo_swap` (season picker, season-type toggle).

Cache key: `(repo_generation: u64, plan_hash: u64, pos_filter,
country_filter, min_gp, forced_columns)`. The `repo_generation`
counter increments on every `App::reload_for_season`.

**Goalies role-class threshold** — GP-share-of-team-minutes
(≥60% = Starter), per user decision B and GLASS issue #14.
Computed on screen entry from the goalie views collected for the
team; cached in `GoaliesState.role_threshold_for_team: HashMap<TeamAbbr, GpSharePoint>`,
invalidated on `repo_swap`.

The threshold is surfaced in the chrome title:
`Goalies · pos=Starters(≥60% TOI)` so the user sees what's applied.

**Performance acceptance criteria**:
- Filter+sort cycle ≤ 1ms for N=700 (criterion-style L0 in Messier.1).
- Render frame ≤ 16ms (60fps headroom) at 200×40 with all panes
  rendered.
- AI prompt cache miss only at v0.24.0 ship boundary
  (`SYSTEM_PROMPT_VERSION` bump in single Messier.6 commit).

### 6. Standardized AI prompt (EDGE/WIRE)

`SYSTEM_PROMPT_VERSION` bumps to `"v2"` in the Messier.6 commit.

Prompt grammar reference grows to include:

- kv form for screen-targeted intent
  ("show goalies with min 20 GP" → `:goalies min-gp=20`)
- Art Ross form for stat queries
  ("centers with 30+ goals last 10 games" → `:query
  pos=C AND g.last10g>=30`)

3-4 disambiguating few-shot examples added.

L0 test asserts new landmarks present:
`"sort=gaa", ":goalies", "min-gp=20", "nationality"`.

### 7. Out of scope

- Saved per-screen filter presets (Stats already has its own;
  extending to other screens is a future Phase).
- Multi-select position/country filters — single-value cycle is
  enough for v1; OR-of-N goes through `:query` from Stats.
- Column hiding for built-in columns beyond `forced_columns`.
- Schedule / Transactions / Playoffs / Tonight — these aren't
  player-list screens; their existing filter UX (search, T/k,
  navigation-only) stays.
- Country-code typed input on Goalies/Depth/Favorites — cycle is
  the UX. Wider sets via `:goalies country=CZE` cmdbar.

---

## Sub-phase ordering

Six sub-phases, ~half-day each. Type modeling (decision C) lands
in Messier.1.

### Messier.1 — `RosterFilterState` extraction + type modeling + memoization

Scope expanded post-review (decisions C + E):

- New `tui::filter_state` module with `CountryCode`, `PosFilter`,
  `GoalieRoleFilter`, `ForcedColumns` (bitflags), `RosterFilterState`,
  `FilterCache`, `RosterKvArgs`, `KvParseError`.
- Migrate `TeamPosFilter` → `PosFilter`; `country_filter:
  Option<&'static str>` → `Option<CountryCode>`; `force_hits_column:
  bool` → `forced_columns: ForcedColumns`.
- `pub use PosFilter as TeamPosFilter;` shim for one commit then
  remove (per FORGE issue #39 — keeps diff reviewable).
- `RosterFilterState::apply()` + `FilterCache` invalidation hooks.
- `insta` parity-snapshot harness — capture Team screen at canonical
  fixture pre-refactor, replay post-refactor, assert byte-equal
  buffers (per BENCH issue #5).
- Criterion-style perf L0 — filter+sort cycle ≤ 1ms for N=700.
- Test budget: ~12 (5 type tests + 4 cache tests + 2 snapshots + 1 perf).

### Messier.2 — Goalies adopts standard matrix

- Add `p` cycle GoalieRoleFilter (All / Starters / Backups).
- Add `n` cycle nationality.
- Add `h` toggle Saves column (extra column independent of sort).
- Add `f` open free-form filter overlay (per GLASS issue #5 —
  consistency pillar).
- Existing `s` (sort) and `m` (min-gp) unchanged; chrome accessor
  expanded.
- GP-share threshold computed once on screen entry, cached.
- Test budget: ~16 (cycles + predicate + threshold + chrome + L1
  dispatch + insta golden).

### Messier.3 — Stats `n` country shortcut + `f` parity verification

- `n` keybind opens FilterEdit overlay pre-filled with `nationality=`.
- Cmdbar `:stats country=CAN` lowers to Art Ross atom (writes
  `free_filter`); `:stats nationality=CAN` is the canonical form.
- L1 round-trip: cmdbar `:stats country=CAN` produces identical IR
  to typing `nationality=CAN` in the FilterEdit overlay.
- Test budget: ~10.

### Messier.4 — Depth adopts position + nationality + free-form

- `p` cycle PosFilter on the depth-rankings list.
- `n` cycle nationality.
- `f` open free-form filter overlay (consistency).
- `s` stays scoring mode toggle.
- Test budget: ~12.

### Messier.5 — Favorites sort + filter

- `s` cycle FavoritesSort (RecentlyAdded / Name / Pos / Team).
- `p` / `n` standard.
- `f` open free-form filter overlay.
- Test budget: ~12.

### Messier.6 — Cmdbar verb-kv grammar + AI prompt v2

- Extend `parse_command` with positional-then-kv form.
- Quoted-string lexer reuse.
- Typed `RosterKvArgs` parser per verb.
- Per-verb `apply_kv` dispatcher (Stats lowers to Art Ross; others
  set RosterFilterState fields).
- `SYSTEM_PROMPT_VERSION = "v2"`.
- AI prompt landmarks updated.
- 3-4 new few-shot examples in prompt.
- Test budget: ~33 (parser unit + per-verb apply + AI prompt
  landmarks + persona harness deltas).

---

## Surface coverage matrix (post-Messier)

| Screen | s | p | n | h | f | / | m | Verb-kv form |
|---|---|---|---|---|---|---|---|---|
| **Team** | sort | pos | nationality | hits col | free | — | min-gp | `:team EDM sort=hits pos=F nationality=CAN` |
| **Goalies** | sort | role | nationality | saves col | free | — | min-gp | `:goalies sort=gaa min-gp=20 pos=Starters` |
| **Stats** | save | (Art Ross) | shortcut | — | full | — | (atom) | `:stats nationality=CAN` lowers to Art Ross |
| **Depth** | scoring | pos | nationality | — | free | — | — | `:depth pos=F` |
| **Favorites** | sort | pos | nationality | — | free | — | — | `:favorites sort=name` |

---

## Test budget (post-review v0.2 — 95 tests)

| Sub-phase | Tests | Cumulative |
|---|---|---|
| Pre-Messier (v0.23.5) | — | 1051 |
| Messier.1 (incl. perf, insta, type) | +12 | 1063 |
| Messier.2 (Goalies p/n/h/f + threshold) | +16 | 1079 |
| Messier.3 (Stats n + cmdbar parity) | +10 | 1089 |
| Messier.4 (Depth p/n/f) | +12 | 1101 |
| Messier.5 (Favorites s/p/n/f) | +12 | 1113 |
| Messier.6 (cmdbar kv + AI prompt) | +33 | 1146 |
| **Total new** | **+95** | **1146** |

Target: ~1146, all green, no regressions in the existing 1051. Insta
golden snapshots ship per affected screen (in-process L0/L1, not L2).

---

## Acceptance criteria for v0.24.0 ship

- ✓ Every player-list screen exposes `s`/`p`/`n`/`h`/`f` consistently.
- ✓ Every per-screen filter is drivable from cmdbar verb-kv form.
- ✓ Per-screen hint row (Adams.9) reflects the new keybinds.
- ✓ `RosterFilterState` extraction passes `insta` parity snapshot
  bit-for-bit.
- ✓ Filter+sort cycle ≤ 1ms for N=700 (criterion L0 in Messier.1).
- ✓ Render frame stays in cache hit path until input event /
  repo swap.
- ✓ COMMANDS.md updated with the unified keybind table.
- ✓ Bin suite ≥ 1146, all green.
- ✓ Clippy clean for new code.
- ✓ `SYSTEM_PROMPT_VERSION = "v2"`; landmarks test updated.
- ✓ AI fallback emits valid v2 grammar (manual sanity check).

---

## Open items still on the table for Messier.6 review

1. Whether `:stats sort=points` should set `app.queries.sort_by` on
   the Stats screen (no Art Ross atom path for sort) or be rejected
   ("Stats sort lives in the `/` picker").
2. Whether to add `R` (capital R) for "reset all filters on this
   screen" — reaches every cycle/toggle in one keystroke. Punt to
   Messier.7 if scope creeps.

---

## Trophy fit

Mark Messier won the Hart and Lester B. Pearson the same season
multiple times — leadership = consistency + excellence across
domains. Phase Messier's leadership is **standardized UX across
every screen**: same keys, same mental model, same cmdbar parity.
The user thinks once and the muscle memory works everywhere.

---

## Phase plan

See `design/plans/2026-05-08-phaseMessier-roster-filters.md` v0.2
for file-level execution map per sub-phase.
