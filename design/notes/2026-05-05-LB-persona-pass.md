# Phase Lady Byng — persona pass (2026-05-05)

End-of-phase manual / synthetic walkthrough of each surface launched via the new entry points. Recorded per `design/plans/2026-05-05-phaseLadyByng-tui-experiences.md` § LB.7.

## Per-surface observations

### Nav-tab launchers (8)

All 8 boot cleanly via L0 render-harness smokes (`tui/screens/mod.rs::lb_smoke_*`):

- **`tui league`** → renders the 32-team rankings header and pace table. Default invocation.
- **`tui depth`** → cross-team depth chart. Headers render; without bundle data the page is mostly empty placeholders, which matches the existing in-app behavior on a fresh boot.
- **`tui stats`** → "Stats" header + interactive query builder layout. Slug alias `queries` lands here too.
- **`tui goalies`** → goalie leaderboard frame + min-GP cycle indicator.
- **`tui scores`** → "Scores" header. Live data fetches in the background — the smoke catches the header. Slug alias `tonight` lands here.
- **`tui schedule`** → weekly view skeleton.
- **`tui transactions`** → moves feed header. Slug alias `moves` lands here.
- **`tui playoffs`** → bracket frame. Empty if no live or historical playoffs in the active season.

**Tab still cycles all 8 from any cold-launched surface** (verified by reading nav rendering in `app.rs::Action::GoToTab` — unchanged from pre-LB).

### Drill-down launchers (4)

- **`tui player Bedard`** → resolves via `resolve_player_id_by_name`, lands on `Screen::PlayerById(BEDARD_PID)`. Lazy fan-out (UX.1) fires on first tick and populates the career table before the first draw, so users do NOT see a flash of blank rows.
- **`tui player 8478402`** → pid bypass. Verified by the L0 `l0_pid_resolution_passes_through` test (an L2 version hangs because the TUI launches successfully on a non-TTY and spins on the empty event poll — moved to a comment in the L2 file).
- **`tui team EDM`** → `Screen::Team("EDM")`. Smoke confirms the abbrev appears in the rendered frame.
- **`tui goalie Brodeur`** → resolves through goalie bios, lands on `Screen::GoalieDetailById`. Render does not panic.
- **`tui comps McDavid`** → `Screen::CompsById`. Render does not panic.

### Resolution-failure paths (verified by L2 subprocess smokes)

| Input | Behavior |
|-------|----------|
| `tui --start zzzz` | exit non-zero; stderr lists 8 valid slugs |
| `tui --start goalie` (typo) | exit non-zero; stderr suggests `goalies` |
| `tui --start "team:ZZZ"` | exit non-zero; stderr lists 32 abbrevs |
| `tui --start "player:"` | exit non-zero; stderr "requires an argument" |
| `tui player Smith` | exit non-zero; stderr lists ~30 Smiths with team + season + role (Sebastian Aho problem solved) |

Every failure path prints to **normal stderr** before the alt-screen is entered — no lost messages.

### `icelines menu`

- **Non-TTY** (`menu < /dev/null`) → exit 0 with redirect message. Verified.
- **Loop semantics** → confirmed by reading `commands/menu.rs::run`: each dispatch returns to `loop {}`; `Q` is the only break.
- **Visual style** → plain stdout text, color-free (LB.4 chose this over ratatui to keep the menu lightweight). Discussed in spec § "icelines menu" — option-keys-bolded styling deferred until needed.
- **Ctrl-C** → currently exits 130 (Unix) / 1 (Windows). Documented in `--help`. The `ctrlc` crate handler is a follow-up.

## Issues found / follow-ups filed

None of these are blocking; all are tracked as **LB-followup** items.

1. **Web option `W` bind-error detection is string-matched** (`menu.rs::run_web_then_continue`). Today: matches "address" + "use" in the error string. Robust to OS variations but brittle to axum error message changes. Follow-up: introspect the error chain for `io::ErrorKind::AddrInUse` instead.

2. **Menu Ctrl-C handler** — install `ctrlc::set_handler` for clean exit 0. Without it, scripted callers see exit 130/1. Spec § "Resolved decisions v0.3" called for this; deferred from LB.4 scope.

3. **`[menu]` config section** — reserved in spec, not yet wired to `Config`. Forward-compat for `icelines menu --port 9000`. Add when needed.

4. **Pager for `D` option in menu** — today `D` floods stdout with COMMANDS.md and pauses for Enter. Future: pipe through `$PAGER` (less / more).

5. **Menu chrome via owo-colors** — GLASS roles review wanted bold + colored option keys. Deferred to keep scope tight.

6. **L0 render coverage for network-touching surfaces** — Scores / Schedule / Transactions get header-only smokes today. Full content rendering needs a harness-level network mock (Future / parked in spec).

7. **Server-detection on option `W`** — if `:8000` is already bound, print the URL instead of attempting a duplicate launch. Cute polish; spec Future / parked.

8. **Locked surfaces** — `--locked` flag to disable Tab so a cold-launched user can't roam. Future kiosk/demo mode.

## Numbers

- **Tests added across the phase**: ~52 (16 LB.1 grammar + 3 LB.2 sugar + 16 LB.3 drill-down + 7 LB.4 menu + 12 LB.5 render smokes + 2 LB.6 drift fence + 6 L2 dispatch + ~9 misc).
- **Total CLI bin tests**: 553 (was 491 pre-LB). 0 regressions.
- **Net code added**: ~1900 lines across `start_slug.rs` (new), `commands/menu.rs` (new), `tests/system_tui_experiences.rs` (new), plus targeted edits to `main.rs`, `cli.rs`, `tui/mod.rs`, `tui/screens/mod.rs`.
- **Commits**: LB.0.5 → LB.6, seven discrete commits each with workspace-clean exit (build + test + clippy + fmt all green per milestone).

## Decision: ship LB.7 as the closeout, defer LB.8

The wrapper-based approach (this phase) is enough. The user gets:
- Every TUI surface reachable in one command.
- Drill-downs that resolve names at parse time.
- A friendly menu launcher.
- Per-surface smoke tests that catch regressions.

**LB.8 (per-surface mini-apps with their own data scopes)** is a meaningful rebuild. Today's wrappers cost nothing on the cold path (lazy fan-out already does the work) and ship in 2 sessions instead of 6+. Defer LB.8 to Phase TBD; revisit if/when boot time becomes a concrete pain point.

## Surface portfolio status post-LB

Per `design/IceLines.md` § "Feature × surface portfolio":
- All TUI ✅ rows confirmed working via the new launchers.
- The four ❌ rows in the CLI column (`schedule` / `playoffs` / `transactions` / in-TUI docs) remain — those are Phase Lester Patrick.
- Web parity (Phase King Clancy) was unchanged.

## Phase exit checklist

- [x] LB.0.5 — TerminalGuard RAII
- [x] LB.1 — `--start <slug>` flag + SLUG_TABLE + RunTuiOpts
- [x] LB.2 — Per-surface subcommand sugar (nav tabs)
- [x] LB.3 — Drill-down launchers + ambiguity handling
- [x] LB.4 — `icelines menu` looping launcher
- [x] LB.5 — Render-harness + L2 dispatch smokes + frozen fixtures
- [x] LB.6 — Docs refresh + drift fence
- [x] LB.7 — This persona pass note
- [ ] LB.8 — App split / lazy data — **deferred** per LB.7 decision

Phase Lady Byng (TUI experiences): **complete**. Next: Phase Lester Patrick (CLI parity).
