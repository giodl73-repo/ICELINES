# Phase Lady Byng — Roles review (2026-05-05)

Captured pre-implementation review of `tui-experiences.md` v0.2 + plan v0.2 by five role agents (GLASS, FORGE, BENCH, WIRE, EDGE). Findings folded into spec v0.3 + plan v0.3. This note exists for the audit trail — if a future session wants to know why the plan grew an LB.0.5 milestone, this is the record.

## Severity classification (5 BLOCKING / 8 HIGH / 9 MEDIUM / 6 LOW)

### BLOCKING — folded into v0.3 as resolved decisions or new milestone

| # | Issue | Role | Resolution in v0.3 |
|---|-------|------|--------------------|
| 1 | Terminal restoration is manual cleanup, not RAII; panic in surface N corrupts menu re-render for surface N+1 | FORGE | New milestone **LB.0.5** introduces `TerminalGuard` with `Drop` impl. BLOCKING prerequisite for LB.4 (menu loop). |
| 2 | `resolve_player_id_by_name` returns `Option<PlayerId>` — silently picks first "Smith" match (Sebastian Aho problem) | EDGE, WIRE | Spec § "Resolution semantics" declares: 0 matches → `Did you mean`; 1 match → resolve; >1 → list all candidates with team+season, exit non-zero before TUI boots. |
| 3 | Menu's "Ctrl-C exits 0" promise is unenforceable without signal handler (Unix exits 130, Windows aborts) | EDGE | LB.4 installs `ctrlc::set_handler` flipping AtomicBool; checked after `read_line`. Documented in `--help`. |
| 4 | Web option `W` panics on `AddrInUse` because `axum::Server::bind(..).unwrap()` | EDGE | LB.4 catches bind error; prints "port 8000 in use — visit http://localhost:8000 if it's already an icelines server"; returns to loop. |
| 5 | Menu test pipe `1\nQ\n` is fundamentally broken — TUI inherits stdin via crossterm raw events and consumes `Q` before menu's `read_line` sees it | BENCH | Spec test strategy switched to: mock `MenuLauncher` trait + unit-test dispatch table; integration-test via manual smoke in LB.7. |

### HIGH — folded into v0.3 spec/plan

| # | Issue | Role | Resolution |
|---|-------|------|-----------|
| 6 | `--render-once` debug flag is wrong default — repo already has `render_app_to_text` harness at `tui/screens/mod.rs:236-298` | BENCH | Spec test strategy uses in-process harness as primary; `--render-once` removed from plan. |
| 7 | Plan misses `Commands::Dashboard` (also calls `run_tui(false)` at `main.rs:364`) and persona test waves 2/3/4 | FORGE, EDGE | LB.1 audit list expanded to: main.rs + persona_scenarios.rs + waves 2/3/4. |
| 8 | clap option (b) nested subcommands preferred over (a) positional — invalid states unrepresentable | FORGE | Spec/plan commit to (b). `TuiCommand` enum with one variant per surface. |
| 9 | `run_tui(no_color, start_screen)` will grow more params; introduce struct now | FORGE | Spec uses `RunTuiOpts { no_color, start_screen }`. Forward-compat for `locked`, `start_season`. |
| 10 | `parse_start_slug` valid-slug list is hand-maintained (drift risk) | WIRE, BENCH | Spec introduces `SLUG_TABLE: &[(&str, ScreenSpec, Stability)]` — single source of truth driving parser, error formatter, `--help`, and a COMMANDS.md drift fence. |
| 11 | Slug stability tier missing — once aliases ship in `--help`, scripts pin to whichever they grep first | WIRE | Spec declares Canonical vs Alias; canonical removal requires one-release WARN cycle; aliases hidden from error suggestions. |
| 12 | Lazy-fan-out flash: `tui player Gretzky` paints first frame with empty career table — looks like Gretzky has zero NHL history | GLASS, EDGE | Spec § "First-frame contract for drill-downs" requires explicit `Loading career…` placeholder. L0 render test asserts placeholder appears for pids not in active repo. |
| 13 | Menu chrome looks like 1990s `dialog` — bare `println!` after colorful TUI is jarring; ConPTY artifact buildup on Windows between dispatches | GLASS | Spec uses `owo-colors` for menu accent; plan calls `clear_screen` between dispatches (`crossterm::terminal::Clear(ClearType::All)`). |

### MEDIUM — noted in spec, address in implementation

14. Cold-entry orientation: confirm nav bar still drawn; add Esc footer hint *(GLASS)* → spec § "App-state changes" requires nav bar + footer hint.
15. `<slug>:<arg>` grammar — declare exactly one colon, arg opaque *(WIRE)* → spec § "<slug>:<arg> grammar" declares this.
16. Hardcoded serve port 8000 → add `[menu]` config section now *(WIRE)* → spec § "[menu] config section" reserves it.
17. Fixture coupling on Bedard *(BENCH)* → frozen fixtures: Gretzky / McDavid pid / Brodeur / EDM.
18. `--render-once` + network surfaces (Scores/Transactions/Schedule) *(EDGE)* → header-only smokes for v1; full render gated on harness-level network mock (deferred to Future/parked).
19. Empty/whitespace `<arg>` *(EDGE)* → reject at parse time before `normalize_name` strips to empty.
20. Trailing whitespace on team abbrev *(EDGE)* → trim before validation.
21. Playoffs cold-launch cliff (historical season + live framing) *(GLASS)* → noted; full handling deferred (in-app `y` covers it).
22. Surface `y` discoverability tip in Playoffs footer *(EDGE)* → covered by Esc-footer hint mechanism.

### LOW / future

23. No JSON twin for `icelines menu` *(WIRE)* → Future/parked.
24. 1×1 TTY guard on `--render-once` *(EDGE)* → Future/parked; `--render-once` removed anyway.
25. Verify `BUNDLED_SEASONS` iteration order is newest-first *(EDGE)* → audit task added to LB.3.
26. Apostrophe / hyphen quoting tests *(EDGE)* → covered as L0 cases in LB.3.
27. 6 negative-path L0 tests *(BENCH)* → LB.3 + LB.5 cover them via the SLUG_TABLE-driven parse tests.
28. **Tiebreaker check**: per `.roles/ROLE.md` FORGE concerns (#1 RAII) outrank GLASS chrome (#13). LB.0.5 ships first.

## Net effect on plan

- New milestone **LB.0.5** (TerminalGuard RAII).
- Net 13 explicit "v0.3 resolved decisions" added to spec.
- Estimated effort bumped from "1–2 sessions" to "2–3 sessions" — primarily because of LB.0.5's panic-restore test plumbing and the SLUG_TABLE/RunTuiOpts refactors in LB.1.
- Test surface grew from "8+4+2" to "10 in-process render smokes + 4 L2 dispatch smokes + ~30 L0 grammar tests + 1 panic-restore test" — counterintuitively LESS subprocess surface than v0.2 because the in-process harness already exists.

## Files updated

- `design/specs/tui-experiences.md` → v0.3
- `design/plans/2026-05-05-phaseLadyByng-tui-experiences.md` → v0.3
- `design/notes/2026-05-05-phaseLadyByng-roles-review.md` → this note (new)

## Roles consulted

- GLASS (Visualization & UX) — `.roles/glass.md`
- FORGE (Rust Engineer) — `.roles/forge.md`
- BENCH (Test Engineer) — `.roles/bench.md`
- WIRE (API & Schema Pipeline) — `.roles/wire.md`
- EDGE (Edge Case Specialist) — `.roles/edge.md`

Skipped (low relevance for a CLI surface phase): SCOUT, TAPE, HART, KEEL, PACE, BROADCAST, KEEL.
