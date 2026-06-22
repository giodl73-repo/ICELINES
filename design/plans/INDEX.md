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
pass, Jim Gregory closed release hardening, Hurricane shipped the current
analytics surface push, Rangers organized the post-Hurricane workflow,
Islanders closed the surface-truth cleanup, and Canadiens now orders the next
major-stats competition push. The next forward work should start with the
strength-state split foundation, then proceed through advanced source authority,
Signals promotion, shift-data policy, browser QA, production packaging,
editorial/stathead workflows, and data freshness authority.

| Plan | Status | Summary |
|------|--------|---------|
| [Phase Canadiens - Major stats systems roadmap](2026-06-22-phaseCanadiens-major-stats-roadmap.md) | Active | Orders the next eight competitive pushes: strength-state splits, advanced source authority, Signals promotion, historical shift policy, browser QA, production packaging, editorial/stathead workflows, and freshness authority. |
| [Phase Canadiens Signals - Source authority](2026-06-22-phaseCanadiensSignals-source-authority.md) | Closed | Adds shared single-player Signals source authority across CLI JSON and Web HTML/JSON while preserving non-promotion boundaries. |
| [Phase Canadiens Source - Data status authority notes](2026-06-22-phaseCanadiensSource-data-status-authority-notes.md) | Closed | Adds shared data-status authority notes so CLI/Web freshness surfaces name optional MoneyPuck coverage and blocked adjacent claims. |
| [Phase Canadiens Source - MoneyPuck authority](2026-06-22-phaseCanadiensSource-moneypuck-authority.md) | Closed | Adds Web/API MoneyPuck skater snapshot authority for leaders while naming covered xG metrics and blocked adjacent advanced claims. |
| [Phase Canadiens Source - Goalie danger source gate](2026-06-22-phaseCanadiensSource-goalie-danger-source-gate.md) | Closed | Adds a Web/API goalie high-danger source gate that blocks high-danger save metrics until verified schema, join, freshness, and non-claim evidence exists. |
| [Phase Canadiens Source - GSAx catalog source block](2026-06-22-phaseCanadiensSource-gsax-catalog-source-block.md) | Closed | Pins the reserved GSAx catalog keys as source-blocked until verified goalie xGA schema, join, freshness, and non-claim evidence exists. |
| [Phase Canadiens Source - Goalie xGA source gate](2026-06-22-phaseCanadiensSource-goalie-xga-source-gate.md) | Closed | Adds a Web/API goalie xGA source gate that blocks GSAx until a verified goalie xGA schema, join, freshness, and non-claim contract exists. |
| [Phase Canadiens Source - Team streak authority](2026-06-22-phaseCanadiensSource-team-streak-authority.md) | Closed | Adds team streak source authority metadata and HTML copy for boxscore and play-by-play leader families. |
| [Phase Canadiens Source - Player streak authority](2026-06-22-phaseCanadiensSource-player-streak-authority.md) | Closed | Adds player streak source authority metadata for boxscore and play-by-play streak families plus matching HTML authority labels. |
| [Phase Canadiens Source - Player outlook season-stat authority](2026-06-22-phaseCanadiensSource-player-outlook-season-authority.md) | Closed | Adds player outlook season-stat authority metadata and HTML copy while keeping schedule authority separate. |
| [Phase Canadiens Source - Outlook schedule authority](2026-06-22-phaseCanadiensSource-outlook-schedule-authority.md) | Closed | Adds schedule authority metadata and HTML labels to scoring outlook surfaces without over-claiming play-by-play or projection authority. |
| [Phase Canadiens Source - Tonight Intel authority](2026-06-22-phaseCanadiensSource-tonight-authority.md) | Closed | Extends the scoring source authority contract to Tonight Intel JSON and HTML surfaces. |
| [Phase Canadiens Source - Scoring authority coverage state](2026-06-22-phaseCanadiensSource-authority-coverage-state.md) | Closed | Adds compact `source_authority.coverage_state` values so scoring consumers can read coverage without interpreting raw source completeness. |
| [Phase Canadiens Source - Scoring authority limitations](2026-06-22-phaseCanadiensSource-authority-limitations.md) | Closed | Adds machine-readable `source_authority.limitations` keys so scoring consumers can distinguish cached play-by-play coverage from non-covered domains. |
| [Phase Canadiens Source - Scoring authority metric family](2026-06-22-phaseCanadiensSource-authority-metrics.md) | Closed | Adds machine-readable `source_authority.covered_metrics` keys for the scoring metrics covered by cached official NHL play-by-play. |
| [Phase Canadiens Source - Scoring authority metadata](2026-06-22-phaseCanadiensSource-scoring-authority.md) | Closed | Adds scoring report JSON and HTML authority metadata that identifies cached official NHL play-by-play as the scoring-event and strength-state source. |
| [Phase Canadiens Strength - Strength summaries](2026-06-22-phaseCanadiensStrength-strength-summaries.md) | Closed | Adds aggregate scoring report By strength rows for even-strength, power-play, and penalty-kill totals across raw NHL situation codes. |
| [Phase Canadiens Strength - Event HTML strength hooks](2026-06-22-phaseCanadiensStrength-event-html-hooks.md) | Closed | Adds shared event strength-state accessors and stable Web event-row `data-*` hooks so individual scoring events match the situation summary metadata. |
| [Phase Canadiens Strength - HTML strength-state hooks](2026-06-22-phaseCanadiensStrength-html-hooks.md) | Closed | Adds stable `data-*` strength-state hooks to Web scoring situation rows while preserving the visible scoring table label. |
| [Phase Canadiens Strength - Structured scoring splits](2026-06-22-phaseCanadiensStrength-structured-splits.md) | Closed | Adds structured `situation_code`, `skater_state`, and `owner_strength_state` fields to scoring situation splits while preserving display labels and avoiding premature leaderboard promotion. |
| [Phase Canadiens Strength - Owner-perspective labels](2026-06-22-phaseCanadiensStrength-owner-perspective-labels.md) | Closed | Promotes cached play-by-play scoring situation splits to event-owner labels like `even strength 5v5 (1551)`, `power play 5v4 (1541)`, and `penalty kill 4v5 (1451)`. |
| [Phase Canadiens Strength - Event owner side](2026-06-22-phaseCanadiensStrength-event-owner-side.md) | Closed | Carries away/home event-owner context from NHL play-by-play into `ScoringEventInput`, setting up team-perspective PP/PK/even-strength labels without promoting new leaderboards yet. |
| [Phase Canadiens Strength - Scoring strength-state labels](2026-06-22-phaseCanadiensStrength-scoring-strength-state-labels.md) | Closed | Starts the strength-state push by normalizing cached play-by-play scoring situation splits to labels like `5v5 (1551)` while preserving raw NHL `situationCode` auditability. |
| [Phase Tigers - Docs boundary truth gate](2026-06-21-phaseTigers-docs-boundary-truth-gate.md) | Closed | Fenced Web `/docs` around persistent report-toggle writes staying a TUI durable-config handoff and the removed mkdocs/static-site frontend staying outside active docs surfaces. |
| [Phase Cougars - Watch editor boundary docs truth gate](2026-06-21-phaseCougars-watch-editor-boundary-docs.md) | Closed | Clarified `COMMANDS.md` and `/docs`: web/TUI watch editors stay player-rule-only while team/deployment preview/save remains CLI-backed. |
| [Phase Quakers - Fantasy command docs truth gate](2026-06-21-phaseQuakers-fantasy-command-docs.md) | Closed | Aligned `COMMANDS.md` and `/docs` with dashboard fantasy mutation recovery: roster-shape set and CSV import are not GET-backed and recover through CLI commands. |
| [Phase Stags - Watch deployment docs truth gate](2026-06-21-phaseStags-watch-deployment-docs.md) | Closed | Aligned `COMMANDS.md` and `/docs` with dashboard watch deployment recovery: CLI preview or `/watchlist` player rules, while arbitrary deployment editing remains deferred. |
| [Phase Americans - Dashboard group docs truth gate](2026-06-21-phaseAmericans-dashboard-group-docs.md) | Closed | Aligned `COMMANDS.md` and `/docs` with Maroons: dashboard group mutation commands are not GET-backed and recover through `/favorites` POST forms or `icelines group`. |
| [Phase Maroons - Dashboard group copy truth gate](2026-06-21-phaseMaroons-dashboard-group-copy.md) | Closed | Updated dashboard command group-mutation recovery copy to preserve GET non-mutation while pointing users to `/favorites` POST forms or `icelines group`. |
| [Phase Barons - Admin docs truth gate](2026-06-21-phaseBarons-admin-docs-truth.md) | Closed | Aligned the embedded command reference and `/docs` route with Phase Seals: web admin install is bundled-season only with exact confirmation, and remove is scoped installed-season deletion with exact confirmation. |
| [Phase Seals - Admin install/remove safety](2026-06-21-phaseSeals-admin-install-remove-safety.md) | Closed | Mounted scoped web admin data install/remove routes: bundled-season install with exact confirmation and no live fetch, plus installed-season remove constrained to `~/.icelines/seasons/<season>` with exact confirmation. |
| [Phase Whalers - Favorites group editor](2026-06-21-phaseWhalers-favorites-group-editor.md) | Closed | Added POST-backed web and JSON mutation routes for named Favorites group create/rename/delete and selected-group member add/remove, while keeping GET navigation read-only and protecting canonical Favorites rename/delete. |
| [Phase Nordiques - Historical MoneyPuck xG](2026-06-21-phaseNordiques-historical-moneypuck-xg.md) | Closed | Added bounded multi-season MoneyPuck fetch with `fetch money-puck --seasons N`, plus season-aware MoneyPuck snapshot reads so historical xG queries use the requested season's sealed snapshot. |
| [Phase Golden Knights - Poach route wording gate](2026-06-21-phaseGoldenKnights-poach-route-wording-gate.md) | Closed | Closed the Poach route wording gate: board/report route rows now carry scoped shared-ViewModel claims while preserving API-envelope and read-only SQLite boundaries. |
| [Phase Blues - Fantasy route wording gate](2026-06-21-phaseBlues-fantasy-route-wording-gate.md) | Closed | Closed the Fantasy route wording gate: `/fantasy` and `/api/v1/fantasy/*` rows now carry scoped read/product claims while preserving browser mutation, missing-state, and read-only SQLite deferrals. |
| [Phase Panthers - Signals row wording gate](2026-06-21-phasePanthers-signals-row-wording-gate.md) | Closed | Closed the Player Signals row wording gate: Signals now carry partial-by-design wording while preserving Capitals non-claims around cache, catalog, filter, leaderboard, ranking, and recommendation authority. |
| [Phase Lightning - Career route wording gate](2026-06-21-phaseLightning-career-route-wording-gate.md) | Closed | Closed the Career/cohort route wording gate: `/career` and `/api/v1/career` now carry partial-by-design wording while preserving Maple Leafs non-claims around TUI handoff, optional local career-history data, and no live read fetch. |
| [Phase Sharks - Analytics-cache route wording gate](2026-06-21-phaseSharks-analytics-cache-route-wording-gate.md) | Closed | Closed the analytics-cache route wording gate: player evidence-card and opponent-scout route rows now carry bounded prepared-cache claims while preserving Stars/Bruins non-claims around full workflows, live recomputation, and autonomous coaching. |
| [Phase Ducks - Favorites/watch route wording gate](2026-06-21-phaseDucks-favorites-watch-route-wording-gate.md) | Closed | Closed the Favorites/watch route wording gate: route rows now read as scoped partials by design for read-only named groups, canonical Favorites mutations, and player watch-rule mutations while preserving richer group/rule edit deferrals. |
| [Phase Sabres - Docs/reference truth gate](2026-06-21-phaseSabres-docs-reference-truth-gate.md) | Closed | Closed the docs/reference truth gate: CLI docs, TUI docs overlay, Web `/docs`, and dashboard/menu docs handoffs use embedded `COMMANDS.md`/`DocsView`, while removed mkdocs/static-site commands and `/site/*` stay outside active claims. |
| [Phase Senators - Admin row wording gate](2026-06-21-phaseSenators-admin-row-wording-gate.md) | Closed | Closed the admin row wording gate: Data install/list/remove, Snapshot operations, Config/report toggles, and admin route rows now read as safe partials by design while preserving Flyers deferrals for web install/remove and persistent report-toggle writes. |
| [Phase Blackhawks - Playoff bracket/detail gate](2026-06-21-phaseBlackhawks-playoff-detail-gate.md) | Closed | Closed the Playoff bracket/detail gate: bundled `PlayoffsView` bracket and game-log detail now has a bounded CLI/TUI/Web/API/Markdown detail-export claim without live fetch, prediction, or new Web series-drilldown claims. |
| [Phase Red Wings - Favorites/watch boundary gate](2026-06-21-phaseRedWings-favorites-watch-gate.md) | Closed | Closed the Favorites/watch boundary gate: read-only named groups, POST-backed canonical Favorites mutations, and player watch-rule create/toggle/delete stay supported, while richer group/rule editing remains blocked on shared mutation contracts. |
| [Phase Maple Leafs - Career/cohort leaders gate](2026-06-20-phaseMapleLeafs-career-cohort-gate.md) | Closed | Closed the Career/cohort leaders gate: CareerView-backed CLI/Web/JSON/dashboard surfaces stay canonical, while TUI remains a tested command-bar handoff by design. |
| [Phase Oilers - Named analytics-cache report promotion gate](2026-06-20-phaseOilers-named-cache-report-promotion.md) | Closed | Closed the WP-009 named analytics-cache report gate: named report is now bounded generic prepared-cache inspection only; workflow-family route claims remain bounded by their phase-specific non-claims. |
| [Phase Canucks - Agent evidence promotion gate](2026-06-20-phaseCanucks-agent-evidence-promotion.md) | Closed | Closed the WP-009 agent-evidence promotion gate: agent evidence is now a bounded prepared-cache evidence summary claim, while named analytics cache report remains generic prepared-cache inspection evidence. |
| [Phase Predators - Postgame promotion gate](2026-06-20-phasePredators-postgame-promotion.md) | Closed | Closed the WP-009 postgame promotion gate: postgame review and adjustments are now bounded prepared-cache report claims, while named report and agent routes remain first-route evidence with explicit non-claims. |
| [Phase Kraken - Practice focus promotion gate](2026-06-20-phaseKraken-practice-focus-promotion.md) | Closed | Closed the WP-009 practice-focus promotion gate: practice focus is now a bounded prepared-cache report claim, while named report, postgame, adjustment, and agent routes remain first-route evidence with explicit non-claims. |
| [Phase Avalanche - Goalie readiness promotion gate](2026-06-20-phaseAvalanche-goalie-readiness-promotion.md) | Closed | Closed the WP-009 goalie-readiness promotion gate: goalie readiness is now a bounded prepared-cache workload claim, while named report, practice, postgame, adjustment, and agent routes remain first-route evidence with explicit non-claims. |
| [Phase Wild - Line-combination promotion gate](2026-06-20-phaseWild-line-combination-promotion.md) | Closed | Closed the WP-009 line-combination promotion gate: line-combination explorer is now a bounded prepared-cache line-combination explorer claim, while named report, goalie, practice, postgame, adjustment, and agent routes remain first-route evidence with explicit non-claims. |
| [Phase Stars - Player evidence-card promotion gate](2026-06-20-phaseStars-player-evidence-card-promotion.md) | Closed | Closed the WP-009 player evidence-card promotion gate: player evidence card is now a bounded prepared-cache player evidence-card claim, while named report, line, goalie, practice, postgame, adjustment, and agent routes remain first-route evidence with explicit non-claims. |
| [Phase Bruins - Opponent scout promotion gate](2026-06-20-phaseBruins-opponent-scout-promotion.md) | Closed | Closed the WP-009 opponent-scout promotion gate: opponent scout is now a bounded prepared-cache scout report claim, while named report, player-card, line, goalie, practice, postgame, adjustment, and agent routes remain first-route evidence with explicit non-claims. |
| [Phase Penguins - Analytics workflow promotion gate](2026-06-20-phasePenguins-analytics-workflow-promotion.md) | Closed | Closed the WP-009 analytics workflow promotion gate: coach dashboard is now a bounded prepared-cache dashboard claim, while named report, scout, player-card, line, goalie, practice, postgame, adjustment, and agent routes remain first-route evidence with explicit non-claims. |
| [Phase Capitals - Signals cache promotion gate](2026-06-20-phaseCapitals-signals-cache-promotion.md) | Closed | Closed the Signals promotion gate: Signals remain direct `PlayerSignalsView` inspection surfaces and stay out of analytics cache, `StatId`, filters, and public leaderboards until future contracts prove cache/source-state and bounded ranking semantics. |
| [Phase Flyers - Admin operation safety gate](2026-06-20-phaseFlyers-admin-safety.md) | Closed | Closed the post-Devils admin safety wave: web data install/remove stay deferred and unmounted, persistent report-toggle writes stay a CLI/TUI durable config handoff, and focused admin route tests cover safe mutations. |
| [Phase Devils - Dashboard visual QA gate](2026-06-20-phaseDevils-dashboard-visual-qa.md) | Closed | Closed the post-Islanders browser-proof wave for `/dashboard`: representative desktop/tablet/mobile capture matrix, automated route/artifact checks, and exact surface-matrix wording without overstating browser, touch, focus, or accessibility coverage. |
| [Phase Islanders - Surface parity cleanup](2026-06-20-phaseIslanders-surface-parity.md) | Closed | Wrapped the post-Rangers surface-truth cleanup: refreshed the surface-parity matrix, tightened admin/docs claims, recorded selected dashboard capture evidence, and rolled up cache-backed partial surfaces without promoting new analytics claims. |
| [Phase Rangers - Post-Hurricane goals](2026-06-20-phaseRangers-goals.md) | Wrapped | Closed the post-Hurricane organization round: NYR workflow proof, gated Signals roster matrix, evidence-bridge decision, layout persistence proof, and lean CLI target-not-met audit without reopening blocked Hurricane source claims. |
| [Phase Hurricane - Signals surface + product-gap roadmap](2026-06-17-phaseHurricane-signals-surface.md) | Wrapped | Product-analytics push named for Carolina's 2026 Cup. Shipped Signals CLI/TUI/Web/Markdown export, MoneyPuck on-ice/schema fixture, player confidence ranges, goalie QS%/SA/60, season-depth honesty, and compact visualization through 6z. |
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
| Historical shift-data bundles | ⭐⭐ | Parked — no supported `fetch shifts` command, fixtures, or source/bundle policy; `sync.capabilities.shifts=off` is enforced |

Cleared in Phase Nordiques:
- MoneyPuck historical xG — shipped bounded multi-season regular-season CSV
  fetch through `icelines fetch money-puck --seasons N`, with season-aware
  snapshot reads for historical xG metrics.

Cleared in Score the Day:
- Fantasy daily delta scoring — shipped via `FantasyDailyDeltaView`,
  `icelines fantasy daily --date`, and `/api/v1/fantasy/daily?date=...`
  from cached finalized boxscores and local FantasyDb rosters.

Cleared in Match the Week:
- Fantasy head-to-head matchup weekly — shipped via `FantasyMatchupWeekView`,
  local `fl_matchups` schedule rows, `icelines fantasy matchup-set`,
  `icelines fantasy matchup --date`, and `/api/v1/fantasy/matchup?date=...`
  from cached finalized daily-delta totals.

Cleared in Import the Rosters:
- Yahoo league CSV roster import — shipped via `FantasyImportView`,
  `icelines_fetch::fantasy_import`, and
  `icelines fantasy import-yahoo --file <path> --league <name> [--dry-run]`;
  TUI/web-dashboard command bars hand off or defer rather than mutating through
  GET.

Cleared in Shape the Rosters:
- Fantasy roster shape enforcement — shipped via `RosterShape` /
  `RosterShapeValidationView`, `fl_leagues.roster_shape`, Yahoo import warnings
  from canonical player positions, CLI `fantasy roster-shape*` commands, TUI/web
  dashboard handoffs, and read-only
  `/api/v1/fantasy/roster-shape[?team=<name>]` validation.

Cleared in Guard the Gates:
- CI cargo fmt + cargo audit gates — `cargo fmt --check` was already blocking;
  `cargo audit` now runs through the CI quality matrix and local
  `scripts/test-slice.ps1 ci-audit`, with warning-class advisories tracked in
  `design/release-checklist.md`.

### Cancelled (referenced in specs but won't ship)

| Item | Why |
|------|-----|
| proof DASHBOARD-SPEC integration | proof project cut 2026-05-01 |
| TUI as proof dashboard renderer | depends on proof |
| Tier 5 — social signals (Reddit) | scope creep; never implemented |
| Tier 6 — beat media RSS line rushes | scope creep; never implemented |
| Per-player site pages | site explosion; not in scope per `site-generation.md` |
