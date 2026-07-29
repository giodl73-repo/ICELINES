# IceLines Showcase

**Who this is for:** someone you would hand the repo to for 15–40 minutes —
a **fantasy manager / hockey analyst** who wants explainable depth and query
without a SaaS account, or a **systems implementer** wiring CLI/TUI/Web through
one domain spine.

**Posture:** local NHL analytics product + research lab. Public NHL/MoneyPuck
sources only. Not an official NHL product, not a betting edge guarantee, not
a claim that every surface is parity-complete, and not a standalone/no-FLETCH
build yet. Governing baseline: [`docs/vtrace/`](docs/vtrace/).

| Audience | Open this first | Time |
|---|---|---|
| Fantasy manager / analyst | [Analyst brief](docs/show/analyst-brief.md) | 15–25 min |
| Systems implementer | [Implementer brief](docs/show/implementer-brief.md) | 20–40 min |
| Either, hands-on | [Getting started](docs/guides/00-getting-started.md) or release binary | 5–15 min |

## One-minute pitch

IceLines is a **single Rust binary** that turns public NHL data into depth
charts, pace-adjusted rankings, a deterministic query grammar, fantasy tools,
and multi-surface views (CLI, TUI workbench, Web). History is first-class:
bundled seasons work offline immediately; fuller history installs from release
bundles. Product language is **The Rink** (Center Ice, Crease, Bench, …) with
**The Insider** as the evidence-aware voice — commands stay stable underneath.

```text
PUBLIC NHL / optional MoneyPuck
        ↓
   icelines-fetch  (snapshots, cache, provenance)
        ↓
   icelines-core   (StatsRepository, ViewModels, scoring)
        ↓
   icelines-query  (one Art Ross intent grammar)
        ↓
   CLI · TUI · Web · exports   (layout only; meaning upstream)
```

## Two doors

### A. Fantasy manager / hockey analyst path

**Question IceLines answers well:** *Who is producing, who is buried, what does
pace say, and how do fantasy category gaps look — without a black-box site?*

| Step | What to look at | Why |
|---|---|---|
| 1 | [Public site](https://giodl73-repo.github.io/ICELINES/) | Zero-install tour |
| 2 | [Analyst brief](docs/show/analyst-brief.md) | Surfaces + claim boundaries |
| 3 | README download path or `icelines tui` | Interactive workbench |
| 4 | [Fantasy guide](docs/guides/03-fantasy.md) / `icelines stathead` | Category gaps, poach, schemes |
| 5 | Optional: team cards under `docs/teams/` | Narrative examples |

**Analyst takeaways:**

- Offline-first start: bundled seasons answer rank/query/team without fetch.
- One product grammar across CLI and TUI (`gaps`, `poach`, simulate-style moves).
- IceCast and sealed showcases are **scenario/evidence demos**, not injury truth.
- Cap/window organization work is planned under VTRACE — check posture before citing as shipped product.

**Do not say:** official NHL stats product, guaranteed fantasy win, betting
advice, or that projections are “true talent.”

### B. Systems implementer path

**Question IceLines answers well:** *Where does meaning live, how do surfaces
stay honest, and what must not land in a renderer?*

| Step | What to look at | Why |
|---|---|---|
| 1 | [Implementer brief](docs/show/implementer-brief.md) | Crate map + invariants |
| 2 | [CODEBASE.md](CODEBASE.md) | Where to write code |
| 3 | [`docs/vtrace/ARCHITECTURE.md`](docs/vtrace/ARCHITECTURE.md) | Current vs target posture |
| 4 | `icelines-cli/src/commands/` + `tui/` | Already modular command/TUI map |
| 5 | `pwsh scripts/test-slice.ps1 list` | Cheap verification slices |

**Implementer takeaways:**

- **Seven invariants** (one spine, one query intent, ViewModel boundary, per-source honesty, surface parity by artifact, cache-as-evidence, explicit lean/standalone targets).
- Fat files still exist (`cli.rs`, `commands/fantasy.rs`, `icecast.rs`, TUI screens) — prefer peeling *within* domain modules over re-flattening into `main`.
- FLETCH/SLICE seams exist; **do not claim standalone** until seams are gone.
- Persona/system tests under `icelines-cli/tests/` are the contract fence.

## Fastest hands-on (both audiences)

```powershell
# from repo root after cargo build --release -p icelines-cli
# or use a GitHub release binary
icelines --version
icelines rank --top 10
icelines team EDM
icelines query leaders --pos C --age-max 23 --sort ppg --top 10
icelines menu
icelines tui
```

Fresh data (network):

```powershell
icelines fetch all
```

## Claim packet (this showcase)

| Field | Value |
|---|---|
| Claim text | IceLines can be shown as a local multi-surface NHL analytics platform with separate analyst and implementer entry paths. |
| Audience | Fantasy managers / hockey analysts; systems implementers. |
| Evidence | README + guides; public site; VTRACE architecture; crate/command layout; test slices. |
| Validation | Documentation + existing CLI/workspace tests; not external fantasy-league certification. |
| Limitations | Surface parity and named-layout persistence still gated; standalone/lean builds are targets; some marketing docs lag VTRACE current/target tables. |
| Non-claims | Official NHL endorsement; betting edge; complete historical authority without installed bundles; IceCast injury/start truth. |
| Review lane | HART / TAPE / GLASS / WIRE (product/data/UI/API roles in `.roles/`). |

## Where not to start

| Avoid leading with… | Why |
|---|---|
| Full `cargo test --workspace` | Long gate; use `scripts/test-slice.ps1` first |
| IceCast injury scenarios as facts | Stress demos, not medical/roster truth |
| `cli.rs` as the mental model | Generated/long clap surface; start at commands + core |
| Standalone/no-dependency claims | Explicitly deferred in VTRACE architecture |

## Related

- Command reference: [`COMMANDS.md`](COMMANDS.md)
- Brand: [`design/specs/brand-the-rink.md`](design/specs/brand-the-rink.md)
- VTRACE index: [`docs/vtrace/`](docs/vtrace/)
- Family applied-systems hub: sibling Infrastructure 2.0 repos are different products; IceLines is the hockey applied system.
