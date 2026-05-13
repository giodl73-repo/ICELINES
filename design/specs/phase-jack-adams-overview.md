# Phase Jack Adams — Overview

**Trophy**: Jack Adams Award (NHL coach of the year — "designs the system, manages the lines, makes the bench coordinate")
**Version**: 1.0 (initial)
**Date**: 2026-05-08
**Status**: Implemented — dashboard default, command bar, fantasy grammar, and opt-in AI fallback landed
**Plan**: `design/plans/2026-05-08-phaseJackAdams-mdi-dashboard.md`

---

## Vision in one paragraph

Today the TUI is **SDI** (Single Document Interface) — one screen
at a time, Tab cycles. Phase Jack Adams adds **MDI** (Multi
Document Interface) — a dashboard view that arranges multiple
existing screens side-by-side. When the user has fullscreen
laptop real estate, they get an espn.com-style "front door":
**Scores ribbon top**, **Favorites left**, **Workspace middle
(swappable)**, **Schedule right**, plus a **chat-CLI command bar
bottom** that drives the workspace ("query country=CAN", "box
edm@bos", "player Bedard"). Each pane reuses an existing screen
module — the Norris state structs + Masterton chrome accessors
slot in directly. Side panes are stable nav/info; the middle is
the workspace that swaps based on what the user asks for.
Adaptive layout: drops Schedule, then Favorites, then collapses
to SDI as the terminal narrows. SDI mode (`--standalone` from
Masterton.3) and the default Tab-nav multi-tab mode both stay
unchanged.

## Locked decisions

| Decision | Choice | Rationale |
|---|---|---|
| Mode name | **MDI** ("multi document") vs **SDI** ("single document") for the existing modes (multi-tab and `--standalone`). Three modes total. | Standard windowed-app terminology; user introduced the framing. The `--standalone` Masterton.3 mode is technically also SDI. |
| Layout | Top ribbon (Scores) + 3 columns (Favorites left / Workspace middle / Schedule right) + footer (focused pane chrome) + command bar (bottom). | Espn.com-style "hockey front door" the user described. Workspace is the primary real estate; sides are stable nav/info. |
| Workspace screen swapping | **Middle changes**, sides stable. The workspace can host: Stats/Queries (default), Player card (when a favorite is clicked), Depth chart, Boxscore detail, Compare, etc. Swapped via command bar OR side-pane interactions. | User's brief: "the thing in the middle can change — so i can choose the middle thing to be depth chart or a different analysis or if i pick a favorite the middle becomes their player card." |
| Focus model | **Implicit on middle / driven by command bar**. Side panes are nav widgets — Favorites Enter swaps middle; Schedule Enter (TBD) opens that game/team in middle. No "Tab between panes" cycling. | User's choice: "i think 3 is the starting point [implicit focus] — maybe you have a 1 line command prompt at the bottom… so maybe its a bit of a chat-based CLI." Simpler model; matches the command-palette mental model. |
| Command bar | Persistent bottom row, always visible. `>` prompt. Type a command, Enter executes, result lands in workspace (or hides/shows panes, etc.). Esc clears the input. The command bar IS the primary nav. | Reuses the Phase Art Ross filter parser for `query` commands; extends with a small slash-command grammar (`/hide schedule`, `/show favorites`, `/help`, `/quit`). |
| Side-pane visibility | **User-toggleable** via keybinds: `Ctrl+H` toggles Favorites, `Ctrl+L` toggles Schedule. Toggles are independent of the adaptive auto-drop. | User's ask: "i can hide the schedule if its in the way and get more real estate in the middle." Adaptive drop handles narrow terminals; manual toggle handles user preference at any width. |
| Adaptive width breakpoints | `≥160 cols`: full MDI (Scores + Favorites + Workspace + Schedule). `120-159`: drop Schedule. `100-119`: drop Favorites. `<100`: collapse to SDI. | Same family as Masterton.1's 120-col chrome threshold. Drops the rightmost pane first (Schedule is more transient than Favorites). |
| Command grammar (v1) | Free-form `query <Phase Art Ross filter>` (delegates to existing `parse_query`); `box <game-id-or-team@team>`; `player <name-or-pid>`; `team <abbrev>`; `goalies` / `transactions` / `playoffs` / `stats` / `depth` (swap workspace to that screen); slash commands `/hide`/`/show`/`/help`/`/quit`. | Builds on Phase Art Ross. The command bar is the "search anything" entry point — typing `Bedard` could disambiguate to player; `country=CAN` to a query. |
| Reuse from Norris/Masterton | Each pane's screen reuses its existing renderer + state struct + chrome accessor. Workspace dispatches to the per-screen renderer based on its current "workspace screen" discriminator. Sides delegate to their respective render fns. | Norris already split state per-screen; Masterton.1 declared chrome per-screen; both slot in. The deferred Masterton.2.x trait migrations are NOT prerequisites — the MDI layout layer is render-only composition; input still flows through App's existing dispatch. |
| Default mode on `icelines tui` | **MDI dashboard** is the normal product entry. `--mdi` remains accepted for explicit launches; `--classic` keeps the older SDI multi-tab UI available. | The earlier opt-in dashboard was too hidden to change product feel. The release after Prince makes Jack Adams visible by default while preserving a classic escape hatch. |
| Out-of-MDI screens | Sub-screens that don't fit cleanly (e.g., Game detail, Series detail) open in the workspace pane when navigated to. They render as the workspace screen until the user navigates away. | The middle IS the screen-stack location; sub-screens land there normally. |
| Workspace screen identity | **Reuse the existing `crate::tui::app::Screen` enum directly.** No separate `WorkspaceScreen` enum. App's existing `screen` field IS the workspace discriminator in MDI mode; switching workspace = mutating `app.screen`. — forge-1 / forge-2 |
| `app.screen` semantics in MDI | **`app.screen` stays the canonical "active screen"** — in MDI it's the workspace's screen. All existing `App::handle` per-screen dispatch keeps working unchanged when input routes to the workspace. The MDI render path consults `app.screen` to know which renderer to call for the middle pane. | Single source of truth; avoids enum drift. — forge-2 |
| Command-bar focus model | **Command bar has focus when input is non-empty OR the user pressed `:` / `/` to enter it.** Otherwise focus is on the workspace (today's keybinds work as in SDI). Esc clears bar input + returns focus to workspace. Up/Down route to bar history when bar is focused; otherwise to workspace's existing handlers. | Concrete focus rules — forge-3. Vim-like; matches the `:` command-mode mental model. |
| Command grammar — verb prefix required | **Strict verb-or-slash prefix in v1.** No bare-disambiguation. `country=CAN` is a parse error; user types `query country=CAN` (or alias `q country=CAN`). Slash commands `/help`, `/quit`, etc. | Removes ambiguity (a Bedard substring search vs a `Bedard` command), keeps grammar predictable. — forge-5 |
| `ParseError` shape | **Structured enum** in `tui::command`: `UnknownCommand(String)`, `MissingArg { command, arg }`, `BadFilter(icelines_query::ParseError)`, `AmbiguousMatch { needle, candidates }`. The bar UI renders targeted messages, not a string blob. | Better UX; matches Phase Art Ross's structured `ParseError` pattern. — forge-4 |
| Chrome row budget | **2 rows of chrome total**: Scores ribbon (top, 1 row) + combined footer/command-bar (bottom, 1 row). When command-bar input is empty, the bottom row shows the workspace's keybind chips (Masterton.1 chrome). When non-empty, the bottom row shows the `>` prompt + input + cursor. | Saves vertical real estate at 24-row terminals. The footer/cmd bar ARE the same row, modal. — glass-1 |
| Scores ribbon overflow strategy | **Priority-ordered render**: LIVE games first, FINAL games next, scheduled-not-yet-started last. Drop trailing games with `… +N more` indicator when overflow. Auto-cycle (rotate) the visible window every ~10s if there's a continuous overflow. | Defines the truncation strategy concretely. — glass-3 |
| Command-bar error channel | **Errors render IN the command-bar row itself** (red text, replaces the `>` prompt for ~2 seconds, then cleared back to chip-mode footer). Also persisted to `app.status` for accessibility / dump-state debugging. | "Game not found" from `box edm@bos` lands somewhere visible without consuming a separate row. — glass-4 |
| SDI ↔ MDI mode flip on resize | **Strict launch-time mode**: `--mdi` is set at launch; subsequent resize narrows panes adaptively (Adams.4) but never flips back to SDI mid-session. | Less jarring than mode-changing on every resize event. — glass-5 |
| Mutual exclusion | **Clap `conflicts_with`** between `--mdi`, `--classic`, and `--standalone`. Passing incompatible modes produces a parse error. | Explicit at the parse layer. — wire-1 |
| Command bar ↔ filter editor state | **Shared**: `query <filter>` from the command bar mutates `app.queries.filter_text` and `app.queries.filter_plan` directly. If the user later opens the Stats filter editor (`f` in workspace=Stats), the editor opens pre-populated with the command-bar filter. | Single source of truth for filter text. — edge-2 |
| Command grammar v1 — write actions | **Favorites mutation in-bar**: `/fav add Bedard`, `/fav remove Bedard` (or `fav add` shortcut). Mutates the `Favorites` group in `GroupDb`; left-pane Favorites list updates next render; workspace optionally swaps to that player's card on `add`. | User's brief: "i can do everything from the CLI so add/remove a favorite". Same backing DB as the player-card `g` keybind. |
| Command grammar v1 — read actions | `team EDM` (workspace = team depth chart), `team EDM season` (workspace = full-season schedule for that team), `roster` / `fantasy roster` (workspace = the user's active fantasy roster), `class 2015` (workspace = draft class), and the v1 reads already in spec (`box`, `player`, `compare`, `goalies`, `transactions`, `playoffs`, `stats`, `depth`). All "pop in the middle." | User's brief: "all of those things pop in the middle." Each command swaps `app.screen` to the relevant Screen variant. |
| AI LLM fallback (Adams.6, opt-in) | **Optional config**. When deterministic `parse_command` returns `Err(UnknownCommand)` AND `[ai] enabled = true` in config, send the raw input + a system prompt describing our command grammar to the configured provider (Claude CLI shell-out via `claude -p` OR Anthropic HTTP API via `ANTHROPIC_API_KEY`). The LLM returns a structured Command JSON we validate + execute. Spinner in the command bar during the call (~1-2s); Esc cancels. | User's brief: "maybe we can even do an AI LLM provider when we want to shell out to an API_KEY or claude -p etc — to help interpret the call into a screen pop." Off by default; config-gated. |
| AI provider config | New `[ai]` section in `~/.icelines/config.toml`: `enabled` (bool), `provider` (`claude-cli` / `anthropic-api`), `api_key_env` (env var name, default `ANTHROPIC_API_KEY`), `model` (default `claude-haiku-4-5` for low latency), `system_prompt_path` (override the bundled prompt). Default config has `enabled = false`. | Read at startup; live-reloadable via `/ai reload`. Env-var indirection so the API key is never persisted in the config file. |
| AI safety contract | LLM output is **always validated** before execution: the LLM returns a `Command` JSON that the existing `parse_command` machinery re-validates as if the user typed it. We never `eval` or shell-out from the LLM's text. If the LLM returns malformed JSON or a Command that fails downstream validation (e.g., unknown player), surface the original parse error to the user, not the LLM hallucination. | LLM is a translation layer, not an execution layer. Defense against prompt injection / hallucination. |

## Sub-phase ordering

```
Adams.1 ─── MDI layout engine + workspace dispatcher (~3 days)
                   │
                   └─→ Adams.2 ─── Command bar parser + executor (~4 days)
                                       │     (expanded grammar: read + write actions)
                                       │
                                       └─→ Adams.3 ─── Side-pane integrations + toggles (~2 days)
                                                            │
                                                            └─→ Adams.4 ─── Adaptive width + auto-drop (~1 day)
                                                                                │
                                                                                └─→ Adams.5 ─── Closeout v0.23.0 (CHANGELOG + tag)
                                                                                                  │
                                                                                                  └─→ Adams.6 ─── AI LLM fallback (~2-3 days, OPT-IN)
                                                                                                                    │
                                                                                                                    └─→ Adams.7 ─── Closeout v0.23.1 (LLM)
```

Adams.1 is foundation — split the render function into MDI vs
SDI paths, render multiple screens to subregions, route input
through App's existing dispatch but with workspace-screen
awareness. Adams.2 is the chat-CLI — parser, executor, expanded
grammar (read + write actions: `team EDM season`, `/fav add`,
`roster`, `class 2015`, plus the v1 reads). Adams.3 wires
Favorites' row-click → workspace swap, Scores ribbon
expand-on-Enter → BoxscoreDetail, etc. Adams.4 polishes the
adaptive layout. Adams.5 cuts v0.23.0 (the deterministic
chat-CLI MDI ships).

Adams.6 is the AI LLM fallback — separate sub-phase + separate
release. Optional config-gated feature; ships in a follow-on
v0.23.1. Keeps the v0.23.0 "core MDI" release clean and
focused; the LLM addition has its own surface area + config +
provider-shell-out work that benefits from a dedicated review.

## Out of scope (deferred or handled separately)

- **Mouse / touch input** — keyboard only. The command bar
  replaces "click on a thing" for nav.
- **User-defined layouts** (drag-resize panes, save layout
  presets) — fixed layout in v1; presets could come later.
- **More than 3 columns + 1 ribbon** — design is locked to the
  espn-style 4-region shape. A "quad split" or NxM grid is a
  separate phase.
- **Live-updating side panes** — Scores ribbon refreshes on the
  existing 30s ticker; Favorites/Schedule re-render on each
  frame from cached state. No new polling threads.
- **Drag-and-drop swap between panes** — middle is the only
  swappable pane; sides are stable.
- **Web / CLI surfaces** — MDI is TUI-only. The web has its own
  page composition model.
- **Persistent command history across sessions** — command bar
  history is in-memory only in v1 (similar to Phase Art Ross's
  filter history within the editor). Persisting could be a
  future polish.
- **AI fallback in v0.23.0** — moved to Adams.6 / v0.23.1
  (separate release). v0.23.0 ships deterministic-only chat-CLI;
  the LLM addition is config-gated and ships separately so the
  core MDI release is clean and the LLM has dedicated review
  surface.
- **Streaming LLM responses** (Adams.6) — v1 issues a single
  request and waits for the full structured response. Streaming
  partial responses into a "thinking…" preview is future polish.
- **Multi-turn LLM conversations** — v1 is single-turn (input →
  Command). Maintaining conversation context across multiple
  command-bar entries would need a separate design.
- **AI command suggestions / proactive prompts** — the LLM only
  fires on deterministic-parse-failure. We never call the LLM
  pre-emptively to suggest commands.

## Surface coverage matrix

Jack Adams is a TUI-only feature. CLI and web surfaces are
unaffected.

| Capability | CLI | TUI | Web |
|---|---|---|---|
| MDI dashboard mode | n/a | Default: `icelines tui`; explicit: `icelines tui --mdi`; classic escape hatch: `icelines tui --classic` | n/a (web has its own page composition) |
| Command bar | n/a | New: bottom-row prompt with parser + executor | n/a |
| Side-pane toggles | n/a | New: `Ctrl+H` / `Ctrl+L` | n/a |
| Adaptive drop | n/a | New: layout shrinks gracefully at narrow widths | n/a |
| Existing keybinds | Unchanged | Unchanged inside the workspace pane (which is a regular screen) | n/a |

## Sub-phase summaries

### Adams.1 — MDI layout engine + workspace dispatcher (~3 days)

New module `tui/mdi.rs` defining:

```rust
pub struct MdiLayout {
    pub show_favorites: bool,
    pub show_schedule: bool,
    pub workspace: WorkspaceScreen,  // discriminator
    pub command_input: String,
}

pub enum WorkspaceScreen {
    Stats,           // default — Queries
    PlayerCard(PlayerId),
    Team(TeamAbbr),
    Depth,
    Goalies,
    Transactions,
    Playoffs,
    BoxscoreDetail(GameId),
    Compare(PlayerId, Option<PlayerId>),
    // ... extensible
}
```

App gains `pub mdi: Option<MdiLayout>` — `Some` when launched in
MDI mode, `None` for SDI (`--classic` and `--standalone`).

Render dispatch in `screens/mod.rs::render` branches:

```rust
pub fn render(f: &mut Frame, app: &App) {
    if let Some(mdi) = &app.mdi {
        render_mdi(f, app, mdi);
    } else {
        render_sdi(f, app);  // today's path, renamed
    }
}
```

`render_mdi`:
- Lays out vertical: Scores ribbon (1) / body (Min) / footer (1) / command bar (1)
- Body: horizontal split by visible side panes
  - Favorites column (`Length(28)` when shown)
  - Workspace column (`Min(0)`)
  - Schedule column (`Length(28)` when shown)
- Each pane calls into the relevant existing render fn:
  - Favorites: `screens::favorites::render`
  - Workspace: dispatch on `mdi.workspace` to the right render fn
  - Schedule: `screens::schedule::render`
  - Scores ribbon: a NEW compact renderer in `screens/misc.rs`
    that fits today's slate into ~1 row with team abbrevs + scores

App's input dispatch in MDI mode routes through the command bar
when the bar is focused (default), or through the workspace
screen's existing dispatch when the bar isn't (Esc to leave bar
focus, anything to re-enter). Side panes don't take focus in v1
— they're driven by command-bar typing or by clicking-via-
hotkeys (e.g., `Ctrl+1` jumps the workspace to whatever is
selected in Favorites).

**Test budget**: ~12 tests — MdiLayout default, workspace
discriminator round-trips, render branches on `app.mdi`, MDI
keybind routing.

### Adams.2 — Command bar parser + executor (~4 days)

New module `tui/command.rs`:

```rust
pub enum Command {
    // Slash commands — meta
    Help,
    Quit,
    Hide(SidePane),
    Show(SidePane),

    // Workspace-swap reads (no args)
    Stats,
    Goalies,
    Transactions,
    Playoffs,
    Depth,
    Roster,            // user's active fantasy roster

    // Workspace-swap reads (with args)
    PlayerCard { name_or_pid: String },
    Team { abbrev: String },
    TeamSeason { abbrev: String },     // full-season schedule
    Compare { left: String, right: Option<String> },
    Box { game: String },              // game_id or "edm@bos"
    Class { year: u16 },               // draft class

    // Write actions
    FavAdd { name_or_pid: String },
    FavRemove { name_or_pid: String },

    // Free-form query — delegates to Phase Art Ross
    Query { filter: String },
}

#[derive(Debug, Clone)]
pub enum ParseError {
    UnknownCommand(String),
    MissingArg { command: &'static str, arg: &'static str },
    BadFilter(icelines_query::ParseError),
    AmbiguousMatch { needle: String, candidates: Vec<String> },
    BadInteger { command: &'static str, raw: String },
}

pub fn parse_command(input: &str) -> Result<Command, ParseError>;
pub fn execute_command(cmd: Command, app: &mut App) -> ExecResult;
```

`execute_command` mutates `app.screen` (workspace), favorites
state (`FavAdd`/`FavRemove`), or filter state (`Query`), and
returns flash text or a downstream error.

Command bar UI:
- Combined footer/cmd-bar row (per glass-1): chip-mode when
  input is empty, `>` prompt-mode when non-empty.
- Up/Down: history (in-memory ring, similar to Wave 24b filter
  history) — only when bar focused.
- Enter: execute; on success, clear input; on parse-error, keep
  input + show error inline (red, replaces `>` prompt for ~2s
  then back to chip-mode footer).
- Esc: clear input + return focus to workspace.
- Tab: auto-complete — slash-commands first, then verbs, then
  resolved candidates (player names from repo, team abbrevs).

**Test budget** (post-review bench-1, expanded for write
actions + grammar): ~40 tests. Parser variants (every Command
shape × valid/missing-arg/bad-arg/ambiguous), executor (every
Command → expected mutation, including FavAdd/FavRemove DB
write through tempdir GroupDb), history ring, Tab completion,
error rendering.

### Adams.3 — Side-pane integrations (~2 days)

- Favorites: render the existing favorites screen state in the
  left column (compact). When the user types `player Bedard` in
  the command bar OR (future) presses Enter on a Favorites row
  via a hotkey, workspace swaps to that player card.
- Schedule: render the existing schedule screen state in the
  right column (compact, 7-day list). When the command bar
  parses `box edm@bos`, the boxscore detail opens in the
  workspace.
- Scores ribbon: new compact renderer in `screens::misc`, takes
  the existing `app.tonight.cache` data and produces a 1-row
  summary line with auto-truncation when overflow.
- Side-pane toggle keybinds: `Ctrl+H` (Favorites), `Ctrl+L`
  (Schedule). Mutates `mdi.show_*` flags; layout reflows on
  next render.

**Test budget**: ~10 tests — pane-toggle keybinds, Favorites
swap → workspace = PlayerCard, command-bar `box edm@bos` →
workspace = BoxscoreDetail.

### Adams.4 — Adaptive width + auto-drop (~1 day)

`MdiLayout` gets an `effective_panes(width: u16)` helper that
returns which panes to actually render at a given width:
- `≥160`: all four (Scores, Favorites, Workspace, Schedule)
- `120-159`: drop Schedule
- `100-119`: drop Favorites
- `<100`: render SDI (the body becomes just the workspace screen
  with normal SDI chrome)

Manual toggles (`mdi.show_favorites = false`) override the
adaptive default — if the user manually hid Favorites at
`≥160`, the layout respects it.

**Test budget**: ~8 tests — every width threshold × every
manual-toggle combination.

### Adams.5 — v0.23.0 closeout (~0.5 day)

- CHANGELOG entry summarizing the deterministic MDI phase
- CLAUDE.md "What's been built" gets a Phase Jack Adams bullet
  (deterministic mode shipped)
- Cargo bump 0.22.0 → 0.23.0 (minor — new mode, new keybinds)
- Commit + tag v0.23.0 + push

### Adams.6 — AI LLM fallback (~2-3 days, OPT-IN)

New module `tui/command_ai.rs`:

```rust
pub trait AiProvider {
    fn translate(&self, input: &str, system_prompt: &str)
        -> Result<Command, AiError>;
}

pub struct ClaudeCli;        // shells out to `claude -p "<prompt>"`
pub struct AnthropicApi;     // HTTP via reqwest + ANTHROPIC_API_KEY
```

Wired into `parse_command`:
1. Run deterministic parse first.
2. On `Err(UnknownCommand)` AND config has `[ai] enabled = true`:
   - Show spinner in command bar ("thinking…")
   - Spawn provider call (async, ~1-2s)
   - LLM returns a structured Command JSON
   - Re-validate the Command via `parse_command` shape rules
     (defense-in-depth: never `eval` the LLM's text)
   - If validation passes, execute; else surface the original
     parse error.
3. Esc cancels the in-flight call.

Config (in `~/.icelines/config.toml`):

```toml
[ai]
enabled = false                       # default off
provider = "claude-cli"               # or "anthropic-api"
api_key_env = "ANTHROPIC_API_KEY"     # never store keys in config
model = "claude-haiku-4-5"            # low-latency default
system_prompt_path = ""               # empty = use bundled prompt
```

The bundled system prompt enumerates our command grammar
(every Command variant + arg shape + a few examples) and
instructs the LLM to return strict JSON matching the Command
schema, no prose.

**Test budget**: ~15 tests:
- AiProvider trait conformance (mock provider returning canned
  Commands)
- Schema validation (LLM returns malformed JSON / unknown
  command / missing required field — all surface clean errors)
- Spinner state machine (idle / in-flight / canceled)
- Config parsing (every `[ai]` field, defaults, env-var
  resolution)
- Integration: deterministic-parse-fail → AI-translate path
  via mock provider

L2 subprocess tests are skipped (LLM calls aren't deterministic
in CI; mock provider covers the integration shape).

### Adams.7 — v0.23.1 closeout (~0.5 day)

- CHANGELOG entry for the AI fallback
- COMMANDS.md gains an `[ai]` config section
- Cargo bump 0.23.0 → 0.23.1 (patch — additive opt-in feature)
- Commit + tag v0.23.1 + push

## Total budget

- ~12-13 working days total (v0.23.0 deterministic MDI: ~9-10
  days; v0.23.1 AI fallback: ~2-3 days additional)
- ~85 new tests across both releases:
  - v0.23.0 (Adams.1-5): ~70 tests (12 layout + 40 command +
    10 integrations + 8 adaptive)
  - v0.23.1 (Adams.6-7): ~15 tests (AI fallback w/ mock provider)
- New modules: `tui/mdi.rs`, `tui/command.rs`, `tui/command_ai.rs`
  (Adams.6)
- App gains: `mdi: Option<MdiLayout>` field
- Two release tags: v0.23.0 (Adams.5) cuts the deterministic
  MDI; v0.23.1 (Adams.7) cuts the AI fallback

## Pre-flight checklist

- [x] v0.22.0 shipped (Phase Masterton complete)
- [x] Bin suite green at HEAD (803/803)
- [x] Per-screen state structs landed (Norris) — workspace
      composition reuses these
- [x] Per-screen chrome accessors landed (Masterton.1) —
      footer in MDI shows the workspace's chrome
- [x] Command-bar parser can reuse `icelines_query::parse_query`
      for `query` commands
- [x] Spec reviewed via role pass
- [x] Adams.1 starts

## Implementation addendum — 2026-05-12

The shipped dashboard extends the original command grammar with the fantasy
workflow completed during the Selke/Campbell/Jack Adams overlap:

- `gaps` / `fantasy gaps` filters active roster gaps by categories and limit.
- `poach` / `fantasy poach` filters the poacher board by category, position,
  availability, candidate kind, and limit.
- `simulate` / `fantasy simulate` applies add/drop/drop-only scenario state and
  can clear the active scenario.
- Fantasy screen shortcuts prefill the same grammar: `g` on Fantasy Gaps, `p`
  on Poach, and `a` on Fantasy Simulation.

The invariant is that TUI fantasy commands lower into the same shared
ViewModels used by CLI text/JSON, web HTML/JSON, and report surfaces.

## Cross-cutting open items

1. **Workspace screen → Screen-trait migration relationship**
   — the deferred Masterton.2.2-2.7 work IS NOT a prerequisite
   for Jack Adams. Each workspace render-fn call goes through
   the existing free-fn renderers (`screens::queries::render`,
   etc.) which take `&App`. Future work could migrate workspace
   dispatch to use the Screen trait, but that's optional.
2. **Side panes in the workspace** — when the workspace shows,
   say, Stats/Queries, what happens to `app.queries` state?
   Same as today: it's persisted on App. Switching workspace to
   PlayerCard doesn't lose Queries' state. Returning to Stats
   restores it. (Mirrors today's tab-cycle behavior.)
3. **Command bar history vs Phase Art Ross filter history** —
   two separate rings. The filter editor (in QueriesState) has
   its own history; the command bar (in MdiLayout) has its own.
   Could be unified later but for v1 they're independent (the
   filter editor only opens when the workspace is Stats; the
   command bar is always available).
4. **Esc semantics in MDI** — Esc on the command bar clears
   input but stays focused (chat-CLI convention). Esc when
   focus is implicitly on workspace pops the workspace screen
   (PlayerCard → Stats; Stats has no pop, stays). Esc-from-leaf
   in MDI stays in MDI (doesn't bounce back to SDI).
5. **Mode switching** — `icelines tui` defaults to the MDI
   dashboard. `--mdi` is accepted for explicit dashboard launches.
   `--classic` opts into the older SDI multi-tab UI. `--standalone`
   opts into single-screen-locked mode (Masterton.3). The mode flags
   are mutually exclusive.
6. **Quit propagation from inside a workspace screen** — same
   as today. The workspace screen's handler can return Quit;
   App propagates.
7. **Tests for the chat-CLI** — heavy on the parser (input
   strings → Command enums) and the executor (Command → state
   mutations). Light on render snapshots since terminal-driven
   visual tests aren't portable.
8. **MDI tests are additive** (post-review bench-2): existing
   803 SDI tests run unchanged because `App::mdi` defaults to
   None and the MDI render path is gated on `Some`. New
   `tui::mdi::tests` and `tui::command::tests` modules carry
   the MDI-specific coverage. No SDI-test churn expected.
9. **Property-style adaptive layout test** (post-review
   bench-4): for every width in 80..200 step 4,
   `MdiLayout::effective_panes(width)` must produce a valid
   layout (no zero/negative widths, sum-of-pane-widths ≤
   available). Cheap; catches edge cases the discrete-threshold
   tests miss.
10. **AI fallback safety boundary** (Adams.6): the LLM is a
    translation layer, NOT an execution layer. LLM output is
    parsed back through `parse_command`'s shape validation
    before execution. Defense against prompt injection /
    hallucination. Never `eval` the LLM's text.
11. **AI fallback latency UX** (Adams.6): provider calls take
    ~1-2s. Spinner in command bar with "thinking…" text. Esc
    cancels the in-flight call. Workspace stays responsive
    during the call (user can navigate while waiting). Cancel
    must not panic if the LLM's response arrives post-cancel.
12. **AI provider auth** (Adams.6): API keys read from env vars
    (default `ANTHROPIC_API_KEY`), NEVER stored in
    `~/.icelines/config.toml`. Keeps the on-disk config safe to
    share / commit. The `claude-cli` provider shells out to
    `claude -p`, which has its own auth path; no config needed.
13. **Mode-switching UX precedence** (post-review wire-1):
    clap rejects incompatible pairs such as `--mdi --standalone`
    and `--mdi --classic` at parse time (`conflicts_with`). Default
    `icelines tui` launches MDI with the requested surface as the
    workspace; `icelines tui --classic` opts into SDI multi-tab.
