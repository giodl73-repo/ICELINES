# Backfill Inventory

Wave: `backcheck-the-phases`
Date: 2026-05-13
Status: Pulse 01 inventory

## Scope read

This inventory was built from the active wave packet, Pulse 01, the governing
role lenses (`bench`, `forge`, `keel`, `tape`, `wire`), and these durable repo
sources:

- `design/phases.md`
- `design/plans/`
- `design/specs/surface-parity.md`
- `design/ARCHITECTURE.md`
- `design/INVARIANTS.md`
- `design/PITFALLS.md`
- `README.md`
- `COMMANDS.md`

No residual below depends on chat history. Each `pulse` item maps to an explicit
pulse number, owner surface, likely files or discovery scope, and gates in that
pulse plan.

## Role lens summary

| Role | Inventory rule applied |
|---|---|
| bench | Residuals that remain user-visible or regression-prone get test gates, not prose-only followups. |
| forge | Pulse boundaries stay implementable: no pulse mixes product code, docs truth, visual capture, and scenario classification. |
| keel | Every cross-surface gap is mapped through ViewModels or the surface parity matrix before a renderer gets work. |
| tape | Data/source gaps are named as warnings, fixtures, or deferred inputs; no pulse asks agents to invent unavailable data. |
| wire | Web/JSON/admin/mutation gaps distinguish read routes from POST-backed mutation intents and forbid GET-backed mutations. |

## Pulse map

| Pulse | Owner surface | Residual covered | Status |
|---|---|---|---|
| 02 | Web dashboard | Jack Adams Web dashboard continuity and workspace preservation. | done |
| 03 | Report/export | Presidents Trophy `TeamSeasonView` markdown/report export parity. | planned |
| 04 | Visual evidence | Prince visual/CREST regression captures for TUI, web, CLI, reports. | planned |
| 05 | Scenario harness | Persona/scenario corpus classification and next test-conversion batch. | planned |
| 06 | TUI/watch UX | Selke watch-rule editing and richer watchlist UX residuals. | planned |
| 07 | Web/admin operations | Ted Lindsay/Jim Gregory operational admin and persistent config/report-toggle partials. | planned |
| 08 | Career/docs parity | Calder career cohort dedicated TUI/report affordance plus generated docs/spec-site verification. | planned |

## Phase-by-phase residual inventory

| Phase in `design/phases.md` | Current implemented state from plans/docs | Residual item | Status | Pulse / owner |
|---|---|---|---|---|
| 1 - Foundation / Calder | Initial Rust workspace, fetch path, team/rank, bundled data are historical shipped work. | No active residual found in current parity/docs; legacy CSV-era pitfalls remain historical until translated to current `StatsRepository` terms. | done | n/a |
| 2 - Site / Lady Byng | mkdocs site and generated team pages now render through ViewModel-backed site/export paths. | Old proof/dashboard integration references are obsolete after proof cancellation. | delete | Deleted from active backlog; keep only as historical note in plans index. |
| Lady Byng TUI experiences | Per-surface TUI launchers, menu loop, and drill-down launchers are documented in README/COMMANDS. | No active residual found. | done | n/a |
| 3 - Projections / Art Ross | Projection/leaderboard surfaces were superseded by Lindsay catalog and Art Ross query rewrite. | No separate projection residual found beyond current query/report surfaces. | done | n/a |
| 4 - History polish / King Clancy | Multi-season history and data hygiene are now represented through bundled seasons and player-card career loading. | Stale web-dashboard claims were already recorded as spec drift; no active product residual here. | done | n/a |
| 5 - Query engine / Bill Masterton | Legacy query leaders/player/compare work is superseded by the Art Ross unified query pipeline. | No active residual found. | done | n/a |
| 6 - Export and dashboards / Mark Messier | `export md` shapes are now marked done in surface parity; old dashboard-engine/proof path was cut. | Retired dashboard-engine/proof integration should stay deleted, not revived. | delete | n/a |
| 7 - TUI v2 redesign / Jack Adams | TUI tabs, admin overlay, season travel, and later MDI dashboard are implemented/documented. | No active residual separate from later Jack Adams MDI/Web pulses. | done | n/a |
| 8 - Spec delta + chunks / Norris | Spec catch-up and chunked snapshot work are documented; architecture states source-specific fallback chains. | Dedicated simultaneous chunked+legacy L1 fallback test remains a future cleanup, not user-facing. | defer | Future data hardening backlog. |
| Phase 8h - Chunked snapshots / Norris | Chunked snapshot/object-store design is shipped and documented. | Uniform installed-bundle fallback for bios/stats is a known architecture asymmetry. | defer | Future data architecture pulse, not this wave's surface backfill. |
| Goalies / Vezina | Goalie repository, 38-season bundled data, CLI/TUI/web parity, and goalie filter rewrites are documented. | No active residual found. | done | n/a |
| Transactions / Selke | ESPN transactions source, classifier, bundled transactions, CLI/TUI/web parity are documented. | No active residual found outside admin/source hardening already covered by operational partials. | done | n/a |
| Edge speed / Maurice Richard | Parked because no public NHL Edge JSON endpoint is known. | NHL Edge skating speed remains externally blocked. | defer | No pulse until a public fixture/source exists. |
| Hart normalization | `StatsRepository`, `PlayerView`, `(player_id, season, type)` key axis, and loader contracts are implemented. | Hart.5c contract-helper followup is not surfaced as an active user gap in current parity docs. | done | n/a |
| Calder multi-league career | Career history store, player-card development arc, web/CLI career views, and Art Ross league atoms are implemented. | Dedicated TUI cohort board/report affordance remains partial. | pulse | Pulse 08 / Career/docs parity |
| Foster favorites/time/data layer | Favorites, date axes, DataStore, EventStream, sync engine, capability matrix, and windowed atoms are implemented. | `shifts=off` remains intentionally locked; no live residual without a source/fixture. | done | n/a |
| Conn Smythe live playoff tracking | Series momentum, playoff leaders, and live game detail route are implemented. | No active residual found. | done | n/a |
| Art Ross query rewrite | Unified parser/planner/executor, sliding windows, EVER/AT, cross-league atoms, and `--explain` are implemented and documented. | No active residual found. | done | n/a |
| Norris TUI architecture refactor | Per-screen state structs extracted and tested. | No active residual; deeper control-flow work moved to Masterton. | done | n/a |
| Masterton screen factoring | Chrome, Screen trait scaffold, and standalone TUI mode are implemented. | Deep per-screen Screen trait migration remains intentionally deferred scaffolding. | defer | Future internal refactor only. |
| Jack Adams TUI MDI dashboard | Default MDI dashboard, command bar, panes, grammar, and optional AI fallback are documented. | No active residual found separate from Jack Adams Web continuity. | done | n/a |
| Jennings stabilization/truth | Plans index says implemented; release/test baseline restored. | `design/phases.md` still labels Jennings as planned; treat as documentation truth drift. | defer | Fold into later phases-doc truth cleanup if this wave expands. |
| Campbell platform/ViewModels | Platform contracts and typed ViewModels back major surfaces. | Remaining mutation-contract wiring appears only as surface partials now owned by Ted/Jim/Selke pulses. | done | n/a |
| Selke fantasy poacher | PoachScore, watch rules, reports, CLI/TUI/web/markdown/JSON surfaces are implemented. | Full TUI watch-rule editing and richer arbitrary-rule editing remain carry-forward. | pulse | Pulse 06 / TUI-watch UX |
| Messier TUI filter/sort consistency | Roster filters, keybinds, and cmdbar KV grammar are implemented. | CLI/web parity carry-forwards were closed by Lester/Ted; no active Messier residual. | done | n/a |
| Lester Patrick CLI parity | Schedule, playoffs, transactions, docs overlay, and docs refresh are implemented. | No active residual found. | done | n/a |
| Ted Lindsay web parity | Route inventory, handler split, major HTML/JSON parity, ViewModel/envelope migration are implemented with tracked partials. | Richer career/favorites/watch UX, admin controls, and generated docs/spec-site verification remain partial. | pulse | Pulses 06, 07, 08 |
| Prince of Wales visual system | Semantic tokens, representative TUI/web/CLI polish, and CREST closeout landed. | Broader visual capture/golden evidence remains worth backfilling. | pulse | Pulse 04 / Visual evidence |
| Jim Gregory release hardening | CI/release smoke/checklist/current-season rollover/artifact verification are closed. | Destructive web install/remove and full persistent web config/report-toggle controls remain deferred operational partials. | pulse | Pulse 07 / Web-admin operations |
| Presidents Trophy team season | `TeamSeasonView`, CLI/TUI/web/dashboard parity, standings/SOS/quality ledger are implemented. | Markdown/report export parity for team season remains explicit carry-forward. | pulse | Pulse 03 / Report/export |
| Jack Adams Web dashboard | Browser dashboard shell, workspace routing, command form, responsive panes, and captures are implemented. | Dashboard continuity residual is already closed by Pulse 02. | done | Pulse 02 / Web dashboard |

## Surface parity residuals

| Matrix row | Residual | Status | Pulse / owner |
|---|---|---|---|
| Team season performance | Report/export artifact missing for `TeamSeasonView`. | pulse | Pulse 03 / report-export |
| Career/cohort leaders | Dedicated richer TUI board/report affordance remains partial. | pulse | Pulse 08 / career-docs |
| Favorites/groups | Richer favorites UX remains partial but shares `FavoritesView` and mutation intents. | pulse | Pulse 06 / watch/favorites UX |
| Fantasy league management | Main dashboard mutations are intentionally deferred; read/product views are done. | defer | Not a current pulse unless mutation scope is explicitly reopened. |
| Watch rules | TUI arbitrary-rule editing and richer rule UX remain partial. | pulse | Pulse 06 / watch UX |
| Data install/list/remove | Web destructive install/remove remains deferred; verify/status are present. | pulse | Pulse 07 / admin operations |
| Snapshot operations | Web activate/delete are present for safe inactive snapshots; keep backend guards. | done | n/a |
| Config/report toggles | Runtime web config exists; full persistent CLI config/report-toggle web UI remains planned. | pulse | Pulse 07 / admin operations |
| Static docs/spec site | Docs route exists; generated docs/spec-site verification remains a named partial. | pulse | Pulse 08 / career-docs |

## Deferred or deleted list

| Item | Decision | Reason |
|---|---|---|
| NHL Edge skating speed / Maurice Richard | defer | No public JSON endpoint or fixture; stop condition forbids network-only tests. |
| Proof dashboard integration / TUI proof renderer | delete | Plans index marks proof integration cancelled; current product uses IceLines-native docs/site surfaces. |
| Old Phase S season-type plan | delete | Folded into Hart's `(season, season_type)` primary key axis. |
| Dashboard-engine proof path | delete | Explicitly cut; no longer part of shipped architecture. |
| Deep Masterton Screen trait migration | defer | Scaffold exists; future internal refactor, no current user-facing gap. |
| Uniform installed-bundle fallback for bios/stats | defer | Real architecture asymmetry, but outside this surface-backfill wave. |
| Legacy CSV-era PITFALLS entries | defer | Preserve as institutional memory; translate to current `StatsRepository`/ViewModel terms before opening implementation pulses. |

## Pulse readiness checklist

- Pulse 03 owns report/export parity only and stops if `TeamSeasonView` lacks a
  required field.
- Pulse 04 owns visual capture evidence only and can document browser tooling
  unavailability instead of expanding product scope.
- Pulse 05 owns scenario classification only; scenario-to-test implementation is
  a later pulse unless a tiny fixture classification needs a proof slice.
- Pulse 06 owns watch/favorites UX residuals and must not introduce GET-backed
  mutations.
- Pulse 07 owns admin/config operational partials and must keep destructive
  actions POST-backed and guarded.
- Pulse 08 owns career/docs parity and must not invent career data that is not in
  `CareerView` or the local career-history store.
