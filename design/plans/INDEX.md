# IceLines Plans Index

Plans track work items from idea to completion.
**Status**: Draft → Active → Implemented → Closed

For the system architecture, see [`../ARCHITECTURE.md`](../ARCHITECTURE.md).
For the app plan (mission, surfaces, non-goals), see [`../IceLines.md`](../IceLines.md).

---

## Active

| Plan | Status | Summary |
|------|--------|---------|
| [Phase Hart — Normalization](2026-04-30-phaseHart-normalization.md) | Implemented | (player_id, season, season_type) primary-key axis. Sub-phases 4.1/5b/5c/6 all shipped 2026-04-30 → 2026-05-02. |
| [Hart.5c — Final Cleanup](2026-05-01-phaseHart-5c-final-cleanup.md) | Implemented (v0.4) | Consumer migration to PlayerView. 5c.0–5c.7.12 shipped 2026-05-01. One followup (F1: contract helpers on PlayerView) tracked in plan. |
| [Hart.5c.6 — TUI Restructure](2026-05-01-phaseHart-5c-6-tui-restructure.md) | Implemented | App owns StatsRepository; TUI screens migrated; user-flow + boot-load tests landed 2026-05-01. |
| [Hart.6 — Playoff Per-Player Data](2026-05-01-phaseHart-6-playoff-data.md) | Implemented (v0.2) | Schema + API client + snapshot tier paths + loader dispatch + fetch CLI + bundled data (current 5 + 33 historical) + cross-team guards. Shipped 2026-05-02. |
| [Phase Lindsay — Stat Catalog](2026-05-02-phaseLindsay-stat-catalog.md) | L.1+L.2 Implemented (v0.4) | Centralized `StatId` dispatch + 107 selectable stats + `--filter` grammar + categorized TUI Queries + career table + `fetch report --kind`. L.1 + L.2 shipped 2026-05-02 (Tier-1 schema + ChunkedManifest v=2 + StatId catalog + read dispatch + filter grammar + ExtraReports cache + goalie bios merge; +110 Lindsay-prefixed tests, 1184 workspace tests passing). L.3-L.8 pending. |

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
| [Phase Vezina — Goalies](../specs/goalies.md) | 2026-04-29 | Vezina | Goalie type, repository, fantasy scoring (deferred), 5 bundled seasons |
| [Phase Selke — Transactions](2026-04-30-phaseT-transactions.md) | 2026-04-30 | Selke | ESPN site.api source, classifier, TUI tab, 5 bundled seasons |
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
| Goalie fantasy scoring (`fantasy team-add` + `score_team`) | ⭐⭐⭐ | Spec `fantasy-leagues.md` defers; `goalie-stats.json` is bundled (Vezina); `to_goalie_scheme_stats_view` exists post-5c.4. ~1 day to wire `RosterEntry::Goalie`. |
| `export md fantasy` shape | ⭐⭐ | Spec defers; plumbing exists post-5c.4. ~1 day. |
| `export md series` shape | ⭐⭐ | Spec defers; `playoffs.json` exists. ~1 day. |
| Bundle shift data for historical seasons | ⭐ | Required for `mates`/`peers` against historical seasons. ~5 MB/season. |
| `headshot.rs` test coverage | ⭐ | Spec calls it out. Manual smoke only today. |
| `tui-admin-overlay` test coverage | ⭐ | Spec calls it out. Needs 5c.6 snapshot harness. |

### Tier 3 — future features

| Item | Value | Blocked on |
|------|-------|------------|
| NHL Edge skating speed (Phase Maurice Richard) | ⭐⭐ | **Parked** — no public JSON endpoint discovered |
| Strength-state 5v5/PP/PK splits (`query-engine` Phase 5C) | ⭐⭐ | Tier 3 shifts + play-by-play join |
| Tier 4 advanced metrics (NST, Evolving Hockey RAPM) | ⭐ | External scraping; large lift |
| Fantasy daily delta scoring | ⭐⭐ | Nothing — backlog |
| Fantasy head-to-head matchup weekly | ⭐⭐ | New `fl_matchups` table + schedule walker |
| Yahoo league CSV roster import | ⭐⭐ | Nothing — backlog |
| MoneyPuck historical xG (multi-season) | ⭐ | Nothing — backlog |
| Fantasy roster shape enforcement | ⭐ | Per-scheme rules in TOML |
| CI: cargo fmt + cargo audit gates | ⭐⭐ | Nothing — backlog |

### Cancelled (referenced in specs but won't ship)

| Item | Why |
|------|-----|
| proof DASHBOARD-SPEC integration | proof project cut 2026-05-01 |
| TUI as proof dashboard renderer | depends on proof |
| Tier 5 — social signals (Reddit) | scope creep; never implemented |
| Tier 6 — beat media RSS line rushes | scope creep; never implemented |
| Per-player site pages | site explosion; not in scope per `site-generation.md` |
