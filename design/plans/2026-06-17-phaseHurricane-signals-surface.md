# Phase Hurricane — Ship Signals to a surface + product-gap roadmap

> **Named for the Carolina Hurricanes' 2026 Stanley Cup win.** Trophy/team phase
> naming follows the IceLines convention (Norris, Masterton, Art Ross, Jack Adams).
> Phase Hurricane is the product-analytics push: turn the differentiated **Signals**
> bet into a shipped surface, then close the modern-analytics gaps.

**Created:** 2026-06-17
**Status:** Deliverable 1 SHIPPED (WP-010 pulse-03, 2026-06-18) — `icelines signals`
CLI + `signals.v1` JSON live, L0+L2 green, docs/parity/wave updated. Deliverable 1b
SHIPPED (WP-010 pulse-04, 2026-06-19) — TUI player-card Signals block plus Web
HTML `/player/:id/signals` and Web JSON `/api/v1/player/:id/signals`. Deliverables
2a SHIPPED (2026-06-19) — MoneyPuck-backed `on-ice-xg-for` and
`on-ice-xg-against` catalog reads now surface the already-parsed 5v5 xGF/xGA
columns. Deliverable 2b AUDITED (2026-06-19) — local schema evidence shows every
currently parsed MoneyPuck value column is now either surfaced or used to compute
a surfaced catalog stat; broader deployment surfacing remains blocked on adding
verified CSV schema columns/fixtures. Deliverable 5a SHIPPED (2026-06-19) —
`query player` / `query compare` career arcs now disclose the newest-5 modern
Tier-1 boundary when rendering older bundled historical/skeleton rows.
Deliverable 5b SHIPPED (2026-06-19) — `query leaders --seasons N` text output
now prints the same disclosure when the aggregate window extends beyond the
modern tier. Deliverable 5c SHIPPED (2026-06-19) — the TUI dashboard player
trend stops calling full bundled history "Last 5 seasons" and adds a compact
newest-5-modern / older-skeleton / missing-unavailable disclosure.
Deliverable 3a SHIPPED (2026-06-19) — `PlayerScoringPaceView` now carries
nullable confidence bands for player outlook projected finishes, and the Web
player outlook table renders the range. Deliverable 3b SHIPPED (2026-06-19) —
`icelines project` now emits `PlayerScoringPaceView`-backed pace outlook ranges
for goals, points, and shots in text/JSON/CSV while preserving its existing
projected-points fields. Deliverable 4a SHIPPED (2026-06-19) —
existing goalie advanced workload/quality metrics (`quality_start_pct`,
`shots_against_per_60`) now flow through `GoaliesView` and `query goalies`;
Deliverable 4b SHIPPED (2026-06-19) — Web `/goalies` HTML and
`/api/v1/goalies` JSON expose the same QS% and SA/60 row fields when goalie
advanced data is loaded; GSAx remains blocked on a verified goalie xGA source.
Deliverable 6a SHIPPED
(2026-06-19) — `query player` / `query compare` career arcs render compact CLI
sparklines for Pts/82 and G/82. Deliverable 6b SHIPPED (2026-06-19) —
`export md compare` adds an inline SVG Pts/82 bundled-career trend. Deliverable
6c SHIPPED (2026-06-19) — Web `/compare` adds the same descriptive inline SVG
Pts/82 bundled-career trend for two-player comparisons. Deliverable 6d SHIPPED
(2026-06-19) — `export md leaders` adds an inline SVG top-returned skater Pts/82
bar chart. Deliverable 6e SHIPPED (2026-06-19) — Web `/leaders` adds the same
descriptive inline SVG current-window Pts/82 bar chart for non-empty skater
results. Deliverable 6f SHIPPED (2026-06-19) — Web `/player/:id` adds an inline SVG
Pts/82 bundled-career trend below the player career table. Remaining Deliverable
2c and broader 6 work continue as follow-on pulses.
Deliverable 6g HARDENED (2026-06-19) — TUI dashboard sparkline renderer now
has focused narrow-width evidence for zero- and one-column chart budgets.
Deliverable 6h SHIPPED (2026-06-19) — Web `/team/:abbrev` adds an inline SVG
active-roster skater Pts/82 bar chart while leaving `/api/v1/team/:abbrev`
unchanged.
Deliverable 6i SHIPPED (2026-06-19) — Web `/goalies` adds an inline SVG SV%
bar chart while leaving `/api/v1/goalies` unchanged.
Deliverable 6j SHIPPED (2026-06-19) — Web scoring outlook pages add an inline
SVG 82-game pace bar chart while leaving their JSON routes unchanged.
Deliverable 6k SHIPPED (2026-06-19) — Web records pages add an inline SVG count
bar chart while leaving records JSON routes unchanged.
Deliverable 6l SHIPPED (2026-06-19) — Web poach/weekly report pages add an
inline SVG poach-score bar chart while leaving Poach report ViewModel and board
JSON contracts unchanged.
Deliverable 6m SHIPPED (2026-06-19) — `export md team-season` adds an inline
SVG quality-ledger bar chart over existing report counters.
Deliverable 6n SHIPPED (2026-06-19) — `export md depth` adds an inline SVG
team-strength bar chart over positive `DepthLeagueView` team totals.
Deliverable 6o SHIPPED (2026-06-19) — `export md fantasy` adds an inline SVG
poach-score bar chart over positive `PoachReportView` fantasy report rows.
Deliverable 6p SHIPPED (2026-06-19) — `export md roster` adds an inline SVG
Pts/82 bar chart over positive rendered roster skater rows.
Deliverable 6q SHIPPED (2026-06-19) — `export md team` adds an inline SVG
Pts/82 bar chart over positive rendered target-team skater rows.
Deliverable 6r SHIPPED (2026-06-19) — `export md series` adds an inline SVG
game-margin bar chart over non-tied playoff series game-log rows.
Deliverable 6s SHIPPED (2026-06-19) — TUI playoff series detail adds a compact
game-margin sparkline over bundled playoff series game-log rows.
Deliverable 6t SHIPPED (2026-06-19) — TUI team-season detail adds a compact
goal-differential sparkline over completed non-tied schedule rows.
Deliverable 6u SHIPPED (2026-06-19) — TUI schedule matchup detail adds a
compact margin sparkline over completed non-tied head-to-head rows.
Deliverable 6v SHIPPED (2026-06-19) — TUI game detail adds compact skater
activity bars under each loaded team leader block.
Deliverable 6w SHIPPED (2026-06-19) — TUI player-records detail adds compact
ASCII count bars beside record opponent rows.
Deliverable 6x SHIPPED (2026-06-19) — TUI goalie leaderboard rows add compact
ASCII SV% quality bars beside the printed save percentage.
Deliverable 6y SHIPPED (2026-06-19) — TUI Stats leaders rows add compact ASCII
primary-metric bars beside the printed leader metric.
**Frame:** product evaluation found IceLines is a great *offline/scriptable/fantasy*
tool but is missing the modern public-analytics layer. The highest-leverage,
most-tractable next step was to **ship the already-built Signals metric family to
real surfaces**. WP-010 now has CLI, TUI player-card, and Web player surfaces;
this doc records that deliverable and sketches the rest of the roadmap as
follow-on pulses.

---

## 0. Product-gap roadmap (priority order)

This is the strategic frame. Each item below is a future work package/pulse; only
**Deliverable 1** is fully specified in this doc.

| # | Gap | Why it matters | Tractability |
|---|---|---|---|
| **1** | **Signals were not on user surfaces** | The one *differentiated* metric bet; methodology + ViewModel already built (WP-010). Pulse 03/04 shipped CLI, TUI player-card, and Web player surfaces. | **Done for CLI/TUI/Web; continue only for report/export/cache/catalog.** |
| 2 | MoneyPuck data under-surfaced | Its CSVs already carry on-ice / deployment / shot data we fetch but don't expose. Pulse 2a wires the reserved on-ice xGF/xGA catalog keys to fetched MoneyPuck data; pulse 2b records that no additional MoneyPuck columns are verified locally yet. | Medium — started |
| 3 | No rest-of-season projections w/ confidence | #1 fantasy ask; today pace is descriptive-only. Pulse 3a adds shared player outlook confidence ranges while keeping the copy descriptive; pulse 3b adds the same pace outlook ranges to `icelines project` text/JSON/CSV. | Medium-Large — started |
| 4 | Goalie eval shallow (SV%/GAA); GSAx only emerging | Table stakes for modern goalie analysis. Pulse 4a surfaces existing goalie advanced workload/quality fields (`QS%`, `SA/60`) in `GoaliesView` and CLI; pulse 4b exposes the same fields in Web `/goalies` HTML/JSON while keeping GSAx pending until xGA source work lands. | Medium — started |
| 5 | "38 seasons" reads deeper than it is (~5 modern Tier-1) | Honesty gap; perspective claims over skeleton seasons. Pulses 5a/5b add CLI career-arc and leaders aggregate disclosure; pulse 5c fixes the TUI dashboard bundled-history trend label/disclosure. | Small — started |
| 6 | No visualization (text/tables only) | Loses the "publication-grade" comparison vs HockeyViz/MoneyPuck. Pulse 6a adds compact CLI career-arc sparklines for Pts/82 and G/82; pulse 6b adds an inline SVG Pts/82 trend to `export md compare`; pulse 6c adds the same trend to Web `/compare`; pulse 6d adds a Markdown leaders Pts/82 SVG bar chart; pulse 6e adds the same chart to Web `/leaders`; pulse 6f adds a Web `/player/:id` Pts/82 career SVG below the career table; pulse 6g hardens TUI dashboard sparkline narrow-width evidence; pulse 6h adds a Web `/team/:abbrev` active-roster Pts/82 SVG; pulse 6i adds a Web `/goalies` SV% SVG; pulse 6j adds a Web scoring outlook 82-game pace SVG; pulse 6k adds a Web records count SVG; pulse 6l adds a Web poach/weekly report score SVG; pulse 6m adds a Markdown team-season quality-ledger SVG; pulse 6n adds a Markdown depth team-strength SVG; pulse 6o adds a Markdown fantasy poach-score SVG; pulse 6p adds a Markdown roster Pts/82 SVG; pulse 6q adds a Markdown team Pts/82 SVG; pulse 6r adds a Markdown series game-margin SVG; pulse 6s adds a TUI playoff series game-margin sparkline; pulse 6t adds a TUI team-season goal-differential sparkline; pulse 6u adds a TUI schedule matchup margin sparkline; pulse 6v adds TUI game-detail skater-activity bars; pulse 6w adds TUI player-records count bars; pulse 6x adds TUI goalie SV% quality bars; pulse 6y adds TUI Stats leaders primary-metric bars. | Large — started |

Conceded out of scope (keep conceded): NHL Edge skating speed, shot-location
heatmaps, predictive "value over replacement", salary-cap value. Revisit only as
deliberate scope expansions.

---

## Deliverable 1 — `icelines signals` CLI surface (WP-010 pulse-03/04)

### Goal
A read-only `icelines signals "<player>"` command (text + `--json`) that renders
the existing `PlayerSignalsView` with full evidence/disclosure honesty. This is
the **minimum promotion step**: it surfaces Signals on one surface (+ JSON twin)
**without** promoting them into `StatId`, leaderboards, or the `--filter` catalog
— exactly what `design/specs/icelines-signals.md` §Promotion rule allows when the
gates below are met.

### What already exists (do not rebuild)
- `icelines-core::signal_metrics` — `SignalMetricId::all()` (3 signals), `descriptor()`,
  `evidence(view)`, `read(view) -> Option<f64>`. Units, polarity, methodology,
  limitations, evidence tiers (`Full`/`Partial`/`Missing`) all defined.
- `icelines-core::view_model::signals::PlayerSignalsView::from_player(ctx, &PlayerView)`
  and `PlayerSignalRow` — carries value `Option<f64>`, `evidence_tier`,
  `missing_inputs`, `methodology`, `limitations`, plus view-level `disclosures` and
  `non_claims`. **It is `Serialize`/`Deserialize` → JSON is nearly free.**

### Implementation steps

**Step 0 — wave bookkeeping.** Decide: extend WP-010 with pulse-03 (recommended)
vs new WP. Create `context/waves/2026-06-02-vtrace-wp010-signals/pulses/pulse-03.md`
and add the row to that wave's `WAVE.md` pulse log. Add an entry to
`design/plans/INDEX.md`.

**Step 1 — core read path (icelines-core/icelines-fetch, if any).** None expected:
`PlayerSignalsView::from_player` already takes a `PlayerView`. Confirm a player can
be resolved + loaded the same way `query player` does:
`icelines_fetch::stats_loader::resolve_player_id_by_name` + `load_player_career_into_repo`
(see `icelines-cli/src/commands/query.rs::run_player` ~line 1685 for the exact
pattern to mirror — historical-name fallback included). Keep all compute in core;
the CLI only resolves + renders.

**Step 2 — new command module `icelines-cli/src/commands/signals.rs`.**
- `pub async fn run_signals(args: SignalsArgs) -> anyhow::Result<()>`.
- Resolve player → load → build `PlayerView` for the active `(season, season_type)`
  → `let view = PlayerSignalsView::from_player(ctx, &player);`
- Text render: one row per signal: `short_label`, value formatted to unit
  (`per 60` → 2 dp) **or `—` / "unavailable"** when `value` is `None`, a polarity
  arrow (higher-better ↑ / lower-better ↓ / neutral ·), and the evidence tier.
  Below the table: methodology + limitations footnotes, then the view's
  `disclosures` and `non_claims` lines verbatim. **Never print 0.0 for a missing
  value** (spec §Evidence contract).
- JSON render (`--json`): serialize a frozen `signals.v1` envelope wrapping
  `PlayerSignalsView` (mirror the envelope shape used by `leaders_json_envelope`
  in `commands/query.rs`). Additive-only.
- Honor `--min-gp` (default to the signal's threshold; below-threshold → `None`
  already handled in core, but surface the reason).

**Step 3 — register the command.**
- Add `Signals { player: String, season: Option<u32>, json: bool, min_gp: Option<u32> }`
  to the subcommand enum in `icelines-cli/src/cli.rs` with a `///` `long_about`
  that includes: one example, the unit/polarity legend, and **the non-claim
  sentence** (not a prediction/betting/injury/deployment/coaching tool).
- Dispatch it in `icelines-cli/src/main.rs` to `commands::signals::run_signals`.
- Add `mod signals;` to the commands module.

**Step 4 — tests (required by CLAUDE.md: new command ⇒ L2).**
- **L0** in `commands/signals.rs` (or core if render helpers land there): value
  formatting, `None` → "unavailable" (never 0.0), polarity arrow mapping, evidence
  tier label.
- **L2** in `icelines-cli/tests/` (new `signals_system.rs` or extend an existing
  system test): invoke the compiled binary `signals "Connor McDavid" --json`,
  assert envelope version, 3 rows, and that a known-missing-input case renders
  `null`/unavailable rather than `0`. Use bundled data only — **no live calls.**
- Consider a persona-wave scenario (`persona_wave11.rs` next index) for the
  "blogger checks a player's physical-engagement signal" flow.

**Step 5 — docs + matrix (same change, per CLAUDE.md docs rule).**
- `COMMANDS.md`: add `signals` with examples + the unit/polarity legend.
- `design/specs/surface-parity.md`: add a Signals row (CLI `done`, JSON `done`,
  TUI/Web `planned`).
- `design/specs/icelines-signals.md` §Promotion rule: record that the CLI/JSON
  surface is now live with product copy + disclosure; keep StatId/leaderboard
  promotion explicitly NOT done.
- README: optional one-line mention under the query/analysis section.

### Promotion-rule gate (must all be true before marking pulse done)
From `design/specs/icelines-signals.md`:
- [ ] product-copy review for the CLI surface (labels, methodology, limitations
      reviewed — run the `.roles` panel: **scout** correctness, **wire** schema,
      **bench** tests; `/review-specs`).
- [ ] source/completeness disclosure for unavailable + partial evidence (the
      `—`/"unavailable" + evidence-tier rendering).
- [ ] parity evidence IF >1 surface renders it. Pulse-03 ships **CLI + JSON twin
      from the same ViewModel** → document them as one ViewModel, two encodings.
      Defer TUI/Web to pulse-04 *with* a parity fence.
- [ ] cache-envelope methodology — N/A (not cached this pulse).
- [ ] explicit refusal of predictive/betting/injury/deployment/coaching claims
      (the `non_claims` line is printed and in `long_about`).

### Validation / green-bar checklist
- [ ] `cargo fmt --check` clean
- [ ] `cargo clippy -- -D warnings` clean (note: pre-existing unrelated lint debt
      in `icelines-fetch` / `icelines-web/tests/l1_router.rs` — don't expand it)
- [ ] `cargo test` green incl. new L0/L2
- [ ] `target/release/icelines signals "Connor McDavid"` and `--json` both sane,
      offline
- [ ] commit identity is `giodl73@gmail.com` (verify `git config user.email`)

### Estimated size
~1 new command module (~250 LOC render), enum+dispatch wiring (~20 LOC), ~8 L0 +
~3 L2 tests, doc edits. **One focused session.** No core/algorithm work — the math
and ViewModel are done.

---

## Deliverable 1b (pulse-04) — TUI + Web parity

Status: shipped 2026-06-19.

- TUI: the player card now includes a Signals block rendered from
  `PlayerSignalsView`, with a canonical `icelines signals "<player>"` command
  and `/player/:id/signals` handoff.
- Web: `/player/:id/signals` HTML + `/api/v1/player/:id/signals` JSON render the
  same `PlayerSignalsView`; active-window lookup falls back to the latest loaded
  career row, mirroring the player-card behavior.
- Fences: TUI L0 checks unavailable evidence text and route handoff; Web L0/L1
  checks unavailable/null evidence, route envelope, player-card link, and route
  inventory. Signals still do not enter `StatId`, filters, leaderboards, reports,
  exports, or analytics cache.

---

## Deliverables 2–6 (sketches — separate work packages)

**2. Surface MoneyPuck on-ice/deployment (WP-009 cache or new).** Status:
started. Pulse 2a stores MoneyPuck 5v5 `onIce_xGoalsFor` and
`onIce_xGoalsAgainst` in `AdvancedStats` and wires the existing
`on-ice-xg-for` / `on-ice-xg-against` catalog keys to those values.

Pulse 2b audit: the local parser currently requires `playerId`, `situation`,
`icetime`, `I_F_xGoals`, `onIce_xGoalsFor`, `onIce_xGoalsAgainst`,
`onIce_corsiFor`, `onIce_corsiAgainst`, `onIce_fenwickFor`, and
`onIce_fenwickAgainst`. The non-identity columns are fully accounted for:
`I_F_xGoals` feeds `ixg`/`ixg-per-60`, `onIce_xGoalsFor` and
`onIce_xGoalsAgainst` feed `on-ice-xg-for`/`on-ice-xg-against` and `xgf-pct`,
and the Corsi/Fenwick for/against columns feed `cf-pct` and `ff-pct`.
The repo has no committed MoneyPuck fixture with verified additional deployment
columns; `tests/fixtures/sample_skaters.csv` is a fantasy benchmark fixture, not
a MoneyPuck source fixture. Do not add more MoneyPuck catalog keys until a
checked CSV fixture or pinned schema evidence is committed.

Remaining work: add a verified MoneyPuck schema fixture, then surface any
unused on-ice/deployment columns as catalog stats with `fetch money-puck` gating.
Cheapest path to real on-ice depth; no new data source.

**3. Rest-of-season projections w/ confidence.** Status: started. The existing
`project` command already has pace/regressed/composite modes and confidence
bands. Pulse 3a moves confidence ranges into the shared
`PlayerScoringPaceView` rows used by Web player outlook HTML/JSON, preserving
nullable behavior below the sample floor or when remaining schedule data is
missing. Pulse 3b wires the standalone `project` command to the same
`PlayerScoringPaceView` for additive text/JSON/CSV pace outlook ranges while
preserving its existing projected-points projection output. Remaining work:
decide whether team outlooks need confidence bands.

**4. Goalie GSAx + workload.** Status: started. Pulse 4a ships the available
workload/quality slice: `GoaliesView` now carries goalie advanced
`quality_start_pct` and `shots_against_per_60`, and `query goalies` exposes them
in text, JSON, and CSV as QS% and SA/60. Pulse 4b exposes the same row fields in
Web `/goalies` HTML and `/api/v1/goalies` JSON when goalie advanced data is
loaded. GSAx, high-danger SV%, and richer goalie analytics remain pending until a
verified goalie xGA / danger source is available.

**5. Season-depth honesty.** Status: started. Pulse 5a makes the
`MODERN_BUNDLED_SEASONS` (5) vs `BUNDLED_SEASONS` (38) split explicit on
`query player` and `query compare` career arcs when the rendered row count
extends beyond the modern tier. The disclosure states that older rows are
historical/skeleton season totals and that missing modern fields render
unavailable, not zero. Pulse 5b adds the same disclosure to `query leaders
--seasons N` text output when the aggregate window extends beyond the modern
tier. Pulse 5c changes the TUI dashboard card trend from a fixed "Last 5
seasons" label to a history-aware bundled trend label and adds a compact
newest-5-modern / older-skeleton / missing-unavailable note for long histories.
Remaining work: add the same completeness line to any future seasons-aggregate
outputs that render or summarize data beyond the modern tier.

**6. Minimal visualization.** Status: started. Pulse 6a adds compact
oldest-to-newest CLI career-arc sparklines for Pts/82 and G/82 on multi-season
`query player` / `query compare` output, reusing the native sparkline renderer.
Pulse 6b adds an inline SVG Pts/82 bundled-career trend to `export md compare`
when both players have at least two bundled career seasons. Pulse 6c adds the
same descriptive inline SVG trend to Web `/compare` HTML after the side-by-side
table, using the existing bundled career rows loaded for compare cards and
leaving the JSON route shape unchanged. Pulse 6d adds an inline SVG bar chart to
`export md leaders` showing the top returned skaters by current-window Pts/82.
Pulse 6e adds the same current-window Pts/82 bar chart to Web `/leaders` for
non-empty skater results while leaving `/api/v1/leaders` unchanged. Pulse 6f
adds an inline SVG bundled regular-season Pts/82 trend to Web `/player/:id`
below the career table when the loaded player card has at least two bundled
career rows, leaving `/api/v1/player/:id` unchanged.
Pulse 6g adds focused TUI dashboard sparkline hardening evidence for zero- and
one-column chart budgets so narrow panels degrade without overflow/panic.
Pulse 6h adds an inline SVG bar chart to Web `/team/:abbrev` showing the top
active-roster skaters by current-window Pts/82 while leaving
`/api/v1/team/:abbrev` unchanged.
Pulse 6i adds an inline SVG bar chart to Web `/goalies` showing returned
goalies by current-window SV% while leaving `/api/v1/goalies` unchanged.
Pulse 6j adds an inline SVG bar chart to Web scoring outlook pages showing
returned outlook rows by descriptive 82-game pace while leaving
`/api/v1/player/:id/outlook` and `/api/v1/team/:abbrev/outlook` unchanged.
Pulse 6k adds an inline SVG bar chart to Web records pages showing positive
record rows by count while leaving `/api/v1/records/player/:id` and
`/api/v1/records/team/:abbrev` unchanged.
Pulse 6l adds an inline SVG bar chart to Web poach/weekly report pages showing
top report candidates by descriptive poach score while leaving the shared
`PoachReportView` fields and `/api/v1/poach` board JSON unchanged.
Pulse 6m adds an inline SVG bar chart to `export md team-season` showing
positive quality-ledger counters from the existing team-season report table.
Pulse 6n adds an inline SVG bar chart to `export md depth` showing positive
team-strength totals from the existing `DepthLeagueView` team table.
Pulse 6o adds an inline SVG bar chart to `export md fantasy` showing positive
poach scores from the existing `PoachReportView` report rows.
Pulse 6p adds an inline SVG bar chart to `export md roster` showing positive
Pts/82 rates from the already-rendered roster rows.
Pulse 6q adds an inline SVG bar chart to `export md team` showing positive
Pts/82 rates from the already-rendered target-team skater rows.
Pulse 6r adds an inline SVG bar chart to `export md series` showing nonzero
game margins from the already-rendered playoff series game log.
Pulse 6s adds a compact game-margin sparkline to TUI playoff series detail for
bundled game-log rows with nonzero margins.
Pulse 6t adds a compact goal-differential sparkline to TUI team-season detail
for completed non-tied schedule rows.
Pulse 6u adds a compact margin sparkline to TUI schedule matchup detail for
completed non-tied head-to-head rows.
Pulse 6v adds compact skater-activity bars to TUI game detail under each loaded
team leader block, using cached boxscore skater stats only.
Pulse 6w adds compact ASCII count bars to TUI player-records detail beside each
opponent row, preserving the numeric count as the controlling value.
Pulse 6x adds compact ASCII SV% quality bars to the TUI goalie leaderboard
beside each printed save percentage, preserving SV% as the controlling value.
Pulse 6y adds compact ASCII primary-metric bars to TUI Stats leaders rows,
preserving the printed leader metric as the controlling value.
Remaining work: broader interactive TUI visualization review plus remaining
SVG chart coverage gaps in secondary web/report shapes.

---

## Open decisions for the executor
1. **WP-010 pulse-03 vs new phase name?** Recommend pulse-03 (continuation) — the
   work package already exists and is the right home. Rename to a trophy-phase only
   if it grows past Signals.
2. **`--json` envelope name:** propose `signals.v1`. Confirm against the envelope
   naming in `docs/schemas` / existing `*.v1` envelopes before freezing.
3. **Does pulse-03 include TUI/Web, or just CLI+JSON?** Recommend CLI+JSON only,
   TUI/Web as pulse-04, to keep the parity-evidence clause cheap to satisfy.
