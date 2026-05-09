# Phase Messier — TUI cross-screen filter/sort consistency

**Trophy**: Mark Messier Leadership Award. Fit: "every screen leads
by example with the same consistent UX." Filtering/sorting on every
player-list screen converges on a single mental model.

**Builds on**: Phase Norris (state extraction), Phase Masterton
(declarative chrome), Phase Jack Adams (MDI dashboard + cmdbar +
per-screen hint row).

**Status**: spec — 2026-05-08

---

## Headline

After Adams.10/.12 brought the Team screen up to a rich filter/sort
UX (`s` sort · `p` pos · `c` country · `h` Hits column · `f` will
land in Messier), the audit flagged that **other player-list screens
expose wildly inconsistent capabilities**. Goalies has `s`/`m` but no
country/position. Stats has the rich Phase Art Ross filter overlay
but no `c` country shortcut. Depth has only `s` (scoring mode).
Favorites has nothing.

Phase Messier fixes the inconsistency by **standardizing a single
keybind matrix across every player-list screen**, hoisting the
filter state into a shared `RosterFilterState`, and adding cmdbar
parity so every per-screen filter is also drivable from `:goalies
sort=gaa pos=any country=CAN`.

After Messier, switching workspace screens never requires re-learning
keys.

---

## Why now

Summary of current per-screen keybind surface (post v0.23.5):

| Screen | s | p | c | h | f | / | m | Other |
|---|---|---|---|---|---|---|---|---|
| **Team** | sort | pos | country | hits | — | — | — | g add to group |
| **Goalies** | sort | — | — | — | — | — | min-gp | — |
| **Stats / Queries** | / picker | — | — | — | filter | — | — | s save · l load · o toggle section |
| **Depth** | scoring | — | — | — | — | — | — | — |
| **Schedule** | — | — | — | — | — | search | — | t today · D date picker |
| **Transactions** | — | T team / k kind | — | — | — | search | — | — |
| **Playoffs** | — | — | — | — | — | — | — | ←/→ round, ↑↓ series |
| **Tonight** | — | — | — | — | — | — | — | — |
| **Favorites** | — | — | — | — | — | — | — | — |

Same letter, different concept across screens — `s` is sort on Team
but scoring-mode on Depth, save on Stats. Same concept, different
letter — sort is `s` on Team but `/` on Stats.

Users shouldn't have to context-switch.

---

## Locked decisions

After applying the lessons from Adams' 8-role review:

1. **Standard keybind matrix** (every player-list screen):

   | Key | Concept | Notes |
   |---|---|---|
   | `s` | cycle sort | per-screen sort enum |
   | `p` | cycle position filter | shared `PosFilter` enum |
   | `c` | cycle country filter | shared `COUNTRY_CYCLE` constant |
   | `h` | toggle Hits column | independent of sort |
   | `f` | open free-form filter overlay | Phase Art Ross grammar |
   | `/` | open search bar (substring) | screen-local |
   | `m` | min-GP threshold | where applicable (Goalies retains; Stats/Team gain) |
   | `r` | refresh | global, unchanged |

2. **Stats screen exception** — `s` stays "save query" (deeply
   muscle-memory'd). Sort on Stats is `/` (the existing 108-stat
   sort picker). The `c` country shortcut is added; it routes
   through the existing free-form filter overlay (`country=XYZ`).
   Documented in chrome.

3. **Shared `RosterFilterState`** — extracted to
   `tui::filter_state::RosterFilterState` carrying: `pos_filter`
   (enum), `country_filter` (Option<&'static str>), `min_gp`
   (Option<u32>), `forced_columns` (`Vec<ColumnId>`), `free_filter`
   (Option<icelines_query::QueryPlan>).

   Each screen's state struct embeds it: `GoaliesState` gains
   `filters: RosterFilterState` etc. Sort enum stays per-screen
   (it's the only thing that varies).

4. **Cmdbar parity** — each per-screen filter is also drivable from
   the cmdbar in the same grammar: `:goalies sort=gaa min-gp=20`,
   `:team EDM pos=LW country=CAN`. Parser extension in
   `tui::command::parse_command` adds a "verb with kv pairs" form.

5. **Out of scope**:
   - Saved per-screen filter presets (Stats already has its own;
     extending to other screens is a future Phase).
   - Column hiding for built-in columns beyond the new `forced_columns`
     mechanic — sticks with the Adams.12 model (toggle on, columns
     append; toggle off, columns disappear).
   - Country-code typed input (Adams.10b deferred). Cycle stays the
     UX; cmdbar `:team-screen country=CZE` covers wider sets.
   - Multi-select position/country filters. Single-value cycle is
     enough for v1; OR-of-N goes through `:query` from Stats.

6. **No new global keybinds** — Tab/Esc/Enter/↑↓ semantics unchanged.
   Adding `m` to Stats and Team is the only new global pattern.

---

## Sub-phase ordering

Six sub-phases, each ~half-day, mirroring Adams' shape:

### Messier.1 — `RosterFilterState` extraction

- Move `pos_filter` / `country_filter` / `min_gp` / `forced_columns`
  fields out of per-screen state structs into a single
  `RosterFilterState`.
- Each player-list screen embeds it: `app.team.filters`,
  `app.goalies.filters`, `app.depth.filters`, `app.favorites.filters`.
- Pure refactor — no UX change. Verify all existing tests pass.
- Test budget: ~5 (default contract, embedding in each screen state).

### Messier.2 — Goalies adopts the standard matrix

- Add `p` cycle position (defaults: All, Starters, Backups —
  goalies don't have position-class but they have role-class).
- Add `c` cycle country.
- Add `h` toggle Saves column.
- Existing `s` (sort) and `m` (min-gp) unchanged; chrome accessor
  updated.
- Test budget: ~10 (cycles, predicate, chrome, L1 dispatch).

### Messier.3 — Stats adds `c` country shortcut

- `c` opens the free-form filter overlay pre-filled with
  `country=` and the cursor positioned. User types code + Enter to
  apply.
- Doesn't change `s` (save) — just adds the `c` shortcut for the
  most common filter dimension.
- Cmdbar `:stats country=CAN` lands as the same flow.
- Test budget: ~6.

### Messier.4 — Depth adopts position + country filter

- `p` cycle position class on the depth-rankings list.
- `c` cycle country.
- `s` stays scoring mode toggle.
- Test budget: ~8.

### Messier.5 — Favorites gets sort + filter

- `s` cycle sort (Recently added / Name / Pos / Team).
- `p` cycle position.
- `c` cycle country.
- Test budget: ~8.

### Messier.6 — Cmdbar parity grammar

- Extend `tui::command::parse_command` with a `<verb> <key>=<value>...`
  form. Verbs: `goalies`, `team`, `stats`, `depth`, `favorites`.
- Keys: `sort`, `pos`, `country`, `min-gp`, `hits`.
- Cmdbar `:goalies sort=gaa pos=any min-gp=20` parses + applies all
  three filter dimensions atomically.
- Test budget: ~15.

---

## Surface coverage matrix (post-Messier)

| Screen | s | p | c | h | f | / | m | Verb-kv form |
|---|---|---|---|---|---|---|---|---|
| **Team** | ✓ | ✓ | ✓ | ✓ | new | — | new | `:team EDM sort=hits pos=F country=CAN` |
| **Goalies** | ✓ | new | new | new | — | — | ✓ | `:goalies sort=gaa min-gp=20` |
| **Stats** | save | new | new | — | ✓ | — | new | `:stats country=CAN` |
| **Depth** | scoring | new | new | — | — | — | — | `:depth pos=F` |
| **Favorites** | new | new | new | — | — | — | — | `:favorites sort=name` |
| **Schedule** | — | — | — | — | — | ✓ | — | (search-only, unchanged) |
| **Transactions** | — | T/k | — | — | — | ✓ | — | (search-only, unchanged) |
| **Playoffs** | — | — | — | — | — | — | — | (navigation-only) |
| **Tonight** | — | — | — | — | — | — | — | (display-only) |

`new` = added in this phase. `✓` = pre-existing. `—` = N/A.

---

## Open items (for review)

1. **Stats `s` collision** — current `s` is "save query". If we
   want strict consistency, sort would move to `s` and save would
   become Ctrl+S or `:stats save`. Strong muscle memory says no.
   Currently locked to "Stats keeps `s`=save; sort stays at `/`".

2. **Goalies position semantic** — goalies don't have C/LW/RW. The
   `p` cycle could be (All / Starters / Backups) using GP as
   the discriminator (e.g., GP ≥ 30 = starter). Or it could be a
   no-op chip "n/a for goalies". Open: should `p` exist on Goalies
   at all?

3. **Cmdbar grammar — kv vs space-form** — `:goalies sort=gaa
   pos=any` vs `:goalies sort gaa pos any` (space-separated). The
   `=` form is unambiguous + parser-cheap; locked unless review
   pushes back.

4. **Free-form `f` on every screen** — adding the Phase Art Ross
   filter overlay to every screen is a substantial UX expansion.
   Could defer to Messier.7. Currently scoped to Team only in
   Messier.

5. **Chrome accessor naming** — every screen's `chrome()` accessor
   takes the screen-state struct. After RosterFilterState extraction,
   they all have the same shape — could we DRY further with a
   `roster_chrome_keybinds()` helper? Open.

---

## Test budget

| Sub-phase | L0 | L1 | L2 |
|---|---|---|---|
| Messier.1 | 5 | 0 | 0 |
| Messier.2 | 8 | 2 | 0 |
| Messier.3 | 4 | 2 | 0 |
| Messier.4 | 6 | 2 | 0 |
| Messier.5 | 6 | 2 | 0 |
| Messier.6 | 12 | 3 | 0 |
| **Total** | **41** | **11** | **0** |

Bin suite target: 1051 → ~1100. No new L2 — TUI subprocess infeasible
per the existing pattern. L2 surface checks (clap docs, surface
composition) inherit from the cmdbar verb table in COMMANDS.md.

---

## Acceptance criteria for v0.24.0 ship

- ✓ Every player-list screen exposes `s`/`p`/`c` (where applicable)
  with consistent semantics.
- ✓ Every per-screen filter is drivable from the cmdbar's verb-kv
  form.
- ✓ Per-screen hint row (Adams.9) reflects the new keybinds via
  declarative chrome.
- ✓ The `RosterFilterState` extraction passes all pre-existing tests
  bit-for-bit.
- ✓ COMMANDS.md updated with the unified keybind table.
- ✓ Bin suite ≥ 1100, all green.
- ✓ Clippy clean for new code.
- ✓ No new persona scenarios fail (1042 + new tests, all green).

---

## Trophy fit

Mark Messier won the Hart and Lester B. Pearson the same season
multiple times — leadership = consistency + excellence across
domains. Phase Messier's leadership is **standardized UX across
every screen**: same keys, same mental model, same cmdbar parity.
The user thinks once and the muscle memory works everywhere.

---

## File touchpoints

- `icelines-cli/src/tui/filter_state.rs` (new) — `RosterFilterState`
- `icelines-cli/src/tui/screens/team.rs` — embed `filters`
- `icelines-cli/src/tui/screens/goalies.rs` — embed `filters`, add
  `p`/`c`/`h`
- `icelines-cli/src/tui/screens/queries.rs` — add `c` country
  shortcut
- `icelines-cli/src/tui/screens/depth.rs` — add `p`/`c`
- `icelines-cli/src/tui/screens/favorites.rs` — add `s`/`p`/`c`
- `icelines-cli/src/tui/command.rs` — verb-kv grammar extension
- `icelines-cli/src/tui/app.rs` — handler routing for new keys
- `COMMANDS.md` — unified keybind table
- `CHANGELOG.md` — v0.24.0 entry

---

## Phase plan

See `design/plans/2026-05-08-phaseMessier-roster-filters.md` for
file-level execution map per sub-phase.
