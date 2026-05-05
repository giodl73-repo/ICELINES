# Phase Lester Patrick — CLI parity for live-data surfaces

**Date**: 2026-05-05
**Status**: Stub — scheduled after Phase Lady Byng
**Trophy**: Lester Patrick — for outstanding service to ice hockey. Fits: filling the surface gaps so every analytical feature is reachable from every surface (CLI / TUI / web). This is service work, not headline work — the user gets parity, not a new capability.
**Predecessor**: Phase Lady Byng (`2026-05-05-phaseLadyByng-tui-experiences.md`).
**Dependencies**: none — all data sources required already exist (NHL API for schedule + scores, ESPN for transactions, MoneyPuck/bundled for playoffs).

---

## Why

Per `design/IceLines.md` § "Feature × surface portfolio", three surfaces have CLI gaps after Phase Lady Byng lands:

| Feature | CLI today | TUI | Web |
|---|---|---|---|
| **Schedule** | ❌ | ✅ `tui schedule` | ✅ `/schedule` |
| **Playoffs** | ❌ | ✅ `tui playoffs` | ✅ `/playoffs` |
| **Transactions** | ❌ | ✅ `tui transactions` | ✅ `/transactions` |
| **Docs (in-TUI)** | ✅ `docs` | ❌ | ✅ `/docs` |

This phase closes them. Each subcommand mirrors the existing `icelines tonight` pattern — simple printer driven by the same data the TUI/web already use.

Why this matters: scripting / cron / CI / external piping. A user who wants `icelines transactions --team EDM > today.txt` can't get there today without parsing the TUI screen-scrape or the web HTML. The CLI is the scriptable surface; gaps in it leak through to "I had to write a wrapper script."

## Surface coverage declared

```
Surface coverage:  CLI ✅ (this phase)   TUI ✅ (Lady Byng)   Web ✅ (King Clancy, shipped v0.13.0)
```

---

## Milestones

### LP.1 — `icelines schedule [team flags]`

Mirrors `icelines tonight`. Prints today's games or a date range. Reuses the schedule fetcher already wired into the TUI's Schedule tab.

**Deliverable**:
- New `Commands::Schedule` variant + `commands/schedule.rs` module.
- Flags: `--team ABBR`, `--days N` (default 7, 1-14 range), `--season YYYYZZZZ` (default current).
- Output: comfy-table with date, away/home, score (if final), status (pre / live / final).
- `--json` / `--csv` flags for scripting.

**Tests**: L0 + L2 mirror the `tonight` pattern. Fixtures: bundled regular-season schedule for 20242025 (deterministic, no network).

### LP.2 — `icelines playoffs [round | series flags]`

Prints the bracket as text. Same data the TUI Playoffs tab + `/playoffs` web page consume.

**Deliverable**:
- New `Commands::Playoffs` variant + `commands/playoffs.rs` module.
- Flags: `--season YYYYZZZZ` (default current — defaults to the most recent completed playoff if current season isn't yet in playoffs), `--round N` (1-4, default all), `--series LETTER` (A-H plus conference finals + finals).
- Output: per-round series with team abbrevs, series record, last game outcome.
- `--json` / `--csv` flags for scripting.

**Tests**: L0 with bundled playoffs from a prior completed season; L2 system smoke.

### LP.3 — `icelines transactions [filters]`

Prints the league-wide moves feed. Same ESPN-sourced feed the TUI Transactions tab + `/transactions` use.

**Deliverable**:
- New `Commands::Transactions` variant + `commands/transactions.rs` module (the existing `commands/transactions.rs` is dead post-Selke — verify, reuse if appropriate, replace if not).
- Flags: `--team ABBR`, `--days N` (default 7), `--type {trade,signing,waiver,recall,assign,injury,...}` (multi-valued), `--player NAME`.
- Output: comfy-table with date, type, team(s), player, summary.
- `--json` / `--csv` flags for scripting.
- Cache the ESPN feed in `~/.icelines/cache/transactions/` with a 30-minute TTL (matches the TUI's polling rate).

**Tests**: L0 with the ESPN fixture in `icelines-fetch/tests/fixtures/`; L2 system smoke.

### LP.4 — In-TUI docs overlay

Adds a `?` (or other) keybind to open COMMANDS.md as a scrollable in-app overlay. The TUI gap in the portfolio matrix.

**Deliverable**:
- New `Screen::Docs` variant (or render as overlay over current screen — choose at impl time based on what fits ratatui's pattern).
- Markdown rendered to ratatui spans (use `pulldown-cmark` to parse, render as styled paragraphs / tables).
- Scrollable with `↑`/`↓`, jump-to-section with `g <letter>`, exit with `Esc`/`?`.
- Same source-of-truth as `icelines docs` (compile-time `include_str!("../../COMMANDS.md")`).

**Tests**: render harness smoke — opens overlay, asserts a known heading appears, scrolls, closes.

### LP.5 — Docs refresh

- `COMMANDS.md` — new sections for `icelines schedule` / `playoffs` / `transactions` with examples.
- Update the "Surface coverage" matrix in `design/IceLines.md` — flip the four ❌ to ✅.
- README primer — show one example per new command.

### LP.6 — Hands-on persona pass

One paragraph each: do the new CLI commands feel like `tonight`'s sibling? Does the in-TUI docs overlay scroll smoothly? File follow-ups for any rough edges.

---

## Out of scope

- **Fantasy on web.** Fantasy stays single-user local-only per the IceLines.md non-goals. If a future phase reverses that, it's its own track.
- **Live-game websocket / push.** `tonight` polls; `schedule` doesn't need real-time. Same here.
- **Cap math / contract value modeling.** Not in scope per IceLines.md non-goals.

---

## Estimated effort

1–2 sessions. Each new CLI subcommand is ~150-200 lines + tests; the docs overlay is the largest single deliverable (~300-400 lines including markdown→ratatui conversion). Total maybe 1500 lines of code + tests.

## Workspace verification

Standard per-milestone exit:
1. `cargo build --workspace` clean.
2. `cargo test -p icelines-cli` clean.
3. `cargo clippy -p icelines-cli -- -D warnings` clean.
4. `cargo fmt --check` clean.

Phase exit additionally walks all four new commands manually + verifies the IceLines.md portfolio matrix shows ✅ across the row.

## Naming conventions

Commit subjects: `Phase Lester Patrick: LP.X — <slug>`. No tag for the phase if it lands inside a v0.X.x release; phase exit may bump version per usual semver.
