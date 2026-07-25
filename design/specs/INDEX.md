# IceLines Specs Index

Specs define what we're building. Each spec covers one feature or screen.

For the system architecture (how features fit together), see [`../ARCHITECTURE.md`](../ARCHITECTURE.md).
For the app plan (mission, surfaces, where we're going), see [`../IceLines.md`](../IceLines.md).
For active work and backlog, see [`../plans/INDEX.md`](../plans/INDEX.md).

**Status legend:**
- `Implemented` — matches the running build
- `Implemented (partial)` — core is built; spec notes specific gaps
- `Draft` — design is complete; not yet built
- `Cancelled` — explicitly out of scope; will not ship

---

## Core Engine Specs

| Spec | Status | Summary |
|------|--------|---------|
| [rust-cli.md](rust-cli.md) | Implemented | 4-crate workspace, CLI commands, fetch pipeline. Yahoo CSV references stale post-Phase-1. |
| [data-sources.md](data-sources.md) | Implemented (partial) | Tiers 0-3 implemented (rosters / Yahoo CSV optional / NHL API stats / shifts). Tiers 4-6 (advanced scraping, social, beat media) **never built**. |
| [data-bundles.md](data-bundles.md) | Implemented | `data install/list/remove`, GitHub Releases, storage layout |
| [cache-model.md](cache-model.md) | Implemented | Snapshot store, tiers, integrity, provenance, chunked layout (Phase 8h) |
| [snapshot-operations.md](snapshot-operations.md) | Implemented | `snapshot list/show/use/verify/delete` CLI |
| [fantasy-scheme.md](fantasy-scheme.md) | Implemented | Scheme engine, weights, compute_fantasy_score |
| [scheme-customization.md](scheme-customization.md) | Implemented | `scheme list/show/fromcsv` CLI, user vs built-in |
| [fantasy-leagues.md](fantasy-leagues.md) | Implemented (partial) | SQLite + skater/goalie scoring + trade eval + local fantasy server. H2H matchups and full web mutation parity deferred. |
| [fantasy-poacher.md](fantasy-poacher.md) | Implemented (partial) | Phase Selke player-poacher scoring, roster gaps, simulation scenarios, watch rules, reports, and CLI/TUI/web ViewModel contracts. Carry-forward: full TUI rule editor and imported roster-need fit. |
| [fantasy-draft-daily-assistant.md](fantasy-draft-daily-assistant.md) | Implemented (partial) | League-specific draft, daily, morning, roster, trade, schedule, goalie, and season-simulation foundations; the linked war-room roadmap owns remaining automation and review work. |
| [team-season-forecast.md](team-season-forecast.md) | Implemented | Game probabilities, league Monte Carlo, playoffs, scenarios, player risk, atomic trades, paired impact, point-in-time replay, sealed-checkpoint movement/history/cards, and calibration. |
| [line-combination-simulation.md](line-combination-simulation.md) | Implemented (foundation) | The Blender and The Bench are implemented; The Cut training-camp roster selection and per-trial roster propagation are specified next. |
| [management-behavior-simulation.md](management-behavior-simulation.md) | Implemented (foundation) | Separate GM roster tendencies from manager deployment behavior; build opponent-style, matchup-line, checking-line, and bilateral-fatigue game plans. |
| [ahl-affiliate-development.md](ahl-affiliate-development.md) | Implemented (foundation) | Affiliate 12F/6D/2G lines enforce the sourced AHL 12-of-18 development-player rule; official roster and cross-league career-game ingestion remains. |
| [ahl-preseason-rollover.md](ahl-preseason-rollover.md) | Implemented (planning foundation) | Reconciles reviewed prior affiliate identities with current camp outcomes and fails closed on positional, identity, organization-status, and waiver gaps. |
| [prospect-development-study.md](prospect-development-study.md) | Implemented (foundation) | Multi-season production, opportunity, injury, and explicit attention-gap analysis identifies hidden and injury-obscured prospect risers. |
| [organization-lineup-forecast.md](organization-lineup-forecast.md) | Implemented (foundation) | The System joins complete NHL and AHL lineups into one UI-neutral 8F-line/6D-pair/4-goalie document with recall ladders. |
| [brand-the-rink.md](brand-the-rink.md) | Implemented | Canonical IceLines product language: The Rink navigation, Ice product family, The Insider voice/evidence labels, and hockey report names. |
| [group-management.md](group-management.md) | Implemented | Player watchlists: SQLite, CRUD, TUI g/f keys |
| [player-analysis.md](player-analysis.md) | Implemented | PlayerFilter, similarity search, career arc |
| [scouting-reports.md](scouting-reports.md) | Implemented | 8-section player report (terminal/markdown/json). Test coverage added in Hart.5c.2. |
| [projection-engine.md](projection-engine.md) | Implemented | pace/regressed/composite modes, age_factor |
| [position-engine.md](position-engine.md) | Implemented | PositionResolver, boxscore eligibility |
| [query-engine.md](query-engine.md) | Implemented (partial) | leaders/player/compare/similar, 30+ sort metrics. **Phase 5C strength-state and Phase 5D Tier 4 metrics never started.** |
| [site-generation.md](site-generation.md) | Implemented | mkdocs build/serve/deploy, deterministic markdown |
| [export-markdown.md](export-markdown.md) | Implemented | 7 of 7 shapes shipped; `fantasy` renders `PoachReportView` and `series` renders `PlayoffsView`. |
| [goalies.md](goalies.md) | Implemented | Phase Vezina shipped. Schema, repository, TUI tab, 38-season bundled goalie data, and fantasy goalie scoring. |
| [transactions.md](transactions.md) | Implemented | Phase Selke shipped. ESPN source, classifier, TUI tab, modern-era bundled transactions. |
| [test-strategy.md](test-strategy.md) | Implemented | L0/L1/L2 tiers, mock NHL API fixture, ~1020 tests |
| [platform-contracts.md](platform-contracts.md) | Draft | Uniform data/query/ViewModel/surface/report/visual contracts for Campbell and later phases. |
| [viewmodels.md](viewmodels.md) | Draft | Typed ViewModel boundary between core/query logic and CLI/TUI/web/report renderers. |
| [ui-neutral-card-system.md](ui-neutral-card-system.md) | Implemented | Versioned core-built documents for prognosis, fantasy roster/draft/morning/trade, prospective simulation, and historical replay across JSON/Web/TUI/reference rendering. |
| [surface-parity.md](surface-parity.md) | Active source of truth | Feature-by-surface matrix, including explicit partials and card renderer parity. |
| [visual-system.md](visual-system.md) | Implemented | Prince of Wales ASPECT visual contract, semantic tokens, representative TUI/web/CLI fences, and CREST closeout notes. |

---

## TUI Specs

| Spec | Status | Summary |
|------|--------|---------|
| [tui.md](tui.md) | Implemented | v1 as-built: 8 tabs (now 7 post-v2), all current screens, key bindings |
| [tui-v2.md](tui-v2.md) | Implemented | v2 redesign: 6+1 tabs (Phase 7a–e shipped; Goalies tab added in Vezina) |
| [tui-experiences.md](tui-experiences.md) | Implemented | Per-surface launchers — `tui --start <slug>`, sugar subcommands, drill-downs, and `icelines menu`. Phase Lady Byng. |
| [tui-admin-overlay.md](tui-admin-overlay.md) | Implemented (basic) | `F` overlay, install status. v2 features (action menu, `:` prompt) Draft. Tests Recommended (not yet written). |
| [depth-chart.md](depth-chart.md) | Implemented (partial) | Cross-team line value rankings + team depth grid. Goalie tier deferred. Multi-position eligibility documented as known limitation (singular post-Hart). |
| [scores.md](scores.md) | Implemented | Live game scores, date navigation, game detail (Phase 7c) |
| [schedule.md](schedule.md) | Implemented | Season schedule, team filter, matchup search (Phase 7d) |
| [playoffs.md](playoffs.md) | Implemented (partial) | Bracket, series tracker (Phase 7e). Historical bundles: 1993-94 only. |
| [season-timetravel.md](season-timetravel.md) | Implemented | Global season picker, 38-season navigation (Phase 7b) |
| [headshot-rendering.md](headshot-rendering.md) | Implemented (reference) | Braille dither algorithm. Tests Recommended (not yet written). |
| [dashboard-engine.md](dashboard-engine.md) | **Cancelled** | proof DASHBOARD-SPEC integration cut 2026-05-01. Document kept for historical reference. |

---

## Reading Order

**To understand the running build**: `../IceLines.md` → `../ARCHITECTURE.md` → individual feature specs by topic.

**Data layer**: `data-sources.md` → `cache-model.md` → `data-bundles.md` → `snapshot-operations.md`

**Player analytics**: `player-analysis.md` → `query-engine.md` → `projection-engine.md` → `scouting-reports.md`

**Fantasy stack**: `fantasy-scheme.md` → `scheme-customization.md` → `fantasy-leagues.md` → `fantasy-poacher.md` → `fantasy-draft-daily-assistant.md`

**TUI**: `tui-v2.md` → `season-timetravel.md` → `scores.md` / `schedule.md` / `playoffs.md` → `tui-admin-overlay.md`

**Cross-surface cards**: `platform-contracts.md` → `viewmodels.md` → `ui-neutral-card-system.md` → `visual-system.md` → `surface-parity.md`

**Live game data**: `scores.md` → `schedule.md` → `transactions.md`

---

## Spec Health

Last audit: 2026-07-22

| Issue | Action taken |
|-------|--------------|
| 30 spec files audited end-to-end | ✓ Complete; gap analysis in `../IceLines.md` |
| `dashboard-engine.md` referenced cancelled proof integration | ✓ Marked Cancelled; document kept for reference |
| `data-sources.md` Tiers 4-6 marked Planned but never built | ✓ Marked partial in this index |
| Various spec status fields drifted post-implementation | ✓ Refreshed in this index 2026-05-01 |
| `query-engine.md` Phase 5C/5D referenced as roadmap | ✓ Marked partial; not in active plan |
| `fantasy-leagues.md` defers goalie scoring; data now exists | ✓ Marked as Tier 2 backlog item |
| `INDEX.md` (plans + specs) status drift | ✓ Both refreshed 2026-05-01 |
