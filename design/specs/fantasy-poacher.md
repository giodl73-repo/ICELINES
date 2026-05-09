# Fantasy Poacher

**Status**: Draft - Phase Selke owns implementation
**Owner**: Phase Selke - fantasy poacher
**Primary contracts**: `platform-contracts.md`, `viewmodels.md`,
`surface-parity.md`, `fantasy-scheme.md`, `group-management.md`

The fantasy poacher turns IceLines from an analysis tool into a weekly decision
tool: who should a fantasy manager add, stream, stash, watch, or avoid before
the league notices?

This spec defines the scoring model, query contract, ViewModels, report shape,
and failure rules for that product surface.

---

## Product Questions

The poacher must answer:

- Who is becoming useful before their season totals show it?
- Who helps a specific scoring scheme or category need?
- Who has schedule value in the next 7 or 14 days?
- Who is available, watched, under-rostered, or manually marked as a target?
- Why is the recommendation credible, risky, stale, or data-limited?

The first implementation does not need live fantasy-platform ownership. It must
work with local groups/watchlists, roster imports when present, scoring schemes,
schedule data, stats, game logs when available, and explicit source-state labels.

---

## Query Contract

All surfaces lower into one typed query:

```text
PoachQuery {
  season,
  season_type,
  scoring_scheme,
  window_days,
  categories,
  positions,
  teams,
  availability_filter,
  candidate_kind,
  schedule_filter,
  min_confidence,
  limit,
  sort
}
```

Required behavior:

- `window_days` is `7` or `14` initially; report workflows may also request a
  named fantasy playoff week once schedule support exists.
- `candidate_kind` is one of `all`, `streamer`, `stash`, `category_specialist`,
  `deployment_riser`, `goalie_streamer`, or `watch_alert`.
- `availability_filter` is one of `any`, `available`, `not_on_user_roster`,
  `watched`, `imported_available`, or `unknown`.
- Bad categories, positions, teams, schemes, and sort keys fail through typed
  parser errors shared by CLI, TUI command bar, web params, and reports.
- The cache/query signature includes season, season type, scoring scheme,
  window, filters, source generation when available, and the active watchlist or
  roster-import generation when applicable.

---

## Source State

Poacher recommendations combine sources with uneven reliability. Missing data is
state, not a zero.

| Source | Examples | Missing behavior |
|---|---|---|
| Season stats | goals, assists, shots, hits, blocks, goalie saves | candidate can still rank, confidence may drop |
| Recent usage | 3/5/10-game trend, recent shots, recent TOI proxy | component becomes estimated or unavailable |
| Schedule | next 7/14 games, off-nights, back-to-backs | schedule component unavailable |
| Transactions/injuries | trade, IR, scratch hints | risk component estimated or unavailable |
| Deployment | line, pair, PP, PK, goalie starts | `DeploymentSignal::Unknown` unless actual or estimated |
| Fantasy availability | import, roster, ownership, groups, watchlists | local fallback is groups/watchlists |
| Scoring scheme | active category weights | required; default scheme allowed |

Freshness is surfaced near the recommendation, in reports, and in JSON.

---

## Typed States

```text
DeploymentSignal =
  Actual(source, fetched_at)
  Estimated(proxy, generated_at)
  Unknown

AvailabilityState =
  Available
  RosteredByUser
  Watched
  ImportedRostered
  ImportedAvailable
  Unknown

RecommendationKind =
  Stream
  Stash
  CategoryFit
  ScheduleEdge
  DeploymentRiser
  GoalieStream
  Risk

Confidence =
  High
  Medium
  Low
  DataLimited

ComponentStatus =
  Measured
  Estimated
  Deferred
  Unavailable
```

Rules:

- `Unknown` deployment or ownership never subtracts from score by itself.
- `Estimated` values name the proxy used.
- `Actual` deployment names source and freshness.
- `DataLimited` is a valid confidence, not an error.
- Every row has at least one explanation and one source-state summary.

---

## PoachScore

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

The initial public score is clamped to `0..100`.

| Component | Direction | Range | Weight | Status | Unit/source |
|---|---:|---:|---:|---|---|
| `opportunity_delta` | positive | `0..20` | `20%` | estimated | role/market gap from deployment proxy, rank gap, or manual availability |
| `deployment_trend` | positive | `0..15` | `15%` | estimated | recent TOI, shots, starts, PP/PK proxy, or actual line data if present |
| `category_fit` | positive | `0..25` | `25%` | measured | scoring scheme weights and category z/percentile fit |
| `schedule_value` | positive | `0..15` | `15%` | measured when schedule exists | next 7/14 games, off-nights, back-to-backs |
| `availability_gap` | positive | `0..10` | `10%` | estimated/local | groups/watchlists/imported roster state |
| `roster_need_fit` | positive | `0..15` | `15%` | deferred initially | user's category/position needs when imported |
| `risk_discount` | negative | `0..30` | subtractive | estimated | scratch/injury/stale/small-sample/unstable-role risk |

Clamp rules:

- Sum positives first, clamp to `0..100`, then subtract `risk_discount`, then
  clamp final score to `0..100`.
- Deferred components contribute `0` and must be listed in unavailable or
  deferred explanations.
- Unavailable components contribute `0` and cannot silently lower confidence
  without an explanation.
- Measured and estimated component values are preserved in JSON/report output.

The first implementation may adjust weights only if the spec is updated in the
same commit as the code and fixture expectations.

---

## Explanation Contract

Every `PoachPlayerRow` includes explanation rows:

```text
PoachExplanation {
  component,
  status,
  impact,
  token,
  message,
  source,
  freshness
}
```

Requirements:

- At least one positive reason or watch alert is present for every
  recommendation.
- Risk is explicit; it is never hidden behind the final number.
- Deferred and unavailable sources appear as omissions when they could affect
  interpretation.
- Report prose may summarize explanation rows but may not invent facts absent
  from them.

---

## ViewModels

### `PoachBoardView`

Required fields:

- `context`
- `query`
- `scoring_scheme`
- `window`
- `rows: Vec<PoachPlayerRow>`
- `source_state`
- `warnings`
- `empty_state`
- `confidence_summary`

`PoachPlayerRow` fields:

- `player_id`
- `display_name`
- `team`
- `position`
- `availability`
- `recommendation_kinds`
- `score`
- `confidence`
- component cells
- deployment signal
- schedule summary
- category-fit summary
- risk summary
- explanations
- semantic tokens

### `WatchRulesView`

Required fields:

- `context`
- `rules`
- `enabled_state`
- `last_fired`
- `unsupported_source_warnings`

Initial watch rules:

- player promoted to actual or estimated top-six role;
- player reaches PP1/PP2 if actual data exists;
- player has rising shot volume over recent window;
- goalie projected/estimated to start during a back-to-back;
- watched player becomes locally or imported available;
- category specialist crosses a scheme-specific threshold.

### `PoachReportView`

Required fields:

- report context and generated timestamp;
- scoring scheme/window;
- source state and omissions;
- sections with stable IDs;
- structured rows/recommendations;
- markdown/JSON equivalent content.

Initial sections:

- top adds;
- streamers;
- stashes;
- category specialists;
- goalie streams;
- watched-player alerts;
- avoid/risk notes;
- source omissions.

---

## Surfaces

CLI:

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

TUI:

- Poach board or workspace panel;
- filters for availability, category, position, team, schedule window, kind;
- detail pane with explanations, source state, and risk;
- watch-rule viewer/editor.

Web:

- `/poach`
- `/watchlist`
- `/reports/poach`
- `/api/v1/poach`
- `/api/v1/watch-rules`

Reports:

- waiver poach board;
- weekly add/drop prep;
- category specialist report;
- goalie streaming report;
- deployment risers/fallers;
- watched-player alerts.

---

## Fixtures

Selke fixtures must include:

- rising skater with estimated deployment trend;
- category specialist who is valuable outside points;
- stash with low current score but strong future signal;
- goalie streamer with schedule value and uncertainty;
- player discounted for stale/injury/scratch risk;
- player with missing ownership/import data that is not penalized;
- player with no line data but strong measured category fit;
- empty-result query with a useful empty state.

All fixture scores use a fixed clock and deterministic tiebreakers.

---

## Pitfalls

- Magic-score overconfidence: score shown without enough explanation.
- Missing deployment data treated as negative evidence.
- Stale availability treated as truth.
- Schedule overfitting: too much weight on games without category fit.
- Ownership imports becoming required for default tests.
- Goalie start uncertainty presented as certainty.
- Report prose drifting away from the ViewModel.
- Surface drift where CLI/TUI/web/reports rank different players.

---

## Invariants

- A poach row is invalid without at least one explanation.
- Scores are deterministic for a fixed fixture, clock, query, and source state.
- Unknown deployment/ownership never subtracts from score by itself.
- Every component exposes status, unit/source, clamp, and value.
- Every result is keyed by `(player_id, season, season_type, scoring_scheme,
  window, query_signature)`.
- CLI/TUI/web/report surfaces render the same `PoachBoardView` or
  `PoachReportView`.
- Reports disclose stale, partial, unavailable, and deferred sources near the
  top.

---

## Out Of Scope

- Betting, odds, or win probability.
- Automated add/drop execution.
- Scraping private fantasy platforms without an explicit import/export boundary.
- Treating unverified ownership data as truth.
- Required true shift/line ingestion if no supported source is available.
