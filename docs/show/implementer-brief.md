# Implementer brief — IceLines

**Timebox:** 20–40 minutes. **Goal:** know the crate spine, surface rules, and
where *not* to put new logic.

## Crate map

| Crate | Owns | Must not own |
|---|---|---|
| `icelines-core` | Domain types, repository views, ViewModels, scoring | Network, files, clap |
| `icelines-fetch` | NHL/MoneyPuck I/O, snapshots, cache, provenance | Fantasy product rules |
| `icelines-query` | Art Ross grammar → plan | Renderer layout |
| `icelines-cli` | clap surface, commands, TUI, process boundary | Silent hockey meaning in widgets |
| `icelines-web` | HTTP/HTML/JSON handlers | Duplicate domain math |
| `icelines-site` | Deferred static site member | Active product claims |

Read [`CODEBASE.md`](../../CODEBASE.md) before adding files.

## Seven invariants (short form)

From [`docs/vtrace/ARCHITECTURE.md`](../vtrace/ARCHITECTURE.md):

1. One domain spine through `StatsRepository` / `PlayerView` / ViewModels.
2. One query intent lowered through `icelines-query`.
3. Renderers choose layout, not hockey meaning.
4. Per-source honesty (no silent zeroes for missing sources).
5. Surface parity by shared artifact, not pixel twin.
6. Analytics cache is evidence layer for future coach/scout surfaces.
7. Standalone and lean-CLI builds are **targets**, not current claims.

## CLI layout reality

Already modular under `icelines-cli/src/commands/*` and `tui/*`. Hot spots for
future peels (do not re-monofile):

| Hotspot | ~shape | Prefer |
|---|---|---|
| `cli.rs` | large clap tree | keep generated/grouped; thin `main` dispatch |
| `commands/fantasy.rs` | large domain | submodules by league/trade/sim |
| `commands/icecast.rs` | large domain | scenario/report submodules |
| `commands/export.rs` / `query.rs` | large | format/intent splits |
| `tui/screens/*` | large screens | stay screen-local; shared chrome in `tui/` |

Doctrine: **new work lands in the owning domain module**, not as a new 2k-line
arm in `main.rs`.

## Verification rhythm

```powershell
pwsh scripts/test-slice.ps1 list
pwsh scripts/test-slice.ps1 quick
pwsh scripts/test-slice.ps1 cli-matrix
# before shared-contract commits:
pwsh scripts/test-slice.ps1 full
```

Persona and system tests under `icelines-cli/tests/` encode Foster/Art Ross
capability fences — extend those when you change cross-surface contracts.

## Dependency posture

FLETCH (and related) seams may still appear for cache/ledger experiments.
**Do not document IceLines as FLETCH-free/standalone** until VTRACE
`REQ-DEP-001` evidence lands. Product parsing and activation stay IceLines-owned
even when fetch ledger helpers are shared.

## Good first implementer tasks

- Add a command thin-wrapper that reuses an existing ViewModel.
- Extend a filter in `icelines-query` with tests.
- Fix a parity fixture where CLI and Web envelopes diverge.
- Peel one oversized command file into `commands/<domain>/mod.rs` children.

## Bad first implementer tasks

- Invent a second scoring meaning inside a TUI widget.
- Claim full surface parity without Gate evidence.
- Rewrite clap into a new framework for taste alone.
