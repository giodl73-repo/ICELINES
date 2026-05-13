# Phase Lady Byng — TUI per-surface experiences

**Date**: 2026-05-05
**Spec**: `design/specs/tui-experiences.md` (v0.3 — post-roles review)
**Status**: Implemented — `--start`, surface sugar, drill-down launchers, and
`icelines menu` are present in the running CLI/TUI
**Trophy**: Lady Byng *(second use; first was Phase 2 Site)*
**Estimated effort**: 2–3 sessions
**Predecessors**: QueryA + QueryB (shipped in v0.13.0). This phase builds the third leg from the v0.13 QueryC conversation: A (shared engine — done), B (CLI parity — done), **C (per-surface entry points — this phase)**.

---

## Why now

`icelines tui` boots the full 8-tab app. To look at one surface (Goalies leaderboard, Scores tonight, Playoffs bracket) the user has to launch the whole thing and press a digit. That's fine for someone who already knows the layout, but it makes it impossible to:

- **Demo one experience.** "Here's IceLines' Goalies tab" requires `icelines tui` + verbal "press 4".
- **Jump straight to a player card.** Today: launch TUI, navigate, type, search, Enter. Tomorrow: `icelines tui player Bedard`.
- **Test one experience in isolation.** Smoke tests today either boot the whole app or nothing.
- **Recommend a surface in docs.** README can't say "to see tonight's games run X" without listing the keybind.

Per-surface entry points solve all four. We don't rewrite the TUI to get there — we just teach `run_tui` to start anywhere.

## Approach summary

Smallest mechanism that works: pass the initial `Screen` to `tui::run_tui` as a parameter, expose it as a CLI flag, then sugar that flag with subcommands. Add a looping `menu` command on top for users who don't want to type a slug at all.

```
icelines menu                                    ← layer 4: interactive picker (LB.4)
  └─→ icelines tui player Bedard                 ← layer 3: drill-down sugar  (LB.3)
        └─→ icelines tui --start player:Bedard   ← layer 3: drill-down flag   (LB.3)
              └─→ icelines tui goalies           ← layer 2: nav-tab sugar     (LB.2)
                    └─→ icelines tui --start goalies   ← layer 1: flag       (LB.1)
                          └─→ run_tui(RunTuiOpts { start_screen: Goalies, .. })
                                ↑
                                LB.0.5: TerminalGuard RAII (BLOCKING for LB.4)
```

Anything broken at the bottom layer surfaces in tests at every higher layer.

---

## Milestones

### LB.0 — Pre-phase capture (entry criterion)

**Status**: Done 2026-05-05.

**Resolved decisions**: see spec § "Resolved decisions". Trophy = Lady Byng (second use); menu loops; drill-down in scope; no per-sugar `--season`; web option always launches the server (with bind-error catch).

**Spec v0.3 additions** (post-roles review): `TerminalGuard` RAII; Sebastian Aho ambiguity path; ctrlc handler; SLUG_TABLE single source of truth; clap option (b); RunTuiOpts struct; in-process render harness reuse; frozen fixtures; Loading-career placeholder; nav bar always visible; Esc footer hint; clear_screen between menu dispatches; `[menu]` config section.

---

### LB.0.5 — `TerminalGuard` RAII (BLOCKING prerequisite for LB.4)

**Why first**: `tui::run_tui` (`icelines-cli/src/tui/mod.rs:28-44`) currently uses manual `disable_raw_mode` + `LeaveAlternateScreen` cleanup. A panic inside `run_loop` skips them and leaves the terminal wedged. **The menu loop (LB.4) cannot ship safely until this is RAII** — a panic inside surface N would corrupt the menu re-render for surface N+1, including in CI smokes that exercise panic paths.

**Deliverable**:
- New `TerminalGuard` struct with `Drop` impl in `icelines-cli/src/tui/mod.rs`:
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
- `run_tui` constructs a `TerminalGuard` immediately after `enable_raw_mode + EnterAlternateScreen`. The current trailing manual cleanup is removed (the guard handles it on both happy-path return and panic unwind).

**Tests**:
- L0 panic-restore test: install a custom panic hook in a test, deliberately panic inside `run_loop`, verify after the test that the terminal is in a clean state. Trick: use `std::panic::catch_unwind` so the test process doesn't actually die. Skim existing tests for `catch_unwind` precedent.
- Compile-time check: `TerminalGuard: !Copy` (it is by default — guards must not be implicitly duplicated).

**Manual smoke**: `RUST_BACKTRACE=1 icelines tui --start nonexistent` (forced panic via assert) — terminal restores cleanly; subsequent `echo hello` shows correctly.

**Exit**: `TerminalGuard` lands; `run_tui` is panic-safe; LB.4 unblocked.

---

### LB.1 — `--start <slug>` flag for nav tabs + `SLUG_TABLE` + `RunTuiOpts`

**Deliverable**:
- New `cli.rs` module section with `SLUG_TABLE: &[(&str, ScreenSpec, Stability)]` per spec. Both canonical and alias slugs in the table; `Stability` enum carries `Canonical | Alias`.
- New `parse_start_slug(s: &str) -> Result<ScreenSpec, StartSlugError>` driven entirely by `SLUG_TABLE`. `StartSlugError` derives `thiserror::Error`, carries `{ input: String, valid: Vec<&'static str> }` (canonical slugs only — aliases hidden from suggestions).
- `ScreenSpec` enum (parameterized variants are placeholders awaiting name/abbrev resolution): `Home | Depth | Queries | Goalies | Tonight | Schedule | Transactions | Playoffs | PlayerById(NeedleOrPid) | Team(String) | GoalieDetailById(NeedleOrPid) | CompsById(NeedleOrPid)`.
- `Resolution` step (synchronous, before TUI boots):
  - `ScreenSpec::PlayerById(NeedleOrPid::Pid(n))` → `Screen::PlayerById(PlayerId(n))` directly.
  - `ScreenSpec::PlayerById(NeedleOrPid::Name(s))` → `resolve_player_id_by_name(s)` then count matches → 0 / 1 / >1 dispatch per spec.
  - Same for `GoalieDetailById` / `CompsById`.
  - `Team(abbrev)` → `.trim().to_uppercase()` then validate against the 32-team set.
- New `RunTuiOpts { no_color: bool, start_screen: Screen }` struct; `run_tui(opts: RunTuiOpts)` signature.
- **`Commands::Tui` change**: from unit variant to `Tui(TuiCommand)` carrying the nested-subcommand enum (LB.2 fills in the variants). For LB.1 only `TuiCommand::Default { start: Option<String> }` exists; the 8 nav-tab variants land in LB.2.
- **`Commands::Dashboard` audit**: that variant in `main.rs:364` also calls `run_tui(false)`; update to `run_tui(RunTuiOpts { no_color: false, start_screen: Screen::Home })`.
- **Persona-wave audit**: grep `Commands::Tui` across `persona_scenarios.rs` + `persona_wave2.rs` / `wave3` / `wave4`; update any literal constructions. Plan flagged this as risk row in v0.2; act on it now.

**Tests** (L0, in `cli.rs` tests module):
- One pass per canonical nav-tab slug → correct ScreenSpec.
- Each alias resolves to its canonical's ScreenSpec.
- Case-insensitivity (`GOALIES`, `Goalies`, `goalies` all resolve identically).
- Unknown slug returns `StartSlugError` whose `valid()` lists CANONICAL slugs only (no alias clutter).
- Whitespace-only input rejected.
- `goalie` (singular typo) returns hint pointing to `goalies` (Levenshtein-1 match).
- **Invariant-locking golden table test**: assert `parse_start_slug("league") == ScreenSpec::Home` for every canonical slug. Fails when somebody renames a slug — protects the public CLI contract.

**Manual smoke**: `icelines tui --start goalies` boots on Goalies; `icelines tui --start zzz` errors with hint listing valid slugs (printed to stderr, NOT inside the alt-screen).

**Exit**: `cargo test -p icelines-cli` clean; manual smoke passes for at least three nav surfaces.

---

### LB.2 — Per-surface subcommand sugar (nested clap, nav tabs only)

**Deliverable**: Extend `TuiCommand` with the 8 nav-tab variants:

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
    Default { start: Option<String> }, // for `--start <slug>`
}
```

Drill-down variants (LB.3) extend the same enum.

`main.rs` dispatch matches the variant, builds a `ScreenSpec`, runs resolution, builds a `Screen`, calls `run_tui`. The dispatch is straight match with no string parsing — clap discriminates at parse time per FORGE review.

**Tests** (L0):
- Each of the 8 nav-tab sugar forms parses to the same Screen as its `--start` equivalent (asserted via `Cli::parse_from(&["icelines", "tui", "goalies"])`).

**Manual smoke**: each of the 8 sugar forms boots on its target tab.

**Exit**: All 8 sugar invocations work; `icelines tui --help` lists them.

---

### LB.3 — Drill-down launchers (player / team / goalie / comps) + Loading-career placeholder

**Deliverable** (CLI side):
- Extend `TuiCommand` with parameterized variants:
  ```rust
  Player { needle: String },
  Team   { abbrev: String },
  Goalie { needle: String },
  Comps  { needle: String },
  ```
- Slug-grammar extension in `parse_start_slug`: accepts `<slug>:<arg>` via `splitn(2, ':')`. Arg opaque to parser. Empty / whitespace-only arg rejected at parse time.
- Sugar dispatch: `tui player Bedard` → `TuiCommand::Player { needle: "Bedard" }`.
- Resolution layer:
  - All-digit arg → bypass name lookup.
  - Name → `resolve_player_id_by_name` (or `..._goalie_...` for `goalie` slug) → match count branch:
    - 0 → `Did you mean ...?` with 5 nearest matches; exit non-zero.
    - 1 → resolve to pid; build Screen.
    - >1 → list all candidates (pid, name, team, most-recent-season); exit non-zero.
  - Team abbrev: trim + uppercase + validate against `icelines_core` team-abbrev validator. Bad abbrev → list 32 valid abbrevs; exit non-zero.
- Resolution failure prints to **stderr in normal terminal mode**. The CLI never enters raw-mode for a resolution failure.
- Diacritic insensitivity: needle goes through `icelines_core::name::normalize_name` (NFD-strip + lowercase). Verified: `name::normalize_name` exists at `icelines-core/src/name/mod.rs`.
- **`BUNDLED_SEASONS` order audit**: verify `resolve_player_id_by_name` walks newest-first per its doc-comment. If not, sort or document the actual order — multi-match listing depends on it.

**Deliverable** (TUI side — Loading-career placeholder):
- Player-card renderer detects "active-season repo doesn't have this pid" and renders `Loading career…` placeholder rows instead of blank. Once the lazy fan-out (`UX.1`) populates the repo, the next paint shows the real career table.
- Same for goalie card.
- L0 render test asserts `Loading career…` appears when constructing a player card with a pid not in the test repo.

**Tests** (L0):
- `player:Bedard` → resolves to a real PlayerId (fixture: Bedard guaranteed in current bundle).
- `player:Gretzky` → resolves via bundled bios (frozen fixture).
- `player:8478402` → bypasses name resolution.
- `team:edm` (lowercase) → uppercases to "EDM".
- `team:"EDM "` (trailing whitespace) → trims to "EDM".
- `team:ZZZ` → error listing 32 valid abbrevs.
- `player:Zzzzzz` → error with `Did you mean` hint, 5 nearest matches.
- `player:Smith` → ambiguous; lists all candidates with team+season.
- `player:Lehkonen` and `player:Léhkonen` → resolve identically (diacritic).
- `player:O'Reilly` → resolves (apostrophe survives clap).
- `player:` / `:Bedard` / `player` / `player: ` (whitespace arg) → all error cleanly at PARSE time, not after lookup.
- Sugar form `tui player Bedard` parses identically to `--start player:Bedard`.
- Multi-word names: `tui player "Connor Bedard"` works.
- Render: player card with pid not in active repo shows `Loading career…` placeholder.

**Manual smoke**:
- `tui player Bedard` lands on Bedard's card; placeholder visible briefly; career table populates after fan-out.
- `tui player Gretzky` lands on Gretzky's card; same flow with historical data.
- `tui player Smith` exits before TUI boots with candidate list.
- `tui team EDM` lands on Edmonton roster.
- `tui player NotARealName` errors before TUI boots.

**Exit**: All four drill-down sugar forms work + parameterized `--start` syntax. Resolution-failure paths print to normal stderr. `Loading career…` placeholder visible on cold-entered drill-downs.

---

### LB.4 — `icelines menu` interactive launcher (with loop)

**Prerequisites**: LB.0.5 (TerminalGuard) + LB.1–LB.3.

**Deliverable**:
- New `Commands::Menu` variant in `cli.rs`.
- New `commands/menu.rs` module:
  - `pub async fn run(cfg: &Config) -> anyhow::Result<()>`
  - `loop { print menu; read choice; dispatch; if quit → break; clear_screen; }`
  - **Non-TTY guard**: `std::io::stdin().is_terminal()` check at entry; non-TTY exits 0 with redirect message.
  - **Ctrl-C handler**: `ctrlc::set_handler` flips `Arc<AtomicBool>`; loop checks after each `read_line`, exits 0 if set. Handler installed once at module entry, removed on Drop. Document in `--help`: "Ctrl-C exits cleanly."
  - **clear_screen between dispatches** (per GLASS): use `crossterm::terminal::Clear(ClearType::All)` before re-printing the menu. Prevents ConPTY artifact buildup on Windows.
  - Dispatch table:
    - `1`–`8` → resolve slug via `parse_start_slug` → `tui::run_tui(RunTuiOpts { ... })`. After return (via TerminalGuard restoration), loop.
    - `P` / `T` / `G` / `C` → sub-prompt for name/abbrev → resolve → drill-down dispatch. Empty input → loop without dispatching. Multi-match → print candidate list, then loop.
    - `W` → `commands::serve::run(...)` with `[menu]` config defaults. **Bind-error catch** (`AddrInUse`): print "port 8000 in use — visit http://localhost:8000 if it's already an icelines server" and loop instead of panicking. Server runs until Ctrl-C; on stop, control returns to the menu.
    - `D` → spawn `$PAGER COMMANDS.md` (or `less`, or stdout if no pager). Wait for pager exit, then loop.
    - `Q` / `q` → `break`.
    - Anything else → "Choose 1-8, P, T, G, C, W, D, or Q." → loop without clearing.
- New `[menu]` config section in `Config` (`icelines-cli/src/config.rs`): `web_port: u16`, `web_bind: Option<String>`. Empty section → hardcoded defaults. Forward-compat for `icelines menu --port 9000` later.
- `main.rs` dispatch arm.

**Tests** (L0):
- Slug-to-Screen mapping for the 8 menu choices is identical to LB.1's (don't duplicate; reuse `parse_start_slug`).
- Dispatch logic mocked: pass a trait-objected `MenuLauncher` (with `launch_tui`/`launch_serve`/`launch_pager` methods) into `menu::run`; mock impl records calls. Pipe synthetic input + assert recorded calls. **The naïve `1\nQ\n` subprocess pipe doesn't work** — the launched TUI would consume `Q` from stdin before the menu's `read_line` saw it. Unit test the dispatch table, integration-test the rest manually.
- Non-TTY guard: simulate non-TTY via mock `IsTerminal` → assert exit 0 with redirect message.
- Bind-error catch: mock `commands::serve::run` returning `AddrInUse` → assert menu loops instead of panicking.

**Tests** (L2 — minimal subprocess coverage):
- `lb_l2_non_tty_menu_exits_clean` — `icelines menu < /dev/null` exits 0, stdout contains "needs an interactive terminal".
- That's the only L2 menu test. The rest is unit-tested via the mocked launcher trait.

**Manual smoke**: each option dispatches; loop returns to prompt after each surface quits; tip line shows the equivalent direct command. Deliberate panic inside a TUI surface (via assert in test build) — terminal restores; menu re-renders cleanly (validates LB.0.5 + clear_screen).

**Exit**: All choices wired; loop semantics confirmed; non-TTY guarded; bind-error caught; ctrlc handled.

---

### LB.5 — In-process render-harness smokes + L2 dispatch smokes + frozen fixtures

**Deliverable** — extend the existing `icelines-cli/src/tui/screens/mod.rs:236-298` harness pattern. The existing `l0_app_renders_every_canonical_landing_screen_without_panic` is the model; add the new `Screen` variants reachable via `--start`.

**L0 render smokes** (in-process, microsecond cost):

```rust
#[test] fn lb_render_league()       { assert_renders(Screen::Home,         "LEAGUE"); }
#[test] fn lb_render_depth()        { assert_renders(Screen::Depth,        "DEPTH"); }
#[test] fn lb_render_stats()        { assert_renders(Screen::Queries,      "STATS"); }
#[test] fn lb_render_goalies()      { assert_renders(Screen::Goalies,      "GOALIES"); }
#[test] fn lb_render_scores()       { assert_renders(Screen::Tonight,      "SCORES"); }       // network-skipped — see below
#[test] fn lb_render_schedule()     { assert_renders(Screen::Schedule,     "SCHEDULE"); }     // network-skipped
#[test] fn lb_render_transactions() { assert_renders(Screen::Transactions, "TRANSACTIONS"); } // network-skipped
#[test] fn lb_render_playoffs()     { assert_renders(Screen::Playoffs,     "PLAYOFFS"); }

#[test] fn lb_render_player_by_pid()
    { assert_renders(Screen::PlayerById(MCDAVID_PID), "McDavid"); }
#[test] fn lb_render_player_loading_career_placeholder()
    { /* construct a player-card scene with a pid NOT in the active repo, assert "Loading career..." appears */ }
#[test] fn lb_render_team_card()
    { assert_renders(Screen::Team("EDM".into()), "EDM"); }
```

Network-touching surfaces (Scores / Schedule / Transactions) get a basic header-rendered smoke now; full content render is gated behind harness-level network mocking, tracked as a Future / parked item.

**L2 subprocess smokes** (clap + main.rs end-to-end):

```rust
#[test] fn lb_l2_unknown_slug_exits_nonzero()        { /* tui --start zzz → exit !=0, stderr "valid slugs" */ }
#[test] fn lb_l2_unknown_player_exits_nonzero()      { /* tui player Zzzzzz → exit !=0, stderr "Did you mean" */ }
#[test] fn lb_l2_ambiguous_player_lists_candidates() { /* tui player Smith → exit !=0, stderr lists candidates */ }
#[test] fn lb_l2_non_tty_menu_exits_clean()          { /* icelines menu < /dev/null → exit 0, "interactive terminal" */ }
```

These four are sufficient to guard the dispatch path; render coverage lives in L0.

**Frozen fixtures** (per BENCH):
- `MCDAVID_PID = PlayerId(8478402)` — pid never changes.
- `GRETZKY_NAME = "Gretzky"` — bundled bio guaranteed since 1979.
- `BRODEUR_NAME = "Brodeur"` — retired goalie, stable bundled bio.
- `STABLE_TEAM = "EDM"` — Edmonton has existed since 1979.

Bedard fine as a SECONDARY active-player smoke but not the only `player:NAME` test fixture.

**Coverage map vs EDGE failure-mode list**:
- Sebastian Aho ambiguity → `lb_l2_ambiguous_player_lists_candidates` ✓
- Diacritic → L0 `parse_start_slug("player:Léhkonen") == parse_start_slug("player:Lehkonen")` ✓
- Apostrophe → L0 `parse_start_slug("player:O'Reilly")` ✓
- Empty/whitespace → L0 grammar tests ✓
- Pid-vs-name overlap → digit-detection at parse time ✓
- Trailing whitespace on team → L0 trim test ✓
- Network-dependent surfaces → punted to harness-mock follow-up ⚠
- Ctrl-C handling → mocked launcher unit test in LB.4 ✓
- Bind error → mocked launcher unit test in LB.4 ✓
- 1×1 TTY → punted to follow-up (document min size in `--help`) ⚠
- Panic restore → `lb_l2_panic_restores_terminal` (catch_unwind in LB.0.5) ✓

**Exit**: All L0 render smokes + 4 L2 dispatch smokes pass; CI green.

---

### LB.6 — Docs refresh

**Deliverable**:
- **COMMANDS.md** — new "TUI surfaces" section listing all 8 nav-tab sugar forms, all 4 drill-down sugar forms, plus `icelines menu` semantics. Mark canonical vs alias slugs explicitly.
- **README.md** — usage primer mentions a few high-value launches: `icelines tui scores`, `icelines tui player Bedard`, `icelines menu`.
- **`icelines tui --help`** (clap `long_about`) — lists the 12 sugar forms with one-line descriptions; calls out `Ctrl-C exits cleanly` for menu.
- **`icelines docs`** auto-includes the COMMANDS.md update (compile-time include).

**Tests**:
- L0 string assertion that `COMMANDS.md` contains every canonical slug from `SLUG_TABLE` (drift fence).
- L0 string assertion that `COMMANDS.md` mentions `icelines menu` and `icelines tui player`.

**Exit**: docs ship the surface inventory; the SLUG_TABLE-vs-COMMANDS.md drift test passes.

---

### LB.7 — Hands-on persona pass

**Deliverable**: A short note (`design/notes/2026-05-05-LB-persona-pass.md`) capturing:
- One paragraph per surface: launched via `icelines tui <slug>`; what's visible at first paint; any obvious bug or polish item.
- Drill-down passes: `tui player Bedard`, `tui player Gretzky` (verifies Loading-career placeholder), `tui player Smith` (verifies ambiguity listing), `tui team EDM`. Does the lazy fan-out feel slow? Is the back-stack right (Esc → ?).
- Menu loop: dispatch each option; confirm return-to-prompt feels natural; deliberately Ctrl-C in the menu vs in a launched surface; intentional panic via assert (validates TerminalGuard).
- Issues filed as follow-up tasks (one per problem found), labelled `LB-followup`.
- A "what's next" section: should LB.8 (App split / lazy data) ship? Or are wrappers good enough?

**Tests**: none — manual exit gate.

**Exit**: walk-through done; follow-ups filed.

---

## Out-of-scope follow-ups (future phase, not this one)

- **LB.8 — App split / lazy data loading.** Per-surface mini-apps with their own data scopes. Big rebuild; defer.
- **`icelines menu <slug>` shortcut.** Skip the prompt.
- **Web option `W` server-detection.** Don't spawn a duplicate; print URL instead.
- **Locked surfaces.** `--locked` to disable Tab.
- **Network mocking for L0 render of Scores/Schedule/Transactions.** Harness-level mock so these three get full rendering coverage, not just header.
- **JSON twin for `icelines menu`.**
- **1×1 TTY guard / min-size enforcement.**

---

## Risks and mitigations

| Risk | Mitigation |
|------|-----------|
| **`Commands::Tui` literal audit misses a call site** (FORGE flagged `Commands::Dashboard` at `main.rs:364`; EDGE flagged persona test waves) | LB.1 audits all four files explicitly: `main.rs`, `persona_scenarios.rs`, `persona_wave2.rs`, `persona_wave3.rs`, `persona_wave4.rs`. CI catches anything missed via compile errors. |
| **`SLUG_TABLE` drifts from COMMANDS.md** (WIRE) | LB.6 ships a string-grep test that asserts every canonical slug appears in COMMANDS.md verbatim. |
| **`resolve_player_id_by_name` slow in L0 tests** (BENCH) | Function is sync, walks bundled bios newest-first; Bedard / Gretzky resolve in <1ms; Hellebuyck (goalie path) walks all 38 skater seasons first then goalie bios — bounded. Acceptable. If iteration cost matters in tests, cache via `OnceLock`. |
| **Bedard retires before Phase Lady Byng ships** (BENCH) | Use frozen fixtures (Gretzky / McDavid pid / Brodeur / EDM) for stable smokes. Bedard is secondary. |
| **Menu loop test piping `1\nQ\n` doesn't work** (BENCH) | Don't try. Mock the dispatch table behind a trait; unit-test the menu's logic; cover the integration path via manual smoke in LB.7. |
| **Cold-launched surface paints empty before lazy fan-out** (GLASS, EDGE) | LB.3 ships `Loading career…` placeholder. L0 render test asserts placeholder appears for pids not in active repo. |
| **Ctrl-C in menu prompt exits 130 instead of 0** (EDGE) | LB.4 installs `ctrlc::set_handler` flipping AtomicBool; loop checks after `read_line`. Documented in `--help`. |
| **Web option `W` panics on `AddrInUse`** (EDGE) | LB.4 catches the bind error explicitly; prints URL hint; returns to loop. |
| **Sebastian Aho problem — `tui player Smith` silently picks one** (EDGE, WIRE) | LB.3 lists all candidates with team+season; exits non-zero before TUI boots. Spec language fixed. |
| **`tui --start "player: "` matches first bio in bundle** (EDGE) | LB.3 rejects empty/whitespace-only arg at PARSE time, before `normalize_name`. |
| **Terminal teardown leaks on panic** (FORGE — BLOCKING) | LB.0.5 lands `TerminalGuard` with Drop impl BEFORE LB.4. |
| **ConPTY artifact buildup on Windows between menu dispatches** (GLASS) | LB.4 calls `clear_screen` between dispatches. |
| **Cold-entered drill-down user thinks Esc = quit** (GLASS) | LB.3 + LB.2 surfaces draw `Esc: League · q: quit · y: season picker` footer when entered cold. |
| **`<slug>:<arg>` collides with future namespacing needs** (WIRE) | Spec declares: exactly one colon; arg opaque; reserved separators going forward. Future nested forms use a different pattern. |
| **Network-touching surfaces (Scores/Schedule/Transactions) panic on render-harness without network** (EDGE) | LB.5 renders header-only smoke for these three; full render gated behind harness-level network mock (deferred). |
| **Slug rename later breaks scripts** (WIRE) | Spec declares stability tier: canonical removal requires one-release WARN cycle to stderr; aliases hidden from error suggestions. |

---

## Workspace verification (per milestone exit)

Each milestone closes with:
1. `cargo build --workspace` clean.
2. `cargo test -p icelines-cli` clean.
3. `cargo clippy -p icelines-cli -- -D warnings` clean.
4. `cargo fmt --check` clean.

Phase exit additionally runs:
5. `cargo test --workspace --no-fail-fast`.
6. Manual hand-walk of all 8 nav surfaces + 4 drill-downs + menu (including panic + Ctrl-C paths).

---

## Commit/tag conventions

- Commit subjects: `Phase Lady Byng (TUI experiences): LB.X — <slug>` (the parenthesis disambiguates from Phase 2 Site, which used `Phase Lady Byng:` in 2026-04-25).
- Plan summary commit at phase exit referencing this file.
- No new tag for the phase if it lands inside v0.13.x; phase exit may bump to v0.14.0 with the LB work bundled in.

---

## Roles + review hooks

The roles most likely to find issues here (per `.roles/`):

- **glass** — TUI rendering: cold-entry orientation, lazy-fan-out flash, clear_screen behavior, footer hint visibility.
- **forge** — Rust quality: TerminalGuard RAII, clap option (b), RunTuiOpts struct, StartSlugError shape, persona-wave audit.
- **bench** — test coverage: in-process render harness reuse, frozen fixtures, mocked menu launcher trait, drift fence on SLUG_TABLE↔COMMANDS.md.
- **wire** — CLI contract: slug stability tier, `<slug>:<arg>` grammar, `[menu]` config section, ambiguity exit semantics.
- **edge** — failure modes: Sebastian Aho ambiguity, empty needle, diacritic/apostrophe, Ctrl-C exit code, bind error, panic restore, BUNDLED_SEASONS order.
- **pace** — performance: drill-down boot cost, lazy fan-out cost on first paint.

Less load-bearing here:
- **scout** (no analytical correctness changes), **tape** (no pipeline changes), **hart** (no domain model changes), **keel** (single-surface phase — no cross-surface convergence concerns this round).
