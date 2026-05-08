# Phase Jack Adams — Plan

**Spec**: `design/specs/phase-jack-adams-overview.md`
**Date**: 2026-05-08
**Status**: Plan — ready for execution
**Dependency**: v0.22.0 (Masterton complete; trait scaffold + chrome
landed; bin suite 803/803 green)

---

## Sub-phase ordering

```
Adams.1 ─── MDI layout engine + workspace dispatcher
Adams.2 ─── Command bar parser + executor (expanded grammar)
Adams.3 ─── Side-pane integrations + toggles
Adams.4 ─── Adaptive width + auto-drop
Adams.5 ─── v0.23.0 closeout (deterministic MDI ships)
Adams.6 ─── AI LLM fallback (opt-in)
Adams.7 ─── v0.23.1 closeout (AI fallback ships separately)
```

Each sub-phase ships as one (sometimes two — extraction + tests)
git commit. Each commit must:

- compile cleanly (`cargo build -p icelines-cli`)
- pass the full bin test suite at the same baseline (no
  regressions to SDI mode)
- not change any user-visible SDI behavior (no keybind / render
  delta when `app.mdi == None`)

## Pre-flight

- [x] v0.22.0 tagged + pushed (Masterton complete)
- [x] `cargo test --workspace` green at HEAD (803/803)
- [x] Spec written + 8-role review applied (13 action items folded)
- [x] phases.md updated with Jack Adams row
- [ ] Adams.1 begins

## Adams.1 — MDI layout engine

**Goal**: Add the MDI render path. SDI mode unchanged. The MDI
path renders Scores ribbon + 3-col body + footer/cmdbar but the
sub-renderers are stubbed for Adams.1 — full integration lands
in Adams.3.

### A.1.1 — `tui/mdi.rs`

```rust
//! Phase Jack Adams.1 — MDI dashboard layout.

#![allow(clippy::module_name_repetitions)]

use crate::tui::app::Screen;

#[derive(Debug)]
pub struct MdiLayout {
    pub show_favorites: bool,
    pub show_schedule: bool,
    pub command_input: String,
    pub command_history: std::collections::VecDeque<String>,
    pub command_history_cursor: Option<usize>,
    pub flash_error: Option<String>,
}

impl Default for MdiLayout {
    fn default() -> Self {
        Self {
            show_favorites: true,
            show_schedule: true,
            command_input: String::new(),
            command_history: std::collections::VecDeque::new(),
            command_history_cursor: None,
            flash_error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidePane {
    Favorites,
    Schedule,
}

/// Visibility decision for the four MDI regions at a given width.
/// Combines manual toggles (mdi.show_*) with adaptive auto-drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneVisibility {
    pub scores: bool,        // always true in MDI
    pub favorites: bool,
    pub workspace: bool,     // always true (it's the middle)
    pub schedule: bool,
}

impl MdiLayout {
    /// Phase Adams.4 — adaptive auto-drop. Manual toggles
    /// override the adaptive default (e.g., user manually hid
    /// Favorites at ≥160 cols → respect the toggle).
    pub fn effective_panes(&self, width: u16) -> PaneVisibility {
        // Adaptive thresholds:
        // ≥160: full MDI
        // 120-159: drop Schedule
        // 100-119: drop Favorites
        // <100: collapse to SDI (caller handles by exiting MDI render)
        let adaptive_favorites = width >= 100;
        let adaptive_schedule = width >= 160;

        PaneVisibility {
            scores: true,
            favorites: self.show_favorites && adaptive_favorites,
            workspace: true,
            schedule: self.show_schedule && adaptive_schedule,
        }
    }

    /// True iff the terminal is too narrow for any MDI rendering
    /// — caller falls back to SDI mode for this frame.
    pub fn collapse_to_sdi(width: u16) -> bool {
        width < 100
    }
}
```

Tests in `mdi::tests`:
- `MdiLayout::default()` has both side panes visible, empty
  command input.
- `effective_panes(160)` shows all four regions.
- `effective_panes(140)` drops Schedule, keeps Favorites.
- `effective_panes(110)` drops both Schedule and Favorites
  (auto), keeps workspace.
- `effective_panes(80)` would drop everything; caller is
  expected to call `collapse_to_sdi(80)` and bail out.
- Manual toggle: `show_favorites = false` at width 200 →
  `effective_panes(200).favorites == false`.
- Property test: every width in 80..200 step 4 → valid
  PaneVisibility (no panic, sum-of-widths reasonable).

### A.1.2 — App field + RunTuiOpts wiring

In `tui/app.rs`:

```rust
/// Phase Adams.1 — MDI dashboard state. Some when launched
/// with `--mdi`; None for SDI modes (today's default and
/// `--standalone`). Render path branches on this.
pub mdi: Option<crate::tui::mdi::MdiLayout>,
```

In `App::new`: `mdi: None,`

In `tui/mod.rs::RunTuiOpts`:

```rust
pub mdi: bool,
```

In `RunTuiOpts::home()`: `mdi: false`

In `run_loop`:

```rust
if opts.mdi {
    app.mdi = Some(crate::tui::mdi::MdiLayout::default());
}
```

### A.1.3 — Clap `--mdi` flag

In `cli.rs::Tui` variant:

```rust
/// Phase Adams.1 — launch the TUI in MDI dashboard mode.
/// Espn-style "front door": Scores ribbon top, Favorites
/// left, Workspace middle (swappable), Schedule right, plus
/// a chat-CLI command bar bottom.
///
/// Mutually exclusive with --standalone.
///
/// Example:
///   icelines tui stats --mdi
#[arg(long, conflicts_with = "standalone")]
mdi: bool,
```

main.rs threads the flag through:

```rust
Commands::Tui { surface, start, standalone, mdi } => {
    // ... resolve start screen as today
    tui::run_tui(tui::RunTuiOpts {
        no_color: false,
        start_screen,
        standalone,
        mdi,
    }).await?;
}
```

### A.1.4 — Render branch

In `screens/mod.rs::render`:

```rust
pub fn render(f: &mut Frame, app: &App) {
    // Phase Adams.1 — MDI ↔ SDI dispatch.
    if let Some(mdi) = &app.mdi {
        let width = f.area().width;
        if !crate::tui::mdi::MdiLayout::collapse_to_sdi(width) {
            render_mdi(f, app, mdi);
            return;
        }
        // Width too narrow — fall through to SDI render path
        // (per glass-5: strict launch-time mode means we don't
        // flip back to SDI mid-session, but at <100 cols we
        // simply render SDI for this frame; resize back ≥100
        // returns to MDI).
    }
    render_sdi(f, app);
}
```

Where `render_sdi` is today's render function renamed.

`render_mdi` (Adams.1 stub):

```rust
fn render_mdi(f: &mut Frame, app: &App, mdi: &crate::tui::mdi::MdiLayout) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Scores ribbon
            Constraint::Min(0),     // body (3-col split)
            Constraint::Length(1),  // combined footer/cmdbar
        ])
        .split(area);

    // Scores ribbon — Adams.1 stub: empty bar with "MDI mode"
    // placeholder. Real ribbon renders in Adams.3.
    f.render_widget(
        Paragraph::new(" MDI mode (Phase Jack Adams)").style(Style::default().fg(Color::DarkGray)),
        chunks[0],
    );

    // Body 3-col split.
    let visible = mdi.effective_panes(area.width);
    let mut constraints: Vec<Constraint> = Vec::new();
    if visible.favorites { constraints.push(Constraint::Length(28)); }
    constraints.push(Constraint::Min(0));
    if visible.schedule { constraints.push(Constraint::Length(28)); }
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(chunks[1]);

    // Pane render dispatch — Adams.1 STUBS.
    let mut idx = 0;
    if visible.favorites {
        render_pane_stub(f, body_chunks[idx], "FAVORITES");
        idx += 1;
    }
    render_pane_stub(f, body_chunks[idx], "WORKSPACE");
    idx += 1;
    if visible.schedule {
        render_pane_stub(f, body_chunks[idx], "SCHEDULE");
    }

    // Combined footer/cmdbar — Adams.1 stub: chip-mode showing
    // workspace chrome. Adams.2 adds the prompt-mode branch.
    f.render_widget(
        Paragraph::new(" > _").style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
}

fn render_pane_stub(f: &mut Frame, area: Rect, label: &str) {
    let block = Block::default().borders(Borders::ALL).title(format!(" {label} "));
    f.render_widget(block, area);
}
```

Adams.3 replaces the stubs with real renderers. Adams.1 just
proves the layout works.

### A.1.5 — App handle: Tab no-op in MDI

```rust
Action::Tab => {
    if self.locked_screen.is_none() && self.mdi.is_none() {
        self.cycle_screen();
    }
    // In MDI: Tab is reserved for command-bar autocompletion
    // (wired in Adams.2). For Adams.1, Tab is a no-op in MDI.
}
Action::TabPrev => {
    if self.locked_screen.is_none() && self.mdi.is_none() {
        self.cycle_screen_back();
    }
}
```

### A.1.6 — Gauntlet

```
cargo build -p icelines-cli         # clean
cargo test -p icelines-cli --bin icelines  # 803 baseline + ~12 new = ~815
cargo test --release -p icelines-cli --test art_ross_w23_tui_filter
cargo test --release -p icelines-cli --test persona_wave23
cargo test --release -p icelines-cli --test persona_wave25
cargo test --release -p icelines-cli --test persona_masterton_standalone
cargo clippy -p icelines-cli --no-deps -- -D warnings  # no new lints
```

L2 subprocess test for `--mdi`:
- `icelines tui --mdi --help` exits clean
- `icelines tui --mdi --standalone` errors with conflict message
  (clap's built-in)

### A.1.7 — Commit

Two commits:

1. `Phase Adams.1 — MDI layout engine + workspace dispatcher`
   (extraction). Bin suite at 803 baseline. Render path
   branches; MDI stubs render. SDI unchanged.
2. `Phase Adams.1 — L0 tests for MdiLayout` — ~12 tests.

## Adams.2 — Command bar parser + executor

### A.2.1 — `tui/command.rs`

Define `Command`, `ParseError`, `parse_command`,
`execute_command` per spec §M.2. Implementation skeleton:

```rust
pub fn parse_command(input: &str) -> Result<Command, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::UnknownCommand("".into()));
    }
    if let Some(rest) = trimmed.strip_prefix('/') {
        return parse_slash(rest);
    }
    let (verb, args) = trimmed.split_once(' ').unwrap_or((trimmed, ""));
    match verb.to_ascii_lowercase().as_str() {
        "help" => Ok(Command::Help),
        "quit" | "q" => Ok(Command::Quit),
        "stats" => Ok(Command::Stats),
        "goalies" => Ok(Command::Goalies),
        "transactions" | "txs" => Ok(Command::Transactions),
        "playoffs" => Ok(Command::Playoffs),
        "depth" => Ok(Command::Depth),
        "roster" => Ok(Command::Roster),
        "player" => parse_player(args),
        "team" => parse_team(args),
        "compare" => parse_compare(args),
        "box" => parse_box(args),
        "class" => parse_class(args),
        "fav" => parse_fav(args),
        "query" | "q" => parse_query(args),
        unknown => Err(ParseError::UnknownCommand(unknown.into())),
    }
}
```

Note `q` shadows both `quit` and `query` — handled via context
(if the input has no args after `q`, it's quit; with args, it's
query). Or pick one and document. Plan: `q` = `quit` (matches
vim convention), `query` is the verb for filters.

### A.2.2 — `execute_command`

Mutates `app.screen`, `app.queries.filter_text`, favorites DB,
etc. Returns either a flash message (on success — written to
`mdi.flash_error` momentarily) or a structured execution error.

### A.2.3 — Command bar UI

In `screens/mod.rs::render_mdi`, replace the cmdbar stub:

```rust
let cmdbar_area = chunks[2];
if mdi.command_input.is_empty() && mdi.flash_error.is_none() {
    // Chip-mode footer: workspace's chrome keybinds + globals
    let chrome = active_chrome(app);
    render_footer_chips(f, &chrome, cmdbar_area);
} else if let Some(err) = &mdi.flash_error {
    // Error mode: red "couldn't run that — try /help"
    f.render_widget(
        Paragraph::new(format!(" ✘ {err}"))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        cmdbar_area,
    );
} else {
    // Prompt mode: "> user_input▌"
    f.render_widget(
        Paragraph::new(format!(" > {}▌", mdi.command_input))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        cmdbar_area,
    );
}
```

### A.2.4 — Input routing

In `App::handle`, when `app.mdi.is_some()`, route action to
the command bar OR workspace based on focus model (forge-3):

```rust
if let Some(mdi) = &mut self.mdi {
    let bar_focused = !mdi.command_input.is_empty()
        || matches!(action, Action::Char(':') | Action::Search);
    if bar_focused {
        return self.handle_command_bar(action);
    }
    // Otherwise route to workspace screen via existing dispatch.
}
```

`handle_command_bar`:
- Char(c) / Space — append to input
- Backspace — pop
- Enter — parse + execute; clear on success, set flash_error on failure
- Esc — clear input, return focus to workspace
- Up/Down — history nav (in-memory ring)
- Tab — autocomplete (defer to Adams.2.5 polish)

### A.2.5 — Tests

~40 tests in `command::tests`:
- Parser variants per command (15+ commands × valid/missing-arg/
  bad-arg)
- Executor mutations (every Command → expected app mutation)
- Slash command dispatch
- History ring (push / dedupe / Up/Down nav)
- Error rendering (parse error → flash_error set)

### A.2.6 — Commit (two)

1. `Phase Adams.2 — command bar parser + executor` (logic)
2. `Phase Adams.2 — L0 tests for command grammar` (~40 tests)

## Adams.3 — Side-pane integrations

### A.3.1 — Real renderers in MDI panes

Replace pane stubs in `render_mdi` with real renderers:
- Favorites pane: call `screens::favorites::render(f, app, area)`
  (existing render fn, just sized to the narrow column)
- Workspace pane: dispatch on `app.screen` to the right
  renderer (mirror the existing match in `render_sdi` body)
- Schedule pane: call `screens::schedule::render(f, app, area)`
- Scores ribbon: NEW compact renderer in `screens::misc`
  (`render_scores_ribbon(f, app, area)`)

### A.3.2 — Scores ribbon implementation

Reads `app.tonight.cache` for today's date. Builds a one-row
summary line:

```
EDM 4-3 BOS · NYR 2-1 PIT · TOR @ MTL 7pm · CHI @ NSH 7:30pm · …
```

Priority order (per glass-3): LIVE games first (most
interesting), FINAL second, scheduled-not-yet-started last.
Truncate trailing with `… +N more` when overflow.

### A.3.3 — Side-pane toggle keybinds

```rust
Action::Char('h') if ctrl_held(...) => {
    if let Some(mdi) = &mut self.mdi {
        mdi.show_favorites = !mdi.show_favorites;
    }
}
Action::Char('l') if ctrl_held(...) => {
    if let Some(mdi) = &mut self.mdi {
        mdi.show_schedule = !mdi.show_schedule;
    }
}
```

(Ctrl-detection wires through Action::Char with modifier — TBD
based on the existing event handling.)

### A.3.4 — Favorites click → workspace swap

In MDI mode, when the user navigates Favorites and presses
Enter:
- Resolve the selected row to a PlayerId
- `app.screen = Screen::PlayerById(pid)` (workspace swaps to
  the player card)

### A.3.5 — Tests + commit

~10 tests. Two commits as the pattern.

## Adams.4 — Adaptive width + auto-drop

`MdiLayout::effective_panes` already implements the adaptive
logic in Adams.1. Adams.4 adds the property-style coverage and
polish:

- Property test: every width in 80..200 step 4 produces a
  valid `PaneVisibility` (sum of allocated col widths fits
  within the available width).
- L0 tests for every threshold transition (160 → 159 drops
  Schedule; 120 → 119 drops Favorites; 100 → 99 collapses).
- Manual-toggle override coverage (every adaptive default ×
  manual toggle combination).

~8 tests. One commit.

## Adams.5 — v0.23.0 closeout

Standard closeout pattern:
- CHANGELOG entry with the architectural diagram
- CLAUDE.md "What's been built" gets a Phase Jack Adams bullet
- Cargo bump 0.22.0 → 0.23.0
- Commit + tag v0.23.0 + push

## Adams.6 — AI LLM fallback

### A.6.1 — `tui/command_ai.rs`

```rust
//! Phase Jack Adams.6 — AI LLM fallback for natural-language
//! command interpretation.
//!
//! When deterministic `parse_command` fails AND the user has
//! enabled `[ai]` in config, send the input to an LLM provider.
//! The LLM returns a structured Command JSON we re-validate
//! through parse_command (defense-in-depth).

#![allow(clippy::module_name_repetitions)]

pub trait AiProvider: Send + Sync {
    fn translate(&self, input: &str, system_prompt: &str)
        -> Result<crate::tui::command::Command, AiError>;
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("provider call failed: {0}")]
    ProviderFailed(String),
    #[error("LLM returned malformed JSON: {0}")]
    BadJson(String),
    #[error("LLM returned a Command we couldn't validate: {0:?}")]
    InvalidCommand(crate::tui::command::ParseError),
    #[error("LLM call timed out (>{0}s)")]
    Timeout(u64),
    #[error("user canceled")]
    Canceled,
}

pub struct ClaudeCli;       // shells out to `claude -p`
pub struct AnthropicApi { /* api_key, model */ }

impl AiProvider for ClaudeCli { /* shell-out impl */ }
impl AiProvider for AnthropicApi { /* HTTP impl */ }
```

### A.6.2 — Config

In `~/.icelines/config.toml`:

```toml
[ai]
enabled = false
provider = "claude-cli"
api_key_env = "ANTHROPIC_API_KEY"
model = "claude-haiku-4-5"
system_prompt_path = ""
```

`config::Config` gains an `ai: AiConfig` field (Default off).

### A.6.3 — Wire into `parse_command`

```rust
pub fn parse_command_with_ai(
    input: &str,
    ai: Option<&dyn AiProvider>,
    system_prompt: &str,
) -> Result<Command, ParseError> {
    match parse_command(input) {
        Ok(cmd) => Ok(cmd),
        Err(ParseError::UnknownCommand(_)) if ai.is_some() => {
            let provider = ai.unwrap();
            match provider.translate(input, system_prompt) {
                Ok(cmd) => {
                    // Re-validate by serializing back to a
                    // command-line and running through the
                    // deterministic parser. Defense-in-depth.
                    let canonical = cmd.to_canonical();
                    parse_command(&canonical)
                }
                Err(_) => Err(ParseError::UnknownCommand(input.into())),
            }
        }
        Err(other) => Err(other),
    }
}
```

### A.6.4 — System prompt

Bundled at compile time via `include_str!`. Enumerates the
Command grammar with examples; instructs the LLM to return
strict JSON only (no prose).

### A.6.5 — Spinner UI

In the command-bar render: when an AI call is in-flight,
replace the prompt with `> thinking…` cycling through frames.
Esc cancels.

### A.6.6 — Tests

~15 tests with a `MockProvider` that returns canned Commands:
- Successful translation flow
- Malformed JSON → BadJson error → original parse error
  surfaces to user
- Provider timeout → cancellation
- Esc cancels in-flight call (mock takes >100ms; Esc fires
  before completion)
- Config parsing (every `[ai]` field; defaults; env-var
  resolution)

L2 skipped (LLM calls aren't deterministic in CI).

## Adams.7 — v0.23.1 closeout

CHANGELOG entry. COMMANDS.md gains an `[ai]` config section.
Cargo bump 0.23.0 → 0.23.1. Commit + tag + push.

## Acceptance for Phase Jack Adams

- All seven sub-phases ship as their own commits (some have
  paired extraction + tests commits)
- Bin suite grows by ~85 (~70 for v0.23.0 + ~15 for v0.23.1):
  803 → 873 → ~888
- Existing 803 SDI tests pass at every commit (no regressions
  to SDI mode)
- L1/L2 integration suites unchanged
  (art_ross_w23_tui_filter, persona_wave23, persona_wave25,
  persona_masterton_standalone)
- `cargo clippy -p icelines-cli --no-deps` introduces no new
  lints
- v0.23.0 cuts after Adams.5 (deterministic MDI ships)
- v0.23.1 cuts after Adams.7 (AI fallback ships separately)
- Saved-query JSON contract unchanged (Phase Art Ross
  preserved)
- `--mdi --standalone` is rejected by clap at parse time

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| MDI render path regresses SDI mode | High | `App::mdi` defaults to None; render branches early. SDI test suite (803) must stay green at every commit. |
| Command-bar focus model creates input ambiguity | Med | Lock the focus rules (forge-3): bar focused when input non-empty OR `:` / `/` pressed. Test exhaustively. |
| Workspace ↔ filter editor state divergence | Med | Single source of truth (`app.queries.filter_text`); command-bar `query` mutates it directly. Filter editor opens pre-populated. — edge-2 |
| LLM hallucination produces unsafe Command | High | Defense-in-depth: LLM output re-validated through `parse_command`. Never `eval` LLM text. Provider call cancellable via Esc. — Adams.6 |
| LLM provider auth keys leaked in config | Low | API keys read from env vars only; never persisted to `~/.icelines/config.toml`. — Adams.6 |
| Adaptive layout produces zero-width pane at edge cases | Low | Property-style test in Adams.4 covers width 80..200 step 4. |
| Tab cycling breaks in MDI | Med | Tab is intercepted in App::handle when `app.mdi.is_some()`; reserved for cmd-bar autocomplete. SDI Tab unchanged. |

## Out-of-plan items (deferred to post-Jack-Adams)

- Mouse / touch input — keyboard only
- User-defined layouts (drag-resize panes)
- Persistent command history across sessions
- Streaming LLM responses (single-request only in v1)
- Multi-turn LLM conversations
- AI proactive suggestions (LLM only fires on parse-failure)
- Per-screen AI specialization (one provider serves all
  commands in v1)
