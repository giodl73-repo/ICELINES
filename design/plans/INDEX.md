# IceLines Plans Index

Plans track work items from idea to completion.
**Status**: Draft → Active → Implemented → Closed

For the system architecture, see [`../ARCHITECTURE.md`](../ARCHITECTURE.md).
For the app plan (mission, surfaces, non-goals), see [`../IceLines.md`](../IceLines.md).
For the final forward-roadmap role review, see
[`../notes/2026-05-09-forward-roadmap-roles-review.md`](../notes/2026-05-09-forward-roadmap-roles-review.md).

---

## Current Forward Roadmap

Jennings restored the measured baseline, Campbell established the shared
ViewModel contract path, the Messier/Lester/Ted/Selke parity wave moved the
major product surfaces onto that path, Prince closed the shared visual-system
pass, and Jim Gregory closed release hardening. The next forward work is now the
explicitly partial UX/admin/doc surfaces tracked in
`design/specs/surface-parity.md`, plus any new feature phase we choose from the
backlog.

| Plan | Status | Summary |
|------|--------|---------|
| [Phase Jennings - Stabilization + Truth](2026-05-09-phaseJennings-stabilization-truth.md) | Implemented | Restored the measured baseline, split CI/test gates into runnable areas, and made follow-on phases depend on explicit local/CI checks. |
| Jennings measured baseline | Recorded | `cargo check --workspace` PASS; `cargo test --workspace --no-fail-fast` PASS; test inventory 4620 `: test` entries at the original Jennings measurement; later CI slices split the gates for faster failure. |
| [Phase Campbell - Platform contracts and ViewModels](2026-05-09-phaseCampbell-platform-viewmodels.md) | Closed | Shared typed ViewModels now back the major CLI/TUI/web/site/report product surfaces; the final closeout routed generated team pages and `tonight trade` through ViewModel contracts. Role review: [`../notes/2026-05-09-campbell-specs-roles-review.md`](../notes/2026-05-09-campbell-specs-roles-review.md). |
| [Phase Selke - Fantasy poacher](2026-05-09-phaseSelke-fantasy-poacher.md) | Implemented | Builds the fantasy poacher: PoachScore, watch rules, reports, streamers/stashes/category specialists, and CLI/TUI/web/markdown/JSON surfaces. Carry-forward: full TUI rule editor. |
| [Phase Messier - TUI filter/sort consistency](2026-05-08-phaseMessier-roster-filters.md) | Implemented | Standardized TUI player-list filter/sort keybinds and cmdbar kv grammar through the shared contract path. Carry-forward: CLI parity in Lester Patrick, web parity in Ted Lindsay, visual polish in Prince of Wales. |
| [Phase Lester Patrick - CLI parity](2026-05-05-phaseLesterPatrick-cli-parity.md) | Implemented | Closed CLI gaps for schedule/playoffs/transactions and in-TUI docs using the post-Messier command vocabulary. Carry-forward: Ted Lindsay verifies web parity. |
| [Phase Ted Lindsay - Web parity](2026-05-09-phaseTedLindsay-web-parity.md) | Implemented with tracked partials | Split the web handler monolith, established the route inventory, normalized major HTML/JSON routes onto ViewModels/envelopes, and left explicit partials for richer UX/admin/docs verification. |
| [Phase Prince of Wales - ASPECT visual system](2026-05-09-phasePrinceOfWales-visual-system.md) | Implemented | Closed the visual-system phase: shared semantic visual tokens, representative TUI scan-rhythm tests, web route layout classes, 80-column CLI fences, and CREST closeout review. Carry-forward: screenshot automation and secondary-route cleanup in Jim Gregory/polish slices. |
| [Phase Jim Gregory - Release hardening](2026-05-09-phaseJimGregory-release-hardening.md) | Closed | Release checklist, current-season rollover procedure and test fence, corrected data freshness docs, optimized binary smoke gate, release artifact verification, and CI path coverage for release-hardening files. |
| [Phase Jack Adams Web - browser dashboard and command surface](2026-05-12-phaseJackAdamsWeb-web-mdi.md) | Implemented | Brings the TUI Jack Adams concepts to the browser: scores ribbon, Favorites/Watchlist and Schedule context panes, swappable workspace, command palette, responsive drawer behavior, and ViewModel-backed panels without a full SPA rewrite. `/` stays the lightweight home preview; `/dashboard` is the explicit command center. |
| [Phase Presidents Trophy - Team season performance](2026-05-12-phasePresidentsTrophy-team-season.md) | Implemented | Adds a distinct team season-performance surface separate from roster/depth: standings context, playoff distance, home/away splits, strength of schedule, quality wins/bad losses, form, remaining schedule pressure, and CLI/TUI/web/dashboard parity through `TeamSeasonView`. Carry-forward: markdown/report export. |
| [Phase Hart — Normalization](2026-04-30-phaseHart-normalization.md) | Implemented | (player_id, season, season_type) primary-key axis. Sub-phases 4.1/5b/5c/6 all shipped 2026-04-30 → 2026-05-02. |
| [Hart.5c — Final Cleanup](2026-05-01-phaseHart-5c-final-cleanup.md) | Implemented (v0.4) | Consumer migration to PlayerView. 5c.0–5c.7.12 shipped 2026-05-01. One followup (F1: contract helpers on PlayerView) tracked in plan. |
| [Hart.5c.6 — TUI Restructure](2026-05-01-phaseHart-5c-6-tui-restructure.md) | Implemented | App owns StatsRepository; TUI screens migrated; user-flow + boot-load tests landed 2026-05-01. |
| [Hart.6 — Playoff Per-Player Data](2026-05-01-phaseHart-6-playoff-data.md) | Implemented (v0.2) | Schema + API client + snapshot tier paths + loader dispatch + fetch CLI + bundled data (current 5 + 33 historical) + cross-team guards. Shipped 2026-05-02. |
| [Phase Lindsay — Stat Catalog](2026-05-02-phaseLindsay-stat-catalog.md) | **COMPLETE** (v0.4) | Centralized `StatId` dispatch + 108 selectable stats + `--filter` grammar + categorized TUI Queries + sort picker + career-table presets + catalog-keyed `--sort` / `--columns` / similarity / depth scoring / fantasy frozen-golden / axum roster JSON + site stat-name grep fence + Tier-2 fetch + cross-product bundle test + cross-ref docs. L.1–L.4 shipped 2026-05-02; L.5 / L.5b / L.6 / L.7 / L.8 shipped 2026-05-03. 200+ Lindsay-prefixed tests, 1275 workspace tests passing. L.3 stdout-golden fence GREEN through every commit since L.3.0 capture. |
| [Phase Lady Byng — TUI experiences](2026-05-05-phaseLadyByng-tui-experiences.md) | Implemented | Per-surface TUI launchers are live: `tui --start <slug>` (nav tabs + drill-down player/team/goalie/comps), `tui goalies` / `tui player Bedard` sugar, and the `icelines menu` looping picker. Spec: `../specs/tui-experiences.md`. Roles review note: `../notes/2026-05-05-phaseLadyByng-roles-review.md`. Trophy reuse — see `design/phases.md`. |

---

## Implemented

| Plan | Completed | Trophy | Summary |
|------|-----------|--------|---------|
| [Phase 1 — Rust CLI Foundation](2026-04-25-rust-cli-foundation.md) | 2026-04-25 | Calder | 4-crate workspace, fetch, team, rank, bundled data |
| [Phase 2 — Site & Analysis](2026-04-25-phase2-site-analysis.md) | 2026-04-25 | Lady Byng | mkdocs site, scheme engine, snapshot store, players command |
| [Phase 3 — TUI & Projections](2026-04-25-phase3-tui-projections.md) | 2026-04-25 | Art Ross | ratatui TUI, projections, career history, scouting |
| [Phase 4 — Data, History & Polish](2026-04-26-phase4-data-history-polish.md) | 2026-04-26 | King Clancy | Multi-season bundled data, L0/L1/L2 test coverage, CI |
| [Phase 5 — Query Engine](2026-04-26-phase5-query-engine.md) | 2026-04-27 | Bill Masterton | `query leaders/player/compare`, --seasons N, improvement sort, 30+ metrics |
| [Phase 6 — Export & Dashboards](2026-04-26-phase6-export-dashboard.md) | 2026-04-29 (partial) | Mark Messier | `export md` 5/7 shapes shipped; `dashboard-engine` CUT (proof cancelled) |
| [Phase 7 — TUI v2 Redesign](2026-04-28-phase7-tui-v2-redesign.md) | 2026-04-28 | Jack Adams | 6/7-tab nav, season time-travel, Scores/Schedule/Playoffs (7a–7e) |
| [Phase 8 — Spec Delta + Chunks](2026-04-28-spec-delta-catchup.md) | 2026-04-29 | Norris | Spec catch-up, snapshot operations, multi-season query support |
| [Phase 8h — Chunked Snapshots](2026-04-29-phase8h-chunked-snapshots.md) | 2026-04-29 | Norris | Content-addressed object store; 10× storage savings on daily snapshots |
| [Phase Vezina — Goalies](../specs/goalies.md) | 2026-04-29 | Vezina | Goalie type, repository, fantasy scoring, 38-season bundled goalie data |
| [Phase Selke — Transactions](2026-04-30-phaseT-transactions.md) | 2026-04-30 | Selke | ESPN site.api source, classifier, TUI tab, modern-era bundled transactions |
| [Phase Hart.4.1 — Test Foundation](2026-04-30-phaseHart-4-1-test-foundation.md) | 2026-04-30 | Hart | StatsRepository LRU + roster sum proptest; fixture builders |
| [Phase Hart.5a/5b — Repo + Adapters](2026-04-30-phaseHart-normalization.md) | 2026-04-30 | Hart | StatsRepository load path; flat_view_legacy adapter; 6+ consumer migrations |
| Hart.5c.0–5c.5 — Consumer migration (PlayerFilter through export.rs) | 2026-05-01 | Hart | See `2026-05-01-phaseHart-5c-final-cleanup.md` |

---

## Folded / Cancelled

| Plan | Status | Note |
|------|--------|------|
| Phase Presidents — Season Type | **Folded into Hart** | (season, type) becomes a primary key axis after Hart |
| [Phase S — Season Type (5-day)](2026-04-30-phaseS-season-type.md) | Superseded by Hart | Original 5-day plan; replaced 2026-04-30 by full normalization |
| proof / mdpath / DASHBOARD-SPEC integration | **Cancelled 2026-05-01** | Out of scope; specs in `dashboard-engine.md` and refs in `export-markdown.md` are stale |

---

## Backlog (v1.1+ — not yet planned)

Each item below has no plan file yet. Star ratings indicate user value; "blocked on"
column tells us why it isn't shipping yet.

### Tier 2 — small unblocks (data path or wiring exists)

| Item | Value | Notes |
|------|-------|-------|
| _None currently active_ | — | Clear the Unblocks moved stale test rows to "cleared" and parked historical shift bundles until a source/capability contract exists. |

Cleared in Clear the Unblocks:
- `headshot.rs` test coverage — focused L0 coverage exists for braille encoding,
  cache markers, clone sharing, and disk-cache behavior.
- `tui-admin-overlay` test coverage — focused L0 coverage exists for key handling,
  render phases, integration rendering, and overlay title styling.

### Tier 3 — future features

| Item | Value | Blocked on |
|------|-------|------------|
| NHL Edge skating speed (Phase Maurice Richard) | ⭐⭐ | **Parked** — no public JSON endpoint discovered |
| Strength-state 5v5/PP/PK splits (`query-engine` Phase 5C) | ⭐⭐ | Tier 3 shifts + play-by-play join |
| Tier 4 advanced metrics (NST, Evolving Hockey RAPM) | ⭐ | External scraping; large lift |
| Fantasy head-to-head matchup weekly | ⭐⭐ | Active in Match the Week: new local `fl_matchups` table + cached daily-delta schedule walker |
| Yahoo league CSV roster import | ⭐⭐ | Nothing — backlog |
| MoneyPuck historical xG (multi-season) | ⭐ | Nothing — backlog |
| Historical shift-data bundles | ⭐⭐ | Parked — no supported `fetch shifts` command, fixtures, or source/bundle policy; `sync.capabilities.shifts=off` is enforced |
| Fantasy roster shape enforcement | ⭐ | Per-scheme rules in TOML |
| CI: cargo fmt + cargo audit gates | ⭐⭐ | Nothing — backlog |

Cleared in Score the Day:
- Fantasy daily delta scoring — shipped via `FantasyDailyDeltaView`,
  `icelines fantasy daily --date`, and `/api/v1/fantasy/daily?date=...`
  from cached finalized boxscores and local FantasyDb rosters.

### Cancelled (referenced in specs but won't ship)

| Item | Why |
|------|-----|
| proof DASHBOARD-SPEC integration | proof project cut 2026-05-01 |
| TUI as proof dashboard renderer | depends on proof |
| Tier 5 — social signals (Reddit) | scope creep; never implemented |
| Tier 6 — beat media RSS line rushes | scope creep; never implemented |
| Per-player site pages | site explosion; not in scope per `site-generation.md` |
