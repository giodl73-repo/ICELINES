# Phase Selke - fantasy poacher

**Date**: 2026-05-09
**Status**: Draft - planned after Campbell; can run after Messier or in parallel with surface parity if scoped carefully
**Trophy**: Frank J. Selke Trophy *(reuse)*. Fit: defensive/two-way value. This phase finds players whose real fantasy usefulness is hidden by role, deployment, category mix, schedule, or market perception.
**Spec seeds**: `design/specs/platform-contracts.md`, `design/specs/fantasy-poacher.md`
**Estimated**: 5-8 sub-phases

---

## Why

IceLines should not only answer "who is good?" It should answer the fantasy
manager's sharper question:

> Who should I add before everyone else notices?

The poacher surface turns IceLines into a decision tool for fantasy adds,
streams, stashes, buy-lows, and category specialists. It combines deployment,
recent trend, scoring scheme fit, schedule, availability/watch groups, and risk
into a `PoachScore`.

This is a Selke phase because it values the hidden two-way game: hits, blocks,
PK/PP usage, minutes, lineup promotion, under-deployed players, and players
helping categories that pure points leaderboards miss.

---

## Role review gates

| Role | Gate |
|---|---|
| HART | `PoachScore` and watch state are keyed by `(player_id, season, season_type)` plus scoring scheme/window. |
| KEEL | Poach board renders through one `PoachBoardView` across CLI/TUI/web/markdown/JSON. |
| TAPE | Deployment, schedule, transaction, and source freshness are surfaced; unavailable source data becomes warnings, not zeros. |
| FORGE | `PoachScore` components are typed, weighted, and explainable; no ad hoc score bags. |
| PACE | Every component documents units, clamp, weight, and whether measured or heuristic. |
| BENCH | Known-value fixture tests cover score components, thresholds, and explanation text. |
| EDGE | Tests cover scratched players, zero GP, stale line data, no schedule, traded player, goalie uncertainty, and unavailable ownership. |
| WIRE | Optional external availability/ownership imports fail gracefully; no live dependency in default tests. |
| SCOUT | Recommendations make hockey sense: promotion, deployment, role, and category fit are not flattened into raw points. |
| GLASS/Broadcast | The board explains "why this player" at a glance and does not hide risk behind one magic score. |

---

## Platform contracts consumed

Selke consumes `design/specs/platform-contracts.md` this way:

- **Data context**: every recommendation carries season/type, scoring scheme,
  source state, freshness, and completeness.
- **Query/filter intent**: CLI flags, TUI filters, web params, and report options
  lower into one typed `PoachQuery`.
- **ViewModel**: `PoachBoardView`, `WatchRulesView`, and `PoachReportView` are
  first-class ViewModels, not renderer-specific tables.
- **Surface parity**: CLI, TUI, web, markdown, and JSON surfaces are planned
  together; any deferred surface is named in the matrix.
- **Report generation**: `report poach` and `report weekly` render from
  `PoachReportView`; markdown/HTML/JSON variants share source data.
- **Visual language**: recommendations expose semantic tokens such as
  `rising`, `stash`, `stream`, `category_fit`, `schedule_edge`, and `risk`.

---

## Core model

```text
PoachScore =
  opportunity_delta
+ deployment_trend
+ category_fit
+ schedule_value
+ availability_gap
+ roster_need_fit
- risk_discount
```

Initial components:

| Component | Meaning |
|---|---|
| `opportunity_delta` | Role is better than market/default perception: top-six minutes, PP bump, promotion, or underused strong player. |
| `deployment_trend` | TOI, line, PP/PK, shot volume, or goalie starts trending up over last 3/5/10 games. |
| `category_fit` | Helps the active scoring scheme/categories: hits, blocks, shots, PPP, goalie saves, etc. |
| `schedule_value` | Next 7/14-day games, off-night games, back-to-back goalie starts, playoff-week density. |
| `availability_gap` | Available, low-rostered, manually marked available, or not on user's roster. |
| `roster_need_fit` | Helps categories or positions the user's fantasy team lacks. |
| `risk_discount` | Scratch risk, low GP, injury/IR uncertainty, unstable line, stale data, small sample. |

Every score row must include an explanation list. A magic number without
"why" is not acceptable.

### Deployment and availability state

Selke must distinguish absence of evidence from negative evidence:

```text
DeploymentSignal = Actual | Estimated | Unknown
AvailabilityState = Available | RosteredByUser | Watched | ImportedRostered | Unknown
```

Rules:

- `Unknown` line/PP/ownership data never subtracts from `PoachScore` by itself.
- `Estimated` deployment must name its proxy, such as TOI trend, recent shot
  volume, game-log role, or manual watch note.
- `Actual` deployment requires a named source and freshness timestamp.
- Recommendations are tagged by confidence: `high`, `medium`, `low`, or
  `data_limited`.
- The explanation list names which components were unavailable or estimated.

---

## Surfaces

### CLI

```bash
icelines poach --days 14
icelines poach --category hits,blocks
icelines poach --streamers --off-nights
icelines poach --stash
icelines poach --team EDM
icelines poach --json
icelines watch player "Matthew Knies" --when pp1
icelines watch deployment --team TOR --line-change
icelines report poach --markdown
icelines report weekly --league default
```

### TUI

Add a Poach screen or workspace panel:

- board columns: player, team, pos, role, line, PP, trend, next games,
  category fit, risk, score;
- filters: available only, category, position, team, schedule window,
  rising/stash/stream;
- detail pane: explanation bullets, source state, last-change evidence;
- watch-rule editor for common rules.

### Web

Routes:

- `/poach`
- `/watchlist`
- `/reports/poach`
- `/api/v1/poach`
- `/api/v1/watch-rules`

### Reports

Markdown/HTML/JSON reports:

- waiver poach board;
- weekly add/drop prep;
- category specialist report;
- goalie streaming report;
- deployment risers/fallers;
- watched-player alerts.

---

## Sub-phase ordering

```text
Selke.1  Fantasy poacher spec and scoring model
Selke.2  Poach ViewModels and fixtures
Selke.3  Deployment/schedule/category feature extraction
Selke.4  CLI poach/report/watch commands
Selke.5  TUI Poach board and watch-rule editor
Selke.6  Web poach board and JSON routes
Selke.7  Reports and weekly workflow
Selke.8  Docs, pitfalls, invariants, and closeout
```

---

## Selke.1 - Spec and scoring model

Create and keep current `design/specs/fantasy-poacher.md`.

Acceptance:

- Component weights and clamps are documented.
- Each component says whether it is measured, estimated, or deferred.
- Missing ownership/availability data has a local fallback: groups/watchlists or
  fantasy roster imports.
- Query, ViewModel, report, fixture, pitfall, and invariant contracts are named
  before code lands.

---

## Selke.2 - ViewModels and fixtures

Add:

- `PoachBoardView`
- `PoachPlayerRow`
- `PoachExplanation`
- `WatchRule`
- `WatchRulesView`
- `PoachReportView`

Acceptance:

- Fixture builds deterministic rows for rising skater, category specialist,
  stash, streamer goalie, and risk-discounted player.
- ViewModels include source state and explanation text.

Initial implementation note:

- Core ViewModel skeletons, typed score components, watch/report shapes, and
  contract fixture tests started in `icelines-core::view_model::poach`.

---

## Selke.3 - Feature extraction

Initial data inputs:

- current season stats;
- recent 3/5/10-game splits where available;
- schedule next 7/14 days;
- transactions/injury/scratch hints where available;
- groups/favorites/watchlists;
- scoring scheme;
- optional deployment data.

Shifts/lines and deployment:

- If public/available line and shift data exists locally, use it.
- If not, model line/PP status as `Unknown` and rely on TOI/recent usage until
  a later data-source phase adds true shift/line ingestion.
- All deployment-derived fields carry `DeploymentSignal`.
- Watch rules can be based on actual signals, estimated signals, or manual
  notes, but the UI/report must label which kind fired.

Acceptance:

- Missing shifts/lines never become false negatives.
- The board distinguishes "no line data" from "not promoted."
- Score explanations identify actual vs estimated vs unknown deployment.

Initial implementation note:

- `PoachBoardView::from_repository` performs the first deterministic extraction
  pass from current skater stats: category fit, estimated opportunity,
  estimated deployment proxy, risk discount, typed source gaps, query filters,
  and contract explanations.

---

## Selke.4 - CLI commands

Implement:

- `icelines poach`
- `icelines watch`
- `icelines report poach`
- `icelines report weekly`

Acceptance:

- `--json` output is clean and scriptable.
- Markdown report uses the same `PoachReportView`.
- Bad player/team/category errors use the shared error catalog.

Initial implementation note:

- `icelines poach` is wired as the first CLI surface over `PoachBoardView`.
  It supports season/type, scheme, category, team, position, top, text output,
  and full ViewModel JSON. `watch` and report commands remain pending.
- `icelines report poach` is wired as the first report surface over
  `PoachReportView`, with markdown output by default and full report JSON via
  `--json`. `watch` and weekly reports remain pending.
- `icelines watch rules`, `icelines watch player`, and
  `icelines watch deployment` are wired as preview surfaces over
  `WatchRulesView`. Persistence and fired-alert history remain pending.
- `icelines report weekly` is wired as a deterministic weekly prep report over
  `PoachReportView`, with top adds, category specialists, deployment risers,
  risk discounts, and watched-player-alert sections. Schedule/import/watch
  alert sections disclose source gaps until those inputs are persistent.

---

## Selke.5 - TUI Poach board

Acceptance:

- A user can see top adds, streamers, stashes, and risks without opening a
  detail pane.
- Detail pane explains score components.
- Watch rules can be viewed and toggled.

---

## Selke.6 - Web poach board

Acceptance:

- `/poach` and `/api/v1/poach` render the same ViewModel.
- Filters are bookmarkable.
- Empty states explain whether no candidates exist or source data is missing.

---

## Selke.7 - Reports and weekly workflow

Acceptance:

- `icelines report weekly` produces a useful fantasy-prep document:
  category needs, schedule edges, streamers, stashes, watched-player alerts,
  and add/drop candidates.
- Report generation is deterministic for a fixed fixture.

---

## Selke.8 - Docs, pitfalls, invariants, closeout

Update:

- `COMMANDS.md`
- `README.md`
- `design/specs/fantasy-poacher.md`
- `design/specs/surface-parity.md`
- `design/specs/viewmodels.md`
- `design/PITFALLS.md`
- `design/INVARIANTS.md`
- `design/plans/INDEX.md`
- `design/phases.md`

Acceptance:

- New pitfalls cover magic-score overconfidence, missing line data, stale
  availability, and schedule overfitting.
- New invariants cover explanation presence, context keys, deterministic scores,
  and missing-source behavior.

---

## Out of scope

- Betting, odds, or win probability.
- Fully automated add/drop execution in a fantasy platform.
- Scraping private fantasy sites without an explicit import/export boundary.
- Treating unverified ownership data as truth.
- True shift/line ingestion if no supported source is available in this phase.
