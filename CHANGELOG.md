# IceLines Changelog

## Unreleased - Phase Prince of Wales closeout

Headline: **Prince of Wales is closed as the visual-system phase: IceLines now
has shared visual tokens, representative TUI/web/CLI visual fences, and a CREST
closeout review.**

### What changed

- Added shared TUI visual helpers and render contracts for representative Team,
  Goalies, Schedule/Scores, and Poach screens at 80x24 and 120x32.
- Consolidated representative web route styling onto shared Prince classes for
  home, leaders, player, team/depth, goalies, scores/schedule/playoffs,
  fantasy, and poach.
- Added `prince_cli_visual` subprocess tests for 80-column no-color CLI output
  on leaders, goalies, and poach, and tightened the poach table/footer to fit.
- Recorded the final CREST/roles verdict as PASS WITH NOTES, with screenshot
  automation and secondary-route polish carried forward to Jim Gregory.

### Jim Gregory release hardening

- Added `scripts/release-smoke.ps1`, a reusable optimized-binary smoke gate for
  version/help, leaders, goalies, TUI help, web serve help, docs, markdown
  export, poach, and URL printing from `serve --no-open`.
- Added `design/release-checklist.md` with versioning rules, release gates,
  artifact names, tag flow, rollback notes, and data/current-season sanity.
- Added `design/current-season-rollover.md`, release bundle tests, and updated
  data freshness docs so embedded-data claims match the actual release
  automation.
- Added release-workflow artifact verification so packaged zip/tarball outputs
  are checked for the expected binary before upload.
- Updated Jim Gregory roadmap/index docs to record release hardening as
  implemented locally with latest remote CI pending.
- Updated CI path filters so release workflow/checklist/rollover docs and
  release-smoke changes run CI; the CI release job now runs the optimized
  release smoke gate.

---

## v0.24.1 - 2026-05-09 - Phase Lester Patrick closeout

Headline: **Lester Patrick is closed as the CLI parity pass: schedule,
playoffs, transactions, and the in-TUI manual/docs overlay are implemented and
covered by focused tests.**

### What changed

- Repaired stale TUI userflow tests that used pre-Favorites/pre-Poach tab
  indices for Scores, Schedule, Transactions, and Playoffs.
- Marked the Lester Patrick plan and roadmap entries implemented.

---

## v0.24.0 - 2026-05-09 - Phase Messier (TUI filter/sort consistency)

Headline: **TUI player-list screens now share one filter/sort vocabulary.
Team, Goalies, Depth, Favorites, and Stats expose consistent keybinds where
the screen supports them, and MDI cmdbar verbs accept typed roster
`key=value` filters through the same parser.**

### What shipped

- Shared `RosterFilterState`, typed roster KV parsing, duplicate-key
  validation, and deterministic command execution for roster filters.
- Goalies, Depth, Favorites, and Team now accept MDI KV commands such as
  `:goalies sort=gaa min-gp=20`, `:depth pos=F`, `:favorites sort=name`, and
  `:team EDM pos=LW country=CAN`.
- `f` on Goalies, Depth, and Favorites pre-fills the MDI command bar with the
  appropriate verb so free-form filters use the same grammar as typed
  commands.
- Stats keeps the Art Ross filter overlay while adding nationality shortcut
  parity and `stats ...` KV lowering through the deterministic parser.
- `COMMANDS.md` now documents the unified TUI keybind matrix and roster KV
  command examples.

### Carry-forward

- CLI parity remains Phase Lester Patrick.
- Web parity remains Phase Ted Lindsay.
- Visual polish remains Phase Prince of Wales.

---

## v0.23.5 — 2026-05-08 — Phase Jack Adams.12 (Team country filter + Hits column toggle)

Headline: **Closes the original Team-screen wishlist from v0.23.0
kickoff. Country filter via `c` (cycles None → CAN → USA → SWE → FIN
→ RUS → CZE → SVK → None). Hits column toggle via `h` shows the Hits
column independent of sort key (so you can sort by Pts/82 and still
see Hits).** Bin suite 1042 → 1051.

### What shipped

- `TeamScreenState.country_filter: Option<&'static str>` — None means
  "all"; Some(code) matches the bio's `nationality_code` (case-
  insensitive).
- `TeamScreenState.force_hits_column: bool` — toggle independent of
  sort. Hits column now renders when `sort=Hits OR force_hits_column`.
- `c` keybind cycles country, `h` keybind toggles Hits column. Chrome
  accessor advertises both.
- COUNTRY_CYCLE constant lists the canonical NHL nationalities. Wider
  sets continue through `:query country=XYZ` from the Stats screen.
- 7 new L0 team tests (country cycle wraps, country label, default
  force_hits=false, COUNTRY_CYCLE includes the canonical codes).
- 2 new L1 tests in mod.rs (`c` cycles country end-to-end, `h` toggles
  hits column end-to-end).

### Test growth

- Bin: 1042 → 1051 (+9)

### Adams arc complete

- v0.23.0 — MDI dashboard, deterministic ship (Adams.1–5)
- v0.23.1 — AI fallback (Adams.6–7)
- v0.23.2 — Cmdbar UX polish: sticky focus, history, Tab leave (Adams.8)
- v0.23.3 — Per-screen sub-command hint row (Adams.9)
- v0.23.4 — Team sort+filter + Depth/Favorites chrome (Adams.10/.11)
- v0.23.5 — Team country filter + Hits column toggle (Adams.12)

The user's original wishlist from the v0.23.0 kickoff: **sort by Pos**,
**show Hits**, **show F or hide D**, **show only LW**, **country=CAN**
— all four addressed. Test growth: 803 → 1051 across the full Adams arc
(+248 net new).

---

## v0.23.4 — 2026-05-08 — Phase Jack Adams.10 / .11 (Team sort+filter, Depth/Favorites chrome)

Headline: **The Team screen gains real per-screen sort/filter capability,
filling the audit gap from v0.23.3. Press `s` to cycle sort (Pace / Name /
Pos / G / Hits) and `p` to cycle position filter (All / F / D / C / LW / RW
/ LD / RD). Depth and Favorites screens get chrome accessors so their
per-screen rows show real keybinds.** Bin suite 1033 → 1042.

### Adams.10 — Team sort + filter

- New `TeamSort` enum (Pace / Name / Position / Goals / Hits) with `next()`
  cyclic stepper. `TeamPosFilter` enum (All / Forwards / Defense / C / LW
  / RW / LD / RD) with `matches(pos_abbrev)` predicate.
- `TeamScreenState { sort, pos_filter }` lives on `App::team`.
- `s` and `p` keybinds on Team screen cycle sort and position filter.
- `team::chrome(&state)` accessor advertises `s=cycle sort · p=cycle pos
  · ↑↓=select · Enter=open card · g=add to group`. Title shows the
  current state (`Team · sort=Hits · pos=F`).
- Render dynamically adds a column when sort is on Hits or Goals (so the
  user sees the value driving the order).
- Country-code filter (`country=CAN` style) deferred to Adams.10b — needs
  bio integration for arbitrary subsets; cmdbar `:query country=CAN` from
  the Stats screen already covers this use case in the meantime.

### Adams.11 — Depth + Favorites chrome

- `depth::chrome(scoring_mode)` — advertises `s=toggle scoring · ↑↓=select
  · Enter=team chart`. Title shows current mode (`Depth · scoring=Pace`).
- `favorites::chrome()` — advertises `g=manage groups · Enter=open card ·
  :fav add=from cmdbar`. Per-screen row no longer falls back to the
  placeholder for these two screens.

### Test growth

- Bin: 1033 → 1042 (+9)
- 9 Team L0 tests (sort cycling, pos-filter cycling, predicate matching,
  default contract, chrome assertions)
- 3 L1 tests in mod.rs (s cycles team sort, p cycles team pos, s on
  Goalies doesn't touch team state — confirms screen-scoped dispatch)
- 1 boundary test repurposed (Team fallback → Team real chrome)

---

## v0.23.3 — 2026-05-08 — Phase Jack Adams.9 (per-screen sub-command hints)

Headline: **The MDI dashboard now shows a cyan per-screen sub-command
strip directly above the global verb cheat sheet. When the workspace is
on Stats, you see `f=filter · /=sort · s=save · l=load · …`; when it's
on Goalies, `s=sort · m=min-gp · …`; when it's on Schedule, `/=search ·
t=today · D=date picker · …`. Switches automatically as `:goalies` /
`:stats` / `:schedule` swap the workspace.** Bin suite 1030 → 1033.

### What shipped

- New `render_mdi_screen_keybinds` row pulls keybinds from
  `active_chrome(app)` (Masterton.1 contract). Each screen's
  declarative `chrome.keybinds` list is the source of truth.
- Adaptive truncation: chips drop with a trailing `…` when the row
  doesn't fit at narrow widths.
- Placeholder for screens without chrome accessors yet (Team / Depth /
  Favorites): `Team: no per-screen keys yet — use cmdbar verbs below`.
  These are tracked for follow-up — see Adams.10 / .11 below.
- 3 new render-level tests confirming Goalies / Stats / Team-fallback
  render the right hints.

### Layout (now 5 rows of chrome instead of 4)

```
[ Scores ribbon                                                     ]  1 row, top
[ Favorites │ Workspace                            │ Schedule        ]  body
[  Stats: f=filter · /=sort · s=save · l=load · …                    ]  per-screen (NEW)
[  stats · goalies · transactions · playoffs · depth · scores · …    ]  cheat sheet
[  > _                                                               ]  cmdbar
```

### Test growth

- Bin: 1030 → 1033 (+3)

### What's deferred to v0.23.4 / .5

Team / Depth / Favorites screens don't have chrome accessors yet, so
their per-screen rows show the placeholder. Adding real sort/filter
keybinds to those screens is tracked as Adams.10 (Team — sort by Pos,
filter by position class, country filter, hits column) and Adams.11
(Depth + Favorites chrome accessors).

---

## v0.23.2 — 2026-05-08 — Phase Jack Adams.8 (cmdbar UX polish)

Headline: **The cmdbar gains sticky focus, history navigation, an
explicit Tab-to-leave control, and an always-visible verb cheat sheet
above the prompt row. User feedback after living with v0.23.0 for an
afternoon: "I want to re-edit the query, up-arrow to cycle, stay in
the edit bar, tab out to leave, and see the top commands I can call."**
Bin suite 1024 → 1030.

### What shipped

- **Sticky focus** — submitting a successful command no longer
  defocuses the bar. Input clears and the prompt stays in `> _` so
  the user can type the next command immediately. Empty Enter still
  defocuses (explicit "I'm done" signal); Esc and Tab also defocus.
- **Up / Down history navigation** — Up walks backward (older) through
  `command_history`, Down walks forward (newer). Past the newest,
  Down returns to live edit (cleared input, cursor=None). Typing or
  Backspace breaks navigation back to live edit. The 50-entry cap
  from Adams.2 still applies.
- **Tab to leave** — pressing Tab while the bar is focused defocuses
  + clears input. Vim-style cmdline cancel.
- **Always-visible cheat sheet** — new yellow row directly above the
  cmdbar lists canonical verbs at all times (no longer gated on
  focus): `stats · goalies · transactions · playoffs · depth · scores ·
  schedule · favorites | team <ABBR> · player <name> · query <filter> ·
  /fav add <name> · /help`. Adapts to width (compact at <100 cols).
- **Updated chip-mode hint** — when bar is empty + unfocused, the
  inline hint now mentions `↑↓ history · Tab leave bar` so the new
  keys are discoverable.

### Examples

```
# Sticky-focus power-user loop
:stats         Enter        # workspace → Stats; bar stays focused
:goalies       Enter        # workspace → Goalies
↑              # bar shows "goalies"
↑              # bar shows "stats"
↓              # bar shows "goalies"
↓              # bar empty (back to live edit)
Tab            # leave bar → workspace gets keystrokes again
```

### Test growth

- Bin: 1024 → 1030 (+6 net new)
- 6 new app-level tests: sticky-focus, Tab-leaves-bar,
  Up-walks-backward, Down-walks-forward, typing-breaks-history-nav,
  Up-with-empty-history.
- Existing tests updated to reflect sticky-focus contract: 1
  app-level + 1 persona scenario (s096 cheat-sheet visibility).

---

## v0.23.1 — 2026-05-08 — Phase Jack Adams.6/.7 (AI fallback for cmdbar)

Headline: **The MDI cmdbar gains an opt-in natural-language fallback.
When the deterministic Phase Art Ross parser rejects an input, the
cmdbar can delegate to a configured LLM provider for interpretation —
"show me young scorers" → `query g >= 25 AND age <= 23`. Off by
default; set `[ai] enabled = true` in `~/.icelines/config.toml` to
opt in.** Bin suite 1005 → 1029.

### What shipped (Adams.6 — Claude CLI provider)

- `icelines-cli/src/ai.rs` — new module with the `AiProvider` trait,
  `AiConfig`, `AiError` (thiserror), `default_system_prompt` (~150-line
  grammar reference covering every cmdbar verb + Phase Art Ross filter
  syntax), `StubProvider` for tests, and `ClaudeCliProvider` that
  shells out to `claude -p`.
- `[ai]` TOML section on `Config` — `enabled`/`provider`/`model`/
  `timeout_secs` keys with sensible defaults (disabled / `claude-cli` /
  `claude-haiku-4-5` / 15s).
- `MdiLayout::ai_pending` holds an in-flight `AiPending` (tokio oneshot
  receiver + original input + provider name + start time). Esc cancels.
- `App::try_spawn_ai_fallback` — on parse error, spawns the provider
  call as a tokio task. `App::mdi_poll_ai` polls each render tick;
  responses are re-parsed through the standard `parse_command` path
  (AI output is treated as untrusted). `App::dispatch_ai_response`
  applies the response, tags history with `ai:<command>` prefix.
- 17 tests (10 ai-module + 7 app-wiring) including AI-disabled
  fallthrough, unparseable-response flash, channel-closed detection.

### What shipped (Adams.7 — Anthropic API provider + closeout)

- `AnthropicApiProvider` impl in `ai.rs` — direct reqwest POST to
  `https://api.anthropic.com/v1/messages`. Reads `$ANTHROPIC_API_KEY`
  at provider construction. Handles HTTP errors, timeouts, and the
  Messages API response shape.
- Provider selection via `[ai] provider = "anthropic-api"` in config.
- 3 additional tests for Anthropic provider (build_provider success,
  no-API-key error path, env-read at construction).
- COMMANDS.md `[ai]` configuration section + example.

### Examples (new in v0.23.1)

```toml
# ~/.icelines/config.toml — opt into AI cmdbar fallback
[ai]
enabled = true
provider = "claude-cli"           # or "anthropic-api"
model = "claude-haiku-4-5"
timeout_secs = 15
```

```bash
# Then in icelines tui --mdi:
# Type :show me young canadian scorers under 23 + Enter
# - Parser rejects (UnknownCommand)
# - AI fallback fires; flash: "asking claude-cli… (Esc to cancel)"
# - Provider returns: "query country = CAN AND age < 23 AND g >= 20"
# - Re-parsed + executed; workspace swaps to Stats with the filter
# - History entry: "ai:query country = CAN AND age < 23 AND g >= 20"
```

If the provider returns a string the parser STILL rejects, the cmdbar
flashes the parser error with the model's response so the user can
edit. Esc at any time aborts an in-flight request.

### Determinism guarantee

- Parser is canonical. AI is supplemental.
- AI output goes through the same `parse_command` path as user input.
- AI failures fall back to the deterministic `ParseError` flash.
- `[ai] enabled = false` (default) is identical to pre-v0.23.0 behavior.

### Test growth

- Bin: 1005 (v0.23.0) → 1024 (v0.23.1), +19 net new
- Adams.6 wiring: 17 (10 ai-module + 7 app-wiring)
- Adams.7 anthropic provider: 2 additional ai-module
- 4 build_provider tests now exercise both providers

---

## v0.23.0 — 2026-05-08 — Phase Jack Adams (MDI dashboard, deterministic ship)

Headline: **The TUI is no longer single-document. `icelines tui --mdi`
launches a multi-pane "front door" dashboard: live Scores ribbon on top,
swappable Workspace in the middle, Favorites + Schedule side panes, plus
a chat-CLI command bar at the bottom that drives every screen swap and
filter from a single text prompt.** Bin suite 803 → 905+, +100 tests.

### What shipped

- **Adams.1** — MDI layout engine + workspace dispatcher.
  `MdiLayout` runtime state, `--mdi` clap flag (mutually exclusive with
  `--standalone`), adaptive `effective_panes(width)` matrix (≥160 full /
  120-159 drop schedule / 100-119 drop favorites / <100 SDI fallback).
  13 layout tests + 6 fence tests + 3 L2 surface checks.
- **Adams.2** — chat-CLI command bar (parser + executor + UI).
  Strict verb-or-slash grammar (`stats`, `goalies`, `query <filter>`,
  `team EDM`, `box <game-id>`, `compare <a> <b>`, `/fav add Bedard`,
  `/hide schedule`, `/help`, `/quit`). Three-mode footer: chip-mode hints
  → `> {input}_` prompt-mode → red `! {error}` flash-mode. Focus on `:`
  or `/`; defocus on Enter / Esc / Backspace-at-empty. 49 command tests
  + 18 app-level tests including a property sweep that proves every
  printable ASCII char (0x20-0x7E) types into the bar (catches the
  event-mapper rewriting `q` → Action::Quit problem).
- **Adams.3** — real pane content + side-pane toggles.
  Workspace dispatches on `app.screen` to the existing per-screen
  renderers (cyan border, dynamic title); Favorites pane (yellow,
  28-col) reuses `favorites::render`; Schedule pane (magenta, 32-col)
  reuses `schedule::render`; Scores ribbon reads the live Tonight
  cache (Loading / Loaded / Error / Idle). New global keybinds: **Ctrl+H**
  toggles Favorites pane, **Ctrl+L** toggles Schedule pane (work even
  while cmdbar is focused). 4 toggle tests.
- **Adams.4** — adaptive width + auto-drop polish.
  Render-level boundary tests at every threshold (200, 160, 159, 120,
  119, 100, 99, plus panic-free property sweep 80..=240); L1 resize
  sequence 200→159→119→99→200 verifies state preservation; L2 surface
  composition checks (`tui --mdi goalies --help`). 17 tests across L0/
  L1/L2.
- **Adams.5** — closeout fixes for the deterministic ship.
   - `mdi_tick_fetch`: Scores ribbon and Schedule pane now auto-fetch
     every render frame regardless of which workspace screen is active
     — previously the data sources only populated when the user visited
     the SDI Tonight or Schedule tabs first. Idempotent (Loading-state
     guarded).
   - **MDI help overlay** — pressing `?` (or typing `/help`) in MDI mode
     now shows a comprehensive command reference: workspace verbs, args,
     write actions, layout commands, global hotkeys. SDI keeps the
     legacy keybind cheat sheet. 4 tests.
   - Chip-mode hint expanded from generic key list to actionable command
     examples (`:stats · :goalies · :team EDM · ^H favs · ^L sched`).
   - Overlay painter extracted to `render_overlays` so MDI gets help /
     admin / season picker / reports / docs / group picker just like
     SDI.

### Examples (new in v0.23.0)

```bash
icelines tui --mdi                         # multi-pane dashboard
icelines tui --mdi goalies                 # MDI launching with goalies as workspace
```

In the dashboard, type:

```
:stats                                     # workspace → Stats
:goalies                                   # workspace → Goalies
:query g >= 30 AND age <= 25               # apply filter, swap to Stats
:team EDM                                  # workspace → Oilers depth chart
:box 2025020001                            # workspace → boxscore
:compare Bedard McTavish                   # head-to-head
/fav add Bedard                            # add to Favorites
/hide schedule                             # hide schedule pane (or Ctrl+L)
/show favorites                            # restore favorites pane
?                                          # full command reference overlay
:q                                         # quit
```

### Trophy fit

Jack Adams — best NHL coach. The MDI dashboard is the "coach" surface:
sees everything at once (Scores ribbon, Schedule, Favorites, Workspace),
calls plays via the cmdbar, drives the team. Spec:
`design/specs/phase-jack-adams-overview.md`. Plan:
`design/plans/2026-05-08-phaseJackAdams-mdi-dashboard.md`.

### What's deferred to v0.23.1

Adams.6 — AI LLM fallback. When `parse_command` rejects an input,
delegate to a configured LLM provider (Anthropic API or `claude -p`
shell) for natural-language → command interpretation. Opt-in via
`~/.icelines/config.toml`; defaults off.

### Test growth

- Bin (TUI + commands): 803 → 905+ (+100 net new across L0/L1/L2)
- Adams.2 cmdbar: 49 command tests + 18 app-level focus/typing tests
- Adams.3 panes/toggles: 4 toggle tests
- Adams.4 boundary: 9 L0 render + 3 L1 resize + 5 L2 surface
- Adams.5 closeout: 2 auto-fetch + 2 help overlay tests

---

## v0.22.0 — 2026-05-08 — Phase Masterton (chrome + standalone mode + Screen trait scaffold)

Headline: **Two user-facing features (declarative chrome,
standalone single-screen mode) + scaffolding for the future
deep-dispatch refactor. The TUI now has a consistent header +
keybind footer across screens, and any surface can be hosted
focused via `--standalone`.** Bin suite 763 → 803, +40 tests.

### What shipped

- **Masterton.1 (v0.21.2)** — declarative TUI chrome.
  `ScreenChrome { title, keybinds }` accessor on each main
  screen; shell renders both consistently. 19 tests.
- **Masterton.2.1** — Screen trait + AppContext + ScreenAction
  + dispatch hub. Foundation for future per-screen migrations
  (deferred, see below). 14 tests.
- **Masterton.3** — `--standalone` flag on `icelines tui`.
  When set, the TUI is locked to one surface: Tab/Shift+Tab
  no-op, tab strip is hidden, the screen's chrome title leads
  the header as a breadcrumb. Per-screen keybinds + cross-
  screen overlays (?, F, y, R, M) still work. 9 tests.

### Examples (new in v0.22.0)

```bash
icelines tui goalies --standalone        # focused goalies leaderboard
icelines tui scores --standalone         # focused live-scores TUI
icelines tui transactions --standalone   # focused transactions feed
```

### What's deferred (honest framing)

The original spec scoped a deep per-screen Screen-trait
migration (Masterton.2.2-2.7). After auditing `App::handle`,
the migration shape was wider than estimated:

- Each screen's render fn takes `&App` (needs repo + dashboard
  + league_context); a real migration would thread AppContext
  through every render fn — 1-2 weeks of behavior-preserving
  work for 6 screens.
- The dispatch isn't structured as one match per screen; it's
  one giant match per Action, with screen-specific branches
  inside each Action arm. Extracting Goalies (the simplest
  screen) requires touching ~5 distinct dispatch sites; full
  migration would touch ~80+ sites across 6 screens.

The Screen trait scaffold (M.2.1) is in place for when a
specific screen needs the deeper isolation (e.g., for genuine
in-process embedding by a third party, or for property-test
infrastructure that needs to run a screen without a real App).
Masterton.3's `--standalone` flag delivers the user-facing
"focused single-screen TUI" feature without requiring the
trait migration first.

### Cumulative across Phases Norris + Norris.6 + Masterton

- App field count: 80+ → 38
- Per-screen state structs: 8 (Queries, Schedule, Transactions,
  Goalies, Playoffs, Tonight, DatePicker, GroupPicker)
- Trait scaffold types: Screen + ScreenAction + OverlayKind +
  AppContext + dispatch hub
- TUI chrome: declarative across 6 main screens
- TUI launch modes: multi-tab (default) + standalone (--standalone)
- Bin suite: 692 (v0.20.3) → 803 (v0.22.0), +111 net new tests

### Internal architecture summary

```
App (orchestrator)
├── State (per-screen, Norris)
│   ├── QueriesState        (17 fields)
│   ├── ScheduleScreenState (8 fields)
│   ├── TransactionsState   (8 fields)
│   ├── GoaliesState        (3 fields)
│   ├── PlayoffsScreenState (3 fields)
│   ├── TonightScreenState  (4 fields)
│   ├── DatePickerState     (4 fields, Norris.6)
│   └── GroupPickerState    (3 fields, Norris.6)
├── Chrome (declarative, Masterton.1)
│   └── tui::screens::<screen>::chrome(state) -> ScreenChrome
├── Dispatch hub (Masterton.2.1)
│   ├── ScreenAction enum
│   ├── OverlayKind enum
│   ├── AppContext (split-borrow)
│   └── App::dispatch / App::make_context
└── Launch modes (Masterton.3)
    ├── Multi-tab (locked_screen = None) — default
    └── Standalone (locked_screen = Some(X)) — Tab no-op
```

## v0.21.2 — 2026-05-08 — Phase Masterton.1 (declarative TUI chrome)

Headline: **Each main TUI screen now declares its header title +
keybind hints via a `ScreenChrome` accessor; the shell renders
both consistently across screens.** Foundation for Masterton.2's
Screen trait migration. Minimal behavior change.

### What's new

- **`tui/chrome.rs`** — new module with `ScreenChrome { title,
  keybinds }`, `KeyHint` (const-constructible, Copy, PartialEq),
  and a 6-key `GLOBAL_KEYBINDS` array (`Tab` / `y` / `Shift+P` /
  `F` / `?` / `q`) extracted from the previously-hardcoded
  hint string in `render_nav`.
- **6 per-screen `chrome()` accessors** (Queries, Schedule,
  Transactions, Tonight, Goalies, Playoffs). Each returns a
  state-aware title (e.g., "Stats / Queries / Filter" when
  FilterEdit mode is active, "Goalies — SV% ↓ · GP ≥ 15"
  reflecting the active sort + threshold) and a keybind set
  scoped to the screen's mode.
- **`screens/mod.rs::render` refactored**:
  - `active_chrome(app)` dispatcher.
  - `render_header(f, app, &chrome, area)` — tabs left, title
    right-aligned in cyan when terminal ≥120 cols (per spec
    glass-1).
  - `render_footer(f, app, &chrome, area)` — chrome chips
    + GLOBAL_KEYBINDS with overflow drop (`…` indicator at
    narrow widths per spec glass-2). When `app.status` is
    non-empty, the transient flash takes priority over chips
    so screens that haven't yet untangled "permanent state"
    from "transient feedback" don't silently lose info.

### What stays the same

- Every keybind unchanged.
- Every render output the same shape (one row header, one row
  footer; body unchanged).
- Saved-query JSON contract unchanged.
- Screens that still write to `app.status` permanently (e.g.,
  "Goalies sort: SV% — GP ≥ 15") keep their footer text intact;
  the chrome chips kick in only once a screen migrates its
  status writes to flash-only. That migration is per-screen and
  ships incrementally in Masterton.2 alongside each Screen
  trait conversion.

### Tests

19 new L0:
- 7 in `tui::chrome::tests` (KeyHint const, Clone, Default,
  PartialEq, Copy, GLOBAL_KEYBINDS head/tail invariants,
  navigation-key presence)
- 12 per-screen chrome contract tests (~2 per screen): default
  state yields a sensible chrome; state changes (mode / search /
  filter) reflect in title or keybinds.

Bin suite: 763 → **782** (+19). All Phase Norris suites
unchanged. L1/L2 integration suites unchanged
(art_ross_w23_tui_filter, persona_wave23, persona_wave25).

### What's next

Masterton.2 — the real break-from-monolith. Each screen
migrates to `impl Screen { fn handle(...); fn render(...);
fn chrome(...) }`. App's giant `handle()` match collapses
one branch at a time. ~3-5 days of incremental commits, one
per screen.

## v0.21.1 — 2026-05-08 — Phase Norris.6 (overlay state extraction)

Headline: **Continuation of Norris. Two cross-screen overlay
clusters extracted into their own state structs in a new
`tui/pickers.rs` module.** No keybind change, no UX delta.

### Norris.6 — what's extracted

**`DatePickerState`** (4 fields, shared between Tonight and Schedule
per Foster.1.4):
- `scores_picker_open` → `date_picker.open`
- `scores_picker_input` → `date_picker.input`
- `scores_picker_err` → `date_picker.err`
- `picker_target` → `date_picker.target`

**`GroupPickerState`** (3 fields, shared between player card and
team roster):
- `group_picker_open` → `group_picker.open`
- `group_picker_list` → `group_picker.list`
- `group_picker_player` → `group_picker.player`

### New module

`tui/pickers.rs` houses both. Distinct from the per-screen state
pattern (Norris.1-4) because these overlays aren't tied to a
single screen — the `d`/`g` key on any supporting screen opens
the same overlay.

### App footprint

- 7 fields collapsed to 2 state structs.
- App field count: 43 → 38.
- Cumulative across Phase Norris: 80+ fields → 38, 50 fields
  moved off App.

### Tests

14 new (705 → 763 cumulative across the phase):
- 11 L0 default-contract tests in
  `tui::pickers::norris_state_tests` (6 DatePicker + 5 GroupPicker).
- 3 L1 sequencing tests in
  `tui::screens::app_snapshot_tests`: open/cancel cycle clears
  input + err; Shift+D target rebinds across Tonight ↔ Schedule;
  group-picker close clears all three fields together.

### What's still on App (intentionally)

After Norris.6, the remaining cross-screen state on App is
heterogeneous and doesn't form natural clusters:
- Time/season axis (5 fields) — every screen reads these.
- UI scaffolding (`status`, `tick`, `selected`, `no_color`,
  `last_auto_refresh`).
- Reports overlay (2 fields) — too small to extract.
- Help/docs overlays (3 fields) — same.
- Bootstrap (`load_state`, `install_state`).
- Caches (`headshot_cache`, `dashboard_panel`, `league_context`).
- Data (`repo`).

Future Norris work (if any) would be marginal yield. The big wins
are landed.

## v0.21.0 — 2026-05-08 — Phase Norris (TUI architecture refactor)

Headline: **Internal refactor. No keybind change, no UX delta. The
3,800-line App god-object is gone — every TUI screen now owns its
state in a per-screen `<Screen>State` struct, accessed as
`app.queries.filter_text` instead of `app.query_filter_text`.** 44
new tests across the phase.

### Why now

Phase Art Ross (v0.20.x) added ~25 fields to App across the
filter-experience push (Wave 23/24/24b/c/d, filter presets, history,
live count, cheatsheet). Adding any new screen meant scrolling past
80+ fields to find the right place. Reading the App definition was
exhausting. Norris catches this BEFORE the next phase compounds it,
while recent additions are still freshest in test coverage.

### Five sub-phases, each shipped as its own commit pair (extraction
+ tests):

- **Norris.1 (pilot, 3 commits)** — `QueriesState` (17 fields):
  every `query_*`, `sort_picker_*`, `career_table_preset` field.
  ~200 mechanical access-site renames across app.rs / screens/
  queries.rs / screens/mod.rs / screens/player.rs. The pilot
  proved the pattern: file-level
  `#![allow(clippy::module_name_repetitions)]`, `#[derive(Debug)]`,
  `pub` fields matching today's `pub struct App` visibility,
  hand-written `Default` impl that mirrors the legacy `App::new`
  init.
- **Norris.2 (2 commits)** — `ScheduleScreenState` (8 fields):
  week + caches + search filter. Suffixed `Screen` to disambiguate
  from the existing `tui::schedule::ScheduleState` per-week
  load-state enum (Idle / Loading / Loaded / Error).
- **Norris.3 (2 commits)** — `TransactionsState` (8 fields): rows
  + filters + cursor. App field is `app.txs` (not `app.transactions`)
  to avoid substring overlap with the legacy `transactions_*` field
  names — `app.transactions_fetched_at` and `app.transactions`
  would both match a bare `app.transactions` regex.
- **Norris.4 (2 commits)** — three small extractions batched:
  `GoaliesState` (3 fields), `PlayoffsScreenState` (3 fields),
  `TonightScreenState` (4 fields). Same `Screen` suffix where the
  simpler name conflicts with an existing load-state enum.
- **Norris.5** — closeout: this changelog entry + CLAUDE.md +
  v0.21.0 tag.

### What stays on App

- Screen discriminator (`screen` / `prev_screen`).
- Cross-screen UI state: `selected`, `status`, `tick`, `no_color`.
- Overlays: `show_help`, `show_admin`, `show_season_picker`,
  `show_reports_overlay`, `group_picker_*`.
- Time/season axis: `active_season`, `active_season_typed`,
  `active_type`, `active_timeframe`.
- Cross-screen pickers: `scores_picker_*` + `picker_target` (shared
  between Tonight and Schedule), `picker_selected`.
- Data: `repo`, `dashboard_panel`, `league_context`,
  `career_loaded_ids`.
- Bootstrap: `load_state`, `install_state`.
- Caches that aren't screen-specific: `headshot_cache`,
  `last_auto_refresh`.
- Config: `reports` (ReportToggles — read by every Tier-1 stat
  visibility check, cross-screen).

App went from 80+ fields to 43; the struct definition is now
~116 lines (was ~250+).

### Test pattern (canonical for the phase)

Each `<Screen>State` ships with two tiers:

1. **L0 contract tests** (~5-10 per state) in
   `tui::screens::<screen>::norris_state_tests`: pin every
   `<Screen>State::default()` invariant — mode, empty active
   state, populated structural state, consistency with helper
   defaults, App::new wiring, Debug derive.
2. **L1 sequencing tests** (~3 per state) in
   `tui::screens::app_snapshot_tests`: chain handler calls
   through realistic multi-step sessions to prove state
   transitions land correctly across actions.

No L2 — the TUI is interactive; subprocess can't drive keystrokes,
so L2 stays scoped to the CLI / web surfaces (already covered).

### Footprint

- 9 commits across the phase (4 extraction + 4 tests + this
  closeout).
- 6 new state structs.
- 43 fields moved off App.
- Bin suite: 692 (v0.20.3) → 749 (v0.21.0); +44 net new tests.
- L1/L2 integration suites unchanged: every Phase Art Ross suite
  (Wave 11/12/14/15/16/17/18/19/20/21/22/22b/23/24/25) passes
  unchanged. Saved-query JSON contract preserved bit-for-bit.

## v0.20.3 — 2026-05-07 — Phase Art Ross polish + cohort filter

Headline: **Filter presets, TUI editor polish, and `query career
--filter`. Saved queries persist the filter text across sessions;
the TUI editor gets history (↑/↓), a live "47 of 712 match" count,
and a `?`-toggleable grammar cheatsheet; the Phase Calder cross-
league cohort cmd accepts the same Phase Art Ross filter grammar
as `query leaders`.** 53 new tests.

### Wave 24 — filter presets

Saved queries now round-trip BOTH the structured field selections
AND the active free-form filter text from the Phase Art Ross
overlay. Loading a preset restores both halves and re-parses the
filter into an active plan.

- v2 envelope: `{"version":2, "fields":[…], "filter_text":"…"}`.
  Switched from hand-rolled string format to `serde_json` so
  quoted strings (`country LIKE "CA*"`) round-trip cleanly.
- Pre-Wave-24 v1 (top-level array) saved queries continue to load
  with `filter_text = ""` — no migration required.
- Parse failure on a saved older-grammar filter is non-fatal: the
  text is preserved, the plan is left empty, status hints at
  re-edit.
- 13 tests (7 L0 schema + 4 L0 handler + 2 L1 DB persistence).

### Wave 24b/c/d — TUI overlay polish

Three quality-of-life additions to the `f` filter editor:

- **History (24b)**: `↑/↓` walks `App::query_filter_history`
  (newest first, capped at 20). Successful Enter pushes the typed
  filter onto the front, deduped against an identical existing
  front. Typing or Backspace breaks navigation so edits don't
  mutate a recalled historical entry. Render shows
  `history N/M` while navigating.
- **Live result count (24c)**: speculative `parse_query` on every
  render. Bio + season-stat filters (no provider needed) report
  `→ 47 of 712 match` against the active-season views in
  microseconds. Sliding-window / career / league filters defer
  to Enter (`→ press Enter to evaluate (data lookup)`) so the
  editor stays responsive. Unparsed input shows
  `(unparsed — keep typing)`.
- **Grammar cheatsheet (24d)**: `?` inside FilterEdit toggles a
  side-panel listing supported atom shapes (bio, stat, sliding-
  window, career, league, EVER+AT, booleans, operators) without
  opening the global help overlay. 60/40 horizontal split when
  on; full-width editor when off. Flag persists across re-entry.
- 19 tests (9 history + 7 live count + 3 cheatsheet).

### Wave 25 — `query career --filter`

The Phase Calder cross-league cohort cmd (`icelines query career
--league OHL --season 20142015`) now accepts the same Phase Art
Ross filter grammar as `query leaders`.

```bash
icelines query career --league OHL --season 20142015 \
  --filter "country=CAN AND pos=C AND age<=18"
icelines query career --league WHL --filter "draft-round<=2"
```

- Bio atoms (`country`, `pos`, `age`, `draft-*`) work as expected.
- The `age` atom anchors on the COHORT year — `age<=18` on a
  2014-15 cohort uses each player's age as of Feb-1-2015 (HR
  convention).
- Stat atoms evaluate against the player's NHL career, not their
  non-NHL league stats — useful for "OHL leaders who later hit
  30 NHL goals" but not for narrowing on the OHL stat line itself
  (use `--sort` for that). Documented in the clap long_about.
- 21 tests (3 L0 + 8 L1 + 10 L2). The L1 suite proves the
  age-at-cohort-year anchor works (McDavid is age 17 on 2014-15,
  age 27 on 2024-25 — the same filter `age<=18` includes/excludes
  him correctly).

### Conn Smythe audit (no code changes)

Verified Phase Conn Smythe shipped completely in v0.19.0
(C.1 series momentum + C.2 cup-run narratives + C.3 live game
tracking). Deferred items (Cup-run probability/xWin% modeling,
historical Cup-run backfill, pre-game previews) are honestly
deferred — need new domains, not polish. No follow-on code
required.

### Test totals

- Full bin suite: 692 passing (+22 since v0.20.2).
- Phase Art Ross integration suite intact (Waves 11/12/14/15/16/
  17/18/19/20/21/22b/23 unchanged).
- v0.20.2 boundary preserved.

## v0.20.2 — 2026-05-07 — Phase Art Ross TUI overlay (Wave 23)

Headline: **The free-form filter grammar reaches the last surface.
Press `f` on the TUI Queries screen to type any Phase Art Ross
filter (`country IN (CAN, USA) AND age<25`, `g.last10g>=5`,
`p.career>=500`, …); CLI / web / TUI now all parse through the
same `parse_query → Constraint::matches` pipeline.** 42 new
tests across L0/L1/L2.

### TUI

- New `QueryMode::FilterEdit` variant + free-form text editor.
  `f` opens; Enter validates via `parse_query`; on success the
  plan is stored on `App` and AND-joined with the structured
  field filter on every subsequent results render. On parse
  error the message renders inline and the editor stays open.
- Esc clears text + plan + error; empty Enter clears the active
  plan (fast "remove filter" gesture); editor preserves typed
  text across re-entry so the user can refine without retyping.
- `f` while typing is intercepted via the same short-circuit
  pattern as the save-name editor, so it doesn't fire
  AddToFavorites.

### Internals

- New `run_query_views_with_pick_and_plan` helper in
  `tui::screens::queries` wraps the legacy field filter and
  applies the parsed plan via `Constraint::matches` on top.
  Provider/clock construction follows the Wave 22 hoist
  pattern (built once per render, not per player).
- Three call sites in `app.rs` and one in
  `screens/queries.rs` switched from `_with_pick` to the new
  `_with_pick_and_plan` variant.

### Test pyramid (Wave 23)

- **L0** — 16 tests: 12 handler tests
  (`tui::app::tests::l0_tui_filter_edit_*`) covering f-key
  entry, typing, backspace, valid/invalid Enter, Esc, empty
  Enter, results-focused fall-through, whitespace Enter, text
  persistence, Quit propagation. 4 helper tests
  (`l0_w23_*`) covering empty-views purity, None-plan
  identity to legacy helper, plan-borrow allows repeated
  calls.
- **L1** — 12 integration tests
  (`art_ross_w23_tui_filter.rs`): real `StatsRepository`
  populated from bundled data; runs `parse_query → legacy
  filter → Constraint::matches` end-to-end and asserts
  membership across 11 filter shapes plus plan ∩ legacy field
  filter intersection.
- **L2** — 14 subprocess tests (`persona_wave23.rs`):
  `tui --help` + `tui stats --help` parse cleanly, COMMANDS.md
  ships the `f` keybind doc (regression guard), 11
  representative Phase Art Ross shapes accepted via
  `query leaders --filter` (indirect parser-parity proof).

### Docs

- COMMANDS.md TUI keybind table now lists `f` for the
  free-form filter overlay.

## v0.20.1 — 2026-05-07 — Phase Art Ross dispatch hardening

Headline: **Seven persona waves (16-22) hardening the new-pipeline
dispatch surfaces. 15 real bugs found and fixed; ~245 new tests.
Cross-surface parity proven empirically.** No grammar changes,
no schema changes — every fix is at the dispatch / wiring seam
between the legacy and new pipelines.

### Bugs surfaced + fixed

- **Wave 16** — CLI `query leaders` only routed `needs_provider`
  plans through the new pipeline, so plain new-grammar atoms
  (`<`, `IN`, `BETWEEN`, `LIKE`) silently fell back to the
  legacy parser, which rejected them with the obsolete "no op"
  error. (2 bugs.)
- **Wave 17** — web `/leaders` HTML route had the same dispatch
  bug as Wave 16. Extracted shared `partition_new_pipeline_filters`
  helper. (1 bug.)
- **Wave 18** — `query goalies` had its own dispatch on top of the
  goalie-filter rewrite (`gp` → `goalie-games`). (1 bug.)
- **Wave 19** — JSON twin `/api/v1/leaders` shared
  `build_leader_result` had its own legacy-only dispatch. (1 bug.)
- **Wave 20** — `query player --peers` and `query compare --similar`
  cohorts. Extracted `partition_filter_dispatch` and
  `build_cli_eval_ctx` helpers across all four CLI sites. (2 bugs.)
- **Wave 21** — cross-surface result parity proof: the same filter
  against the same repo, run via library / web JSON / web HTML /
  CLI, must match. (25/25 parity tests green.)
- **Wave 22** — perf: provider/clock construction was inside the
  per-player `.filter()` closure on both web routes. Hoisted to
  one allocation per request.
- **Wave 22b** — output-format correctness: 17 tests asserting the
  `LeadersEnvelope` JSON shape stays identical across legacy and
  new-pipeline filters. Closes the residual schema-drift risk.

### Test budget

- Wave 11 retro fixes: 4 obsolete tests updated for v0.20 acceptance
  (`<`, `>`, `!=`, `country=CAN OR country=USA` are now legal).
- Wave 16 (CLI `query leaders`): 100 binary-subprocess tests.
- Wave 17 (web HTML): 25 tests.
- Wave 18 (CLI `query goalies` + sibling shortcuts): 40 tests.
- Wave 19 (web JSON API): 25 tests.
- Wave 20 (`query player`, `query compare` cohorts): 20 tests.
- Wave 21 (cross-surface parity): 25 tests.
- Wave 22b (envelope shape): 17 tests.
- **Total: ~252 new tests.** Combined Phase Art Ross test budget
  now ~676.

### Internals

- `icelines-query::Constraint::needs_provider()` no longer gates
  pipeline routing — every successfully-parsed plan goes through
  the new pipeline. The legacy fallback only fires when
  `parse_query` itself returns `Err`.
- New helpers: `partition_filter_dispatch` (CLI),
  `partition_new_pipeline_filters` (web). Single source of truth
  for "is this filter new-grammar?".
- `prefer_new_error` heuristic surfaces helpful new-pipeline error
  messages (`IncompatiblePredicate`, `EmptySet`, `FeatureNotYet`)
  instead of falling through to the legacy parser's generic
  "has no op" diagnostic.
- `bio_matches` (executor) falls back to `SeasonStats.goalie_bios`
  when `PlayerIdentity.bio` is empty — fixes Country / Draft /
  Age atoms on goalie views (Wave 14b).

## v0.20.0 — 2026-05-07 — Phase Art Ross

Headline: **The query system is the centerpiece of IceLines now.
Unified parser → planner → executor lands. Sliding-window streak
atoms, historical EVER queries across all 38 bundled seasons,
cross-league career atoms, on-demand data fetch, `--explain`
plan tree.** ~3 100 → 3 500 tests; +424 in the phase.

### Vision queries that now parse + evaluate

```bash
# "5 goals over 10 games, age <= 25" — current-season streak
icelines query leaders --filter "g.last10g>=5 AND age<=25"

# Same question over the player's entire career, across all 38
# bundled seasons (intra-season only, axis-typed, lockout skip)
icelines query leaders --filter "g.any10g>=5 EVER AT age<=25"

# "Junior elite cohorts" — cross-league career filter
icelines query leaders --filter "league.tier=Junior AND p.career.junior>=200"

# Inspect any plan tree without running the query
icelines query leaders --filter "g.last10g>=5 AND age<=25" --explain
```

### Added — sub-phases A.0 through A.5

- **A.0 IR + planner skeleton.** New `icelines-query::plan` module
  with n-ary `Constraint::All(Vec)/Any(Vec)/Not(Box)` IR + typed
  `Predicate { Scalar, Member, Pattern, Range }` (shape-by-
  construction makes invalid combinations like `LIKE 5` fail at
  parse, not at evaluate). `parse_query(FilterInput) -> Result<
  QueryPlan, Vec<ParseError>>` is the single front door for CLI /
  web / TUI. `DataProvider` trait owned by `icelines-query` (the
  dependency-inversion seam — preserves the crate-layering rule).
  `EvalCtx` is `!Send`-pinned via `compile_fail` doctest.
- **A.1 grammar expansion.** Strict `<` / `>` / `!=` operators
  (with `<>` typo hint suggesting `!=`); `IN (a,b,c)` / `NOT IN`
  set membership (empty `IN ()` rejected at parse); `BETWEEN x
  AND y` numeric range; `LIKE "Mc*"` with NFD-normalization so
  ASCII patterns reach Slafkovský / Stützle / Kämpf / Björk;
  `~` / `!~` substring sugar. Plus 7 new bio atoms: `pos=C`,
  `team=EDM`, `team.any=EDM`, `draft-round<=2`,
  `draft-overall<=10`, `birth-state=ON`, `nationality=USA`,
  `rookie-season>=20212022`.
- **A.2 sliding-window atoms.** New atom shape `<stat>.last<N><u>`
  where `u` is `g` (games), `d` (days), `w` (weeks), or `m`
  (months). Optional scope modifiers: `.allteams` (any stint
  this season) / `.career` (cross-season tail). `WindowPolicy`
  enum: `RequireFull` (default) / `AllowPartial` /
  `AllowPartialAbove(N)`. Mid-season-trade aware — current-team
  filter applies BEFORE the trailing-N cut.
- **A.2.4 IcelinesProvider + CLI wiring.** `IcelinesProvider`
  walks the boxscore manifest + builds `GameStatLine` records
  for sliding-window evaluation. CLI's filter dispatch routes
  needs-provider filters through the new pipeline; legacy
  pipeline preserved for everything else.
- **A.2.5 polish (12 review action items).** Killed silent
  placeholders: legacy `Constraint::matches` deleted; `team.career=`
  rejected at parse with `FeatureNotYet { ships_in: "A.4" }`;
  `name LIKE` error message no longer claims a field that doesn't
  exist; `current_team=None` returns Empty instead of falling
  back to all-stints. `EvalCtx::new` no longer calls `Utc::now()` —
  takes explicit `today` + `season`; `from_clock(&dyn Clock)`
  integrates Foster F.0's `MockClock` for tests. Diagnostics:
  `g.last10z>=5` → `UnknownWindowUnit { unit: 'z' }` with g/d/w/m
  suggestions; `g.last0g>=5` → `ZeroWindowSize`; `g.last1000g>=5`
  → `WindowSizeOutOfRange { size: 1000, max: 255 }` (no silent
  truncation). `parse_or` / `parse_and` use clean accumulator
  pattern (no sentinel-replacement smell).
- **A.2.6 coverage close.** 11 end-to-end executor tests with
  synthetic StatsRepository + canned MockProvider exercising
  `sliding_window_matches` against a real `PlayerView`. 6
  `--explain` golden snapshot tests pinning exact tree
  rendering. 5 missing `IncompatiblePredicate` parser tests.
- **Wave 12 — 200 adversarial filter scenarios** on the new
  grammar. Surfaced 1 real bug (`rookie-season>=N` was only in
  the text-field map; numeric path returned `UnknownStat`).
- **A.3 historical EVER + AT-age slicing.** New atoms:
  `p.career>=500` (LifetimeSum), `p.streak>=15` (LongestStreak),
  `g.any10g>=5 EVER` (AnyWindow — short-circuits on first
  satisfying season), `g.seasons-with>=5` (SeasonsWith). `AT
  age<=22` modifier on any career atom — supports scalar
  (`age<=22`, `age<25`) and range (`age BETWEEN 20 AND 25`)
  predicate shapes. Lockout 2004-05 skipped (no data, no
  partial-mark) per the spec. HR Feb-1 age convention via
  existing `compute_age`.
- **A.4 cross-league career atoms.** `league=OHL` /
  `league NOT IN (NHL)` / `league.tier=Junior` (uses Phase
  Calder's canonical `LeagueTier` classification — Pro /
  Junior / College / International / Other). Stat-aggregate
  3-dot keys: `p.career.junior>=200`, `p.career.nhl>=500`,
  `p.career.ohl>=300`. `IcelinesProvider::fetch_career_history`
  reads `~/.icelines/career_history.json` (Phase Calder cache).
- **A.5 `--explain` flag.** `icelines query leaders --filter X
  --explain` prints the parsed `QueryPlan` tree + data
  requirements without running the query. Pair with `--json`
  for the `explain.v1` envelope (frozen v1 — additive changes
  only; breaking changes ship as `explain.v2`). Useful for
  debugging complex filters and confirming how the planner
  routes atoms across legacy / sliding-window / career-aggregate
  / cross-league sub-evaluators.

### Architecture

- **One front door, four sub-evaluators.** `parse_query` consumes
  any `FilterInput` ({Cli(String), Form(String), Tui(Vec<C>)})
  and produces a typed `Constraint` tree. `Constraint::matches(
  view, &EvalCtx)` walks the tree once; n-ary `All`/`Any` short-
  circuit naturally. Routing: `Bio` and `SeasonStat` need only
  the active-season repo; `SlidingWindow` calls
  `provider.fetch_game_lines`; `CareerAggregate` walks per-
  season game streams; `CareerLeague` reads career history.
- **Layering preserved.** `icelines-query` does NOT import
  `icelines-fetch`. The `DataProvider` trait owned by query is
  implemented by `icelines-fetch::query_provider::IcelinesProvider`
  and injected by the surface (CLI / web / TUI).
- **Fail-closed defaults.** Missing data → atom returns false.
  Wrapping in `NOT` flips the legacy missing-data semantic
  through correctly. `--strict` mode (when wired in v0.21) gates
  partial-data results.
- **Backward compat.** Every filter expression that parsed in
  v0.19.1 continues to parse and produce identical results,
  including the FIXED behavior of the 3 Wave 11 production bugs
  (goalie compound rewrite, paren-wrapped bio atoms,
  `--filter`+`--week` loud rejection).

### Test budget

- v0.19.1 baseline: 3 081 workspace tests
- v0.20.0: 3 505 (+424 across the phase)
  - 45 A.0 + 64 A.1 + 34 A.2 + 8 A.2.4 + 4 A.2.5 + 22 A.2.6
  - 30 A.3 + 23 A.4 + 12 A.5 explain
  - +200 Wave 12 adversarial scenarios
- All Phase Art Ross gates green: Wave 11 (201) + A.0 parity (4)
  + A.2 executor (11) + A.3 career (10) + A.4 league (11) +
  A.5 explain (12) + Wave 12 (200) + the 5-role checkpoint
  review review-action items (12) all closed.

### Real bugs surfaced + fixed during the phase

- 5 silent-placeholder fixes from the 5-role A.2 checkpoint review
- 1 from Wave 12: `rookie-season>=N` numeric routing
- 1 from A.2.6: planner's SlidingWindow render was stale

### Deferred to v0.20.1+

- Cross-surface parity tests (CLI / web / TUI all parse identically)
- `--strict` flag wired through to error before any fetch
- Per-season sharded `BoxscoreIndex` with LRU cap (today's
  `IcelinesProvider` walks the full manifest)
- Criterion benchmark for `EVER` cold/warm budgets (≤8s / ≤2s)
- Surface swap (replace `parse_filter_expr` with `parse_query`
  on every `--filter` site, not just sliding-window)
- `query career` integration with cross-league atoms
- `SeasonAxis::Playoff` partition in the executor

## v0.19.1 — 2026-05-06

Headline: **3 production filter bugs surfaced by Wave 11 (200
adversarial scenarios) + Wave 10 UX polish. ~1 855 → 2 056 tests.**

### Fixed
- **Goalie compound filter rewrite ate boundary characters.**
  `icelines query goalies --filter "gp>=10 AND sv%>=0.9"` was
  silently corrupting the input to `goalie-games>=10 ANDAsv%>=0.9`
  (later just `ANDsv%>=0.9` after a partial fix). Root cause:
  `goalie_filter_rewrite_expr` had a bare `continue` inside an
  inner `for kw in ["AND","OR","NOT"]` loop where `continue 'outer`
  was needed; after matching a keyword and advancing `i`, the
  outer while-loop kept executing with the stale `c` captured at
  the top, then re-pushed it into the next atom. Compounded with
  `flush_atom` only preserving trailing whitespace (not leading)
  around the rewritten core. Both fixed; any compound goalie
  filter now parses correctly.
  ([icelines-cli/src/commands/query.rs](icelines-cli/src/commands/query.rs))
- **Bio atoms broken when wrapped in outer parens.**
  `--filter "(age<=24 AND p>=10)"` failed with
  `unknown stat key "age"` because `extract_bio` didn't recurse
  into a single-paren-wrapped expression — the catalog parser
  then saw `age` (which isn't a catalog stat). Added
  `peel_outer_parens` helper + recursive `extract_bio_into`.
  ([icelines-query/src/lib.rs](icelines-query/src/lib.rs))
- **`query leaders --week`/`--month` silently dropped `--filter`.**
  The dispatcher routed to `run_windowed_leaders(top, sort, json)`
  ignoring `filters`. Added a loud rejection at the dispatch
  boundary pointing the user at `icelines favorites --range week`
  for the populated path; full filter wiring will land in Phase
  Art Ross. ([icelines-cli/src/main.rs](icelines-cli/src/main.rs))

### Polished (from Wave 10)
- `icelines favorites --date 2014-10-08` empty-state now echoes the
  date back in the header (was: only populated state showed it).
- `icelines data-status` documented in `COMMANDS.md` (shipped in
  Foster +2, undocumented).
- Three global-flag long_about strings (`--no-live`, `--no-dashboards`,
  `--no-setup`) were 200-390 chars each on a single line in non-TTY
  output; restructured into shorter paragraphs so `--help` lines stay
  under 130 cols when piped.
- Bare `icelines` (no args) intentionally prints a friendly landing
  and exits 0 (deliberate UX, documented in Wave 10 #025).
- One unused-import warning cleaned up in `favorites_view.rs`.

### Tests
- **Wave 11 — 201 filter-grammar adversarial scenarios** across 10
  sections: boolean precedence + associativity, atom-op stress,
  bio + stat interplay, windowed atom precedence, paren / whitespace
  edges, conflicting / tautological predicates, goalies subcommand
  rewrites, alias coverage, pathological inputs (deep nesting, long
  chains, Unicode, scientific notation), output truthfulness
  (commutativity, De Morgan's laws, inclusion-exclusion).
  ([icelines-cli/tests/persona_wave11.rs](icelines-cli/tests/persona_wave11.rs))
- **Wave 10 — 100 UX consistency + truthfulness scenarios** across
  8 sections: K2.4 envelope shape, exit-code consistency, error
  message format, date / team format consistency, output stream
  discipline, COMMANDS.md ↔ binary parity, CLAUDE.md ↔ binary parity,
  `--help` quality (no dev jargon, line-width caps, examples
  present). ([icelines-cli/tests/persona_wave10.rs](icelines-cli/tests/persona_wave10.rs))
- New L0 unit tests for `peel_outer_parens` + recursive `extract_bio`
  paths in `icelines-query`.

### Note
The Wave 11 fixes set the stage for **Phase Art Ross** (next): a
unified query architecture with sliding-window streak atoms, career
aggregates across all 38 bundled seasons, cross-league career-history
atoms, on-demand data fetch driven by the query plan, and a
`--explain` view of the plan tree.

## v0.13.0 — 2026-05-03

Headline: **38 seasons bundled in (1987-88 → 2025-26), Reports overlay,
boolean filter grammar, and ~1720 tests across 4 persona-scenario waves.**
Binary grew 23 MB → 57 MB to fit the full historical era.

### Added
- **L.7b 38-season bundle.** `BUNDLED_SEASONS` now covers every NHL
  season from 1987-88 forward except the 2004-05 lockout. Refactored
  `bundled.rs` to a table-driven layout (228 lookup-table entries,
  one macro per season). Mario Lemieux's 1992-93 (218.7 Pts/82) and
  Wayne Gretzky's 1987-88 (190.9 Pts/82) are queryable from a fresh
  binary with no `data install` needed.
- **Phase Reports — `R` overlay in TUI.** Toggleable Tier-1 reports
  (realtime / timeonice / goalsForAgainst / goalie-advanced /
  goalie-savesByStrength). Disabled reports drop their columns from
  career tables, sort pickers, and query results. Persists to
  `~/.icelines/config.toml`. New `Config::reports` field +
  `ReportToggles::is_stat_visible(stat)` gate. Removes the noisy
  "Missing data: realtime" banner.
- **UX.1 — Lazy career loader on player card open.** `app.repo` LRU
  cap bumped 8 → 80. Opening a player card fans out across all 38
  bundled seasons, pulling that player's career into the repo. ~50 ms
  per first open, cached after. McDavid surfaces 11+ regular seasons,
  Gretzky 12, Crosby 18+, Ovechkin 18+.
- **UX.2 — `[/]` discoverability hint** in Queries title bar.
- **UX.3 — Tab unconditionally cycles screens.** Pre-UX.3, Tab
  toggled section expand/collapse on Queries (trapped users on the
  Stats tab). Section toggle moved to `o`. New tests pin the rebind.
- **Gaps.1 — Short filter aliases.** `--filter "g>=50"` works as
  `goals>=50`. Aliases: `g`/`a`/`p`/`pts`/`s`/`gp`/`ppg`/`gpg`/`apg`/
  `+/-`/`pim`/`pen`/`blk`/`tk`/`gv`/`mis`/`fow%`/`pace`/`sv%`/`sv`/
  `ga`/`sa`/`w`/`l`/`so`/`ot`. Filter keys are case-insensitive.
- **Gaps.2 — `query player --seasons N`.** Full bundled-history
  career arc on the CLI. Default 38 = full history.
- **Gaps.3 — `query compare --seasons N`.** Multi-season head-to-head
  with each player's career arc printed alongside.
- **Gaps.4 — Goalie filter rewrite.** `query goalies --filter "gp>=15"`
  rewrites `gp`→`goalie-games` before parsing; `starts`→`goalie-starts`.
  Error messages hint goalie-specific keys.
- **Gaps.5 — `query player` accepts goalies.** `query player Patrick Roy`
  resolves now (chains skater + goalie bios).
- **Gaps.6 — Cross-bundled name lookup.** `query player Wayne Gretzky`
  resolves without `--season` via `resolve_player_id_by_name` walking
  bundled bios + lazy career fan-out.
- **Filter.OR — Boolean filter grammar.** `--filter` now accepts AND /
  OR / NOT / parens. Recursive descent parser, precedence NOT > AND > OR.
  Bare atoms still route through `stat_filters` for normalization;
  compound expressions go to new `expr_filters`. Multiple `--filter`
  flags ANDed at top level. 19 new L0 tests.
- **`icelines docs` subcommand.** Embeds `COMMANDS.md` via
  `include_str!()` so the full command reference ships inside the
  binary. No internet needed to learn the CLI.
- **`COMMANDS.md`** — single-page command reference with every
  subcommand, examples, the alias table, the filter grammar BNF, and
  the TUI keybind matrix.
- **Rich `--help` long_about** for top-level + `query leaders`,
  `query player`, `query compare`, `query goalies`. Examples,
  alias hints, and filter grammar inline in `--help` output.
- **400 persona-scenario tests** across 4 waves
  (`persona_scenarios.rs`, `persona_wave2.rs`, `persona_wave3.rs`,
  `persona_wave4.rs`). Cover: historical seasons, multi-filter
  patterns, lazy career loading, Reports overlay, goalie filter
  rewrite, JSON/CSV output, bundle integrity, robustness, edge cases.

### Changed
- **Workspace tests: ~1720** (up from ~1275). 400 new persona scenarios
  + 19 filter-expr tests + Reports / UX / Gaps coverage.
- **Binary size: 57 MB** (up from 23 MB). 33 historical seasons + 5
  current = 38 total at ~1 MB / season bundled JSON.
- **Player loading API**: legacy `PlayerRepository::new(store, season).load_all()`
  references in CLAUDE.md replaced with the actual
  `icelines_fetch::stats_loader::load_into_repo(season, season_type, store)`
  surface.
- **`load_into_repo` LRU cap**: 8 → 80 windows so historical fan-outs
  don't evict the active season.
- **`from_cli_key` is case-insensitive** and accepts the alias map.

### Fixed
- "Missing data: realtime" banner removed — was noise, not signal.
  Phase Reports overlay handles per-report visibility properly.
- NHL API breaking change: `pim` removed from `/skater/realtime`.
  Schema field made `Option<u32>` with `#[serde(default)]`.
- 19951996 / 19961997 unbundled-season tests swapped to 20042005
  (lockout, never bundled) to remain truly unbundled after L.7b.

### Docs
- **CLAUDE.md** — refreshed AI-instruction surface. Removed misleading
  references to deleted `PlayerRepository`, "5 seasons bundled",
  "338 tests", and the cancelled proof / DASHBOARD-SPEC integration.
  Added sections on the Reports overlay, lazy career loader,
  short-alias rule, goalie filter rewrite, Filter.OR grammar.
- **README.md** — bundled count 5 → 38, test count 338 → 1720.
  New sections: catalog filter grammar with alias table, multi-season
  player/compare examples, TUI keybind reference (R, y, Shift+P, o,
  `[`, `]`, `/`).
- **COMMANDS.md (new)** — single-page reference designed for AIs and
  new users. Embedded into the binary via `icelines docs`.

## Unreleased

### Changed
- Phase 8j (rev): Native sparklines, proof_lib back to dev-only.
  `proof:chart` directives don't compose inside `proof:region` bodies
  (filed at design/proof-bug-report.md), so the dashboard compositor
  was wrapping plain text we already lay out cheaply with ratatui.
  - New `tui::sparkline` module (~80 lines, zero new deps) renders
    Unicode block sparklines `▁▂▃▄▅▆▇█` from a `&[f64]`.
  - `tui::dashboard_panel` rewritten to build lines natively. Identity
    + counting stats + bundled history trend, in 14 lines of panel.
  - Players with 5 bundled seasons get two sparklines + a latest-season
    anchor (e.g., `25-26 → G 48 Pts 138`). Players with one season show
    that season's row. Players with no bundled history get the pace
    fallback. `if a player has less than 5 we can just show the seasons
    they have` — done.
  - proof_lib + tempfile demoted from runtime back to dev-deps. The
    smoke test (`tests/proof_lib_smoke.rs`) keeps the integration
    paved if we re-introduce proof for site dashboard generation.
  - `--dashboards` flag remains as the opt-in toggle for the panel.
  - 9 new L0 tests in `tui::sparkline` (empty input, single value,
    constant series, increasing walk, bucket-when-overflow, negatives,
    real McDavid trend shape, width-clamps-to-input). Dashboard panel
    tests rewritten for the native renderer; total 622 → ~625 tests.
- Phase 8j (cont.): Real player stats in the dashboard panel + CI-ready
  proof pinning.
  - `tui::dashboard_panel` now compiles a per-player proof source
    (name, team, position, G/A/Pts/+/-/PP-Pts/Shots, GP/PPG/Pts-82
    rate stats) and caches by `nhl_id`. Each player's compile happens
    once and the rendered lines are reused on every subsequent frame.
    Long names truncate with an ellipsis; missing rate stats render
    as em-dashes so the layout never collapses.
  - Player screen pulls per-player lines via `lines_for_player(p)`
    instead of the static placeholder.
  - Output stripper now unwraps proof's `<!-- proof:compiled -->`
    markers and ` ```dashboard ` code-fence wrapper so the panel
    shows just the rendered region content.
  - **CI fix**: switched `proof_lib` from `path = "../../proof"` to
    `git = "...", rev = "9c5d456e"`. icelines release builds no
    longer need proof + mdpath checked out as siblings; cargo fetches
    them transitively from GitHub. Local fast-iteration preserved
    via a gitignored `.cargo/config.toml` with `[patch]` overrides
    pointing at the sibling repos. Template at
    `.cargo/config.toml.example`. Updated `design/proof_lib.md` to
    document the pattern.
  - Companion proof commit `9c5d456e` pins mdpath the same way.
  - 8 new L0 tests in dashboard_panel (build-source content, real
    stats render, cache-by-nhl_id, em-dash for missing fields,
    name-truncation helper, plus three strip-unwrap tests covering
    the proof:compiled scaffolding). 619 tests workspace-wide.
- Phase 8j: Proof-compiled dashboard panel — opt-in TUI feature flag.
  - `proof_lib` is now a runtime dependency of icelines-cli. The CLI
    binary always carries the proof code so toggling the flag at
    runtime needs no rebuild. Pinned by local path while pre-1.0.
  - New `--dashboards` global CLI flag, `ICELINES_DASHBOARDS=1` env
    var, and `dashboards = true` config key — same precedence pattern
    as the existing `--no-live` flag (CLI > env > config > default).
    Off by default while the integration matures.
  - `tui::dashboard_panel` module compiles a baked-in
    `*.dashboard.source.md` template via `proof_lib::compile_file`
    (disk roundtrip via `tempfile::tempdir()`, cached on first frame
    via `Arc<Mutex<Option<Vec<String>>>>`). Compile failures fall back
    to a single `[dashboard error]` line — never panics out of render.
  - Player detail screen (`tui::screens::player`) splits to three
    panes when the flag is on AND screen width ≥ 100 cols: headshot
    | stats | dashboard. Below the threshold the layout is unchanged.
  - 4 L0 precedence tests (matches the live-feeds shape), 5 L0
    panel-compile tests (compile + cache + front-matter strip + error
    fallback), 2 L0 player-screen render-guard tests, and 2 L2
    subprocess tests covering `--help` documentation and global flag
    acceptance. 611 tests workspace-wide, all green.
- Phase 8f.9: User schemes load from `~/.icelines/schemes/*.toml`.
  Closes the long-standing Phase-2 TODO. `scheme list` now shows user
  schemes alongside builtins (labelled `user`); `scheme show NAME`
  resolves user schemes first so a `~/.icelines/schemes/yahoo-standard.toml`
  cleanly overrides the builtin. Malformed user files are skipped with
  a warning rather than breaking the listing — `scheme show` still
  errors loud on a malformed exact-name match. SkaterWeights and
  GoalieWeights gain `#[serde(default)]` so partial schemes (only set
  the stats you score) parse without listing every field. 5 L0 tests
  cover the round-trip, override priority, builtin fallback, malformed
  skip, and empty-dir paths (using a process-global Mutex to serialize
  HOME-env mutations).
- Phase 8f.8: `icelines data verify [SEASON|--all]` checks SHA-256
  hashes of installed bundle files against a manifest written at
  install time. Catches partial downloads and post-install tampering.
  `data install` now writes `manifest.json` next to bios.json /
  stats.json (and playoffs.json when present) covering each file's
  SHA-256, season ID, and a versioned schema. Verify reports `✓` per
  clean bundle, `✗` with named mismatches when a file changes, and
  `?` for legacy bundles installed before this manifest existed.
  `--all` walks every installed season. New `to_hex()` helper avoids
  pulling in the `hex` crate. 6 L0 tests (file_sha256, manifest
  roundtrip, tamper detection, missing-file detection, no-manifest
  fallback) + 3 L2 subprocess tests (no-install hint, tampered
  bundle exit, clean bundle success).
- Phase 8f.7: `icelines scheme from-csv` now supports ESPN, Sleeper,
  and Fantrax CSVs in addition to Yahoo. Each platform has a dialect
  with `signatures` (signature columns for auto-detection) and
  `stat_cols` (column → normalized stat-key map). Auto-detection picks
  the dialect with the most signature hits; ties break in declaration
  order (Yahoo first, preserving Phase-5 behavior on ambiguous CSVs).
  New `--platform yahoo|espn|sleeper|fantrax` flag overrides
  auto-detection. Unrecognized headers error with a `--platform` hint.
  Output now includes the detected platform plus column-to-key
  mappings (`G (P) → goals`). New `scheme_dialects` module with 11 L0
  tests + 5 L2 subprocess tests covering autodetect, override,
  unknown-platform, and unrecognized-format paths.
- Phase 8f.6: `icelines group export/import/rename` for portable groups.
  - `group export NAME [--out PATH]` writes one group's members + metadata
    to JSON (default stdout, `--out file.json` for a file). Wire format
    is stable + versioned for future migrations.
  - `group import PATH [--as NEWNAME]` reads back a previously-exported
    JSON file and recreates the group with all members; `--as` lets
    users clone a group under a new name without editing the file.
  - `group rename OLD NEW` updates the group name, carrying members
    via a deferred-FK transaction (sqlite's `defer_foreign_keys = ON`).
    Same-name is a noop; collision errors with a clear message.
  - GroupDb gains `rename_group`, `add_members_bulk`, and
    `group_description` helpers backing the new commands. 5 L1 db tests
    + 3 L2 subprocess tests (export → import roundtrip, rename
    moves members, export-to-stdout).
- Phase 8f.5: `icelines scheme show NAME --source` prints the scheme as
  pretty JSON instead of the human-readable table. Useful for copy/paste,
  diffing two schemes, or piping into `jq`. The default (no flag) still
  emits the readable layout. Scheme already derived Serialize so the
  change is minimal. 1 L2 test verifies valid JSON with name + skater +
  goalie fields.
- Phase 8f.4: `--season YYYYZZZZ` flag on `query leaders/player/compare`.
  Pins the query to a specific bundled season instead of the current one
  — `icelines query leaders --season 20242025 --top 10` shows last
  season's leaders without changing config. Validates against
  `icelines_fetch::BUNDLED_SEASONS` (currently 2021-22 → 2025-26) and
  rejects unknown seasons with a copyable hint listing the bundled IDs.
  Mutually exclusive with `--seasons N` (the multi-season aggregate);
  combining the two errors with a clear explanation. New
  `load_all_players_for_season(Option<&str>)` helper backs all three
  query commands. 3 L0 validator tests + 5 L2 subprocess tests
  (success, unbundled-error, conflict-error, player + compare paths).
- Phase 8c: Historical playoffs bundle. New `playoffs_bundle` module
  defines the `PlayoffsBundle` JSON schema (rounds → series → per-game
  results with optional goal scorers) and a `to_bracket()` conversion
  that drops cleanly into the existing TUI render path. `bundled::
  load_playoffs(season)` resolves installed bundle first then the
  binary-embedded copy. Hand-authored `data/seasons/19931994/
  playoffs.json` ships as the first fixture — full 4-round NYR Cup run
  with per-game results for the Cup Final. `tui::playoffs` now consults
  bundled data before any network call; historical seasons never hit
  the live API. `render_series_body` renders the per-game log when
  present (Game N · date · home N–N away · series-after) and falls
  back to the existing "X game(s) played" hint for live-API series.
  Closes the `Per-game scores + scorers ship with playoffs.json (v2)`
  TODO. 11 L0 tests in `playoffs_bundle` + `bundled`, 5 L0 tests in
  TUI cache + render paths, 2 L1 integration tests covering the full
  load → convert → render chain.
- Phase 8f.2 + 8f.3: snapshot prune + diff
  - `icelines snapshot prune --keep N [--dry-run]` keeps the newest N
    sealed snapshots per tier and deletes the rest. Active snapshot is
    always preserved; drafts are excluded from the keep count. Pair with
    `snapshot gc` to reclaim chunk space. 5 L0 + 2 L2 tests.
  - `icelines snapshot diff <A> <B>` compares two chunked snapshots and
    reports player-level changes (added / removed / changed bios /
    changed stats). O(n) hash-set diff via the chunked layout — exact
    and fast. Legacy snapshots error with a hint to run `rebuild
    --chunked` first. 4 L0 + 1 L2 tests.
- Phase 8f.1: live-feeds toggle — `--no-live` global CLI flag,
  `ICELINES_NO_LIVE` env var, and `live = false` config key all suppress
  NHL API fetches in Scores / Schedule / Playoffs / boxscore + the auto-
  refresh timer. Precedence: CLI > env > config > default(on). When
  disabled, each live tab renders an explicit "Live feeds disabled —
  re-enable with …" message via the standard error path. 4 L0 precedence
  tests + 2 L2 (flag accepted globally, `--help` documents it).
- Phase 8d: `icelines export md <shape>` — writes deterministic markdown
  tables with YAML front-matter for proof DASHBOARD-SPEC consumption.
  Five shapes shipped: `leaders`, `team`, `depth`, `compare`, `roster`.
  `fantasy` and `series` are stubbed with deferred messages (need
  FantasyDb glue and historical playoffs.json respectively). Output
  goes to `~/.icelines/reports/{shape}.md` by default; pass `--out -`
  for stdout. 13 L0 tests + 3 L2 subprocess tests. `export-markdown.md`
  spec status flipped from `Planned` → `Implemented (partial)`.
- Phase 8h: Chunked snapshot store — content-addressed per-player chunks
  with SHA-256 deduplication. New `icelines-fetch::chunkstore` module
  (put/get/exists/delete with sharded layout); `SnapshotStore` extended
  with `write_chunked_stats`, `read_chunked_stats`, `is_chunked`, refs
  table (`chunkrefs.json`), `gc_chunks`, `recompute_refs`,
  `rebuild_chunked` (legacy → chunked migration). Two new CLI ops:
  `icelines snapshot rebuild --chunked <name>` and
  `icelines snapshot gc [--dry-run]`. `bundled::load_*_with_fallback`
  prefers chunked active snapshot then falls back to legacy → bundled.
  25 new tests (12 ChunkStore + 11 chunked snapshot + 2 L2). Storage
  reduction: ~10–15× for daily-cadence snapshots over a season.
- Phase 8b: Scores auto-refresh — live Scores tab polls every 30s. New
  `should_auto_refresh` pure function + `App::tick_auto_refresh` driven
  from the TUI event loop. "Updated Xs ago" indicator in the Scores
  title. Timer arms on tab entry / `t` jump to today, disarms on date
  change, never fires on past dates. 8 L0 + 2 render tests.
- Phase 8a: Test catch-up for previously-shipped features. 27 new tests:
  - **Scouting** (`commands/scouting.rs`) — extracted `validate_format` +
    pure `render_report() -> String`; 7 L0 tests cover all 3 formats,
    section presence, low-GP path; 3 L2 subprocess tests verify exit
    codes and JSON parseability.
  - **Admin overlay** (`tui/app.rs`, `screens/misc.rs`) — 5 L0 keystroke
    tests (capital `F` toggle, Esc closes, Tab blocked, lowercase f
    untouched) + 4 L0 render tests (Idle / Downloading / Done / Error
    phases). Added `InstallState::force_phase` for deterministic test
    drives.
  - **Headshot rendering** (`tui/headshot.rs`) — extracted
    `pixels_to_braille()` + `DOT_X` / `DOT_Y` / `THRESHOLD` constants;
    8 L0 tests cover braille bit layout, threshold contract, cache
    round-trip, loading/error placeholders, Arc-shared clone semantics.
- 10 new specs in `design/specs/` covering previously-homeless features:
  group-management, fantasy-leagues, data-bundles, site-generation,
  scouting-reports, scheme-customization, snapshot-operations,
  tui-admin-overlay, export-markdown (planned), headshot-rendering
  (reference). Specs INDEX updated.
- Phase 7c gap-fix: Scores tab — date navigation (`←/→`), `d` date picker
  (ISO or `MM/DD`), `t` back-to-today, per-game boxscore detail
  (goals/assists, goalies, series context for playoffs), per-date and
  per-boxscore caches
- Phase 7e: Playoffs tab — list-style bracket from `/v1/playoff-bracket/{year}`,
  per-round navigation, per-series detail with summary and "if needed" game
  hints, off-season / error states, `r` retry
- Phase 7d: Schedule tab — weekly view with date navigation, team and matchup
  search (`/SEA`, `/NYR WSH`), team-season detail and head-to-head matchup
  views, per-week + per-team caches with `r` retry, `t` jump to today
- TUI guide (`docs/guides/06-tui.md`) — covers Phase 7a–7e: six-tab nav,
  season time-travel, Scores, Schedule with search and matchups, Playoffs
  bracket and series detail
- `icelines query compare --comps` — contract comparable finder (in progress)
- Season data expansion to 2000-01 (in progress)

---

## v1.0.0 — 2026-04-26

IceLines v1: migrated from C:\src\NHL\fantasy-tracker to C:\src\icelines.
Clean repo structure matching proof/mdpath conventions.

### Architecture
- 4-crate Rust workspace: icelines-core, icelines-fetch, icelines-site, icelines-cli
- 5 seasons bundled in binary (20212022–20252026, ~4.3MB total)
- PlayerRepository — single authoritative data loading API
- 338 tests: L0 unit, L1 integration, L2 system + mock NHL API fixture

### Data pipeline
- NHL API client: bios, stats, realtime, rosters, contracts, schedule
- MoneyPuck xG/CF%/FF% integration (silo'd, optional)
- NHL realtime stats: hits, blocked_shots, giveaways, takeaways, PIM
- Snapshot store with SHA-256 integrity, provenance chain, tiered architecture
- Contract data: expiry_year, expiry_type (UFA/RFA/ELC)

### Player model
- 50+ fields covering all-situations, PP, SH, shot metrics, physical, bio, draft, contract
- Multi-season aggregate (`--seasons N`) across bundled history
- Y/Y improvement sort (`--sort improvement`)
- Duplicate player dedup (NHL API emits multiple rows for traded players)

### Commands
- `icelines fetch` — rosters, stats, realtime, positions, contracts, moneypuck
- `icelines query leaders/player/compare` — 30+ sort metrics, percentiles, JSON/CSV
- `icelines fantasy` — SQLite leagues/teams, scoring, trade simulation, axum HTTP server
- `icelines rank/team/players/history/project/scouting/mates/peers/class/compare`
- `icelines group/scheme/snapshot/data/tui/tonight/schedule`
- `icelines build/serve/deploy` — mkdocs static site

### Repo process
- CLAUDE.md — session context, crate ownership, rules
- CODEBASE.md — where to write code, full module map
- design/ — specs, plans, invariants, pitfalls
- docs/ — generated output, team pages
- .roles/ — 8 domain review roles
- design/plans/INDEX.md, design/specs/INDEX.md
