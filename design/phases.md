# IceLines Phases — Trophy Naming

Each phase of the project is named after an NHL trophy that captures its
character. The matchup is intentional: the Vezina is for goalies, the
Norris is for defensemen who shore up the structure, the Hart is for
the foundational MVP-tier work that touches everything. The names are
load-bearing — they show up in commits, tags, and plan filenames — so
adding a new phase means picking a trophy that fits.

## Active mapping

| Phase | Trophy | What it covered | Rationale |
|---|---|---|---|
| 1 — Foundation | **Calder** | Initial Rust CLI, fetch pipeline, snapshot store | Rookie of the Year — the project's first draft. |
| 2 — Site | **Lady Byng** | mkdocs static site, ranked-team index, lineup cards | Sportsmanship + skill — the polished, restrained, public-facing output. |
| LB — TUI experiences (2026-05) | **Lady Byng** *(reuse — second use)* | Per-surface TUI launchers (`tui goalies`, `tui player Bedard`), looping `icelines menu`, drill-down resolution. Same polish/UX angle. | Reuse acknowledged: commits use `Phase Lady Byng (TUI experiences):` to disambiguate. Plan: `plans/2026-05-05-phaseLadyByng-tui-experiences.md`. |
| 3 — Projections | **Art Ross** | TUI projections leaderboard, pace_82 sorting | Most points — the scoring title, the headline leaderboard. |
| 4 — History polish | **King Clancy** | Career history, historical seasons, data hygiene | Leadership + community — long-term data quality work that benefits every later phase. |
| 5 — Query engine | **Bill Masterton** | `query leaders/player/compare`, --seasons N, percentiles | Perseverance — the hard, unglamorous filter/sort/score plumbing. |
| 6 — Export & dashboards | **Mark Messier** | `export md`, dashboard panel, scout cards | Leadership Award — multi-surface vision, leading data into other tools. |
| 7 — TUI v2 redesign | **Jack Adams** | 7-tab layout, admin overlay, season time-travel | Coach of the Year — strategic restructuring of the whole system. |
| 8 — Spec delta + chunks | **Norris** | Spec catch-up, chunked snapshots, data integrity | Best defenseman — defensive, structural, plays both ends, foundational. |
| G — Goalies | **Vezina** | Goalie type, repository, fantasy goalie scoring, 38-season bundled goalie data | The goalie trophy. Obvious. |
| T — Transactions | **Selke** | Trades/waivers/signings/IR feed, ESPN source, TUI tab | Best defensive forward — two-way work spanning data ingestion + UI. |
| E — Edge speed | **Maurice Richard** | Skating speed leaderboard (blocked: no public API) | Most goals — pure scoring ability — but the data isn't accessible. Parked. |
| Hart — Normalization | **Hart** | Full data model normalization — `PlayerIdentity` + `SeasonStats` keyed by (player_id, season, type) | League MVP — most valuable single piece of work; touches every consumer. Subsumes Phase Presidents (season-type). |
| Calder — Multi-league career | **Calder** *(reuse — third use)* | NHL landing-endpoint career history; pre-NHL development arc on player card; cohort leaderboards via `query career` | Reuse: same trophy as Phase 1. Same character — multi-league rookie + early-career data. |
| Foster — Favorites + time travel + unified data layer | **Foster** | Favorites dashboard, time-travel on Scores/Schedule/Playoffs, unified DataStore + sharded manifest, EventStream, sync engine, capability matrix, windowed atoms, per-night stat lines | Foster Hewitt Memorial (broadcaster) — "keeping you informed". The personal-dashboard angle: favorites are about the user's particular informational lens. |
| Conn Smythe — Live playoff tracking | **Conn Smythe** *(in progress 2026-05-06)* | Series momentum, Cup-run player narratives, live game tracking surface | Playoff MVP. Builds on Foster's rails to surface playoff-specific narratives. Spec: `conn-smythe-overview.md`. |
| Art Ross — Query system rewrite (centerpiece) | **Art Ross** *(reuse — second use; in progress 2026-05-06)* | Unified parser → planner → executor IR; sliding-window streak atoms (`g.last10g>=5`); historical `EVER` queries across 38 seasons (`g.any10g>=5 EVER AT age<=22`); cross-league career atoms (`league=OHL`); on-demand data fetch; `--explain` plan visibility; one front door for CLI / web / TUI. | Reuse: same trophy as Phase 3 (Projections). Same character — points-leader-class flexible queries. Per the user, this is THE centerpiece: "nobody else is going to do as good a job on hockey queries." Spec: `phase-art-ross-overview.md`. Plan: `plans/2026-05-06-phaseArtRoss-overview.md`. Post-8-role-review with 18 action items applied. |
| Norris — TUI architecture refactor | **Norris** *(planned 2026-05-07)* | Extract per-screen state structs out of the 3,800-line `App` god-object: `QueriesState`, `ScheduleState`, `TransactionsState`, etc. Pure internal refactor — no keybind change, no UX delta. | Best defenseman — anchors the back end, foundational structural play. Picking guide: "Foundational, structural, both-ends work → Norris." Spec: `phase-norris-overview.md`. Plan: `plans/2026-05-07-phaseNorris-tui-state-extraction.md`. |
| Masterton — TUI screen-trait factoring | **Bill Masterton** *(planned 2026-05-08)* | Factor TUI **control flow** out of the App monolith via a `Screen` trait owning state + render + handle + chrome. App becomes a thin orchestrator. Standalone runner for hosting any single screen with no tab strip. Consistent declarative chrome (header + footer keybind chips) across screens. | Perseverance + dedication to hockey — fits the long-term unglamorous architecture follow-on after Norris. Spec: `phase-masterton-overview.md`. Plan: `plans/2026-05-08-phaseMasterton-tui-screen-trait.md`. |
| Jack Adams — TUI MDI dashboard | **Jack Adams** *(planned 2026-05-08)* | Multi-document interface — espn-style "front door" with Scores ribbon top, Favorites left, swappable Workspace middle, Schedule right, plus a chat-CLI command bar bottom. Adaptive width drops panes from edges; user can manually toggle side-pane visibility. Reuses Norris state structs + Masterton chrome accessors. Adams.6 adds an opt-in AI/LLM fallback when deterministic parsing fails. | Coach of the year — "designs the system, makes the bench coordinate." Same defensive structural-systems character as Norris/Masterton. Spec: `phase-jack-adams-overview.md`. Two releases: v0.23.0 (deterministic MDI), v0.23.1 (AI fallback). |
| Jennings - Stabilization + truth pass | **William M. Jennings** *(planned 2026-05-09)* | Restore build-green, fix config-test drift, record measured baseline, reconcile stale docs/plans before Messier. | Defensive excellence: allow fewer regressions before adding new feature pressure. Plan: `plans/2026-05-09-phaseJennings-stabilization-truth.md`. |
| Campbell - Platform contracts and ViewModels | **Clarence S. Campbell Bowl** *(closed 2026-05-12)* | Defined data/query/ViewModel/surface/visual contracts and introduced typed ViewModels between core/query and renderers. | Conference architecture: the common ice between the data engine and every surface. Plan: `plans/2026-05-09-phaseCampbell-platform-viewmodels.md`. |
| Selke - Fantasy poacher | **Frank J. Selke** *(reuse; implemented 2026-05-09)* | PoachScore, watch rules, deployment/schedule/category feature extraction, and CLI/TUI/web/markdown/JSON fantasy-poacher surfaces. | Two-way value: identifies hidden category, deployment, and schedule edges before the league notices. Plan: `plans/2026-05-09-phaseSelke-fantasy-poacher.md`. |
| Messier - TUI filter/sort consistency | **Mark Messier** *(reuse; implemented 2026-05-09)* | Standard keybind/filter matrix across TUI player-list screens plus cmdbar kv grammar using the shared contract path. | Leadership = consistent behavior across screens. Plan: `plans/2026-05-08-phaseMessier-roster-filters.md`. |
| Lester Patrick - CLI parity | **Lester Patrick** *(implemented 2026-05-09)* | Closed CLI gaps for schedule, playoffs, transactions, and in-TUI docs using the post-Messier command vocabulary. | Outstanding service to hockey: finish the surface gaps so useful data is reachable everywhere. Plan: `plans/2026-05-05-phaseLesterPatrick-cli-parity.md`. |
| Ted Lindsay - Web parity | **Ted Lindsay** *(implemented with tracked partials)* | Split the web handler monolith, create a surface parity matrix, and bring major web HTML/JSON behavior into CLI/TUI parity. | Players' choice: make the browser surface one a regular user would choose. Plan: `plans/2026-05-09-phaseTedLindsay-web-parity.md`. |
| Prince of Wales - ASPECT visual system | **Prince of Wales Trophy** *(closed 2026-05-12)* | Applied the DEGAS ASPECT rubric to TUI/web/CLI visual quality: shared visual tokens, scan rhythm, responsive web polish, CLI readability, and practical visual fences. | Conference champion polish before the final release push. Plan: `plans/2026-05-09-phasePrinceOfWales-visual-system.md`. Closeout: `notes/2026-05-12-prince-closeout-crest-review.md`. |
| Jim Gregory - Release hardening | **Jim Gregory** *(implemented; CI pending)* | CI gates, release checklist, current-season rollover, bundled-data freshness, binary smoke discipline, and release artifact verification. | GM of the Year: manage the whole organization, not just the on-ice feature work. Plan: `plans/2026-05-09-phaseJimGregory-release-hardening.md`. |

## Future / parked

| Phase | Trophy | Status | What it would cover |
|---|---|---|---|
| Presidents — Season type | **Presidents'** | **Folded into Hart** | Regular vs playoff scoping. Naturally falls out of the (player_id, season, type) primary key after Hart. |
| (TBD) — Scoring scheme builder | **Frank Selke** | Future | Defensive-leaning fantasy scheme (penalty kills, shorthanded points, blocks-heavy) — a complement to the offense-tuned Yahoo/ESPN defaults. |

## Naming conventions

- **Plan filenames**: `YYYY-MM-DD-phase{Trophy}-{slug}.md` — e.g.
  `2026-04-30-phaseHart-normalization.md`. Trophy is the canonical
  identifier; the slug is for filesystem readability.
- **Tag names**: continue using semver (`v0.12.0`), but reference the
  phase trophy in the tag annotation message.
- **Commit subjects**: `Phase {Trophy}: short description` — e.g.
  `Phase Vezina: goalie repository + bundled goalie data`. (Existing
  commits used letter codes like `G.4`; future commits switch to
  trophy names.)
- **Memory entries**: refer to phases by trophy when documenting
  decisions for future sessions.

## Why bother

Three reasons:
1. **Mnemonic**: "Vezina was the goalie phase" sticks; "G.4 was
   the team-roster goalie strip" doesn't.
2. **Tone**: the project is hockey-domain; the names should be too.
3. **Scope discipline**: picking a trophy forces an honest read on
   what the phase IS. If you can't name it after a trophy, the scope
   is probably tangled.

## Picking a trophy

Match the trophy's character to the phase's character:
- **Foundational, structural, both-ends work** → Norris.
- **MVP impact, touches everything** → Hart.
- **Pure scoring / leaderboard** → Art Ross / Maurice Richard.
- **Polish, surface, public-facing** → Lady Byng.
- **Strategic restructuring** → Jack Adams.
- **Two-way concern, multi-surface** → Selke.
- **Long-term, unglamorous, infrastructure** → Bill Masterton / King Clancy.
- **Goalie or playoff-specific work** → Vezina / Conn Smythe.

If two trophies fit, pick the one with the more specific mandate — the
trophy should narrow the scope, not widen it.
