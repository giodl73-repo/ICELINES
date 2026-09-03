# IceLines — NHL Analytics Platform

IceLines is an offline-first NHL analytics application built in Rust. It combines
depth charts, pace-adjusted rankings, historical comparisons, fantasy tools, and
explainable season forecasts in one binary with CLI, TUI, and web interfaces.

Thirty-eight seasons—from 1987-88 through 2025-26, excluding the 2004-05
lockout—ship with the application. Core queries work without an account,
database, or initial data download.

[Live site](https://giodl73-repo.github.io/ICELINES/) ·
[Latest release](https://github.com/giodl73-repo/ICELINES/releases/latest) ·
[Showcase](SHOWCASE.md) ·
[Getting started](docs/guides/00-getting-started.md) ·
[Command reference](COMMANDS.md) ·
[Contributing](CONTRIBUTING.md)

> IceLines is an independent analytics project. It is not affiliated with or
> endorsed by the NHL, and forecasts are evidence-backed scenarios rather than
> guarantees.

## What you can do

- Explore current and historical player rankings with composable filters.
- Inspect team depth charts, line combinations, and player deployment.
- Compare career arcs and find historical player similarities.
- Evaluate fantasy category gaps, waiver options, schedules, and roster moves.
- Run deterministic IceCast scenarios with explicit evidence and uncertainty.
- Use the same domain models through terminal reports, an interactive TUI, or a
  local web dashboard.

IceLines calls this product layout **The Rink**: Center Ice for the league, the
Red Line for offense, the Blue Line for defense, the Crease for goalies, the
Bench for fantasy decisions, the Penalty Box for constraints, and the Goal Line
for possible outcomes. Stable command names remain the underlying interface.

## Try it

Download the archive for your platform from the
[latest release](https://github.com/giodl73-repo/ICELINES/releases/latest),
extract the single `icelines` or `icelines.exe` binary, and run:

```bash
icelines rank --top 10
icelines query leaders --pos C --age-max 23 --sort ppg --top 15
icelines query player "Connor McDavid" --percentiles
icelines query compare "Wayne Gretzky" "Mario Lemieux" --seasons 38
icelines team EDM
icelines tui
```

Bundled history works immediately. Fetching is optional when you want current
rosters, schedules, injuries, or other live data:

```bash
icelines fetch all
icelines tui scores
icelines tui goalies
icelines serve
```

If you do not know which surface to open, use `icelines menu`. For curated query
recipes, use `icelines stathead`. The complete offline reference is available
through `icelines docs`.

## Build from source

You need Git and the stable Rust toolchain. The first build also resolves the
workspace's pinned Git dependencies.

```bash
git clone https://github.com/giodl73-repo/ICELINES.git
cd ICELINES
cargo build --release
./target/release/icelines --help
```

On Windows, run `target\release\icelines.exe --help` from PowerShell.

## Choose your path

| Goal | Start here |
|---|---|
| Tour the product without installing it | [Public site](https://giodl73-repo.github.io/ICELINES/) |
| Evaluate IceLines as a hockey analyst | [Analyst brief](docs/show/analyst-brief.md) |
| Understand the implementation | [Implementer brief](docs/show/implementer-brief.md) |
| Learn the CLI progressively | [Getting-started guide](docs/guides/00-getting-started.md) |
| Find every command and option | [Command reference](COMMANDS.md) |
| Decide where code belongs | [Codebase guide](CODEBASE.md) |
| Understand requirements and verification | [VTRACE baseline](docs/vtrace/) |

Focused guides cover [queries](docs/guides/01-query.md),
[team depth](docs/guides/02-team-depth.md),
[fantasy workflows](docs/guides/03-fantasy.md),
[data](docs/guides/04-data.md),
[historical comparisons](docs/guides/05-comps-history.md), and the
[TUI](docs/guides/06-tui.md).

## Data and evidence

| Source | Purpose | Access |
|---|---|---|
| Bundled IceLines data | 38 historical seasons | Included; no setup |
| NHL public API | Current stats, rosters, bios, schedules, and game data | `icelines fetch all` |
| MoneyPuck | Optional public advanced metrics | `icelines fetch money-puck` |
| CapWages | Optional licensed contract data | API key required |
| Local contract CSV | User-maintained contract overlay with provenance | Local file |
| GitHub Releases | Optional season refresh bundles | `icelines data install` |

IceLines keeps source state, provenance, missing-data status, and forecast
assumptions visible. A missing source does not silently become an empty or
zero-valued result.

## Architecture

```text
NHL API · bundled history · optional sources
                    │
             icelines-sources
                    │
              icelines-fetch
                    │
               icelines-core
                    │
              icelines-query
                    │
          CLI · TUI · Web · exports
```

| Crate | Responsibility |
|---|---|
| `icelines-core` | Domain types, scoring, projections, and shared view models |
| `icelines-query` | Query grammar, planning, and execution |
| `icelines-sources` | Typed source contracts and source-state boundaries |
| `icelines-fetch` | API clients, snapshots, caches, and bundled-data loading |
| `icelines-cli` | CLI commands and Ratatui application |
| `icelines-web` | Axum HTML and JSON surfaces |
| `icelines-site` | Static-site generation support |

Business meaning lives below the presentation layers so CLI, TUI, web, and
exports can share the same contracts. See [CODEBASE.md](CODEBASE.md) before
adding a module and [the platform contracts](design/specs/platform-contracts.md)
before changing cross-surface behavior.

IceLines is intentionally the NHL specialist in a small portfolio. Generic
selector/fold-plan work belongs in SLICE, and generic fetch/cacheline mechanics
belong in FLETCH; IceLines consumes pinned contracts from those projects.

## Development

Run the focused slice nearest your change while iterating:

```powershell
pwsh scripts/test-slice.ps1 list
pwsh scripts/test-slice.ps1 quick
pwsh scripts/test-slice.ps1 workspace-check
```

Before submitting a cross-crate or shared-contract change, run:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pwsh scripts/release-smoke.ps1
```

Read [CONTRIBUTING.md](CONTRIBUTING.md) for repository boundaries and review
expectations. In particular, tests and examples should prefer synthetic teams
and players; personal fantasy data and team-specific what-if notebooks belong
outside this reusable product repository.

## Project references

- [CHANGELOG.md](CHANGELOG.md) — release and compatibility history
- [COMMANDS.md](COMMANDS.md) — exhaustive command reference
- [CODEBASE.md](CODEBASE.md) — ownership and code-placement guide
- [SECURITY.md](SECURITY.md) — vulnerability reporting
- [design/release-checklist.md](design/release-checklist.md) — release procedure
- [docs/vtrace/](docs/vtrace/) — governing specification and verification baseline

## License

Software and ordinary software documentation are MIT-licensed. Original
non-software content is CC BY-NC 4.0. Third-party and source data retain their
source-specific rights. See [LICENSE](LICENSE) for the complete terms.
