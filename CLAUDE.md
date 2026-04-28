# IceLines — Claude Code Context

**Project**: IceLines NHL analytics + fantasy platform  
**Binary**: `icelines` (Rust CLI)  
**Repo root**: `C:/src/icelines/`  
**Working dir: `C:/src/icelines/` (workspace root)

---

## Crate ownership — where to write code

| What you're adding | Crate | Why |
|--------------------|-------|-----|
| Data types, Player struct, filters, scheme scoring, projections | `icelines-core` | Pure logic, no I/O, no network |
| NHL API fetch, snapshot store, bundled data, MoneyPuck, aggregate | `icelines-fetch` | All I/O and data loading |
| Markdown site generation, mkdocs templates | `icelines-site` | Site-only concerns |
| CLI commands, argument parsing, TUI, HTTP server | `icelines-cli` | Thin UI layer only |

**Rule**: Business logic belongs in `icelines-core` or `icelines-fetch`, never in `icelines-cli`. CLI commands call library functions — they don't compute anything themselves.

**Crate dependency chain** (lower can't import higher):
```
icelines-core   (no internal deps)
icelines-fetch  (depends on icelines-core)
icelines-site   (depends on icelines-core, icelines-fetch)
icelines-cli    (depends on all three)
```

---

## Common commands

```bash
# Build
cargo build                          # debug
cargo build --release -p icelines-cli  # release binary

# Test
cargo test                           # all crates
cargo test -p icelines-core          # one crate
cargo clippy -- -D warnings          # must be clean
cargo fmt --check                    # must be clean

# Run (after release build)
target/release/icelines.exe query leaders --pos C --top 10
target/release/icelines.exe fantasy league-create "My League"

# proof — documentation linting and guide compilation
# proof binary lives in the workspace target at C:\src\target\debug\proof.exe
# Build proof first if needed: cd C:/src && cargo build
C:/src/target/debug/proof check .                  # lint all markdown
C:/src/target/debug/proof check . --errors-only    # errors only
bash scripts/build-guides.sh                       # compile src/guides/ → docs/guides/
bash scripts/build-guides.sh --check               # validate without writing
```

---

## Key constants and files

- **Current season**: `icelines_core::CURRENT_SEASON = 20_252_026` — change here each October, nowhere else
- **Bundled data**: `src/icelines-fetch/src/bundled.rs` — 5 seasons embedded via `include_bytes!()`
- **Player loading**: always use `PlayerRepository::new(store, season).load_all()` — never reach into snapshot store directly from a command
- **Snapshot store**: `~/.icelines/snapshots/` — never hardcode paths, use `Config::load()?.snapshot_dir()`
- **SQLite DB**: `~/.icelines/icelines.db` — shared by GroupDb and FantasyDb

---

## Testing tiers

| Tier | Location | Rule |
|------|----------|------|
| L0 unit | `#[cfg(test)]` inside each `.rs` file | Pure logic, no I/O, microseconds |
| L1 integration | `src/icelines-fetch/tests/` | Real structs, no network, tempdir |
| L2 system | `src/icelines-cli/tests/system_tests.rs` | Invokes compiled binary as subprocess |

**Every new feature needs at least L0 tests. New commands need L2 tests.**

The mock NHL API fixture is at `src/icelines-fetch/tests/mock_nhl_api.rs` — use `httpmock` there, not in L0 tests.

---

## Architecture rules

1. **No live network calls in tests** — all L1/L2 tests use bundled data or httpmock
2. **No season literals** — use `CURRENT_SEASON` / `CURRENT_SEASON_STR`, not `"20252026"`
3. **Dedup players by nhl_id** — the NHL bios API emits multiple rows for traded players; `PlayerRepository` deduplicates but stay alert
4. **Option<T> for all nullable API fields** — `shooting_pct`, `toi_per_game_sec`, `faceoff_win_pct` etc. are null in real data
5. **MoneyPuck is silo'd** — all MoneyPuck code lives in `icelines-fetch/src/moneypuck.rs`; removing it only requires deleting that file and the Option fields on Player
6. **CLI commands are async** — all `run()` functions are `async fn` dispatched by `tokio::main`

---

## What's been built

- `icelines fetch` — NHL API data pipeline (bios, stats, realtime, rosters, contracts)
- `icelines query leaders/player/compare` — full query engine (30+ sort metrics, --seasons N, --sort improvement, percentiles, JSON/CSV export)
- `icelines fantasy` — full fantasy league (SQLite, scoring, trades, axum HTTP server)
- `icelines rank/team/players/history/project/scouting/mates/peers/compare/class`
- `icelines tui` — ratatui interactive dashboard (8 screens)
- `icelines build/serve/deploy` — mkdocs static site
- 338 tests across L0/L1/L2 including mock NHL API fixture

## Pending (see design/plans/INDEX.md)
- NHL Edge skating speed stats
- `icelines export md` — markdown data tables for proof/mdpath integration
- Fantasy daily delta scoring
- Historical season queries (`--season 20242025`)
- DASHBOARD-SPEC integration (waiting on proof)

---

## Roles

Eight domain roles in `.roles/` review from different angles:
- **scout** — player analysis correctness
- **tape** — data pipeline integrity  
- **forge** — Rust code quality and safety
- **edge** — query engine and filter logic
- **bench** — test coverage and quality
- **glass** — TUI and rendering
- **pace** — performance and algorithmic efficiency
- **wire** — API contracts and schema evolution

Run `/review-specs` to invoke all roles on a spec or implementation.

---

## Proof/mdpath integration (future)

When `proof` DASHBOARD-SPEC is ready:
- `icelines export md` generates stats tables at `~/.icelines/reports/`
- Each TUI screen becomes a `.dashboard.source.md` template
- `proof compile --width N --height N` renders the template
- TUI renders the compiled ASCII string
- See `design/specs/dashboard-engine.md` and memory at `C:/Users/giodl/.claude/projects/C--src-NHL/proof_integration.md`
