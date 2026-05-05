# IceLines TUI — Per-Surface Experiences

**Version**: 0.3 (draft, post-roles review)
**Date**: 2026-05-05
**Status**: Spec — pre-implementation
**Phase**: Lady Byng *(second use — first was Phase 2 Site; both share the polish/UX angle)*

> Companion specs: `tui.md` (v1 as-built), `tui-v2.md` (current 6-tab redesign), `tui-admin-overlay.md`.
>
> Portfolio context: `../IceLines.md` § "Feature × surface portfolio" — Lady Byng closes the TUI side of the matrix. The CLI gaps that remain (`schedule` / `playoffs` / `transactions` / in-TUI docs) are scheduled for Phase Lester Patrick (`../plans/2026-05-05-phaseLesterPatrick-cli-parity.md`).

---

## Goal

Each TUI surface (League, Stats, Goalies, Scores, Schedule, Transactions, Playoffs) should be **invokable in isolation** so it can be tested, iterated, demoed, and recommended individually. Today the only entry point is `icelines tui`, which boots the full 8-tab app on `Home`. To look at the Goalies tab the user must launch the whole app and press `4`.

This phase adds:

1. A `--start <slug>` flag on `icelines tui` (cheapest mechanism — no app rewrite).
2. Per-surface subcommand sugar (`icelines tui goalies`, `icelines tui scores`, …).
3. Drill-down launchers (`icelines tui player Bedard`, `tui team EDM`, …).
4. An interactive looping launcher (`icelines menu`).
5. Per-surface smoke tests so each experience has at least one boot+render assertion.

The full multi-tab app stays the default (`icelines tui` with no args). Per-surface invocations are conveniences, not replacements.

## Non-goals

- **Not** rewriting the TUI App into per-surface mini-apps. Today's App boots, jumps to the requested screen, and keeps Tab/digit navigation working for users who want to roam.
- **Not** lazy-loading data per surface (e.g., skipping the bundled-skater load when only Goalies is needed). The startup data load stays the same; surface targeting is purely a UX layer.
- **Not** changing keybindings or in-app navigation. Tab still cycles all 8 tabs even when entered via `icelines tui goalies`.
- **Not** building a separate web menu. `icelines menu` is terminal-only.

---

## TUI surface inventory (today)

| # | Surface | Screen variant | Purpose | Live? |
|---|---------|----------------|---------|-------|
| 1 | **League** | `Home` | 32-team list ranked by pace | yes |
| 2 | **Depth** | `Depth` | Cross-team line value rankings | yes |
| 3 | **Stats** | `Queries` | Interactive query/filter/sort builder | yes |
| 4 | **Goalies** | `Goalies` | Goalie leaderboard + sort cycle + min-GP filter | yes |
| 5 | **Scores** | `Tonight` | Live NHL schedule + boxscore drill-down | yes |
| 6 | **Schedule** | `Schedule` | Weekly view + team / matchup search | yes |
| 7 | **Transactions** | `Transactions` | League-wide moves feed (ESPN-sourced) | yes |
| 8 | **Playoffs** | `Playoffs` | Bracket + series/game drill-down | yes |
|   | *(admin)* | `Fetch` | Season bundle install — overlay only | yes |
|   | *(overlay)* | `Help` | Keybind reference | yes |
|   | *(drill)* | `Team(abbrev)`, `PlayerById`, `CompsById`, `DepthTeam`, `ScheduleTeam`, `ScheduleMatchup`, `SeriesDetail`, `GameDetail`, `GoalieDetailById`, `GroupDetail` | Reachable via Enter; not in nav | yes |

Drill-down launchers (player card, team card, goalie card, comps) **are** in scope. Player-name resolution piggybacks on the same `resolve_player_id_by_name` path the CLI's `query player` already uses, so a typed name maps to a `PlayerId` before the TUI launches.

---

## CLI surface

### Slug grammar — single source of truth

All slug parsing flows through a single table:

```rust
const SLUG_TABLE: &[(&str, ScreenSpec, Stability)] = &[
    ("league",       ScreenSpec::Home,         Stability::Canonical),
    ("depth",        ScreenSpec::Depth,        Stability::Canonical),
    ("stats",        ScreenSpec::Queries,      Stability::Canonical),
    ("queries",      ScreenSpec::Queries,      Stability::Alias),     // alias for stats
    ("goalies",      ScreenSpec::Goalies,      Stability::Canonical),
    ("scores",       ScreenSpec::Tonight,      Stability::Canonical),
    ("tonight",      ScreenSpec::Tonight,      Stability::Alias),     // alias for scores
    ("schedule",     ScreenSpec::Schedule,     Stability::Canonical),
    ("transactions", ScreenSpec::Transactions, Stability::Canonical),
    ("moves",        ScreenSpec::Transactions, Stability::Alias),     // alias for transactions
    ("playoffs",     ScreenSpec::Playoffs,     Stability::Canonical),
    // parameterized
    ("player:",      ScreenSpec::PlayerById,   Stability::Canonical),
    ("team:",        ScreenSpec::Team,         Stability::Canonical),
    ("goalie:",      ScreenSpec::GoalieById,   Stability::Canonical),
    ("comps:",       ScreenSpec::CompsById,    Stability::Canonical),
];
```

`ScreenSpec` differs from `Screen` for the parameterized variants — it's a "what we resolved before async name lookup" placeholder. The CLI dispatch resolves `ScreenSpec → Screen` after running name/abbrev resolution.

The same `SLUG_TABLE` drives:
- `parse_start_slug(s) -> Result<ScreenSpec, StartSlugError>`
- The error formatter's "valid slugs" list
- The `--help` long_about for `icelines tui`
- The COMMANDS.md generator (or a unit test that asserts parity)

**Stability tier**:
- **Canonical** slugs are part of the public CLI contract. Removing one is a breaking change requiring a one-release deprecation cycle (`tonight is deprecated, use scores` printed to stderr, then removed in the following release).
- **Alias** slugs are convenience renames. Aliases can be added freely; removal still triggers a one-release WARN cycle to be polite to scripts.

### `<slug>:<arg>` grammar

- Exactly **one** colon separates slug from arg. `splitn(2, ':')`.
- Arg is **opaque** to the parser — no nested colons, no parsing inside the arg until name/abbrev resolution.
- Empty arg (`player:`, `team:`) is rejected at parse time.
- Whitespace-only arg (`player: `, `team:   `) is rejected at parse time *before* `normalize_name` strips it (otherwise an empty needle would silently match the first bio in the bundle).

### `icelines tui [--start <slug>]`

The default invocation is unchanged: `icelines tui` boots on League. The new flag is optional.

```bash
icelines tui                      # boots League (today's behavior)
icelines tui --start goalies      # boots directly on Goalies tab
icelines tui --start scores       # boots on Scores
icelines tui --start playoffs     # boots on Playoffs
```

Recognized nav-tab slugs (canonical / alias):

| Slug | Aliases | Screen |
|------|---------|--------|
| `league` *(default)* | — | `Home` |
| `depth` | — | `Depth` |
| `stats` | `queries` | `Queries` |
| `goalies` | — | `Goalies` |
| `scores` | `tonight` | `Tonight` |
| `schedule` | — | `Schedule` |
| `transactions` | `moves` | `Transactions` |
| `playoffs` | — | `Playoffs` |

Invalid slugs error with a list of valid choices. `goalie` (singular typo) → suggest `goalies`. The error is printed to **stderr in normal terminal mode** (NOT inside the alt-screen) — resolution failure happens before raw-mode is entered.

### Per-surface subcommand sugar

Implemented via clap **nested subcommands** (option (b) per FORGE review):

```rust
enum TuiCommand {
    League,
    Depth,
    Stats,
    Goalies,
    Scores,
    Schedule,
    Transactions,
    Playoffs,
    // Drill-downs:
    Player { needle: String },
    Team   { abbrev: String },
    Goalie { needle: String },
    Comps  { needle: String },
    // Default fallback for `--start`:
    Default { start: Option<String> },
}
```

The nested form makes invalid states unrepresentable (`TuiCommand::Team { abbrev: String }` can't carry a player name) and makes `--help` self-documenting. The `Default { start: Option<String> }` variant carries `--start <slug>` as a string for the parser to dispatch.

```bash
icelines tui league              # = icelines tui --start league
icelines tui goalies             # = icelines tui --start goalies
icelines tui scores
icelines tui schedule
icelines tui playoffs
icelines tui transactions
icelines tui depth
icelines tui stats
```

### Drill-down launchers

```bash
icelines tui --start player:Bedard         # boots PlayerById(Bedard's pid)
icelines tui --start player:8478402        # explicit pid
icelines tui --start team:EDM              # boots Team("EDM")
icelines tui --start goalie:"Connor Hellebuyck"
icelines tui --start comps:McDavid         # boots CompsById(...)

# Sugar form
icelines tui player Bedard
icelines tui player 8478402
icelines tui team EDM
icelines tui goalie "Connor Hellebuyck"
icelines tui comps McDavid
```

Recognized parameterized slugs:

| Slug:arg | Screen | Resolution |
|----------|--------|------------|
| `player:<name-or-pid>` | `PlayerById(pid)` | name → `resolve_player_id_by_name` |
| `team:<abbrev>` | `Team(abbrev)` | uppercase + `.trim()`, validated against the 32-team set |
| `goalie:<name-or-pid>` | `GoalieDetailById(pid)` | same as player (skater bios then goalie bios) |
| `comps:<name-or-pid>` | `CompsById(pid)` | same as player |

**Resolution semantics** (resolved by EDGE/WIRE review):

1. **Pid form** (all digits): bypass name resolution.
2. **Single match**: resolve to that pid.
3. **No match**: error before TUI boots, with `Did you mean ...?` hint listing the nearest 5 names. Same hint as `query player`.
4. **Multi-match (Sebastian Aho problem)**: error before TUI boots, listing all candidates with team + most-recent-season. Spec language:
   ```
   Ambiguous name "Smith" — pick one:
     player:8474141  Reilly Smith   (NYR · 2024-25)
     player:8474567  Brendan Smith  (CAR · 2018-19)
     player:8473449  Craig Smith    (DAL · 2023-24)
     player:8470187  Ben Smith      (TOR · 2017-18)
   ```
   The user re-runs with the pid or with the full unambiguous name.
5. **Empty / whitespace arg**: rejected at parse time, before name lookup. (`tui --start "player: "` does NOT silently match the first bio in the bundle.)
6. **Diacritic-insensitive**: needle is normalized via `icelines_core::name::normalize_name` (NFD-strip + lowercase). `player:Lehkonen` and `player:Léhkonen` resolve identically.
7. **Trailing whitespace** on team abbrev: trimmed before validation. `team:"EDM "` → `"EDM"`.

Resolution failure (any path that doesn't return Ok) prints to **stderr in normal terminal mode**, never inside the alt-screen. Exit code is non-zero.

Drill-down launchers do **not** load the full 38-season historical bundle on boot — they trigger the same lazy career fan-out (`UX.1`) on first paint that the in-app player card uses today.

**First-frame contract for drill-downs**: when the active-season repo doesn't contain the resolved pid (e.g., `tui player Gretzky` lands on a current-season repo with no Gretzky row), the player card renders an explicit `Loading career…` placeholder row (NOT blank) until the lazy fan-out completes. The fan-out is async; the placeholder is what the user sees for ~50 ms. Once the fan-out finishes, the career table populates.

Drill-down sugar forms accept `--season` is **not** offered (in-app `y` is the time-travel mechanism, per Resolved decisions).

### `icelines menu` — interactive looping launcher

```
$ icelines menu

  ICELINES v0.13.0 — pick a surface

  1. League         32-team rankings
  2. Stats          interactive query builder
  3. Goalies        goalie leaderboard
  4. Scores         tonight's games + boxscores
  5. Schedule       weekly + season schedule
  6. Playoffs       bracket + series detail
  7. Transactions   league-wide moves feed
  8. Depth          cross-team depth chart

  P. Player card    (will prompt for name)
  T. Team card      (will prompt for abbrev)
  G. Goalie card    (will prompt for name)
  C. Comps          (will prompt for name)
  W. Web dashboard  http://localhost:8000
  D. Docs           command reference
  Q. Quit

  Choose [1-8 / P / T / G / C / W / D / Q]: _
```

**Loop semantics**: after a launched surface quits the menu re-renders and prompts again. `Q` is the only way out. Between dispatches the menu calls `clear_screen` to prevent ConPTY artifact build-up on Windows (per GLASS).

**Visual style**: the menu uses `owo-colors` for accent on the title + canonical option keys (matching the CLI table aesthetic — see `query leaders` output). Not ratatui — the menu is plain stdout so users without a full terminal still see something readable. Heading bold, option keys (1-8 / P / T / etc.) bold + colored, body text default.

**Dispatch table**:

- **1–8** → `tui::run_tui(RunTuiOpts { start_screen, .. })`. After the TUI quits, `clear_screen()` then loop.
- **P / T / G / C** → sub-prompt for the name/abbrev:
  ```
    Player name (or pid): _
  ```
  Empty input cancels back to the main menu without dispatching. Resolution-failure paths use the same Sebastian-Aho-style listing as the CLI flag form.
- **W** → `commands::serve::run(...)` with defaults from the new `[menu]` config section (port, bind, etc.). Bind error (`AddrInUse`) is **caught**, printed as `port 8000 in use — visit http://localhost:8000 if it's already an icelines server` then loop. Server runs until Ctrl-C; on stop, control returns to the menu.
- **D** → print `COMMANDS.md` paginated through `$PAGER` (or `less` if unset, or stdout if no pager available). Wait for pager exit, then loop.
- **Q** / **q** → exit 0.
- **Anything else** → "Choose 1-8, P, T, G, C, W, D, or Q." → loop without clearing.

**Ctrl-C handling** (resolved by EDGE review): The spec's earlier "Ctrl-C exits 0" promise is unenforceable without an installed signal handler. **The implementation will install a `ctrlc::set_handler`** that flips an `AtomicBool`; the menu loop checks it after each `read_line` and exits 0 if set. Without the handler the user gets exit 130 (Unix) / 1 (Windows). Documented in `--help` long_about so scripted callers know.

**Non-TTY safety**: when stdin is not a terminal (piped / redirected), `icelines menu` exits 0 with a one-line message (`icelines menu needs an interactive terminal — use 'icelines tui --start <slug>' instead`) rather than blocking. Detected via `std::io::IsTerminal`.

`menu` does **not** accept positional args today. Future: `icelines menu goalies` short-circuit; deferred.

### `[menu]` config section

A new `[menu]` section in `~/.icelines/config.toml` carries menu defaults so future flag adds (`icelines menu --port 9000`) don't break the CLI signature:

```toml
[menu]
# Defaults for the W (web dashboard) option.
web_port = 8000
web_bind = "127.0.0.1"
# Reserved: pager override, theme.
```

Empty / missing section uses hardcoded defaults. WIRE review caught the paint risk; section is added pre-emptively.

---

## App-state changes

`run_tui` takes a single struct parameter — `RunTuiOpts` — to pre-empt the next param add (locked surface, custom season, etc.):

```rust
pub struct RunTuiOpts {
    pub no_color:     bool,
    pub start_screen: Screen,
    // future: pub locked: bool, pub start_season: Option<Season>, ...
}

pub async fn run_tui(opts: RunTuiOpts) -> anyhow::Result<()>
```

Existing call sites (`Commands::Tui` *and* `Commands::Dashboard` in `main.rs`) pass `RunTuiOpts { no_color, start_screen: Screen::Home }` for back-compat. The `--start` flag (and its sugar / drill-down forms) resolves to a `Screen` variant in `cli.rs` before `run_tui` is invoked.

For drill-down `Screen` variants (`PlayerById`, `Team`, `GoalieDetailById`, `CompsById`), the same `prev_screen` fallback chain applies: pressing Esc on a cold-entered drill-down screen pops to its natural parent (Home / Goalies / etc.). The cold-entered drill-down also draws a footer hint:

```
Esc: League · q: quit · y: season picker
```

so the user knows where Esc takes them. Without the hint, "Esc means quit" is the natural mental model and the navigation feels surprising.

**Nav bar**: the existing nav bar continues to render on every surface (no changes). When entered via `tui goalies`, the nav bar shows with `Goalies` highlighted. This makes Tab discoverable without a tutorial.

**No new App fields.** The `Screen` variant carries everything the launcher needs to set up state.

### Terminal teardown — RAII guard (BLOCKING fix)

Today's `run_tui` (`icelines-cli/src/tui/mod.rs:28-44`) has manual cleanup: `disable_raw_mode` + `LeaveAlternateScreen` run after `run_loop` returns. A panic inside `run_loop` skips them and corrupts the terminal — fatal for the menu loop, which would re-render onto a wedged screen.

**Before LB.4 (menu loop) ships, `run_tui` must be wrapped in a `TerminalGuard` struct with a `Drop` impl:**

```rust
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = io::stdout().flush();
    }
}
```

Constructed at the top of `run_tui`, dropped on either return or unwinding panic. This makes terminal restoration **panic-safe** and is a prerequisite for the menu loop, not an optional polish item. Tracked as **LB.0.5** in the plan.

---

## Test strategy

### L0 (unit) — primary harness reuses existing patterns

The repo already has a render harness at `icelines-cli/src/tui/screens/mod.rs:236-298` (`render_app_to_text` + `l0_app_renders_every_canonical_landing_screen_without_panic`). **This is the primary test surface for surface-rendering smokes** — extend it with the new `Screen` variants reachable via `--start`. Per BENCH review.

Per-slug tests in `cli.rs` covering start-slug → Screen mapping. ~30 cases including:

**Slug grammar:**
- Each canonical slug → correct ScreenSpec.
- Each alias → same ScreenSpec as its canonical form.
- Case-insensitivity (`GOALIES`, `Goalies`, `goalies` all resolve identically).
- Unknown slug returns `StartSlugError` whose `valid()` lists the canonical slugs (aliases hidden from the suggestion list — fewer choices for the user to digest).
- Empty / whitespace input rejected.
- `goalie` (singular typo) returns hint pointing to `goalies`.

**Parameterized:**
- `player:Bedard` → `ScreenSpec::PlayerById` with deferred name resolution.
- `player:8478402` → bypasses name resolution.
- `team:edm` → uppercases to "EDM".
- `team:"EDM "` (trailing space) → trims to "EDM".
- `team:ZZZ` → error listing valid abbrevs.
- `player:Zzzzzz` → error with `Did you mean` hint.
- `player:Smith` → error listing all matches (Sebastian Aho path).
- Malformed: `player:`, `:Bedard`, `player` (no colon) all error cleanly.
- `player: ` (whitespace-only arg) rejected at parse time, NOT silently matched.
- Diacritic-insensitive: `player:Lehkonen` and `player:Léhkonen` resolve identically.
- Apostrophe: `player:O'Reilly` resolves (clap survives the quote).
- Hyphen: `player:"O'Reilly-Smith"` resolves.
- Sugar form `tui player Bedard` parses identically to `--start player:Bedard`.

**Invariant locking:**
- A golden test asserts `parse_start_slug("league") == ScreenSpec::Home` for every canonical slug. Fails loudly when somebody renames a slug — protects the public CLI contract.
- A grep test asserts every canonical slug appears in COMMANDS.md.

### L0 (in-process render harness)

Per-Screen render smokes via the existing `render_app_to_text` harness:

```rust
#[test] fn lb_render_league()       { assert_renders(Screen::Home,        "LEAGUE"); }
#[test] fn lb_render_goalies()      { assert_renders(Screen::Goalies,     "GOALIES"); }
// ... (one per nav tab, then drill-downs)
#[test] fn lb_render_player_card()  { assert_renders_player_card_loading_placeholder(); }
```

In-process avoids subprocess + pty fragility and runs in <1 ms each.

### L2 (system) — minimal subprocess coverage

A small set of L2 smokes guards the CLI dispatch end-to-end (clap + main.rs + run_tui). These use the existing test harness pattern, NOT a new `--render-once` flag (per BENCH review):

- `lb_l2_unknown_slug_exits_nonzero` — `tui --start zzz` exits non-zero, stderr contains "valid slugs".
- `lb_l2_unknown_player_exits_nonzero` — `tui player ZZZZZZ` exits non-zero, stderr contains "Did you mean".
- `lb_l2_ambiguous_player_lists_candidates` — `tui player Smith` exits non-zero, stderr contains all candidates.
- `lb_l2_non_tty_menu_exits_clean` — `icelines menu < /dev/null` exits 0, stdout contains "needs an interactive terminal".

**Menu-loop test correction** (per BENCH review): the naïve `1\nQ\n` pipe doesn't work — the launched TUI inherits stdin and consumes `Q` as a quit keystroke before the menu's `read_line` ever sees it. Test the menu's dispatch table **in isolation** with the `tui::run_tui` and `commands::serve::run` calls mocked behind a trait. The dispatch logic + clear-screen + non-TTY guard get unit coverage; the integration is covered by the manual phase-exit walkthrough.

### Fixture stability (BENCH)

Smokes use **frozen historical** fixtures, not current-season players who could retire / move:

- `player:Gretzky` (resolves via bundled bios — guaranteed stable across all 38 seasons).
- `player:8478402` (McDavid pid — pid never changes even if name spelling does).
- `goalie:Brodeur` (retired, stable bundled bio).
- `team:EDM` (Edmonton has existed continuously since 1979).

Bedard is fine as a SECONDARY active-player smoke but should not be the only `player:NAME` test fixture.

### Network-touching surfaces (EDGE)

Scores / Transactions / Schedule fetch live data on boot. The render-harness tests skip those three for now (rendering an empty schedule frame would still grep "SCORES" header and pass for the wrong reason). Mock at the harness layer in a follow-up; for v1 these three surfaces have manual smoke only.

### Manual exit gate

Hand walk-through: each surface launched via its subcommand renders correctly, navigates, and quits cleanly. The menu loop returns to the prompt after each surface quit, including after a deliberately-panicked surface (validates `TerminalGuard`).

---

## Phase milestones (preview — full breakdown in plan doc)

1. **LB.0** — Pre-phase entry-criterion: confirm trophy + open-question answers. *Done — see Resolved decisions below.*
2. **LB.0.5** — Terminal `TerminalGuard` RAII (BLOCKING prerequisite for LB.4).
3. **LB.1** — `--start <slug>` flag + `SLUG_TABLE` + `RunTuiOpts` + nav-tab L0 tests + `Commands::Dashboard` audit + persona-wave audit.
4. **LB.2** — Per-surface subcommand sugar (nested clap subcommands per FORGE).
5. **LB.3** — Drill-down launchers (`player:`/`team:`/`goalie:`/`comps:`) + ambiguity handling + `Loading career…` placeholder + L0 tests.
6. **LB.4** — `icelines menu` interactive launcher with loop + `ctrlc` handler + bind-error catch on option `W`.
7. **LB.5** — In-process render-harness smokes + L2 dispatch smokes + frozen fixtures.
8. **LB.6** — Docs refresh: COMMANDS.md, `--help` long_about, README primer.
9. **LB.7** — Hands-on persona pass; capture issues; create follow-ups.

(Deferred — not in this phase) **LB.8** — App split / lazy data loading.

---

## Resolved decisions (2026-05-05 Q&A)

1. **Trophy** → **Lady Byng** (second use; first was Phase 2 Site).
2. **Menu loop** → loops; returns to prompt after each surface quits. `Q` (or Ctrl-C with handler installed) exits.
3. **Drill-down launchers** → in scope (LB.3).
4. **Per-sugar `--season` flag** → no. In-app `y` only.
5. **Web menu integration** → option `W` always launches the server inline (with bind-error catch).

## Resolved decisions (v0.3 — post-roles review)

6. **Terminal teardown** → RAII `TerminalGuard` with Drop impl, landed in LB.0.5 BEFORE menu loop ships.
7. **Ambiguous name resolution** → list all candidates with team+season, exit non-zero before TUI boots. Sebastian Aho problem: "Smith" lists all four; user re-runs with pid or unambiguous name.
8. **Ctrl-C in menu** → `ctrlc` crate handler flips AtomicBool checked after `read_line`. Documented in `--help`.
9. **Web option `W` bind error** → caught; print URL hint; return to menu loop.
10. **Slug stability tier** → canonical vs alias declared in spec; one-release deprecation cycle for canonical removal; aliases shown in `--help` but not in error suggestions.
11. **`<slug>:<arg>` grammar** → exactly one colon; `splitn(2, ':')`; arg opaque to parser; empty/whitespace arg rejected before lookup.
12. **`SLUG_TABLE`** → single source of truth driving parser + error formatter + `--help` + COMMANDS.md grep test.
13. **`RunTuiOpts` struct** → forward-compatible `run_tui` signature (locked surface + season time-travel land in this struct in future phases).
14. **clap option (b)** → nested subcommands. Invalid states unrepresentable.
15. **`Commands::Dashboard` audit** → also calls `run_tui(false)`; LB.1 must update both.
16. **Persona test wave audit** → check `persona_scenarios` + `persona_wave2/3/4` for `Commands::Tui` literals.
17. **Render harness reuse** → in-process `render_app_to_text` (existing) is primary; `--render-once` debug flag dropped from plan.
18. **Frozen fixtures** → Gretzky / McDavid pid / Brodeur / EDM. Bedard is secondary.
19. **`Loading career…` placeholder** → drill-down player cards render explicit placeholder, never blank rows.
20. **Nav bar always visible** → cold-entered surfaces still draw the nav bar with the entered tab highlighted; Tab is discoverable.
21. **Esc footer hint** → cold-entered drill-downs draw `Esc: League · q: quit · y: season picker` footer.
22. **`clear_screen` between menu dispatches** → prevents ConPTY artifact buildup on Windows.
23. **`[menu]` config section** → reserved now even if empty, so `icelines menu --port 9000` doesn't break the contract later.

## Future / parked

- **Locked surfaces**: `--locked` to disable Tab. Future kiosk/demo mode.
- **`icelines menu <slug>` shortcut**: skip the prompt.
- **Web option `W` server-detection**: don't spawn a second server if `:8000` is already bound; print the URL instead.
- **App split / lazy data (LB.8)**: per-surface mini-apps with their own data scope.
- **Network mocking for L0 render of Scores/Transactions/Schedule**: harness-level mock so these three get rendering coverage too.
- **JSON twin for `icelines menu`**: scripted equivalent. Not pressing.
- **1×1 TTY guard**: refuse render if `cols<40 || rows<10`. Document min size in `--help`.
- **Apostrophe / hyphen edge cases**: `tui player "O'Reilly-Smith"` — covered in L0 tests above; promote to a dedicated test set if regressions ever appear.
