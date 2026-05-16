# TUI Admin Overlay — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Implemented (basic) · v2 features Draft
**Extends**: `tui-v2.md` (six-tab nav, season picker)

---

## Purpose

A modal overlay (triggered by **`F`**) that exposes administrative
operations — install status, fetch hints, season management — without
crowding the main tab bar. Replaces the v1 "Fetch+Install" tab.

---

## Trigger and dismissal

| Key | Action |
|-----|--------|
| `F` (capital, with shift) | Toggle the overlay open/closed |
| `Esc` | Close the overlay |
| `q` / `Ctrl+C` | Quit the application (overlay or not) |

Lowercase `f` is reserved for the **add-to-Favorites** instant-add
key on player-list screens (see `group-management.md`). The capital
`F` is intentional so neither shadows the other.

While the overlay is open, all other keys (number jumps, `/` search,
`Tab` cycle, etc.) are inert — the overlay consumes input.

---

## Layout

A centered popup (~44 cols × 50% height), bordered, yellow title:

```
Admin — Esc to close

  Admin commands (run in terminal):

    icelines fetch all
      -> refresh all NHL data

    icelines data list
      -> show installed seasons

    icelines data install 20032004
      -> install a historical season

  -----------------------------------------
    No install in progress.

    Esc to close
```

The bottom block is the **install status line** — driven by
`InstallState::phase()` from `tui/loader.rs`:

| Phase | Status line |
|-------|-------------|
| `Idle` | `No install in progress.` |
| `Downloading(season)` | `Installing {season}…` (cyan) |
| `Done(season, kb)` | `✓ {season} installed ({kb} KB)` (green) |
| `Error(season, msg)` | `✗ Failed: {msg}` (red) |

The line updates live at ~10 Hz as the install progresses. Closing
the overlay does **not** cancel the install — it continues in the
background and the status bar at the bottom of the main TUI keeps
ticking the spinner.

---

## v1 vs v2 features

### v1 (shipped)

The overlay is a **read-only menu**: it lists CLI commands the user
can run in their terminal, plus the live install status. There is
no in-overlay action — pressing keys other than `Esc` / `q` does
nothing.

### v2 (planned, draft)

Two additions are designed for v2:

1. **In-overlay actions** — selectable rows that trigger background
   commands. Cursor `↑↓`, `Enter` to invoke:

   ```
   ▶ Install latest season         (icelines data install)
     Refresh active snapshot       (icelines fetch all)
     List installed seasons
     Verify active snapshot integrity
   ```

   Long-running tasks (fetch, install) spawn background tokio tasks
   exactly like the season-picker `i` key already does.

2. **`:` command prompt** — colon enters a command-line mode at the
   bottom of the screen, accepting any `icelines` subcommand:

   ```
   :data install 19931994█
   ```

   The prompt parses with the same `clap` Command tree used by the
   binary, runs the command in-process (no subprocess), and pipes
   stdout into a scrollable result pane.

   This is a power-user feature deferred until the v1 menu proves
   stickier than expected.

---

## State

```rust
pub show_admin: bool,                        // App field
pub install_state: crate::tui::loader::InstallState,
```

`install_state` is shared with the season picker; both surfaces show
the same install progress.

---

## Interaction with the season picker

The season picker (`y` key, see `season-timetravel.md`) and the admin
overlay are **independent modals**. They cannot both be open at once;
opening one closes the other implicitly through the App state machine
(only one `show_*` flag is checked per render).

A common workflow:
1. Press `y` → see seasons, notice 19931994 is `[not installed]`.
2. Press `i` → install starts in the background.
3. Press `Esc` → close picker.
4. Press `F` → see install progress in the admin overlay.
5. When `✓ 19931994 installed` appears, press `Esc`, then `y` again
   to load the new season.

---

## Decisions (Open Questions resolved)

1. **Capital `F`, not lowercase `f`**: lowercase is the
   add-to-Favorites instant key on player-list screens. The overlay
   needs a key that doesn't conflict on any tab.

2. **Read-only menu in v1**: Keeps the overlay's surface minimal.
   The CLI is the canonical interface for admin tasks; the overlay
   is documentation + status, not a second control plane.

3. **No background-task list view**: There's only one install slot
   in v1 (`spawn_install` rejects concurrent installs). When v2 adds
   parallel tasks, a "running tasks" panel will appear.

4. **No log scrollback**: The status line shows the latest event
   only. Full logs (fetch warnings, etc.) go to stderr and are not
   surfaced in the TUI in v1.

5. **`:` command prompt deferred**: Powerful but adds a parsing
   surface and edge cases. Defer until users ask.

---

## Test coverage

The overlay is implemented (toggle on `Char('F')` at `app.rs`,
`show_admin` field, `render_admin` in `screens/misc.rs`) and now has dedicated
L0 coverage.

Covered in `tui/app.rs::tests`:
- `l0_admin_overlay_opens_on_capital_f_key` — capital `F` opens the overlay.
- `l0_admin_overlay_closes_on_esc` — `Esc` closes it.
- `l0_admin_overlay_blocks_other_keys` — keys such as `Tab` are no-ops while
  open.
- `l0_admin_overlay_does_not_open_on_lowercase_f` — lowercase `f` remains
  separate from the admin overlay.
- `l0_admin_overlay_capital_f_key_toggles_off` — capital `F` toggles an open
  overlay closed.

Covered in render tests:
- `l0_render_admin_idle_phase_shows_no_install`
- `l0_render_admin_downloading_phase_shows_spinner`
- `l0_render_admin_error_phase_shows_red`
- `l0_render_admin_done_phase_shows_check_and_size`
- `l0_app_admin_overlay_renders_when_show_admin_is_true`
- `l0_tui_scenario_admin_overlay_opens_and_closes`
- `l0_admin_overlay_title_style_is_yellow`

---

## Future work (v2+)

| Item | Priority | Why deferred |
|------|----------|--------------|
| In-overlay action menu | HIGH | Adds real value — single-key admin |
| `:` command prompt | MED | Power user; complex to parse |
| Multi-task panel | MED | Wait for v2 parallel installs |
| Log scrollback pane | LOW | stderr is fine for now |
| Cancel button on running install | MED | Tokio task cancel; needs care |
| Snapshot operations menu | MED | Pair with snapshot-operations.md |
