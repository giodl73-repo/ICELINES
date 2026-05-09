# Phase Messier — execution plan

**Spec**: `design/specs/phase-messier-overview.md`
**Target release**: v0.24.0
**Estimated**: 6 sub-phases × ~half-day each

---

## Sub-phase Messier.1 — RosterFilterState extraction

### Files
- **NEW**: `icelines-cli/src/tui/filter_state.rs` — the shared struct.
- **MODIFY**: `icelines-cli/src/tui/mod.rs` — register module.
- **MODIFY**: `icelines-cli/src/tui/screens/team.rs`,
  `goalies.rs`, `depth.rs`, `favorites.rs` — embed `filters` field.

### Code sketch

```rust
// tui/filter_state.rs
use icelines_query::QueryPlan;

#[derive(Debug, Clone, Default)]
pub struct RosterFilterState {
    /// Position-class filter. Shared across player-list screens
    /// because positions are universal.
    pub pos_filter: PosFilter,
    /// 3-letter ISO country code; None = all.
    pub country_filter: Option<&'static str>,
    /// Minimum games played. Where applicable (Goalies has it
    /// existing; Stats / Team gain it in Messier.6).
    pub min_gp: Option<u32>,
    /// Columns to show beyond the screen's defaults. Toggled
    /// by `h` etc.
    pub forced_columns: Vec<ColumnId>,
    /// Optional free-form Phase Art Ross plan applied as an
    /// additional filter pass.
    pub free_filter: Option<QueryPlan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PosFilter { #[default] All, Forwards, Defense, C, LW, RW, LD, RD, G }

impl PosFilter {
    pub fn next(self) -> Self { /* cycle */ }
    pub fn matches(self, abbrev: &str) -> bool { /* shared predicate */ }
    pub fn label(self) -> &'static str { /* "All"/"F"/.../"G" */ }
}

// Country cycle re-exported from team.rs's COUNTRY_CYCLE.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnId { Hits, Blocks, Toi, Saves, /* … */ }
```

### Migration

The existing `TeamScreenState.pos_filter`, `country_filter`,
`force_hits_column` fields move into a `filters: RosterFilterState`
sub-field. Tests assert behavior is identical:

```rust
// Before
app.team.pos_filter == TeamPosFilter::Forwards
// After
app.team.filters.pos_filter == PosFilter::Forwards
```

The Adams.10 enums (`TeamPosFilter`, `TeamSort`) get unified —
`TeamPosFilter` deletes; the shared `PosFilter` takes over.
`TeamSort` stays per-screen (it's the only thing that varies).

### Gauntlet

- All Adams.10/.12 tests pass with the new struct shape.
- `cargo build` clean; clippy clean for new module.
- No new warnings.

### Acceptance

Pure refactor, zero UX change. If the Team screen demos identically
to v0.23.5, Messier.1 lands.

---

## Sub-phase Messier.2 — Goalies adopts standard matrix

### Files
- **MODIFY**: `icelines-cli/src/tui/screens/goalies.rs` — add
  filter cycle handlers; expand chrome.
- **MODIFY**: `icelines-cli/src/tui/app.rs` — add `c`/`h` arms for
  Goalies (`p` arm needs role-class semantic decision).

### Decision required

**Goalies position semantic** — open item from spec. Three options:

(a) `p` cycles **All / Starters / Backups** using a GP threshold
    (e.g., GP ≥ 30 = Starter for the active season; recompute
    threshold from active season length). Most defensible —
    actually filters meaningful subsets.

(b) `p` is a no-op chip "n/a for goalies" — keeps the matrix
    consistent visually but adds nothing functional.

(c) Skip `p` on Goalies entirely.

**Recommendation: (a)** — use a 33%-of-team-GP threshold (e.g.,
27 GP for an 82-game season). Implement as
`PosFilter::GoalieRole(Starter|Backup|All)` or split: keep
`PosFilter` for skaters, add `GoalieRoleFilter` enum scoped to
Goalies state.

### Code sketch

```rust
// goalies.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GoalieRoleFilter {
    #[default] All,
    Starters,  // GP >= season_length / 3
    Backups,   // GP < season_length / 3
}

pub struct GoaliesState {
    pub sort: u8,                  // existing
    pub min_gp: u32,               // existing
    pub selected: usize,           // existing
    pub filters: RosterFilterState, // adds country + forced_columns + free_filter
    pub role_filter: GoalieRoleFilter, // goalie-specific
}
```

### Gauntlet

- `s` cycles unchanged.
- `m` cycles unchanged.
- `p` cycles `GoalieRoleFilter` and applies via a render filter pass.
- `c` cycles country.
- `h` toggles a `Saves` extra column.
- Chrome lists all five (`s p c h m`).
- 10 tests: cycle wraps, predicate, chrome, L1 dispatch.

### Acceptance

Press `:goalies` Enter, then `p` `p` `p` cycles All → Starters →
Backups → All. Press `c` cycles countries. Per-screen hint row
shows `s=cycle sort · p=cycle role · c=cycle country · h=toggle
saves col · m=cycle min-gp · …`.

---

## Sub-phase Messier.3 — Stats `c` country shortcut

### Files
- **MODIFY**: `icelines-cli/src/tui/screens/queries.rs` — extend
  chrome's keybind list with `c`.
- **MODIFY**: `icelines-cli/src/tui/app.rs` — add Char('c') arm
  for Stats screen that opens the filter editor pre-filled.

### Code sketch

```rust
// app.rs handler
} else if self.screen == Screen::Queries && c == 'c' {
    // Open the existing filter editor with `country=` pre-filled.
    self.queries.mode = QueryMode::FilterEdit;
    self.queries.filter_text = "country=".to_owned();
    // Cursor positioned after the `=`. (Implementation note:
    // queries.rs FilterEdit mode renders text_input; cursor
    // tracking is implicit by string length.)
    self.status = "Type a 3-letter country code, Enter to apply".to_owned();
}
```

### Gauntlet

- Stats screen with `c` opens FilterEdit mode with input
  `"country="`.
- User types `CAN` and Enter — filter applies via the existing
  Phase Art Ross path.
- Esc cancels back to Build mode.
- Cmdbar `:stats country=CAN` lands as the same flow (Messier.6
  picks this up).

### Acceptance

Discoverable: `c` on Stats opens the pre-filled overlay; the
chrome row advertises it. 6 tests.

---

## Sub-phase Messier.4 — Depth position + country filter

### Files
- **MODIFY**: `icelines-cli/src/tui/screens/depth.rs` — add filters
  field (RosterFilterState), filter logic in render fn.
- **MODIFY**: `icelines-cli/src/tui/app.rs` — add `p`/`c` arms.

### Code sketch

```rust
// depth.rs
pub struct DepthScreenState {
    pub filters: RosterFilterState,
}

pub fn chrome(mode: ScoringMode, state: &DepthScreenState) -> ScreenChrome {
    let title = format!(
        "Depth · scoring={} · pos={} · country={}",
        mode.label(), state.filters.pos_filter.label(), state.filters.country_label()
    );
    let keybinds = vec![
        KeyHint::new("s", "toggle scoring"),
        KeyHint::new("p", "cycle pos"),
        KeyHint::new("c", "cycle country"),
        // …
    ];
    ScreenChrome { title, keybinds }
}
```

### Gauntlet

- `p` cycles pos, `c` cycles country, `s` still toggles scoring.
- Depth rankings list filtered live.

### Acceptance

8 tests. Per-screen hint row shows the new keybinds.

---

## Sub-phase Messier.5 — Favorites sort + filter

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

### Gauntlet

- `s` cycles RecentlyAdded → Name → Pos → Team → wrap.
- `p` / `c` standard.
- Sort changes the order in the rendered list deterministically.

### Acceptance

8 tests. The favorites list reorders + filters correctly under
each cycle position.

---

## Sub-phase Messier.6 — Cmdbar verb-kv grammar

### Files
- **MODIFY**: `icelines-cli/src/tui/command.rs` — extend
  `parse_command` to accept `<verb> <key>=<value>...`.
- **MODIFY**: `icelines-cli/src/tui/command.rs` — extend
  `Command` enum with carrying `kv: Vec<(String, String)>`.
- **MODIFY**: `icelines-cli/src/tui/command.rs` — extend
  `execute_command` to apply kv pairs to the matching screen
  state after the verb's normal swap.

### Code sketch

```rust
pub enum Command {
    Goalies { kv: Vec<(String, String)> },
    Team { abbrev: String, kv: Vec<(String, String)> },
    Stats { kv: Vec<(String, String)> },
    Depth { kv: Vec<(String, String)> },
    Favorites { kv: Vec<(String, String)> },
    // … existing variants unchanged
}

fn parse_kv_pairs(rest: &str) -> Vec<(String, String)> {
    rest.split_whitespace()
        .filter_map(|tok| tok.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

// In execute_command:
Command::Goalies { kv } => {
    app.screen = Screen::Goalies;
    apply_goalies_kv(app, &kv);
    ExecResult::Continue
}

fn apply_goalies_kv(app: &mut App, kv: &[(String, String)]) {
    for (k, v) in kv {
        match k.as_str() {
            "sort" => app.goalies.set_sort_by_label(v),
            "min-gp" => app.goalies.min_gp = v.parse().unwrap_or(app.goalies.min_gp),
            "country" => app.goalies.filters.country_filter = canonicalize_country(v),
            "pos" | "role" => app.goalies.role_filter = parse_goalie_role(v),
            "hits" | "saves" => /* toggle column */,
            _ => /* flash unknown key */,
        }
    }
}
```

### Gauntlet

- `:goalies sort=gaa` parses + applies.
- `:team EDM pos=LW country=CAN` parses + applies (verb takes
  positional `<abbrev>` plus kv).
- Unknown keys flash a clear error: `unknown key "foo" — try
  sort/pos/country/min-gp`.
- Existing bare `:goalies` (no kv) still works.
- Test budget: 15 (parse + apply + dispatch + error paths).

### Acceptance

Power user can drive every per-screen filter dimension from the
cmdbar. AI fallback (Adams.6) gains substantially more expressive
power because the system prompt's grammar reference grows to
include kv form.

---

## Risks

1. **Migration churn** — `TeamPosFilter` → `PosFilter` rename
   cascades across team.rs, app.rs, tests. Mitigation: do
   Messier.1 as a *pure* refactor with bit-for-bit test parity
   before any new keybinds land.

2. **Goalies role-filter UX** — the GP threshold for "starter" is
   subjective. Mitigation: surface the threshold in the chrome
   title so the user sees what's being applied.

3. **Cmdbar grammar conflict** — adding kv pairs to existing
   verbs could collide if a future verb gains a positional arg
   that looks like `key=value`. Mitigation: every kv pair MUST
   contain `=`; bare positional args must NOT (already true for
   today's grammar).

4. **`forced_columns` UX vagueness** — `Vec<ColumnId>` lets you
   force multiple columns on, but the user has no list-of-toggles
   UI. Mitigation: `h` toggles Hits; future `b` toggles Blocks;
   etc. Each column gets one keybind.

5. **AI prompt grows** — the system prompt for AI fallback (Adams.6)
   needs updating with the new kv grammar. Mitigation: bump
   `SYSTEM_PROMPT_VERSION` so prompt cache invalidates cleanly.

---

## Acceptance for v0.24.0 ship

Inherits from spec acceptance criteria. Plus:

- Plan file (this) walked through with the user before
  implementation begins on Messier.2+.
- Messier.1 lands as a separate commit with zero behavioral diff.
- Each subsequent Messier.X lands as its own commit; the suite
  ships as v0.24.0 once Messier.6 closes the cmdbar parity.
- COMMANDS.md gets a unified per-screen keybind table.

---

## Test budget summary

| Sub-phase | Bin tests added | Cumulative |
|---|---|---|
| Pre-Messier (v0.23.5) | — | 1051 |
| Messier.1 | +5 | 1056 |
| Messier.2 | +12 | 1068 |
| Messier.3 | +6 | 1074 |
| Messier.4 | +8 | 1082 |
| Messier.5 | +8 | 1090 |
| Messier.6 | +15 | 1105 |

Target: ~1100, all green, no regressions in the existing 1051.
